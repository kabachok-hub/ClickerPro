import customtkinter as ctk
import tkinter as tk
from tkinter import filedialog, messagebox
import json
from typing import Callable, Optional, List, Dict, Any
from src.core.macro_engine import MacroRecorder, MacroPlayer, MacroConfig

class MacroTab(ctk.CTkScrollableFrame):
    def __init__(self, parent, on_macro_ready_callback: Optional[Callable[[List[Dict]], None]] = None):
        super().__init__(parent, fg_color="transparent")
        self.on_macro_ready = on_macro_ready_callback
        self.grid_columnconfigure((0, 1), weight=1)

        self.recorder = MacroRecorder()
        self.recorded_actions: List[Dict[str, Any]] = []

        # --- Section 1: Record Controls ---
        self.rec_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.rec_frame.grid(row=0, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.rec_frame.grid_columnconfigure((0, 1, 2, 3), weight=1)

        ctk.CTkLabel(self.rec_frame, text="🎬 Запись макроса в реальном времени", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=4, sticky="w", padx=16, pady=(12, 6))

        self.btn_record = ctk.CTkButton(
            self.rec_frame,
            text="🔴 Начать запись",
            font=("Segoe UI", 12, "bold"),
            height=34,
            corner_radius=8,
            fg_color="#dc2626",
            hover_color="#b91c1c",
            command=self._toggle_recording
        )
        self.btn_record.grid(row=1, column=0, columnspan=2, sticky="ew", padx=16, pady=8)

        self.chk_moves = ctk.CTkCheckBox(self.rec_frame, text="Записывать движение курсора (траекторию)", fg_color="#2563eb", hover_color="#1d4ed8")
        self.chk_moves.grid(row=1, column=2, columnspan=2, sticky="w", padx=16, pady=8)

        self.status_rec_lbl = ctk.CTkLabel(self.rec_frame, text="Готов к записи...", font=("Segoe UI", 11), text_color="#94a3b8")
        self.status_rec_lbl.grid(row=2, column=0, columnspan=4, sticky="w", padx=16, pady=(0, 10))

        # --- Section 2: Actions Viewer & File Ops ---
        self.view_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.view_frame.grid(row=1, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.view_frame.grid_columnconfigure(0, weight=1)

        header_ops = ctk.CTkFrame(self.view_frame, fg_color="transparent")
        header_ops.grid(row=0, column=0, sticky="ew", padx=16, pady=(12, 6))
        header_ops.grid_columnconfigure(0, weight=1)

        ctk.CTkLabel(header_ops, text="📋 Список записанных действий", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, sticky="w")

        btn_box = ctk.CTkFrame(header_ops, fg_color="transparent")
        btn_box.grid(row=0, column=1, sticky="e")

        self.btn_export = ctk.CTkButton(
            btn_box, 
            text="💾 Экспорт JSON", 
            width=110, 
            height=26,
            corner_radius=6,
            fg_color="#334155",
            hover_color="#475569",
            command=self._export_json
        )
        self.btn_export.pack(side="left", padx=3)

        self.btn_import = ctk.CTkButton(
            btn_box, 
            text="📂 Импорт JSON", 
            width=110, 
            height=26,
            corner_radius=6,
            fg_color="#334155",
            hover_color="#475569",
            command=self._import_json
        )
        self.btn_import.pack(side="left", padx=3)

        self.btn_clear = ctk.CTkButton(
            btn_box, 
            text="🗑️ Очистить", 
            width=80, 
            height=26,
            corner_radius=6,
            fg_color="#dc2626",
            hover_color="#b91c1c",
            command=self._clear_actions
        )
        self.btn_clear.pack(side="left", padx=3)

        # Actions Textbox
        self.actions_textbox = ctk.CTkTextbox(self.view_frame, height=130, corner_radius=8)
        self.actions_textbox.grid(row=1, column=0, sticky="ew", padx=16, pady=(4, 12))
        self.actions_textbox.insert("1.0", "[Макрос пуст. Нажмите 'Начать запись' для записи кликов и клавиш]")
        self.actions_textbox.configure(state="disabled")

        # --- Section 3: Playback Settings ---
        self.play_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.play_frame.grid(row=2, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.play_frame.grid_columnconfigure((0, 1, 2, 3), weight=1)

        ctk.CTkLabel(self.play_frame, text="⚡ Настройки воспроизведения", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=4, sticky="w", padx=16, pady=(12, 6))

        # Speed Multiplier
        ctk.CTkLabel(self.play_frame, text="Скорость воспроизведения:").grid(row=1, column=0, sticky="w", padx=16, pady=6)
        self.speed_selector = ctk.CTkOptionMenu(
            self.play_frame,
            values=["0.5x (Замедленно)", "1.0x (Оригинал)", "1.5x", "2.0x (Удвоено)", "5.0x (Быстро)", "10.0x (Максимум)"],
            width=170,
            fg_color="#2563eb",
            button_color="#1d4ed8"
        )
        self.speed_selector.set("1.0x (Оригинал)")
        self.speed_selector.grid(row=1, column=1, sticky="w", padx=4, pady=6)

        # Stop condition
        self.stop_mode_var = ctk.StringVar(value="loops")
        self.radio_loops = ctk.CTkRadioButton(
            self.play_frame, 
            text="Повторить раз:", 
            variable=self.stop_mode_var, 
            value="loops"
        )
        self.radio_loops.grid(row=2, column=0, sticky="w", padx=16, pady=(6, 12))

        self.entry_loops = ctk.CTkEntry(self.play_frame, width=70)
        self.entry_loops.insert(0, "1")
        self.entry_loops.grid(row=2, column=1, sticky="w", padx=4, pady=(6, 12))

        self.radio_inf = ctk.CTkRadioButton(
            self.play_frame, 
            text="Бесконечный цикл", 
            variable=self.stop_mode_var, 
            value="infinite"
        )
        self.radio_inf.grid(row=2, column=2, columnspan=2, sticky="w", padx=16, pady=(6, 12))

    def _toggle_recording(self):
        if not self.recorder.is_recording:
            record_moves = bool(self.chk_moves.get())
            self.recorder.start(record_mouse_moves=record_moves)
            self.btn_record.configure(text="⏹️ Остановить запись", fg_color="#ef4444", hover_color="#dc2626")
            self.status_rec_lbl.configure(text="🔴 ИДЕТ ЗАПИСЬ... Выполняйте действия на экране", text_color="#f87171")
        else:
            actions = self.recorder.stop()
            self.recorded_actions = actions
            self.btn_record.configure(text="🔴 Начать запись", fg_color="#dc2626", hover_color="#b91c1c")
            self.status_rec_lbl.configure(text=f"Запись завершена. Записано действий: {len(actions)}", text_color="#34d399")
            self._update_actions_view()
            if self.on_macro_ready:
                self.on_macro_ready(self.recorded_actions)

    def _update_actions_view(self):
        self.actions_textbox.configure(state="normal")
        self.actions_textbox.delete("1.0", "end")
        
        if not self.recorded_actions:
            self.actions_textbox.insert("1.0", "[Макрос пуст. Нажмите 'Начать запись']")
        else:
            lines = []
            for i, act in enumerate(self.recorded_actions[:500]):
                t = act.get("type", "")
                d = act.get("delay_ms", 0)
                if "mouse" in t:
                    lines.append(f"#{i+1:03d} [{t.upper()}] Btn/Dir: {act.get('button', act.get('dy', ''))} (X:{act.get('x',0)}, Y:{act.get('y',0)}) | Пауза: {d}мс")
                elif "key" in t:
                    lines.append(f"#{i+1:03d} [{t.upper()}] Клавиша: '{act.get('key', '')}' | Пауза: {d}мс")
            
            if len(self.recorded_actions) > 500:
                lines.append(f"... и еще {len(self.recorded_actions) - 500} действий.")
            self.actions_textbox.insert("1.0", "\n".join(lines))
            
        self.actions_textbox.configure(state="disabled")

    def _clear_actions(self):
        self.recorded_actions = []
        self._update_actions_view()
        self.status_rec_lbl.configure(text="Макрос очищен.", text_color="#94a3b8")

    def _export_json(self):
        if not self.recorded_actions:
            messagebox.showinfo("Инфо", "Нет записанных действий для экспорта.")
            return
        path = filedialog.asksaveasfilename(defaultextension=".json", filetypes=[("JSON Files", "*.json")])
        if path:
            try:
                with open(path, "w", encoding="utf-8") as f:
                    json.dump(self.recorded_actions, f, indent=2, ensure_ascii=False)
                messagebox.showinfo("Успешно", f"Макрос сохранен в:\n{path}")
            except Exception as e:
                messagebox.showerror("Ошибка", str(e))

    def _import_json(self):
        path = filedialog.askopenfilename(filetypes=[("JSON Files", "*.json")])
        if path:
            try:
                with open(path, "r", encoding="utf-8") as f:
                    self.recorded_actions = json.load(f)
                self._update_actions_view()
                self.status_rec_lbl.configure(text=f"Загружен макрос: {len(self.recorded_actions)} действий", text_color="#34d399")
                if self.on_macro_ready:
                    self.on_macro_ready(self.recorded_actions)
            except Exception as e:
                messagebox.showerror("Ошибка", str(e))

    def get_config(self) -> MacroConfig:
        cfg = MacroConfig()
        cfg.actions = list(self.recorded_actions)
        
        speed_txt = self.speed_selector.get()
        if "0.5x" in speed_txt: cfg.speed_multiplier = 0.5
        elif "1.5x" in speed_txt: cfg.speed_multiplier = 1.5
        elif "2.0x" in speed_txt: cfg.speed_multiplier = 2.0
        elif "5.0x" in speed_txt: cfg.speed_multiplier = 5.0
        elif "10.0x" in speed_txt: cfg.speed_multiplier = 10.0
        else: cfg.speed_multiplier = 1.0

        cfg.stop_condition = self.stop_mode_var.get()
        try:
            cfg.repeat_loops = int(self.entry_loops.get() or 1)
        except Exception:
            cfg.repeat_loops = 1

        return cfg

    def set_config(self, cfg_dict: dict):
        if "actions" in cfg_dict:
            self.recorded_actions = list(cfg_dict["actions"])
            self._update_actions_view()
        if "speed_multiplier" in cfg_dict:
            sm = cfg_dict["speed_multiplier"]
            if sm == 0.5: self.speed_selector.set("0.5x (Замедленно)")
            elif sm == 2.0: self.speed_selector.set("2.0x (Удвоено)")
            elif sm == 5.0: self.speed_selector.set("5.0x (Быстро)")
            elif sm == 10.0: self.speed_selector.set("10.0x (Максимум)")
            else: self.speed_selector.set("1.0x (Оригинал)")
        if "repeat_loops" in cfg_dict:
            self.entry_loops.delete(0, "end")
            self.entry_loops.insert(0, str(cfg_dict["repeat_loops"]))
