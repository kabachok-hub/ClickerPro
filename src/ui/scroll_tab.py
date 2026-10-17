import customtkinter as ctk
import tkinter as tk
from typing import Callable, Optional
from src.core.scroll_engine import ScrollConfig
from src.ui.coordinate_picker import CoordinatePicker

class ScrollTab(ctk.CTkScrollableFrame):
    def __init__(self, parent, on_config_changed: Optional[Callable[[], None]] = None):
        super().__init__(parent, fg_color="transparent")
        self.on_config_changed = on_config_changed
        self.grid_columnconfigure((0, 1), weight=1)

        # --- Section 1: Direction & Intensity ---
        self.dir_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.dir_frame.grid(row=0, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.dir_frame.grid_columnconfigure((0, 1, 2, 3), weight=1)

        ctk.CTkLabel(self.dir_frame, text="📜 Параметры скролла", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=4, sticky="w", padx=16, pady=(12, 6))

        # Direction
        ctk.CTkLabel(self.dir_frame, text="Направление:", font=("Segoe UI", 11)).grid(row=1, column=0, sticky="w", padx=16, pady=6)
        self.dir_selector = ctk.CTkOptionMenu(
            self.dir_frame,
            values=["Вниз (Down)", "Вверх (Up)", "Вправо (Right)", "Влево (Left)"],
            width=160,
            fg_color="#2563eb",
            button_color="#1d4ed8"
        )
        self.dir_selector.set("Вниз (Down)")
        self.dir_selector.grid(row=1, column=1, sticky="w", padx=4, pady=6)

        # Ticks
        ctk.CTkLabel(self.dir_frame, text="Делений за шаг:", font=("Segoe UI", 11)).grid(row=1, column=2, sticky="w", padx=16, pady=6)
        self.ticks_entry = ctk.CTkEntry(self.dir_frame, width=70)
        self.ticks_entry.insert(0, "1")
        self.ticks_entry.grid(row=1, column=3, sticky="w", padx=4, pady=6)

        # Smooth Scroll Switch
        self.smooth_switch = ctk.CTkSwitch(self.dir_frame, text="Плавная микро-прокрутка (Smooth Scroll)", progress_color="#2563eb")
        self.smooth_switch.select()
        self.smooth_switch.grid(row=2, column=0, columnspan=4, sticky="w", padx=16, pady=(4, 12))

        # --- Section 2: Speed & Interval ---
        self.speed_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.speed_frame.grid(row=1, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.speed_frame.grid_columnconfigure((0, 1, 2, 3), weight=1)

        ctk.CTkLabel(self.speed_frame, text="⚡ Частота прокрутки", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=4, sticky="w", padx=16, pady=(12, 6))

        ctk.CTkLabel(self.speed_frame, text="Интервал между шагами (мс):", font=("Segoe UI", 11)).grid(row=1, column=0, sticky="w", padx=16, pady=(6, 12))
        self.interval_entry = ctk.CTkEntry(self.speed_frame, width=80)
        self.interval_entry.insert(0, "50")
        self.interval_entry.grid(row=1, column=1, sticky="w", padx=4, pady=(6, 12))

        ctk.CTkLabel(self.speed_frame, text="🎲 Рандомизация (± мс):", font=("Segoe UI", 11)).grid(row=1, column=2, sticky="w", padx=16, pady=(6, 12))
        self.jitter_entry = ctk.CTkEntry(self.speed_frame, width=80)
        self.jitter_entry.insert(0, "0")
        self.jitter_entry.grid(row=1, column=3, sticky="w", padx=4, pady=(6, 12))

        # --- Section 3: Position ---
        self.loc_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.loc_frame.grid(row=2, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.loc_frame.grid_columnconfigure((0, 1, 2, 3), weight=1)

        ctk.CTkLabel(self.loc_frame, text="🎯 Позиция курсора при скролле", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=4, sticky="w", padx=16, pady=(12, 6))

        self.loc_mode_var = ctk.StringVar(value="cursor")
        
        self.radio_cursor = ctk.CTkRadioButton(
            self.loc_frame, 
            text="Текущее положение мыши", 
            variable=self.loc_mode_var, 
            value="cursor"
        )
        self.radio_cursor.grid(row=1, column=0, columnspan=2, sticky="w", padx=16, pady=6)

        self.radio_fixed = ctk.CTkRadioButton(
            self.loc_frame, 
            text="Фиксированные координаты:", 
            variable=self.loc_mode_var, 
            value="fixed"
        )
        self.radio_fixed.grid(row=2, column=0, sticky="w", padx=16, pady=(6, 12))

        self.coord_subframe = ctk.CTkFrame(self.loc_frame, fg_color="transparent")
        self.coord_subframe.grid(row=2, column=1, columnspan=2, sticky="w", padx=4, pady=(6, 12))

        ctk.CTkLabel(self.coord_subframe, text="X:").pack(side="left", padx=2)
        self.entry_x = ctk.CTkEntry(self.coord_subframe, width=60)
        self.entry_x.insert(0, "500")
        self.entry_x.pack(side="left", padx=2)

        ctk.CTkLabel(self.coord_subframe, text="Y:").pack(side="left", padx=2)
        self.entry_y = ctk.CTkEntry(self.coord_subframe, width=60)
        self.entry_y.insert(0, "500")
        self.entry_y.pack(side="left", padx=2)

        self.btn_pick = ctk.CTkButton(
            self.loc_frame,
            text="🎯 Захватить [F8]",
            width=130,
            fg_color="#334155",
            hover_color="#475569",
            command=self._pick_coordinates
        )
        self.btn_pick.grid(row=2, column=3, sticky="w", padx=8, pady=(6, 12))

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
            text="По числу скроллов:", 
            variable=self.stop_mode_var, 
            value="count"
        )
        self.radio_count.grid(row=2, column=0, sticky="w", padx=16, pady=(6, 12))

        self.entry_count = ctk.CTkEntry(self.stop_frame, width=90)
        self.entry_count.insert(0, "200")
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

    def _pick_coordinates(self):
        picker = CoordinatePicker(on_picked_callback=self._set_picked_coords)
        picker.start_picking(self.winfo_toplevel())

    def _set_picked_coords(self, x: int, y: int):
        self.entry_x.delete(0, "end")
        self.entry_x.insert(0, str(x))
        self.entry_y.delete(0, "end")
        self.entry_y.insert(0, str(y))
        self.loc_mode_var.set("fixed")

    def get_config(self) -> ScrollConfig:
        cfg = ScrollConfig()

        dir_txt = self.dir_selector.get()
        if "Вверх" in dir_txt: cfg.direction = "up"
        elif "Вправо" in dir_txt: cfg.direction = "right"
        elif "Влево" in dir_txt: cfg.direction = "left"
        else: cfg.direction = "down"

        try:
            cfg.ticks = int(self.ticks_entry.get() or 1)
        except Exception:
            cfg.ticks = 1

        cfg.smooth = bool(self.smooth_switch.get())

        try:
            cfg.interval_ms = float(self.interval_entry.get() or 50)
        except Exception:
            cfg.interval_ms = 50.0

        try:
            cfg.random_delay_ms = float(self.jitter_entry.get() or 0)
        except Exception:
            cfg.random_delay_ms = 0.0

        cfg.location_type = self.loc_mode_var.get()
        try:
            cfg.fixed_x = int(self.entry_x.get() or 0)
            cfg.fixed_y = int(self.entry_y.get() or 0)
        except Exception:
            cfg.fixed_x, cfg.fixed_y = 0, 0

        cfg.stop_condition = self.stop_mode_var.get()
        try:
            cfg.repeat_count = int(self.entry_count.get() or 200)
        except Exception:
            cfg.repeat_count = 200

        try:
            cfg.max_time_seconds = float(self.entry_timer.get() or 30)
        except Exception:
            cfg.max_time_seconds = 30.0

        return cfg

    def set_config(self, cfg_dict: dict):
        if "direction" in cfg_dict:
            d = cfg_dict["direction"]
            if d == "up": self.dir_selector.set("Вверх (Up)")
            elif d == "right": self.dir_selector.set("Вправо (Right)")
            elif d == "left": self.dir_selector.set("Влево (Left)")
            else: self.dir_selector.set("Вниз (Down)")

        if "ticks" in cfg_dict:
            self.ticks_entry.delete(0, "end")
            self.ticks_entry.insert(0, str(cfg_dict["ticks"]))
        if "smooth" in cfg_dict:
            if cfg_dict["smooth"]: self.smooth_switch.select()
            else: self.smooth_switch.deselect()
        if "interval_ms" in cfg_dict:
            self.interval_entry.delete(0, "end")
            self.interval_entry.insert(0, str(cfg_dict["interval_ms"]))
        if "random_delay_ms" in cfg_dict:
            self.jitter_entry.delete(0, "end")
            self.jitter_entry.insert(0, str(cfg_dict["random_delay_ms"]))
