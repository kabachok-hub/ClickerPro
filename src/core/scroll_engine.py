import threading
import time
import random
from dataclasses import dataclass
from typing import Callable, Optional

from src.core.win_input import (
    mouse_scroll_vertical,
    mouse_scroll_horizontal,
    set_mouse_position
)
from src.core.precision_timer import precision_sleep, FrequencyMeter

@dataclass
class ScrollConfig:
    direction: str = "down"            # "up", "down", "left", "right"
    ticks: int = 1                     # Notches per step (1 notch = 120 win32 delta)
    interval_ms: float = 50.0          # Delay between scroll events
    random_delay_ms: float = 0.0       # Jitter in delay
    smooth: bool = False               # Micro-step smooth scroll
    location_type: str = "cursor"      # "cursor", "fixed"
    fixed_x: int = 0
    fixed_y: int = 0
    stop_condition: str = "infinite"   # "infinite", "count", "time"
    repeat_count: int = 100
    max_time_seconds: float = 60.0

class ScrollEngine:
    def __init__(self, on_update_callback: Optional[Callable[[int, float], None]] = None):
        self.config = ScrollConfig()
        self.is_running = False
        self._thread: Optional[threading.Thread] = None
        self._stop_event = threading.Event()
        self.total_scrolls = 0
        self.freq_meter = FrequencyMeter(window_size=0.5)
        self.on_update = on_update_callback
        self.start_time = 0.0

    def start(self, config: Optional[ScrollConfig] = None):
        if self.is_running:
            return
        if config:
            self.config = config
            
        self.is_running = True
        self._stop_event.clear()
        self.total_scrolls = 0
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
            self.on_update(self.total_scrolls, 0.0)

    def _worker(self):
        last_callback_time = time.perf_counter()
        
        while not self._stop_event.is_set():
            # Stop condition checks
            if self.config.stop_condition == "count" and self.total_scrolls >= self.config.repeat_count:
                break
            if self.config.stop_condition == "time":
                if time.perf_counter() - self.start_time >= self.config.max_time_seconds:
                    break

            # Fixed coordinate positioning if enabled
            if self.config.location_type == "fixed":
                set_mouse_position(self.config.fixed_x, self.config.fixed_y)

            # Delta calculation
            wheel_delta = int(self.config.ticks * 120)
            
            if self.config.smooth:
                # Break down delta into smooth mini-steps
                steps = max(2, self.config.ticks * 4)
                sub_delta = int(wheel_delta / steps)
                for _ in range(steps):
                    if self._stop_event.is_set():
                        break
                    if self.config.direction == "up":
                        mouse_scroll_vertical(sub_delta)
                    elif self.config.direction == "down":
                        mouse_scroll_vertical(-sub_delta)
                    elif self.config.direction == "right":
                        mouse_scroll_horizontal(sub_delta)
                    elif self.config.direction == "left":
                        mouse_scroll_horizontal(-sub_delta)
                    precision_sleep(0.005)
            else:
                if self.config.direction == "up":
                    mouse_scroll_vertical(wheel_delta)
                elif self.config.direction == "down":
                    mouse_scroll_vertical(-wheel_delta)
                elif self.config.direction == "right":
                    mouse_scroll_horizontal(wheel_delta)
                elif self.config.direction == "left":
                    mouse_scroll_horizontal(-wheel_delta)

            self.total_scrolls += self.config.ticks
            self.freq_meter.record_hit()

            now = time.perf_counter()
            if now - last_callback_time >= 0.05:
                last_callback_time = now
                if self.on_update:
                    self.on_update(self.total_scrolls, self.freq_meter.get_current_frequency())

            base_delay = self.config.interval_ms / 1000.0
            if self.config.random_delay_ms > 0:
                jitter_sec = random.uniform(-self.config.random_delay_ms, self.config.random_delay_ms) / 1000.0
                delay = max(0.001, base_delay + jitter_sec)
            else:
                delay = max(0.001, base_delay)

            precision_sleep(delay)

        self.is_running = False
        if self.on_update:
            self.on_update(self.total_scrolls, 0.0)
