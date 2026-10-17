import tkinter as tk
import threading
import time
from typing import Callable, Optional
from pynput import mouse, keyboard
from src.core.win_input import get_mouse_position

class CoordinatePicker:
    def __init__(self, on_picked_callback: Callable[[int, int], None]):
        self.on_picked = on_picked_callback
        self.is_active = False
        self._overlay: Optional[tk.Toplevel] = None
        self._mouse_listener: Optional[mouse.Listener] = None
        self._keyboard_listener: Optional[keyboard.Listener] = None

    def start_picking(self, parent_root: tk.Tk):
        if self.is_active:
            return
        self.is_active = True

        # Create full-screen transparent/semi-transparent overlay with crosshairs
        self._overlay = tk.Toplevel(parent_root)
        self._overlay.attributes("-fullscreen", True)
        self._overlay.attributes("-alpha", 0.3)
        self._overlay.attributes("-topmost", True)
        self._overlay.config(bg="black", cursor="crosshair")

        info_label = tk.Label(
            self._overlay,
            text="Кликните в любую точку экрана или нажмите [F8] для выбора координат\nНажмите [Esc] для отмены",
            font=("Arial", 16, "bold"),
            fg="white",
            bg="black"
        )
        info_label.pack(pady=40)

        coord_label = tk.Label(
            self._overlay,
            text="X: 0, Y: 0",
            font=("Arial", 20, "bold"),
            fg="#00ffcc",
            bg="black"
        )
        coord_label.pack(pady=10)

        def _update_coords():
            if not self.is_active or not self._overlay:
                return
            x, y = get_mouse_position()
            try:
                coord_label.config(text=f"X: {x}, Y: {y}")
                self._overlay.after(30, _update_coords)
            except Exception:
                pass

        self._overlay.after(30, _update_coords)
        
        # Click handler
        def _on_click(event):
            x, y = get_mouse_position()
            self._finish(x, y)

        self._overlay.bind("<Button-1>", _on_click)
        self._overlay.bind("<Escape>", lambda e: self._cancel())
        self._overlay.bind("<F8>", lambda e: _on_click(None))

    def _finish(self, x: int, y: int):
        self._cleanup()
        if self.on_picked:
            self.on_picked(x, y)

    def _cancel(self):
        self._cleanup()

    def _cleanup(self):
        self.is_active = False
        if self._overlay:
            try:
                self._overlay.destroy()
            except Exception:
                pass
            self._overlay = None
