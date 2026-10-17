import threading
import time
import json
from dataclasses import dataclass, field
from typing import Callable, Optional, List, Dict, Any

from pynput import mouse, keyboard

from src.core.win_input import (
    mouse_down,
    mouse_up,
    mouse_scroll_vertical,
    mouse_scroll_horizontal,
    set_mouse_position,
    key_down,
    key_up
)
from src.core.precision_timer import precision_sleep

@dataclass
class MacroConfig:
    actions: List[Dict[str, Any]] = field(default_factory=list)
    speed_multiplier: float = 1.0
    repeat_loops: int = 1         # 0 = infinite
    stop_condition: str = "loops" # "loops", "infinite", "time"
    max_time_seconds: float = 60.0

class MacroRecorder:
    def __init__(self, record_mouse_moves: bool = False):
        self.record_mouse_moves = record_mouse_moves
        self.is_recording = False
        self.actions: List[Dict[str, Any]] = []
        self._last_time = 0.0
        self._mouse_listener: Optional[mouse.Listener] = None
        self._keyboard_listener: Optional[keyboard.Listener] = None

    def start(self, record_mouse_moves: bool = False):
        if self.is_recording:
            return
        self.record_mouse_moves = record_mouse_moves
        self.actions = []
        self.is_recording = True
        self._last_time = time.perf_counter()

        self._mouse_listener = mouse.Listener(
            on_move=self._on_mouse_move if self.record_mouse_moves else None,
            on_click=self._on_mouse_click,
            on_scroll=self._on_mouse_scroll
        )
        self._keyboard_listener = keyboard.Listener(
            on_press=self._on_key_press,
            on_release=self._on_key_release
        )

        self._mouse_listener.start()
        self._keyboard_listener.start()

    def stop(self) -> List[Dict[str, Any]]:
        if not self.is_recording:
            return self.actions
            
        self.is_recording = False
        if self._mouse_listener:
            self._mouse_listener.stop()
            self._mouse_listener = None
        if self._keyboard_listener:
            self._keyboard_listener.stop()
            self._keyboard_listener = None

        return self.actions

    def _record_event(self, action_type: str, data: Dict[str, Any]):
        now = time.perf_counter()
        delay_sec = max(0.0, now - self._last_time)
        self._last_time = now
        
        event = {
            "type": action_type,
            "delay_ms": round(delay_sec * 1000.0, 1),
            **data
        }
        self.actions.append(event)

    def _on_mouse_move(self, x: int, y: int):
        self._record_event("mouse_move", {"x": int(x), "y": int(y)})

    def _on_mouse_click(self, x: int, y: int, button: mouse.Button, pressed: bool):
        btn_str = "left"
        if button == mouse.Button.right:
            btn_str = "right"
        elif button == mouse.Button.middle:
            btn_str = "middle"
        elif button == mouse.Button.x1:
            btn_str = "x1"
        elif button == mouse.Button.x2:
            btn_str = "x2"

        action_type = "mouse_down" if pressed else "mouse_up"
        self._record_event(action_type, {"button": btn_str, "x": int(x), "y": int(y)})

    def _on_mouse_scroll(self, x: int, y: int, dx: int, dy: int):
        self._record_event("mouse_scroll", {"x": int(x), "y": int(y), "dx": int(dx), "dy": int(dy)})

    def _normalize_key(self, key) -> str:
        try:
            if hasattr(key, 'char') and key.char:
                return key.char
            if hasattr(key, 'name'):
                return key.name
            return str(key).replace("Key.", "")
        except Exception:
            return str(key)

    def _on_key_press(self, key):
        k_str = self._normalize_key(key)
        self._record_event("key_down", {"key": k_str})

    def _on_key_release(self, key):
        k_str = self._normalize_key(key)
        self._record_event("key_up", {"key": k_str})


class MacroPlayer:
    def __init__(self, on_update_callback: Optional[Callable[[int, int], None]] = None):
        self.config = MacroConfig()
        self.is_playing = False
        self._thread: Optional[threading.Thread] = None
        self._stop_event = threading.Event()
        self.completed_loops = 0
        self.current_step = 0
        self.on_update = on_update_callback
        self.start_time = 0.0

    def start(self, config: Optional[MacroConfig] = None):
        if self.is_playing:
            return
        if config:
            self.config = config
        if not self.config.actions:
            return

        self.is_playing = True
        self._stop_event.clear()
        self.completed_loops = 0
        self.current_step = 0
        self.start_time = time.perf_counter()

        self._thread = threading.Thread(target=self._worker, daemon=True)
        self._thread.start()

    def stop(self):
        if not self.is_playing:
            return
        self._stop_event.set()
        self.is_playing = False
        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=0.3)
        self._thread = None
        if self.on_update:
            self.on_update(self.completed_loops, self.current_step)

    def _worker(self):
        speed = max(0.1, self.config.speed_multiplier)
        
        while not self._stop_event.is_set():
            if self.config.stop_condition == "loops" and self.config.repeat_loops > 0:
                if self.completed_loops >= self.config.repeat_loops:
                    break
            if self.config.stop_condition == "time":
                if time.perf_counter() - self.start_time >= self.config.max_time_seconds:
                    break

            for idx, action in enumerate(self.config.actions):
                if self._stop_event.is_set():
                    break

                self.current_step = idx
                
                # Apply delay
                delay_ms = float(action.get("delay_ms", 0.0)) / speed
                if delay_ms > 0:
                    precision_sleep(delay_ms / 1000.0)

                if self._stop_event.is_set():
                    break

                act_type = action.get("type", "")
                if act_type == "mouse_move":
                    set_mouse_position(int(action.get("x", 0)), int(action.get("y", 0)))
                elif act_type == "mouse_down":
                    set_mouse_position(int(action.get("x", 0)), int(action.get("y", 0)))
                    mouse_down(action.get("button", "left"))
                elif act_type == "mouse_up":
                    set_mouse_position(int(action.get("x", 0)), int(action.get("y", 0)))
                    mouse_up(action.get("button", "left"))
                elif act_type == "mouse_scroll":
                    dy = int(action.get("dy", 0))
                    dx = int(action.get("dx", 0))
                    if dy != 0:
                        mouse_scroll_vertical(dy * 120)
                    if dx != 0:
                        mouse_scroll_horizontal(dx * 120)
                elif act_type == "key_down":
                    key_down(action.get("key", ""))
                elif act_type == "key_up":
                    key_up(action.get("key", ""))

                if self.on_update and idx % 5 == 0:
                    self.on_update(self.completed_loops, self.current_step)

            self.completed_loops += 1
            if self.on_update:
                self.on_update(self.completed_loops, self.current_step)

        self.is_playing = False
        if self.on_update:
            self.on_update(self.completed_loops, self.current_step)
