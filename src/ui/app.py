import tkinter as tk
import customtkinter as ctk
from typing import Optional

from src.core.mouse_engine import MouseEngine
from src.core.keyboard_engine import KeyboardEngine
from src.core.scroll_engine import ScrollEngine
from src.core.multipoint_engine import MultiPointEngine
from src.core.macro_engine import MacroPlayer
from src.core.hotkey_manager import HotkeyManager, play_audio_feedback
from src.core.profile_manager import ProfileManager
from src.core.precision_timer import cleanup_timer

from src.ui.mouse_tab import MouseTab
from src.ui.keyboard_tab import KeyboardTab
from src.ui.scroll_tab import ScrollTab
from src.ui.multipoint_tab import MultiPointTab
from src.ui.macro_tab import MacroTab
from src.ui.settings_tab import SettingsTab
from src.ui.overlay_window import FloatingOverlay
from src.ui.coordinate_picker import CoordinatePicker

ctk.set_appearance_mode("dark")
ctk.set_default_color_theme("blue")

class UltimateClickerApp(ctk.CTk):
    def __init__(self):
        super().__init__()

        self.title("⚡ Автокликер Pro — Умный кликер мыши и клавиатуры")
        self.geometry("900x670")
        self.minsize(840, 600)
        self.configure(fg_color="#12151c")

        # Set Icon
        import os
        icon_path = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), "assets", "icon.ico")
        if os.path.exists(icon_path):
            try:
                self.iconbitmap(icon_path)
            except Exception:
                pass

        # Profile Manager
        self.profile_mgr = ProfileManager()

        # Engines
        self.mouse_engine = MouseEngine(on_update_callback=self._on_engine_update)
        self.kb_engine = KeyboardEngine(on_update_callback=self._on_engine_update)
        self.scroll_engine = ScrollEngine(on_update_callback=self._on_engine_update)
        self.multipoint_engine = MultiPointEngine(on_update_callback=self._on_engine_update)
        self.macro_player = MacroPlayer(on_update_callback=self._on_macro_update)

        # State
        self.active_tab_name = "mouse"
        self.is_running = False
        self.live_cps = 0.0
        self.total_count = 0

        # Hotkeys
        self.hotkey_mgr = HotkeyManager()
        self.hotkey_mgr.register_callback("start_stop", self._safe_toggle_active_engine)
        self.hotkey_mgr.register_callback("emergency_stop", self._safe_stop_all)
        self.hotkey_mgr.register_callback("pick_coord", self._trigger_coord_picker)
        self.hotkey_mgr.start()

        # Layout
        self.grid_columnconfigure(1, weight=1)
        self.grid_rowconfigure(0, weight=1)

        self._build_sidebar()
        self._build_content_area()
        self._build_bottom_bar()

        # Floating HUD Overlay
        self.overlay_hud = FloatingOverlay(self, on_toggle_callback=self._safe_toggle_active_engine)

        # Show default tab
        self._select_tab("mouse")

        # Window closing handler (Silent, no sound)
        self.protocol("WM_DELETE_WINDOW", self._on_close)

    def _build_sidebar(self):
        self.sidebar_frame = ctk.CTkFrame(self, width=220, corner_radius=0, fg_color="#181b24")
        self.sidebar_frame.grid(row=0, column=0, rowspan=2, sticky="nsew")
        self.sidebar_frame.grid_rowconfigure(8, weight=1)

        # App Title & Brand
        brand_frame = ctk.CTkFrame(self.sidebar_frame, fg_color="transparent")
        brand_frame.grid(row=0, column=0, padx=14, pady=(18, 14), sticky="ew")

        from PIL import Image
        import os
        avatar_path = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), "assets", "icon.png")
        if os.path.exists(avatar_path):
            try:
                pil_img = Image.open(avatar_path)
                ctk_img = ctk.CTkImage(light_image=pil_img, dark_image=pil_img, size=(38, 38))
                img_lbl = ctk.CTkLabel(brand_frame, image=ctk_img, text="")
                img_lbl.pack(side="left", padx=(0, 10))
            except Exception:
                pass

        text_brand = ctk.CTkFrame(brand_frame, fg_color="transparent")
        text_brand.pack(side="left", fill="both")

        logo_label = ctk.CTkLabel(
            text_brand, 
            text="⚡ АВТОКЛИКЕР", 
            font=("Segoe UI", 15, "bold"),
            text_color="#38bdf8"
        )
        logo_label.pack(anchor="w")

        sub_label = ctk.CTkLabel(
            text_brand,
            text="Панель автоматизации",
            font=("Segoe UI", 10),
            text_color="#64748b"
        )
        sub_label.pack(anchor="w")

        # Nav Buttons
        self.nav_buttons = {}
        tabs = [
            ("mouse", "🖱️ Кликер мыши"),
            ("keyboard", "⌨️ Клавиатура"),
            ("scroll", "📜 Авто-скролл"),
            ("multipoint", "🎯 Маршрут точек"),
            ("macro", "🎬 Макросы"),
            ("settings", "⚙️ Настройки")
        ]

        for idx, (tab_id, tab_label) in enumerate(tabs, start=1):
            btn = ctk.CTkButton(
                self.sidebar_frame,
                text=f"  {tab_label}",
                font=("Segoe UI", 12, "bold"),
                height=38,
                corner_radius=8,
                fg_color="transparent",
                text_color="#94a3b8",
                anchor="w",
                hover_color="#222834",
                command=lambda t=tab_id: self._select_tab(t)
            )
            btn.grid(row=idx, column=0, padx=12, pady=4, sticky="ew")
            self.nav_buttons[tab_id] = btn

        # Sidebar Live Status Card
        self.side_status_card = ctk.CTkFrame(self.sidebar_frame, corner_radius=10, fg_color="#202531")
        self.side_status_card.grid(row=9, column=0, padx=12, pady=16, sticky="ew")
        
        self.side_status_lbl = ctk.CTkLabel(
            self.side_status_card, 
            text="🔴 ОСТАНОВЛЕН", 
            font=("Segoe UI", 11, "bold"),
            text_color="#f87171"
        )
        self.side_status_lbl.pack(pady=(10, 4))

        self.side_cps_lbl = ctk.CTkLabel(
            self.side_status_card, 
            text="CPS: 0.0\nВсего: 0", 
            font=("Segoe UI", 11),
            text_color="#94a3b8"
        )
        self.side_cps_lbl.pack(pady=(0, 10))

    def _build_content_area(self):
        self.content_frame = ctk.CTkFrame(self, fg_color="transparent")
        self.content_frame.grid(row=0, column=1, sticky="nsew", padx=12, pady=(12, 0))
        self.content_frame.grid_columnconfigure(0, weight=1)
        self.content_frame.grid_rowconfigure(0, weight=1)

        # Tab Views
        self.tab_mouse = MouseTab(self.content_frame)
        self.tab_keyboard = KeyboardTab(self.content_frame)
        self.tab_scroll = ScrollTab(self.content_frame)
        self.tab_multipoint = MultiPointTab(self.content_frame)
        self.tab_macro = MacroTab(self.content_frame)
        self.tab_settings = SettingsTab(
            self.content_frame,
            profile_manager=self.profile_mgr,
            on_hotkey_changed=self._on_hotkey_changed,
            on_sound_toggled=self._on_sound_toggled,
            on_overlay_toggled=self._on_overlay_toggled,
            on_profile_loaded=self._apply_profile_data,
            get_current_state_callback=self._get_all_state_data
        )

        self.tabs_dict = {
            "mouse": self.tab_mouse,
            "keyboard": self.tab_keyboard,
            "scroll": self.tab_scroll,
            "multipoint": self.tab_multipoint,
            "macro": self.tab_macro,
            "settings": self.tab_settings
        }

    def _build_bottom_bar(self):
        self.bottom_frame = ctk.CTkFrame(self, height=60, corner_radius=10, fg_color="#181b24")
        self.bottom_frame.grid(row=1, column=1, sticky="ew", padx=12, pady=12)
        self.bottom_frame.grid_columnconfigure(0, weight=1)

        # Info string
        self.bot_status_lbl = ctk.CTkLabel(
            self.bottom_frame,
            text="Режим: Кликер мыши  |  Статус: Готов к работе  |  Всего кликов: 0",
            font=("Segoe UI", 12),
            text_color="#94a3b8"
        )
        self.bot_status_lbl.grid(row=0, column=0, sticky="w", padx=20, pady=10)

        # Prominent Action Button
        self.btn_main_toggle = ctk.CTkButton(
            self.bottom_frame,
            text="▶ ЗАПУСК [F6]",
            font=("Segoe UI", 13, "bold"),
            width=180,
            height=38,
            corner_radius=8,
            fg_color="#10b981",
            hover_color="#059669",
            command=self._safe_toggle_active_engine
        )
        self.btn_main_toggle.grid(row=0, column=1, sticky="e", padx=16, pady=10)

    def _select_tab(self, tab_name: str):
        self.active_tab_name = tab_name
        
        for t_name, tab_widget in self.tabs_dict.items():
            tab_widget.grid_forget()
            
            btn = self.nav_buttons.get(t_name)
            if btn:
                if t_name == tab_name:
                    btn.configure(fg_color="#2563eb", text_color="#ffffff")
                else:
                    btn.configure(fg_color="transparent", text_color="#94a3b8")

        current_tab = self.tabs_dict.get(tab_name)
        if current_tab:
            current_tab.grid(row=0, column=0, sticky="nsew")

        self._update_status_bar()

    def _safe_toggle_active_engine(self):
        self.after(0, self._toggle_active_engine)

    def _toggle_active_engine(self):
        if not self.is_running:
            self._start_engine()
        else:
            self._stop_engine()

    def _start_engine(self):
        self.is_running = True
        self.total_count = 0
        self.live_cps = 0.0

        if self.active_tab_name == "mouse":
            cfg = self.tab_mouse.get_config()
            self.mouse_engine.start(cfg)
        elif self.active_tab_name == "keyboard":
            cfg = self.tab_keyboard.get_config()
            self.kb_engine.start(cfg)
        elif self.active_tab_name == "scroll":
            cfg = self.tab_scroll.get_config()
            self.scroll_engine.start(cfg)
        elif self.active_tab_name == "multipoint":
            cfg = self.tab_multipoint.get_config()
            self.multipoint_engine.start(cfg)
        elif self.active_tab_name == "macro":
            cfg = self.tab_macro.get_config()
            self.macro_player.start(cfg)

        play_audio_feedback(is_start=True, sound_enabled=self.hotkey_mgr.sound_enabled)
        self._update_status_bar()

    def _stop_engine(self):
        self.mouse_engine.stop()
        self.kb_engine.stop()
        self.scroll_engine.stop()
        self.multipoint_engine.stop()
        self.macro_player.stop()

        self.is_running = False
        play_audio_feedback(is_start=False, sound_enabled=self.hotkey_mgr.sound_enabled)
        self._update_status_bar()

    def _safe_stop_all(self):
        self.after(0, self._stop_engine)

    def _trigger_coord_picker(self):
        self.after(0, lambda: CoordinatePicker(on_picked_callback=self._on_picked_coord).start_picking(self))

    def _on_picked_coord(self, x: int, y: int):
        if self.active_tab_name == "mouse":
            self.tab_mouse._set_picked_coords(x, y)
        elif self.active_tab_name == "scroll":
            self.tab_scroll._set_picked_coords(x, y)
        elif self.active_tab_name == "multipoint":
            self.tab_multipoint._set_picked_coords(x, y)

    def _on_engine_update(self, total: int, cps: float = 0.0, *args):
        self.total_count = total
        self.live_cps = cps
        self.after(0, self._update_status_bar)

    def _on_macro_update(self, loops: int, step: int):
        self.total_count = loops
        self.live_cps = 0.0
        self.after(0, self._update_status_bar)

    def _update_status_bar(self):
        hk = self.hotkey_mgr.start_stop_key.upper()
        mode_titles = {
            "mouse": "Мышь",
            "keyboard": "Клавиатура",
            "scroll": "Скролл",
            "multipoint": "Маршрут точек",
            "macro": "Макрос",
            "settings": "Настройки"
        }
        curr_mode = mode_titles.get(self.active_tab_name, self.active_tab_name)

        if self.is_running:
            self.btn_main_toggle.configure(text=f"⏹ ОСТАНОВИТЬ [{hk}]", fg_color="#ef4444", hover_color="#dc2626")
            self.side_status_lbl.configure(text=f"🟢 РАБОТАЕТ ({curr_mode.upper()})", text_color="#34d399")
            self.bot_status_lbl.configure(
                text=f"🟢 РАБОТАЕТ  |  Режим: {curr_mode}  |  Скорость: {self.live_cps:.1f} CPS  |  Всего: {self.total_count}",
                text_color="#34d399"
            )
        else:
            self.btn_main_toggle.configure(text=f"▶ ЗАПУСК [{hk}]", fg_color="#10b981", hover_color="#059669")
            self.side_status_lbl.configure(text="🔴 ОСТАНОВЛЕН", text_color="#f87171")
            self.bot_status_lbl.configure(
                text=f"Режим: {curr_mode}  |  Статус: Готов к работе  |  Выполнено: {self.total_count}",
                text_color="#94a3b8"
            )

        self.side_cps_lbl.configure(text=f"CPS: {self.live_cps:.1f}\nВсего: {self.total_count}")
        self.overlay_hud.update_state(self.is_running, curr_mode, self.total_count, self.live_cps, hk)

    def _on_hotkey_changed(self, start_k: str, pick_k: str, emerg_k: str):
        self.hotkey_mgr.start_stop_key = start_k.lower()
        self.hotkey_mgr.pick_coord_key = pick_k.lower()
        self.hotkey_mgr.emergency_stop_key = emerg_k.lower()
        self._update_status_bar()

    def _on_sound_toggled(self, enabled: bool):
        self.hotkey_mgr.sound_enabled = enabled

    def _on_overlay_toggled(self, enabled: bool):
        if enabled:
            self.overlay_hud.show_overlay()
        else:
            self.overlay_hud.hide_overlay()

    def _get_all_state_data(self) -> dict:
        return {
            "tab": self.active_tab_name,
            "mouse": self.tab_mouse.get_config().__dict__,
            "keyboard": self.tab_keyboard.get_config().__dict__,
            "scroll": self.tab_scroll.get_config().__dict__,
            "multipoint": self.tab_multipoint.get_config().__dict__
        }

    def _apply_profile_data(self, data: dict):
        if "tab" in data:
            self._select_tab(data["tab"])
        if "mouse" in data:
            self.tab_mouse.set_config(data["mouse"])
        if "keyboard" in data:
            self.tab_keyboard.set_config(data["keyboard"])
        if "scroll" in data:
            self.tab_scroll.set_config(data["scroll"])
        if "multipoint" in data:
            self.tab_multipoint.set_config(data["multipoint"])
        self._update_status_bar()

    def _on_close(self):
        # Completely silent shutdown - stop engines quietly without beeps
        self.mouse_engine.stop()
        self.kb_engine.stop()
        self.scroll_engine.stop()
        self.multipoint_engine.stop()
        self.macro_player.stop()
        self.hotkey_mgr.stop()
        cleanup_timer()
        self.destroy()
