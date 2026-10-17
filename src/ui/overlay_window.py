import customtkinter as ctk
import tkinter as tk
from typing import Callable, Optional

class FloatingOverlay(ctk.CTkToplevel):
    def __init__(self, parent, on_toggle_callback: Callable[[], None]):
        super().__init__(parent)
        self.parent = parent
        self.on_toggle = on_toggle_callback
        
        self.title("AutoClicker HUD")
        self.geometry("240x130+100+100")
        self.attributes("-topmost", True)
        self.attributes("-alpha", 0.92)
        self.overrideredirect(True) # Borderless floating window
        
        self.grid_columnconfigure(0, weight=1)
        self.grid_rowconfigure(1, weight=1)
        
        # Draggable Header
        self.header_frame = ctk.CTkFrame(self, height=28, corner_radius=8, fg_color="#181b24")
        self.header_frame.grid(row=0, column=0, sticky="ew", padx=3, pady=(3, 0))
        self.header_frame.grid_columnconfigure(0, weight=1)
        
        self.title_lbl = ctk.CTkLabel(
            self.header_frame, 
            text="⚡ МИНИ-ТАБЛО HUD", 
            font=("Segoe UI", 11, "bold"),
            text_color="#38bdf8"
        )
        self.title_lbl.grid(row=0, column=0, sticky="w", padx=10)
        
        self.close_btn = ctk.CTkButton(
            self.header_frame, 
            text="✕", 
            width=22, 
            height=22, 
            corner_radius=4,
            fg_color="transparent", 
            hover_color="#ef4444",
            command=self.hide_overlay
        )
        self.close_btn.grid(row=0, column=1, sticky="e", padx=4)
        
        # Drag mechanics
        self._drag_start_x = 0
        self._drag_start_y = 0
        self.header_frame.bind("<ButtonPress-1>", self._start_drag)
        self.header_frame.bind("<B1-Motion>", self._on_drag)
        self.title_lbl.bind("<ButtonPress-1>", self._start_drag)
        self.title_lbl.bind("<B1-Motion>", self._on_drag)
        
        # Body Content
        self.body_frame = ctk.CTkFrame(self, corner_radius=8, fg_color="#12151c")
        self.body_frame.grid(row=1, column=0, sticky="nsew", padx=3, pady=3)
        self.body_frame.grid_columnconfigure((0, 1), weight=1)
        
        # Status Label
        self.status_lbl = ctk.CTkLabel(
            self.body_frame, 
            text="🔴 ОСТАНОВЛЕН", 
            font=("Segoe UI", 12, "bold"),
            text_color="#f87171"
        )
        self.status_lbl.grid(row=0, column=0, columnspan=2, pady=(4, 2))
        
        # CPS and Count Stats
        self.stats_lbl = ctk.CTkLabel(
            self.body_frame, 
            text="CPS: 0.0 | Всего: 0", 
            font=("Segoe UI", 11),
            text_color="#94a3b8"
        )
        self.stats_lbl.grid(row=1, column=0, columnspan=2, pady=(0, 4))
        
        # Toggle Button
        self.action_btn = ctk.CTkButton(
            self.body_frame,
            text="▶ ЗАПУСК [F6]",
            font=("Segoe UI", 11, "bold"),
            height=28,
            corner_radius=6,
            fg_color="#10b981",
            hover_color="#059669",
            command=self.on_toggle
        )
        self.action_btn.grid(row=2, column=0, columnspan=2, padx=12, pady=(0, 6), sticky="ew")

        # Hide initially
        self.withdraw()

    def _start_drag(self, event):
        self._drag_start_x = event.x
        self._drag_start_y = event.y

    def _on_drag(self, event):
        deltax = event.x - self._drag_start_x
        deltay = event.y - self._drag_start_y
        x = self.winfo_x() + deltax
        y = self.winfo_y() + deltay
        self.geometry(f"+{x}+{y}")

    def show_overlay(self):
        self.deiconify()
        self.attributes("-topmost", True)

    def hide_overlay(self):
        self.withdraw()

    def update_state(self, is_running: bool, mode_name: str, count: int, cps: float, hotkey: str = "F6"):
        if is_running:
            self.status_lbl.configure(text=f"🟢 {mode_name.upper()}", text_color="#34d399")
            self.action_btn.configure(text=f"⏹ СТОП [{hotkey.upper()}]", fg_color="#ef4444", hover_color="#dc2626")
        else:
            self.status_lbl.configure(text="🔴 ОСТАНОВЛЕН", text_color="#f87171")
            self.action_btn.configure(text=f"▶ СТАРТ [{hotkey.upper()}]", fg_color="#10b981", hover_color="#059669")

        self.stats_lbl.configure(text=f"CPS: {cps:.1f} | Всего: {count}")
