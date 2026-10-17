import threading
import time
import winsound
from typing import Callable, Optional, Dict
from pynput import keyboard, mouse

def play_audio_feedback(is_start: bool, sound_enabled: bool = False):
    """Plays subtle audio feedback only if explicitly enabled (silent by default)."""
    if not sound_enabled:
        return
    def _play():
        try:
            # Use soft Windows system sounds instead of harsh frequency beeps
            if is_start:
                winsound.MessageBeep(winsound.MB_OK)
            else:
                winsound.MessageBeep(winsound.MB_ICONASTERISK)
        except Exception:
            pass
    threading.Thread(target=_play, daemon=True).start()

class HotkeyManager:
    def __init__(self):
        self.keyboard_listener: Optional[keyboard.Listener] = None
        self.mouse_listener: Optional[mouse.Listener] = None
        self.callbacks: Dict[str, Callable[[], None]] = {}
        
        self.start_stop_key = "f6"
        self.pick_coord_key = "f8"
        self.emergency_stop_key = "f12"
        self.sound_enabled = False # Sound is completely OFF by default
        
        self._pressed_keys = set()
        self._lock = threading.Lock()

    def start(self):
        self.stop()
        self.keyboard_listener = keyboard.Listener(
            on_press=self._on_key_press,
            on_release=self._on_key_release
        )
        self.mouse_listener = mouse.Listener(
            on_click=self._on_mouse_click
        )
        self.keyboard_listener.start()
        self.mouse_listener.start()

    def stop(self):
        if self.keyboard_listener:
            try:
                self.keyboard_listener.stop()
            except Exception:
                pass
            self.keyboard_listener = None
        if self.mouse_listener:
            try:
                self.mouse_listener.stop()
            except Exception:
                pass
            self.mouse_listener = None

    def register_callback(self, action: str, callback: Callable[[], None]):
        self.callbacks[action] = callback

    def _normalize_key(self, key) -> str:
        try:
            if hasattr(key, 'char') and key.char:
                return key.char.lower()
            if hasattr(key, 'name') and key.name:
                return key.name.lower()
            return str(key).replace("Key.", "").lower()
        except Exception:
            return str(key).lower()

    def _on_key_press(self, key):
        k_str = self._normalize_key(key)
        with self._lock:
            self._pressed_keys.add(k_str)

        if k_str == self.start_stop_key.lower():
            if "start_stop" in self.callbacks:
                self.callbacks["start_stop"]()
                
        elif k_str == self.pick_coord_key.lower():
            if "pick_coord" in self.callbacks:
                self.callbacks["pick_coord"]()
                
        elif k_str == self.emergency_stop_key.lower():
            if "emergency_stop" in self.callbacks:
                self.callbacks["emergency_stop"]()

    def _on_key_release(self, key):
        k_str = self._normalize_key(key)
        with self._lock:
            self._pressed_keys.discard(k_str)

    def _on_mouse_click(self, x, y, button, pressed):
        if not pressed:
            return
        btn_str = ""
        if button == mouse.Button.x1:
            btn_str = "mouse_x1"
        elif button == mouse.Button.x2:
            btn_str = "mouse_x2"
        elif button == mouse.Button.middle:
            btn_str = "mouse_middle"

        if btn_str and btn_str == self.start_stop_key.lower():
            if "start_stop" in self.callbacks:
                self.callbacks["start_stop"]()
