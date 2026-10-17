import customtkinter as ctk
import tkinter as tk
from typing import Callable, Optional
from src.core.mouse_engine import MouseConfig
from src.ui.coordinate_picker import CoordinatePicker

class MouseTab(ctk.CTkScrollableFrame):
    def __init__(self, parent, on_config_changed: Optional[Callable[[], None]] = None):
        super().__init__(parent, fg_color="transparent")
        self.on_config_changed = on_config_changed
        self.grid_columnconfigure((0, 1), weight=1)

        # --- Section 1: Button & Action ---
        self.btn_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.btn_frame.grid(row=0, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.btn_frame.grid_columnconfigure((0, 1, 2, 3), weight=1)

        ctk.CTkLabel(self.btn_frame, text="🖱️ Параметры клика", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=4, sticky="w", padx=16, pady=(12, 6))

        # Mouse Button
        ctk.CTkLabel(self.btn_frame, text="Кнопка мыши:", font=("Segoe UI", 11)).grid(row=1, column=0, sticky="w", padx=16, pady=6)
        self.btn_selector = ctk.CTkOptionMenu(
            self.btn_frame,
            values=["Левая (LMB)", "Правая (RMB)", "Средняя (MMB)", "Боковая 1 (X1)", "Боковая 2 (X2)"],
            width=160,
            fg_color="#2563eb",
            button_color="#1d4ed8"
        )
        self.btn_selector.set("Левая (LMB)")
        self.btn_selector.grid(row=1, column=1, sticky="w", padx=4, pady=6)

        # Click Type
        ctk.CTkLabel(self.btn_frame, text="Тип клика:", font=("Segoe UI", 11)).grid(row=1, column=2, sticky="w", padx=16, pady=6)
        self.type_selector = ctk.CTkOptionMenu(
            self.btn_frame,
            values=["Одиночный клик", "Двойной клик", "Тройной клик", "Зажатие (Hold)"],
            width=160,
            fg_color="#2563eb",
            button_color="#1d4ed8"
        )
        self.type_selector.set("Одиночный клик")
        self.type_selector.grid(row=1, column=3, sticky="w", padx=4, pady=(6, 12))

        # --- Section 2: Speed & Interval ---
        self.speed_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.speed_frame.grid(row=1, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.speed_frame.grid_columnconfigure((0, 1, 2, 3), weight=1)

        ctk.CTkLabel(self.speed_frame, text="⚡ Скорость и частота кликов", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=4, sticky="w", padx=16, pady=(12, 6))

        self.speed_mode_var = ctk.StringVar(value="cps")
        
        # CPS Mode Radio
        self.radio_cps = ctk.CTkRadioButton(
            self.speed_frame, 
            text="Режим CPS (кликов в сек):", 
            variable=self.speed_mode_var, 
            value="cps"
        )
        self.radio_cps.grid(row=1, column=0, sticky="w", padx=16, pady=8)

        # CPS Slider & Entry
        self.cps_slider = ctk.CTkSlider(self.speed_frame, from_=1, to=200, number_of_steps=199, command=self._on_slider_cps)
        self.cps_slider.set(20)
        self.cps_slider.grid(row=1, column=1, columnspan=2, sticky="ew", padx=8, pady=8)

        self.cps_entry = ctk.CTkEntry(self.speed_frame, width=70)
        self.cps_entry.insert(0, "20")
        self.cps_entry.grid(row=1, column=3, sticky="w", padx=8, pady=8)
        self.cps_entry.bind("<KeyRelease>", self._on_entry_cps)

        # Interval Mode Radio
        self.radio_interval = ctk.CTkRadioButton(
            self.speed_frame, 
            text="Пользовательский интервал:", 
            variable=self.speed_mode_var, 
            value="interval"
        )
        self.radio_interval.grid(row=2, column=0, sticky="w", padx=16, pady=8)

        # Interval Inputs Frame
        self.interval_subframe = ctk.CTkFrame(self.speed_frame, fg_color="transparent")
        self.interval_subframe.grid(row=2, column=1, columnspan=3, sticky="w", padx=8, pady=8)

        ctk.CTkLabel(self.interval_subframe, text="Час:").pack(side="left", padx=2)
        self.entry_hrs = ctk.CTkEntry(self.interval_subframe, width=45)
        self.entry_hrs.insert(0, "0")
        self.entry_hrs.pack(side="left", padx=2)

        ctk.CTkLabel(self.interval_subframe, text="Мин:").pack(side="left", padx=2)
        self.entry_mins = ctk.CTkEntry(self.interval_subframe, width=45)
        self.entry_mins.insert(0, "0")
        self.entry_mins.pack(side="left", padx=2)

        ctk.CTkLabel(self.interval_subframe, text="Сек:").pack(side="left", padx=2)
        self.entry_secs = ctk.CTkEntry(self.interval_subframe, width=45)
        self.entry_secs.insert(0, "0")
        self.entry_secs.pack(side="left", padx=2)

        ctk.CTkLabel(self.interval_subframe, text="Мс:").pack(side="left", padx=2)
        self.entry_ms = ctk.CTkEntry(self.interval_subframe, width=55)
        self.entry_ms.insert(0, "50")
        self.entry_ms.pack(side="left", padx=2)

        # Humanizer Jitter
        ctk.CTkLabel(self.speed_frame, text="🎲 Рандомизация задержки (± мс):", font=("Segoe UI", 11)).grid(row=3, column=0, sticky="w", padx=16, pady=(4, 12))
        self.jitter_entry = ctk.CTkEntry(self.speed_frame, width=70)
        self.jitter_entry.insert(0, "0")
        self.jitter_entry.grid(row=3, column=1, sticky="w", padx=8, pady=(4, 12))

        # --- Section 3: Location Coordinates ---
        self.loc_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.loc_frame.grid(row=2, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.loc_frame.grid_columnconfigure((0, 1, 2, 3), weight=1)

        ctk.CTkLabel(self.loc_frame, text="🎯 Позиция курсора", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=4, sticky="w", padx=16, pady=(12, 6))

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
        self.radio_fixed.grid(row=2, column=0, sticky="w", padx=16, pady=6)

        # Coord Inputs Frame
        self.coord_subframe = ctk.CTkFrame(self.loc_frame, fg_color="transparent")
        self.coord_subframe.grid(row=2, column=1, columnspan=2, sticky="w", padx=4, pady=6)

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
        self.btn_pick.grid(row=2, column=3, sticky="w", padx=8, pady=6)

        # Coordinate Scatter Jitter
        ctk.CTkLabel(self.loc_frame, text="🎲 Разброс координат (± px):", font=("Segoe UI", 11)).grid(row=3, column=0, sticky="w", padx=16, pady=(4, 12))
        self.entry_scatter = ctk.CTkEntry(self.loc_frame, width=70)
        self.entry_scatter.insert(0, "0")
        self.entry_scatter.grid(row=3, column=1, sticky="w", padx=8, pady=(4, 12))

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
            text="По числу кликов:", 
            variable=self.stop_mode_var, 
            value="count"
        )
        self.radio_count.grid(row=2, column=0, sticky="w", padx=16, pady=(6, 12))

        self.entry_count = ctk.CTkEntry(self.stop_frame, width=90)
        self.entry_count.insert(0, "100")
        self.entry_count.grid(row=2, column=1, sticky="w", padx=8, pady=(6, 12))

        self.radio_time = ctk.CTkRadioButton(
            self.stop_frame, 
            text="По таймеру (сек):", 
            variable=self.stop_mode_var, 
            value="time"
        )
        self.radio_time.grid(row=2, column=2, sticky="w", padx=16, pady=(6, 12))

        self.entry_timer = ctk.CTkEntry(self.stop_frame, width=90)
        self.entry_timer.insert(0, "30")
        self.entry_timer.grid(row=2, column=3, sticky="w", padx=8, pady=(6, 12))

    def _on_slider_cps(self, val):
        self.cps_entry.delete(0, "end")
        self.cps_entry.insert(0, str(int(val)))

    def _on_entry_cps(self, event):
        try:
            val = float(self.cps_entry.get())
            if 1 <= val <= 200:
                self.cps_slider.set(val)
        except Exception:
            pass

    def _pick_coordinates(self):
        picker = CoordinatePicker(on_picked_callback=self._set_picked_coords)
        picker.start_picking(self.winfo_toplevel())

    def _set_picked_coords(self, x: int, y: int):
        self.entry_x.delete(0, "end")
        self.entry_x.insert(0, str(x))
        self.entry_y.delete(0, "end")
        self.entry_y.insert(0, str(y))
        self.loc_mode_var.set("fixed")

    def get_config(self) -> MouseConfig:
        cfg = MouseConfig()
        
        btn_txt = self.btn_selector.get()
        if "Правая" in btn_txt: cfg.button = "right"
        elif "Средняя" in btn_txt: cfg.button = "middle"
        elif "Боковая 1" in btn_txt: cfg.button = "x1"
        elif "Боковая 2" in btn_txt: cfg.button = "x2"
        else: cfg.button = "left"

        type_txt = self.type_selector.get()
        if "Двойной" in type_txt: cfg.click_type = "double"
        elif "Тройной" in type_txt: cfg.click_type = "triple"
        elif "Зажатие" in type_txt: cfg.click_type = "hold"
        else: cfg.click_type = "single"

        cfg.mode = self.speed_mode_var.get()
        try:
            cfg.cps = float(self.cps_entry.get())
        except Exception:
            cfg.cps = 20.0

        try:
            h = float(self.entry_hrs.get() or 0)
            m = float(self.entry_mins.get() or 0)
            s = float(self.entry_secs.get() or 0)
            ms = float(self.entry_ms.get() or 0)
            cfg.interval_ms = (h * 3600 + m * 60 + s) * 1000 + ms
            if cfg.interval_ms <= 0:
                cfg.interval_ms = 50.0
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

        try:
            cfg.jitter_radius = int(self.entry_scatter.get() or 0)
        except Exception:
            cfg.jitter_radius = 0

        cfg.stop_condition = self.stop_mode_var.get()
        try:
            cfg.repeat_count = int(self.entry_count.get() or 100)
        except Exception:
            cfg.repeat_count = 100

        try:
            cfg.max_time_seconds = float(self.entry_timer.get() or 30)
        except Exception:
            cfg.max_time_seconds = 30.0

        return cfg

    def set_config(self, cfg_dict: dict):
        if "button" in cfg_dict:
            b = cfg_dict["button"]
            if b == "right": self.btn_selector.set("Правая (RMB)")
            elif b == "middle": self.btn_selector.set("Средняя (MMB)")
            elif b == "x1": self.btn_selector.set("Боковая 1 (X1)")
            elif b == "x2": self.btn_selector.set("Боковая 2 (X2)")
            else: self.btn_selector.set("Левая (LMB)")

        if "click_type" in cfg_dict:
            t = cfg_dict["click_type"]
            if t == "double": self.type_selector.set("Двойной клик")
            elif t == "triple": self.type_selector.set("Тройной клик")
            elif t == "hold": self.type_selector.set("Зажатие (Hold)")
            else: self.type_selector.set("Одиночный клик")

        if "mode" in cfg_dict:
            self.speed_mode_var.set(cfg_dict["mode"])
        if "cps" in cfg_dict:
            self.cps_entry.delete(0, "end")
            self.cps_entry.insert(0, str(cfg_dict["cps"]))
            try:
                self.cps_slider.set(float(cfg_dict["cps"]))
            except Exception:
                pass
        if "interval_ms" in cfg_dict:
            self.entry_ms.delete(0, "end")
            self.entry_ms.insert(0, str(cfg_dict["interval_ms"]))
        if "random_delay_ms" in cfg_dict:
            self.jitter_entry.delete(0, "end")
            self.jitter_entry.insert(0, str(cfg_dict["random_delay_ms"]))
        if "location_type" in cfg_dict:
            self.loc_mode_var.set(cfg_dict["location_type"])
        if "fixed_x" in cfg_dict:
            self.entry_x.delete(0, "end")
            self.entry_x.insert(0, str(cfg_dict["fixed_x"]))
        if "fixed_y" in cfg_dict:
            self.entry_y.delete(0, "end")
            self.entry_y.insert(0, str(cfg_dict["fixed_y"]))
        if "jitter_radius" in cfg_dict:
            self.entry_scatter.delete(0, "end")
            self.entry_scatter.insert(0, str(cfg_dict["jitter_radius"]))
        if "stop_condition" in cfg_dict:
            self.stop_mode_var.set(cfg_dict["stop_condition"])
        if "repeat_count" in cfg_dict:
            self.entry_count.delete(0, "end")
            self.entry_count.insert(0, str(cfg_dict["repeat_count"]))
        if "max_time_seconds" in cfg_dict:
            self.entry_timer.delete(0, "end")
            self.entry_timer.insert(0, str(cfg_dict["max_time_seconds"]))
