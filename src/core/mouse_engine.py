import threading
import time
import random
from dataclasses import dataclass, field
from typing import Callable, Optional

from src.core.win_input import (
    mouse_click,
    mouse_double_click,
    mouse_triple_click,
    mouse_down,
    mouse_up,
    get_mouse_position,
    set_mouse_position,
)
from src.core.precision_timer import precision_sleep, FrequencyMeter

@dataclass
class MouseConfig:
    button: str = "left"               # "left", "right", "middle", "x1", "x2"
    click_type: str = "single"         # "single", "double", "triple", "hold"
    mode: str = "cps"                  # "cps" or "interval"
    cps: float = 10.0                  # Clicks per second
    interval_ms: float = 100.0         # Custom interval in ms
    random_delay_ms: float = 0.0       # Jitter in delay (± ms)
    location_type: str = "cursor"      # "cursor", "fixed", "jitter"
    fixed_x: int = 0
    fixed_y: int = 0
    jitter_radius: int = 0             # Pixel scatter
    stop_condition: str = "infinite"   # "infinite", "count", "time"
    repeat_count: int = 100
    max_time_seconds: float = 60.0

class MouseEngine:
    def __init__(self, on_update_callback: Optional[Callable[[int, float], None]] = None):
        self.config = MouseConfig()
        self.is_running = False
        self._thread: Optional[threading.Thread] = None
        self._stop_event = threading.Event()
        self.total_clicks = 0
        self.freq_meter = FrequencyMeter(window_size=0.5)
        self.on_update = on_update_callback
        self.start_time = 0.0

    def start(self, config: Optional[MouseConfig] = None):
        if self.is_running:
            return
        if config:
            self.config = config
        
        self.is_running = True
        self._stop_event.clear()
        self.total_clicks = 0
        self.freq_meter = FrequencyMeter(window_size=0.5)
        self.start_time = time.perf_counter()
        
        self._thread = threading.Thread(target=self._worker, daemon=True)
        self._thread.start()

    def stop(self):
        if not self.is_running:
            return
        self._stop_event.set()
        self.is_running = False
        
        # If in hold mode, make sure we release the mouse button
        if self.config.click_type == "hold":
            mouse_up(self.config.button)
            
        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=0.3)
        self._thread = None
        
        if self.on_update:
            self.on_update(self.total_clicks, 0.0)

    def _worker(self):
        last_callback_time = time.perf_counter()
        
        # If hold mode is selected
        if self.config.click_type == "hold":
            if self.config.location_type == "fixed":
                set_mouse_position(self.config.fixed_x, self.config.fixed_y)
            mouse_down(self.config.button)
            while not self._stop_event.is_set():
                if self.config.stop_condition == "time":
                    if time.perf_counter() - self.start_time >= self.config.max_time_seconds:
                        break
                precision_sleep(0.01)
            mouse_up(self.config.button)
            self.is_running = False
            return

        while not self._stop_event.is_set():
            # Check stop conditions
            if self.config.stop_condition == "count" and self.total_clicks >= self.config.repeat_count:
                break
            if self.config.stop_condition == "time":
                if time.perf_counter() - self.start_time >= self.config.max_time_seconds:
                    break

            # Handle location
            if self.config.location_type == "fixed":
                target_x = self.config.fixed_x
                target_y = self.config.fixed_y
                if self.config.jitter_radius > 0:
                    target_x += random.randint(-self.config.jitter_radius, self.config.jitter_radius)
                    target_y += random.randint(-self.config.jitter_radius, self.config.jitter_radius)
                set_mouse_position(target_x, target_y)
            elif self.config.location_type == "jitter" and self.config.jitter_radius > 0:
                cur_x, cur_y = get_mouse_position()
                set_mouse_position(
                    cur_x + random.randint(-self.config.jitter_radius, self.config.jitter_radius),
                    cur_y + random.randint(-self.config.jitter_radius, self.config.jitter_radius)
                )

            # Perform Click
            if self.config.click_type == "single":
                mouse_click(self.config.button, hold_time=0.0005)
                self.total_clicks += 1
            elif self.config.click_type == "double":
                mouse_double_click(self.config.button, delay_between=0.02)
                self.total_clicks += 2
            elif self.config.click_type == "triple":
                mouse_triple_click(self.config.button, delay_between=0.02)
                self.total_clicks += 3

            self.freq_meter.record_hit()

            # Trigger UI callback periodically (~20Hz)
            now = time.perf_counter()
            if now - last_callback_time >= 0.05:
                last_callback_time = now
                if self.on_update:
                    self.on_update(self.total_clicks, self.freq_meter.get_current_frequency())

            # Calculate sleep duration
            if self.config.mode == "cps":
                base_delay = 1.0 / max(0.1, self.config.cps)
            else:
                base_delay = self.config.interval_ms / 1000.0

            if self.config.random_delay_ms > 0:
                jitter_sec = (random.uniform(-self.config.random_delay_ms, self.config.random_delay_ms)) / 1000.0
                delay = max(0.0001, base_delay + jitter_sec)
            else:
                delay = max(0.0001, base_delay)

            precision_sleep(delay)

        self.is_running = False
        if self.on_update:
            self.on_update(self.total_clicks, 0.0)
