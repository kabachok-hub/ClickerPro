import threading
import time
import random
from dataclasses import dataclass, field
from typing import Callable, Optional, List

from src.core.win_input import (
    mouse_click,
    mouse_double_click,
    get_mouse_position,
    set_mouse_position
)
from src.core.precision_timer import precision_sleep, FrequencyMeter

@dataclass
class ClickPoint:
    x: int
    y: int
    button: str = "left"          # "left", "right", "middle", "double_left"
    delay_after_ms: float = 200.0 # Delay after click in ms
    jitter_radius: int = 0        # Random jitter around point in px

@dataclass
class MultiPointConfig:
    points: List[dict] = field(default_factory=list) # [{"x": 100, "y": 200, "button": "left", "delay_after_ms": 200, "jitter_radius": 0}]
    order: str = "sequential"     # "sequential", "random"
    smooth_movement: bool = False # Move smoothly between points
    move_speed_ms: float = 50.0   # Duration of smooth movement
    stop_condition: str = "infinite" # "infinite", "loops", "time"
    repeat_loops: int = 10
    max_time_seconds: float = 60.0

def _smooth_move(start_x: int, start_y: int, end_x: int, end_y: int, duration_sec: float, stop_event: threading.Event):
    if duration_sec <= 0.005:
        set_mouse_position(end_x, end_y)
        return
    steps = max(5, int(duration_sec / 0.008))
    step_duration = duration_sec / steps
    
    for i in range(1, steps + 1):
        if stop_event.is_set():
            break
        t = i / steps
        # Smooth ease-in-out curve: 3*t^2 - 2*t^3
        ease_t = 3 * (t ** 2) - 2 * (t ** 3)
        cur_x = int(start_x + (end_x - start_x) * ease_t)
        cur_y = int(start_y + (end_y - start_y) * ease_t)
        set_mouse_position(cur_x, cur_y)
        precision_sleep(step_duration)

class MultiPointEngine:
    def __init__(self, on_update_callback: Optional[Callable[[int, int, float], None]] = None):
        self.config = MultiPointConfig()
        self.is_running = False
        self._thread: Optional[threading.Thread] = None
        self._stop_event = threading.Event()
        self.total_clicks = 0
        self.completed_loops = 0
        self.freq_meter = FrequencyMeter(window_size=0.5)
        self.on_update = on_update_callback
        self.start_time = 0.0

    def start(self, config: Optional[MultiPointConfig] = None):
        if self.is_running:
            return
        if config:
            self.config = config
            
        if not self.config.points:
            return
            
        self.is_running = True
        self._stop_event.clear()
        self.total_clicks = 0
        self.completed_loops = 0
        self.freq_meter = FrequencyMeter(window_size=0.5)
        self.start_time = time.perf_counter()
        
        self._thread = threading.Thread(target=self._worker, daemon=True)
        self._thread.start()

    def stop(self):
        if not self.is_running:
            return
        self._stop_event.set()
        self.is_running = False
        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=0.3)
        self._thread = None
        
        if self.on_update:
            self.on_update(self.total_clicks, self.completed_loops, 0.0)

    def _worker(self):
        last_callback_time = time.perf_counter()
        
        while not self._stop_event.is_set():
            # Check stop conditions
            if self.config.stop_condition == "loops" and self.completed_loops >= self.config.repeat_loops:
                break
            if self.config.stop_condition == "time":
                if time.perf_counter() - self.start_time >= self.config.max_time_seconds:
                    break

            points_to_visit = list(self.config.points)
            if self.config.order == "random":
                random.shuffle(points_to_visit)

            for pt in points_to_visit:
                if self._stop_event.is_set():
                    break
                    
                target_x = int(pt.get("x", 0))
                target_y = int(pt.get("y", 0))
                jitter = int(pt.get("jitter_radius", 0))
                
                if jitter > 0:
                    target_x += random.randint(-jitter, jitter)
                    target_y += random.randint(-jitter, jitter)
                    
                button = pt.get("button", "left").lower()
                delay_after = float(pt.get("delay_after_ms", 200.0)) / 1000.0
                
                # Move to target
                if self.config.smooth_movement:
                    curr_x, curr_y = get_mouse_position()
                    move_duration = max(0.01, self.config.move_speed_ms / 1000.0)
                    _smooth_move(curr_x, curr_y, target_x, target_y, move_duration, self._stop_event)
                else:
                    set_mouse_position(target_x, target_y)

                if self._stop_event.is_set():
                    break

                # Perform Click
                if button == "double_left":
                    mouse_double_click("left")
                    self.total_clicks += 2
                elif button == "right":
                    mouse_click("right")
                    self.total_clicks += 1
                elif button == "middle":
                    mouse_click("middle")
                    self.total_clicks += 1
                else:
                    mouse_click("left")
                    self.total_clicks += 1

                self.freq_meter.record_hit()

                now = time.perf_counter()
                if now - last_callback_time >= 0.05:
                    last_callback_time = now
                    if self.on_update:
                        self.on_update(self.total_clicks, self.completed_loops, self.freq_meter.get_current_frequency())

                precision_sleep(max(0.001, delay_after))

            self.completed_loops += 1

        self.is_running = False
        if self.on_update:
            self.on_update(self.total_clicks, self.completed_loops, 0.0)
