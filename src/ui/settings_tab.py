import customtkinter as ctk
import tkinter as tk
from tkinter import messagebox
from typing import Callable, Optional, List
from src.core.profile_manager import ProfileManager

class SettingsTab(ctk.CTkScrollableFrame):
    def __init__(
        self, 
        parent, 
        profile_manager: ProfileManager,
        on_hotkey_changed: Optional[Callable[[str, str, str], None]] = None,
        on_sound_toggled: Optional[Callable[[bool], None]] = None,
        on_overlay_toggled: Optional[Callable[[bool], None]] = None,
        on_profile_loaded: Optional[Callable[[dict], None]] = None,
        get_current_state_callback: Optional[Callable[[], dict]] = None
    ):
        super().__init__(parent, fg_color="transparent")
        self.profile_mgr = profile_manager
        self.on_hotkey_changed = on_hotkey_changed
        self.on_sound_toggled = on_sound_toggled
        self.on_overlay_toggled = on_overlay_toggled
        self.on_profile_loaded = on_profile_loaded
        self.get_current_state = get_current_state_callback
        
        self.grid_columnconfigure((0, 1), weight=1)

        # --- Section 1: Global Hotkeys ---
        self.hk_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.hk_frame.grid(row=0, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.hk_frame.grid_columnconfigure((0, 1, 2, 3), weight=1)

        ctk.CTkLabel(self.hk_frame, text="⌨️ Горячие клавиши (Hotkeys)", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=4, sticky="w", padx=16, pady=(12, 6))

        # Start/Stop Hotkey
        ctk.CTkLabel(self.hk_frame, text="Запуск / Остановка:").grid(row=1, column=0, sticky="w", padx=16, pady=6)
        self.start_hk_selector = ctk.CTkOptionMenu(
            self.hk_frame,
            values=["F6", "F7", "F8", "F9", "F10", "F11", "F12", "mouse_x1", "mouse_x2", "mouse_middle"],
            fg_color="#2563eb",
            button_color="#1d4ed8",
            command=self._apply_hotkeys
        )
        self.start_hk_selector.set("F6")
        self.start_hk_selector.grid(row=1, column=1, sticky="w", padx=4, pady=6)

        # Coordinate Picker Hotkey
        ctk.CTkLabel(self.hk_frame, text="Захват координат:").grid(row=1, column=2, sticky="w", padx=16, pady=6)
        self.pick_hk_selector = ctk.CTkOptionMenu(
            self.hk_frame,
            values=["F8", "F9", "F10", "F11", "F12"],
            fg_color="#2563eb",
            button_color="#1d4ed8",
            command=self._apply_hotkeys
        )
        self.pick_hk_selector.set("F8")
        self.pick_hk_selector.grid(row=1, column=3, sticky="w", padx=4, pady=6)

        # Emergency Stop
        ctk.CTkLabel(self.hk_frame, text="Экстренная остановка:").grid(row=2, column=0, sticky="w", padx=16, pady=(6, 12))
        self.emerg_hk_selector = ctk.CTkOptionMenu(
            self.hk_frame,
            values=["F12", "escape", "F11"],
            fg_color="#334155",
            button_color="#475569",
            command=self._apply_hotkeys
        )
        self.emerg_hk_selector.set("F12")
        self.emerg_hk_selector.grid(row=2, column=1, sticky="w", padx=4, pady=(6, 12))

        # --- Section 2: General Features ---
        self.gen_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.gen_frame.grid(row=1, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.gen_frame.grid_columnconfigure((0, 1), weight=1)

        ctk.CTkLabel(self.gen_frame, text="🔔 Звуки и Оверлей", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=2, sticky="w", padx=16, pady=(12, 6))

        # Audio Switch - OFF by default
        self.sound_switch = ctk.CTkSwitch(
            self.gen_frame, 
            text="Звуковое подтверждение при старте/стопе", 
            progress_color="#2563eb",
            command=self._toggle_sound
        )
        self.sound_switch.deselect() # Completely silent by default
        self.sound_switch.grid(row=1, column=0, sticky="w", padx=16, pady=6)

        # Floating Overlay Switch
        self.overlay_switch = ctk.CTkSwitch(
            self.gen_frame, 
            text="Плавающее мини-табло (HUD поверх всех окон)", 
            progress_color="#2563eb",
            command=self._toggle_overlay
        )
        self.overlay_switch.grid(row=1, column=1, sticky="w", padx=16, pady=6)

        # Theme Selector
        ctk.CTkLabel(self.gen_frame, text="Тема оформления:").grid(row=2, column=0, sticky="w", padx=16, pady=(6, 12))
        self.theme_menu = ctk.CTkOptionMenu(
            self.gen_frame,
            values=["Dark", "Light", "System"],
            fg_color="#334155",
            button_color="#475569",
            command=self._change_theme
        )
        self.theme_menu.set("Dark")
        self.theme_menu.grid(row=2, column=1, sticky="w", padx=4, pady=(6, 12))

        # --- Section 3: Presets & Profiles ---
        self.prof_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.prof_frame.grid(row=2, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.prof_frame.grid_columnconfigure((0, 1, 2, 3), weight=1)

        ctk.CTkLabel(self.prof_frame, text="📁 Профили и готовые пресеты", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=4, sticky="w", padx=16, pady=(12, 6))

        ctk.CTkLabel(self.prof_frame, text="Выбрать профиль:").grid(row=1, column=0, sticky="w", padx=16, pady=6)
        
        self.profile_selector = ctk.CTkOptionMenu(
            self.prof_frame,
            values=self._get_profile_names(),
            width=200,
            fg_color="#334155",
            button_color="#475569"
        )
        self.profile_selector.grid(row=1, column=1, sticky="w", padx=4, pady=6)

        self.btn_load_prof = ctk.CTkButton(
            self.prof_frame,
            text="📂 Загрузить",
            width=100,
            fg_color="#0284c7",
            hover_color="#0369a1",
            command=self._load_selected_profile
        )
        self.btn_load_prof.grid(row=1, column=2, sticky="w", padx=4, pady=6)

        self.btn_del_prof = ctk.CTkButton(
            self.prof_frame,
            text="🗑️ Удалить",
            width=90,
            fg_color="#dc2626",
            hover_color="#b91c1c",
            command=self._delete_selected_profile
        )
        self.btn_del_prof.grid(row=1, column=3, sticky="w", padx=4, pady=6)

        # Save current as profile
        ctk.CTkLabel(self.prof_frame, text="Имя нового профиля:").grid(row=2, column=0, sticky="w", padx=16, pady=(8, 12))
        self.new_prof_entry = ctk.CTkEntry(self.prof_frame, width=200)
        self.new_prof_entry.insert(0, "Мой_Пресет_1")
        self.new_prof_entry.grid(row=2, column=1, sticky="w", padx=4, pady=(8, 12))

        self.btn_save_prof = ctk.CTkButton(
            self.prof_frame,
            text="💾 Сохранить пресет",
            width=150,
            fg_color="#059669",
            hover_color="#047857",
            command=self._save_current_as_profile
        )
        self.btn_save_prof.grid(row=2, column=2, columnspan=2, sticky="w", padx=4, pady=(8, 12))

    def _get_profile_names(self) -> List[str]:
        names = self.profile_mgr.list_profiles()
        return names if names else ["Нет профилей"]

    def _refresh_profiles(self):
        names = self._get_profile_names()
        self.profile_selector.configure(values=names)
        if names:
            self.profile_selector.set(names[0])

    def _apply_hotkeys(self, val=None):
        if self.on_hotkey_changed:
            start_k = self.start_hk_selector.get()
            pick_k = self.pick_hk_selector.get()
            emerg_k = self.emerg_hk_selector.get()
            self.on_hotkey_changed(start_k, pick_k, emerg_k)

    def _toggle_sound(self):
        if self.on_sound_toggled:
            self.on_sound_toggled(bool(self.sound_switch.get()))

    def _toggle_overlay(self):
        if self.on_overlay_toggled:
            self.on_overlay_toggled(bool(self.overlay_switch.get()))

    def _change_theme(self, theme_name: str):
        ctk.set_appearance_mode(theme_name.lower())

    def _load_selected_profile(self):
        name = self.profile_selector.get()
        if not name or name == "Нет профилей":
            return
        data = self.profile_mgr.load_profile(name)
        if data and self.on_profile_loaded:
            self.on_profile_loaded(data)
            messagebox.showinfo("Успешно", f"Профиль '{name}' загружен!")

    def _save_current_as_profile(self):
        name = self.new_prof_entry.get().strip()
        if not name:
            messagebox.showerror("Ошибка", "Введите название профиля")
            return
        if self.get_current_state:
            data = self.get_current_state()
            self.profile_mgr.save_profile(name, data)
            self._refresh_profiles()
            self.profile_selector.set(name)
            messagebox.showinfo("Успешно", f"Профиль '{name}' сохранен!")

    def _delete_selected_profile(self):
        name = self.profile_selector.get()
        if not name or name == "Нет профилей":
            return
        if self.profile_mgr.delete_profile(name):
            self._refresh_profiles()
            messagebox.showinfo("Успешно", f"Профиль '{name}' удален.")
