import customtkinter as ctk
import tkinter as tk
from typing import Callable, Optional, List, Dict
from src.core.multipoint_engine import MultiPointConfig
from src.ui.coordinate_picker import CoordinatePicker

class MultiPointTab(ctk.CTkScrollableFrame):
    def __init__(self, parent, on_config_changed: Optional[Callable[[], None]] = None):
        super().__init__(parent, fg_color="transparent")
        self.on_config_changed = on_config_changed
        self.grid_columnconfigure((0, 1), weight=1)

        self.points_data: List[Dict] = [
            {"x": 600, "y": 400, "button": "left", "delay_after_ms": 200, "jitter_radius": 0},
            {"x": 800, "y": 400, "button": "left", "delay_after_ms": 200, "jitter_radius": 0},
            {"x": 700, "y": 600, "button": "right", "delay_after_ms": 300, "jitter_radius": 2}
        ]

        # --- Section 1: Add New Point Frame ---
        self.add_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.add_frame.grid(row=0, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.add_frame.grid_columnconfigure((0, 1, 2, 3, 4), weight=1)

        ctk.CTkLabel(self.add_frame, text="➕ Добавление точки в маршрут", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=5, sticky="w", padx=16, pady=(12, 6))

        # Coordinates
        ctk.CTkLabel(self.add_frame, text="X:").grid(row=1, column=0, sticky="e", padx=2, pady=4)
        self.entry_x = ctk.CTkEntry(self.add_frame, width=60)
        self.entry_x.insert(0, "500")
        self.entry_x.grid(row=1, column=1, sticky="w", padx=2, pady=4)

        ctk.CTkLabel(self.add_frame, text="Y:").grid(row=1, column=2, sticky="e", padx=2, pady=4)
        self.entry_y = ctk.CTkEntry(self.add_frame, width=60)
        self.entry_y.insert(0, "500")
        self.entry_y.grid(row=1, column=3, sticky="w", padx=2, pady=4)

        self.btn_pick = ctk.CTkButton(
            self.add_frame,
            text="🎯 Захват [F8]",
            width=110,
            fg_color="#334155",
            hover_color="#475569",
            command=self._pick_coordinates
        )
        self.btn_pick.grid(row=1, column=4, sticky="w", padx=8, pady=4)

        # Button & Delay
        ctk.CTkLabel(self.add_frame, text="Кнопка:").grid(row=2, column=0, sticky="e", padx=2, pady=(4, 12))
        self.btn_choice = ctk.CTkOptionMenu(
            self.add_frame,
            values=["Левая", "Правая", "Средняя", "Двойной клик"],
            width=110,
            fg_color="#2563eb",
            button_color="#1d4ed8"
        )
        self.btn_choice.set("Левая")
        self.btn_choice.grid(row=2, column=1, sticky="w", padx=2, pady=(4, 12))

        ctk.CTkLabel(self.add_frame, text="Пауза (мс):").grid(row=2, column=2, sticky="e", padx=2, pady=(4, 12))
        self.entry_delay = ctk.CTkEntry(self.add_frame, width=60)
        self.entry_delay.insert(0, "200")
        self.entry_delay.grid(row=2, column=3, sticky="w", padx=2, pady=(4, 12))

        self.btn_add = ctk.CTkButton(
            self.add_frame,
            text="➕ Добавить точку",
            width=140,
            fg_color="#059669",
            hover_color="#047857",
            command=self._add_point_from_inputs
        )
        self.btn_add.grid(row=2, column=4, sticky="w", padx=8, pady=(4, 12))

        # --- Section 2: Points List Table ---
        self.list_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.list_frame.grid(row=1, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.list_frame.grid_columnconfigure(0, weight=1)

        header_bar = ctk.CTkFrame(self.list_frame, fg_color="transparent")
        header_bar.grid(row=0, column=0, sticky="ew", padx=16, pady=(12, 6))
        header_bar.grid_columnconfigure(0, weight=1)

        ctk.CTkLabel(header_bar, text="🎯 Маршрут точек (Клик-лист)", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, sticky="w")
        
        self.btn_clear_all = ctk.CTkButton(
            header_bar,
            text="🗑️ Очистить всё",
            width=110,
            height=26,
            corner_radius=6,
            fg_color="#dc2626",
            hover_color="#b91c1c",
            command=self._clear_all_points
        )
        self.btn_clear_all.grid(row=0, column=1, sticky="e")

        self.points_container = ctk.CTkFrame(self.list_frame, fg_color="transparent")
        self.points_container.grid(row=1, column=0, sticky="ew", padx=12, pady=(4, 12))
        self.points_container.grid_columnconfigure(0, weight=1)

        # --- Section 3: Navigation & Movement ---
        self.nav_frame = ctk.CTkFrame(self, corner_radius=10, fg_color="#181b24")
        self.nav_frame.grid(row=2, column=0, columnspan=2, sticky="ew", padx=10, pady=6)
        self.nav_frame.grid_columnconfigure((0, 1, 2, 3), weight=1)

        ctk.CTkLabel(self.nav_frame, text="🚗 Порядок обхода и перемещение", font=("Segoe UI", 13, "bold"), text_color="#38bdf8").grid(row=0, column=0, columnspan=4, sticky="w", padx=16, pady=(12, 6))

        ctk.CTkLabel(self.nav_frame, text="Порядок обхода:").grid(row=1, column=0, sticky="w", padx=16, pady=6)
        self.order_selector = ctk.CTkOptionMenu(
            self.nav_frame,
            values=["По порядку (1 → 2 → 3...)", "Случайный выбор точки"],
            width=200,
            fg_color="#2563eb",
            button_color="#1d4ed8"
        )
        self.order_selector.set("По порядку (1 → 2 → 3...)")
        self.order_selector.grid(row=1, column=1, sticky="w", padx=4, pady=6)

        self.smooth_switch = ctk.CTkSwitch(self.nav_frame, text="Плавное ведение курсора", progress_color="#2563eb")
        self.smooth_switch.grid(row=2, column=0, sticky="w", padx=16, pady=(6, 12))

        ctk.CTkLabel(self.nav_frame, text="Время движения (мс):").grid(row=2, column=1, sticky="e", padx=4, pady=(6, 12))
        self.move_speed_entry = ctk.CTkEntry(self.nav_frame, width=70)
        self.move_speed_entry.insert(0, "60")
        self.move_speed_entry.grid(row=2, column=2, sticky="w", padx=4, pady=(6, 12))

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
            text="По числу кругов (циклов):", 
            variable=self.stop_mode_var, 
            value="loops"
        )
        self.radio_count.grid(row=2, column=0, sticky="w", padx=16, pady=(6, 12))

        self.entry_loops = ctk.CTkEntry(self.stop_frame, width=90)
        self.entry_loops.insert(0, "10")
        self.entry_loops.grid(row=2, column=1, sticky="w", padx=4, pady=(6, 12))

        self.radio_time = ctk.CTkRadioButton(
            self.stop_frame, 
            text="По таймеру (сек):", 
            variable=self.stop_mode_var, 
            value="time"
        )
        self.radio_time.grid(row=2, column=2, sticky="w", padx=16, pady=(6, 12))

        self.entry_timer = ctk.CTkEntry(self.stop_frame, width=90)
        self.entry_timer.insert(0, "60")
        self.entry_timer.grid(row=2, column=3, sticky="w", padx=4, pady=(6, 12))

        self._refresh_points_view()

    def _pick_coordinates(self):
        picker = CoordinatePicker(on_picked_callback=self._set_picked_coords)
        picker.start_picking(self.winfo_toplevel())

    def _set_picked_coords(self, x: int, y: int):
        self.entry_x.delete(0, "end")
        self.entry_x.insert(0, str(x))
        self.entry_y.delete(0, "end")
        self.entry_y.insert(0, str(y))

    def _add_point_from_inputs(self):
        try:
            x = int(self.entry_x.get() or 0)
            y = int(self.entry_y.get() or 0)
            btn_txt = self.btn_choice.get()
            btn = "left"
            if "Правая" in btn_txt: btn = "right"
            elif "Средняя" in btn_txt: btn = "middle"
            elif "Двойной" in btn_txt: btn = "double_left"
            
            delay = float(self.entry_delay.get() or 200)
            self.points_data.append({
                "x": x,
                "y": y,
                "button": btn,
                "delay_after_ms": delay,
                "jitter_radius": 0
            })
            self._refresh_points_view()
        except Exception:
            pass

    def _delete_point(self, idx: int):
        if 0 <= idx < len(self.points_data):
            self.points_data.pop(idx)
            self._refresh_points_view()

    def _clear_all_points(self):
        self.points_data = []
        self._refresh_points_view()

    def _refresh_points_view(self):
        for widget in self.points_container.winfo_children():
            widget.destroy()

        if not self.points_data:
            lbl = ctk.CTkLabel(self.points_container, text="Маршрут пуст. Добавьте точки выше.", text_color="#64748b")
            lbl.pack(pady=12)
            return

        for idx, pt in enumerate(self.points_data):
            row_frame = ctk.CTkFrame(self.points_container, fg_color="#202531", corner_radius=8)
            row_frame.pack(fill="x", pady=3, padx=4)
            row_frame.grid_columnconfigure((0, 1, 2, 3), weight=1)

            btn_name = "Левый клик"
            if pt.get("button") == "right": btn_name = "Правый клик"
            elif pt.get("button") == "middle": btn_name = "Средний клик"
            elif pt.get("button") == "double_left": btn_name = "2x Левый клик"

            ctk.CTkLabel(row_frame, text=f"#{idx+1} [X: {pt.get('x',0)}, Y: {pt.get('y',0)}]", font=("Segoe UI", 11, "bold"), text_color="#38bdf8").grid(row=0, column=0, sticky="w", padx=12, pady=6)
            ctk.CTkLabel(row_frame, text=f"{btn_name}", font=("Segoe UI", 11)).grid(row=0, column=1, sticky="w", padx=4, pady=6)
            ctk.CTkLabel(row_frame, text=f"Пауза: {pt.get('delay_after_ms', 200)} мс", font=("Segoe UI", 11), text_color="#94a3b8").grid(row=0, column=2, sticky="w", padx=4, pady=6)

            del_btn = ctk.CTkButton(
                row_frame, 
                text="✕", 
                width=26, 
                height=24, 
                corner_radius=4,
                fg_color="transparent", 
                hover_color="#ef4444",
                command=lambda i=idx: self._delete_point(i)
            )
            del_btn.grid(row=0, column=3, sticky="e", padx=8, pady=6)

    def get_config(self) -> MultiPointConfig:
        cfg = MultiPointConfig()
        cfg.points = list(self.points_data)
        
        ord_txt = self.order_selector.get()
        cfg.order = "random" if "Случайный" in ord_txt else "sequential"
        cfg.smooth_movement = bool(self.smooth_switch.get())
        
        try:
            cfg.move_speed_ms = float(self.move_speed_entry.get() or 60)
        except Exception:
            cfg.move_speed_ms = 60.0

        cfg.stop_condition = self.stop_mode_var.get()
        try:
            cfg.repeat_loops = int(self.entry_loops.get() or 10)
        except Exception:
            cfg.repeat_loops = 10

        try:
            cfg.max_time_seconds = float(self.entry_timer.get() or 60)
        except Exception:
            cfg.max_time_seconds = 60.0

        return cfg

    def set_config(self, cfg_dict: dict):
        if "points" in cfg_dict:
            self.points_data = list(cfg_dict["points"])
            self._refresh_points_view()
        if "order" in cfg_dict:
            if cfg_dict["order"] == "random":
                self.order_selector.set("Случайный выбор точки")
            else:
                self.order_selector.set("По порядку (1 → 2 → 3...)")
        if "smooth_movement" in cfg_dict:
            if cfg_dict["smooth_movement"]: self.smooth_switch.select()
            else: self.smooth_switch.deselect()
        if "move_speed_ms" in cfg_dict:
            self.move_speed_entry.delete(0, "end")
            self.move_speed_entry.insert(0, str(cfg_dict["move_speed_ms"]))
        if "repeat_loops" in cfg_dict:
            self.entry_loops.delete(0, "end")
            self.entry_loops.insert(0, str(cfg_dict["repeat_loops"]))
