import customtkinter as ctk
import tkinter as tk
from typing import Callable, Optional, List
from pynput import keyboard
from src.core.keyboard_engine import KeyboardConfig

class KeyboardTab(ctk.CTkScrollableFrame):
    def __init__(self, parent, on_config_changed: Optional[Callable[[], None]] = None):
        super().__init__(parent, fg_color="transparent")
        self.on_config_changed = on_config_changed
        self.grid_columnconfigure((0, 1), weight=1)

        # Single key selection state
        self.selected_key = "space"
        self.is_listening_key = False
        self._key_listener: Optional[keyboard.Listener] = None

        # Combo selection state
        self.selected_combo_keys: List[str] = ["ctrl", "v"]
        self.is_listening_combo = False
        self._combo_listener: Optional[keyboard.Listener] = None
        self._combo_active_keys: set = set()
        self._combo_collected: list = []

        # --- Section 1: Mode Selector ---
        self.mode_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.mode_frame.grid(row=0, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.mode_frame.grid_columnconfigure(0, weight=1)

        ctk.CTkLabel(self.mode_frame, text="⌨️ Режим клавиатурного кликера", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, sticky="w", padx=16, pady=(12, 6))

        self.kb_mode_var = ctk.StringVar(value="single")
        self.mode_segmented = ctk.CTkSegmentedButton(
            self.mode_frame,
            values=["Одиночная клавиша", "Комбинация (Combo)", "Последовательность", "Авто-печать текста"],
            selected_color="#2563eb",
            selected_hover_color="#1d4ed8",
            command=self._on_mode_change
        )
        self.mode_segmented.set("Одиночная клавиша")
        self.mode_segmented.grid(row=1, column=0, sticky="ew", padx=16, pady=(4, 14))

        # --- Section 2: Mode Specific Panels ---
        self.content_container = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.content_container.grid(row=1, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.content_container.grid_columnconfigure((0, 1, 2, 3), weight=1)

        # 2.1 Single Key Panel
        self.panel_single = ctk.CTkFrame(self.content_container, fg_color="transparent")
        self.panel_single.grid_columnconfigure((0, 1, 2, 3), weight=1)
        
        ctk.CTkLabel(self.panel_single, text="Клавиша для спама:", font=("Segoe UI", 11)).grid(row=0, column=0, sticky="w", padx=16, pady=8)
        
        # Interactive Key Selection Button
        self.single_key_btn = ctk.CTkButton(
            self.panel_single,
            text="⌨️ [ SPACE ]",
            font=("Segoe UI", 12, "bold"),
            width=180,
            height=34,
            corner_radius=8,
            fg_color="#2563eb",
            hover_color="#1d4ed8",
            command=self._start_single_key_capture
        )
        self.single_key_btn.grid(row=0, column=1, sticky="w", padx=4, pady=8)

        ctk.CTkLabel(self.panel_single, text="Действие:", font=("Segoe UI", 11)).grid(row=0, column=2, sticky="w", padx=16, pady=8)
        self.single_action_selector = ctk.CTkOptionMenu(
            self.panel_single,
            values=["Нажатие (Tap)", "Удержание (Hold)"],
            width=140,
            fg_color="#334155",
            button_color="#475569"
        )
        self.single_action_selector.set("Нажатие (Tap)")
        self.single_action_selector.grid(row=0, column=3, sticky="w", padx=4, pady=8)

        # Hint below single key button
        self.key_hint_lbl = ctk.CTkLabel(
            self.panel_single,
            text="Нажмите на кнопку и нажмите любую клавишу на клавиатуре для быстрого выбора",
            font=("Segoe UI", 10),
            text_color="#94a3b8"
        )
        self.key_hint_lbl.grid(row=1, column=0, columnspan=4, sticky="w", padx=16, pady=(0, 8))

        # 2.2 Combo Panel
        self.panel_combo = ctk.CTkFrame(self.content_container, fg_color="transparent")
        self.panel_combo.grid_columnconfigure((0, 1, 2), weight=1)
        
        ctk.CTkLabel(self.panel_combo, text="Комбинация клавиш:", font=("Segoe UI", 11)).grid(row=0, column=0, sticky="w", padx=16, pady=8)
        
        # Interactive Combo Selection Button
        self.combo_btn = ctk.CTkButton(
            self.panel_combo,
            text="⌨️ [ CTRL + V ]",
            font=("Segoe UI", 12, "bold"),
            width=220,
            height=34,
            corner_radius=8,
            fg_color="#2563eb",
            hover_color="#1d4ed8",
            command=self._start_combo_capture
        )
        self.combo_btn.grid(row=0, column=1, sticky="w", padx=4, pady=8)

        # Hint below combo button
        self.combo_hint_lbl = ctk.CTkLabel(
            self.panel_combo,
            text="Нажмите на кнопку и зажмите комбинацию на клавиатуре (например Ctrl+V, Alt+Tab)",
            font=("Segoe UI", 10),
            text_color="#94a3b8"
        )
        self.combo_hint_lbl.grid(row=1, column=0, columnspan=3, sticky="w", padx=16, pady=(0, 8))

        # 2.3 Sequence Panel (Skills rotation)
        self.panel_seq = ctk.CTkFrame(self.content_container, fg_color="transparent")
        self.panel_seq.grid_columnconfigure((0, 1, 2, 3), weight=1)

        ctk.CTkLabel(self.panel_seq, text="Шаги последовательности (Ротация скиллов):", font=("Segoe UI", 11, "bold")).grid(row=0, column=0, columnspan=4, sticky="w", padx=16, pady=(8, 4))
        
        self.seq_textbox = ctk.CTkTextbox(self.panel_seq, height=90, width=400, corner_radius=8)
        self.seq_textbox.insert("1.0", "1 : 150\n2 : 150\n3 : 150\ne : 200")
        self.seq_textbox.grid(row=1, column=0, columnspan=4, sticky="ew", padx=16, pady=4)
        
        ctk.CTkLabel(self.panel_seq, text="Формат: [Клавиша] : [Задержка в мс после нажатия]", font=("Segoe UI", 10), text_color="#94a3b8").grid(row=2, column=0, columnspan=4, sticky="w", padx=16, pady=(0, 8))

        # 2.4 Text Typer Panel
        self.panel_text = ctk.CTkFrame(self.content_container, fg_color="transparent")
        self.panel_text.grid_columnconfigure((0, 1), weight=1)

        ctk.CTkLabel(self.panel_text, text="Текст для автоматического ввода:", font=("Segoe UI", 11)).grid(row=0, column=0, sticky="w", padx=16, pady=4)
        self.text_entry = ctk.CTkEntry(self.panel_text, width=320)
        self.text_entry.insert(0, "GG WP! Auto Clicker Active!")
        self.text_entry.grid(row=1, column=0, sticky="ew", padx=16, pady=4)

        ctk.CTkLabel(self.panel_text, text="Задержка между буквами (мс):", font=("Segoe UI", 11)).grid(row=2, column=0, sticky="w", padx=16, pady=4)
        self.char_delay_entry = ctk.CTkEntry(self.panel_text, width=70)
        self.char_delay_entry.insert(0, "20")
        self.char_delay_entry.grid(row=2, column=1, sticky="w", padx=4, pady=(4, 8))

        # Show initial panel
        self._show_panel("single")

        # --- Section 3: Speed & Interval ---
        self.speed_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.speed_frame.grid(row=2, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.speed_frame.grid_columnconfigure((0, 1, 2, 3), weight=1)

        ctk.CTkLabel(self.speed_frame, text="⏱️ Интервалы и задержки", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=4, sticky="w", padx=16, pady=(12, 6))

        ctk.CTkLabel(self.speed_frame, text="Интервал повтора цикла (мс):", font=("Segoe UI", 11)).grid(row=1, column=0, sticky="w", padx=16, pady=(6, 12))
        self.interval_entry = ctk.CTkEntry(self.speed_frame, width=90)
        self.interval_entry.insert(0, "100")
        self.interval_entry.grid(row=1, column=1, sticky="w", padx=4, pady=(6, 12))

        ctk.CTkLabel(self.speed_frame, text="🎲 Случайный джиттер (± мс):", font=("Segoe UI", 11)).grid(row=1, column=2, sticky="w", padx=16, pady=(6, 12))
        self.jitter_entry = ctk.CTkEntry(self.speed_frame, width=90)
        self.jitter_entry.insert(0, "0")
        self.jitter_entry.grid(row=1, column=3, sticky="w", padx=4, pady=(6, 12))

        # --- Section 4: Stop Condition ---
        self.stop_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.stop_frame.grid(row=3, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.stop_frame.grid_columnconfigure((0, 1, 2, 3), weight=1)

        ctk.CTkLabel(self.stop_frame, text="🛑 Условия остановки", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=4, sticky="w", padx=16, pady=(12, 6))

        self.stop_mode_var = ctk.StringVar(value="infinite")

        self.radio_inf = ctk.CTkRadioButton(
            self.stop_frame, 
            text="Бесконечно (до нажатия F6)", 
            variable=self.stop_mode_var, 
            value="infinite"
        )
        self.radio_inf.grid(row=1, column=0, columnspan=2, sticky="w", padx=16, pady=6)

        self.radio_count = ctk.CTkRadioButton(
            self.stop_frame, 
            text="По числу повторов:", 
            variable=self.stop_mode_var, 
            value="count"
        )
        self.radio_count.grid(row=2, column=0, sticky="w", padx=16, pady=(6, 12))

        self.entry_count = ctk.CTkEntry(self.stop_frame, width=90)
        self.entry_count.insert(0, "50")
        self.entry_count.grid(row=2, column=1, sticky="w", padx=4, pady=(6, 12))

        self.radio_time = ctk.CTkRadioButton(
            self.stop_frame, 
            text="По таймеру (сек):", 
            variable=self.stop_mode_var, 
            value="time"
        )
        self.radio_time.grid(row=2, column=2, sticky="w", padx=16, pady=(6, 12))

        self.entry_timer = ctk.CTkEntry(self.stop_frame, width=90)
        self.entry_timer.insert(0, "30")
        self.entry_timer.grid(row=2, column=3, sticky="w", padx=4, pady=(6, 12))

    def _normalize_key_name(self, key) -> str:
        try:
            if hasattr(key, 'char') and key.char:
                return key.char.lower()
            if hasattr(key, 'name') and key.name:
                name = key.name.lower()
                if name in ("ctrl_l", "ctrl_r"): return "ctrl"
                if name in ("alt_l", "alt_r", "alt_gr"): return "alt"
                if name in ("shift_l", "shift_r"): return "shift"
                if name in ("cmd", "cmd_l", "cmd_r", "win"): return "win"
                return name
            s = str(key).replace("Key.", "").lower()
            if s in ("ctrl_l", "ctrl_r"): return "ctrl"
            if s in ("alt_l", "alt_r", "alt_gr"): return "alt"
            if s in ("shift_l", "shift_r"): return "shift"
            return s
        except Exception:
            return str(key).lower()

    # --- Single Key Capture ---
    def _start_single_key_capture(self):
        if self.is_listening_key or self.is_listening_combo:
            return
        
        self.is_listening_key = True
        self.single_key_btn.configure(
            text="👉 Нажмите клавишу...",
            fg_color="#ef4444",
            hover_color="#dc2626"
        )
        self.key_hint_lbl.configure(
            text="🔴 Нажмите любую клавишу на клавиатуре прямо сейчас...",
            text_color="#f87171"
        )

        def on_press(key):
            k_str = self._normalize_key_name(key)
            self.after(0, lambda: self._on_single_key_captured(k_str))
            return False

        self._key_listener = keyboard.Listener(on_press=on_press)
        self._key_listener.start()

    def _on_single_key_captured(self, k_str: str):
        self.is_listening_key = False
        self.selected_key = k_str
        self.single_key_btn.configure(
            text=f"⌨️ [ {k_str.upper()} ]",
            fg_color="#2563eb",
            hover_color="#1d4ed8"
        )
        self.key_hint_lbl.configure(
            text=f"✅ Выбрана клавиша: '{k_str.upper()}'. Нажмите кнопку снова для смены",
            text_color="#34d399"
        )
        if self._key_listener:
            try:
                self._key_listener.stop()
            except Exception:
                pass
            self._key_listener = None

    # --- Combo Capture ---
    def _start_combo_capture(self):
        if self.is_listening_combo or self.is_listening_key:
            return
        
        self.is_listening_combo = True
        self._combo_active_keys.clear()
        self._combo_collected.clear()

        self.combo_btn.configure(
            text="👉 Зажмите комбинацию...",
            fg_color="#ef4444",
            hover_color="#dc2626"
        )
        self.combo_hint_lbl.configure(
            text="🔴 Зажмите нужные клавиши вместе (например, Ctrl+V) и отпустите их...",
            text_color="#f87171"
        )

        def on_press(key):
            k_str = self._normalize_key_name(key)
            self._combo_active_keys.add(k_str)
            if k_str not in self._combo_collected:
                self._combo_collected.append(k_str)
            
            preview = " + ".join([k.upper() for k in self._combo_collected])
            self.after(0, lambda p=preview: self.combo_btn.configure(text=f"👉 [ {p} ]"))

        def on_release(key):
            k_str = self._normalize_key_name(key)
            self._combo_active_keys.discard(k_str)

            if len(self._combo_active_keys) == 0 and len(self._combo_collected) > 0:
                result = list(self._combo_collected)
                self.after(0, lambda r=result: self._on_combo_captured(r))
                return False

        self._combo_listener = keyboard.Listener(on_press=on_press, on_release=on_release)
        self._combo_listener.start()

    def _on_combo_captured(self, combo_keys: List[str]):
        self.is_listening_combo = False
        self.selected_combo_keys = combo_keys
        combo_str = " + ".join([k.upper() for k in combo_keys])
        
        self.combo_btn.configure(
            text=f"⌨️ [ {combo_str} ]",
            fg_color="#2563eb",
            hover_color="#1d4ed8"
        )
        self.combo_hint_lbl.configure(
            text=f"✅ Выбрана комбинация: '{combo_str}'. Нажмите кнопку снова для смены",
            text_color="#34d399"
        )
        if self._combo_listener:
            try:
                self._combo_listener.stop()
            except Exception:
                pass
            self._combo_listener = None

    def _on_mode_change(self, val):
        if "Одиночная" in val:
            self._show_panel("single")
        elif "Комбинация" in val:
            self._show_panel("combo")
        elif "Последовательность" in val:
            self._show_panel("sequence")
        elif "Авто-печать" in val:
            self._show_panel("text")

    def _show_panel(self, mode_name: str):
        self.panel_single.grid_forget()
        self.panel_combo.grid_forget()
        self.panel_seq.grid_forget()
        self.panel_text.grid_forget()

        if mode_name == "single":
            self.panel_single.grid(row=0, column=0, columnspan=4, sticky="ew", padx=6, pady=6)
        elif mode_name == "combo":
            self.panel_combo.grid(row=0, column=0, columnspan=4, sticky="ew", padx=6, pady=6)
        elif mode_name == "sequence":
            self.panel_seq.grid(row=0, column=0, columnspan=4, sticky="ew", padx=6, pady=6)
        elif mode_name == "text":
            self.panel_text.grid(row=0, column=0, columnspan=4, sticky="ew", padx=6, pady=6)

    def get_config(self) -> KeyboardConfig:
        cfg = KeyboardConfig()
        
        mode_val = self.mode_segmented.get()
        if "Одиночная" in mode_val:
            cfg.mode = "single"
        elif "Комбинация" in mode_val:
            cfg.mode = "combo"
        elif "Последовательность" in mode_val:
            cfg.mode = "sequence"
        elif "Авто-печать" in mode_val:
            cfg.mode = "text"

        cfg.key = self.selected_key.strip() or "space"
        action_txt = self.single_action_selector.get()
        cfg.action = "hold" if "Удержание" in action_txt else "tap"

        cfg.combo_keys = list(self.selected_combo_keys) if self.selected_combo_keys else ["ctrl", "v"]

        # Parse sequence
        raw_seq = self.seq_textbox.get("1.0", "end").strip().splitlines()
        seq_steps = []
        for line in raw_seq:
            if ":" in line:
                parts = line.split(":", 1)
                k = parts[0].strip()
                try:
                    d = float(parts[1].strip())
                except Exception:
                    d = 100.0
                if k:
                    seq_steps.append({"key": k, "delay_ms": d})
        cfg.sequence_steps = seq_steps

        cfg.text_content = self.text_entry.get()
        try:
            cfg.char_delay_ms = float(self.char_delay_entry.get() or 20)
        except Exception:
            cfg.char_delay_ms = 20.0

        try:
            cfg.interval_ms = float(self.interval_entry.get() or 100)
        except Exception:
            cfg.interval_ms = 100.0

        try:
            cfg.random_delay_ms = float(self.jitter_entry.get() or 0)
        except Exception:
            cfg.random_delay_ms = 0.0

        cfg.stop_condition = self.stop_mode_var.get()
        try:
            cfg.repeat_count = int(self.entry_count.get() or 50)
        except Exception:
            cfg.repeat_count = 50

        try:
            cfg.max_time_seconds = float(self.entry_timer.get() or 30)
        except Exception:
            cfg.max_time_seconds = 30.0

        return cfg

    def set_config(self, cfg_dict: dict):
        if "mode" in cfg_dict:
            m = cfg_dict["mode"]
            if m == "single":
                self.mode_segmented.set("Одиночная клавиша")
                self._show_panel("single")
            elif m == "combo":
                self.mode_segmented.set("Комбинация (Combo)")
                self._show_panel("combo")
            elif m == "sequence":
                self.mode_segmented.set("Последовательность")
                self._show_panel("sequence")
            elif m == "text":
                self.mode_segmented.set("Авто-печать текста")
                self._show_panel("text")

        if "key" in cfg_dict:
            self.selected_key = str(cfg_dict["key"]).lower()
            self.single_key_btn.configure(text=f"⌨️ [ {self.selected_key.upper()} ]")
        if "combo_keys" in cfg_dict:
            self.selected_combo_keys = [str(k).lower() for k in cfg_dict["combo_keys"]]
            combo_str = " + ".join([k.upper() for k in self.selected_combo_keys])
            self.combo_btn.configure(text=f"⌨️ [ {combo_str} ]")
        if "sequence_steps" in cfg_dict:
            lines = [f"{s.get('key', '')} : {s.get('delay_ms', 100)}" for s in cfg_dict["sequence_steps"]]
            self.seq_textbox.delete("1.0", "end")
            self.seq_textbox.insert("1.0", "\n".join(lines))
        if "text_content" in cfg_dict:
            self.text_entry.delete(0, "end")
            self.text_entry.insert(0, str(cfg_dict["text_content"]))
        if "interval_ms" in cfg_dict:
            self.interval_entry.delete(0, "end")
            self.interval_entry.insert(0, str(cfg_dict["interval_ms"]))
        if "random_delay_ms" in cfg_dict:
            self.jitter_entry.delete(0, "end")
            self.jitter_entry.insert(0, str(cfg_dict["random_delay_ms"]))
