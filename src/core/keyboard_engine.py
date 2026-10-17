import threading
import time
import random
from dataclasses import dataclass, field
from typing import Callable, Optional, List

from src.core.win_input import (
    key_press,
    key_down,
    key_up,
    key_combo,
    send_text_unicode
)
from src.core.precision_timer import precision_sleep, FrequencyMeter

@dataclass
class KeySequenceStep:
    key: str
    delay_after_ms: float = 100.0

@dataclass
class KeyboardConfig:
    mode: str = "single"               # "single", "combo", "sequence", "text"
    key: str = "space"                 # Single key
    action: str = "tap"                # "tap", "hold"
    combo_keys: List[str] = field(default_factory=lambda: ["ctrl", "c"])
    sequence_steps: List[dict] = field(default_factory=list) # [{"key": "1", "delay_ms": 100}]
    text_content: str = "Hello World!"
    interval_ms: float = 100.0         # Repeat interval for single/combo/sequence/text
    random_delay_ms: float = 0.0       # Jitter in delay
    char_delay_ms: float = 20.0        # Delay between characters in text mode
    stop_condition: str = "infinite"   # "infinite", "count", "time"
    repeat_count: int = 100
    max_time_seconds: float = 60.0

class KeyboardEngine:
    def __init__(self, on_update_callback: Optional[Callable[[int, float], None]] = None):
        self.config = KeyboardConfig()
        self.is_running = False
        self._thread: Optional[threading.Thread] = None
        self._stop_event = threading.Event()
        self.total_presses = 0
        self.freq_meter = FrequencyMeter(window_size=0.5)
        self.on_update = on_update_callback
        self.start_time = 0.0

    def start(self, config: Optional[KeyboardConfig] = None):
        if self.is_running:
            return
        if config:
            self.config = config
            
        self.is_running = True
        self._stop_event.clear()
        self.total_presses = 0
        self.freq_meter = FrequencyMeter(window_size=0.5)
        self.start_time = time.perf_counter()
        
        self._thread = threading.Thread(target=self._worker, daemon=True)
        self._thread.start()

    def stop(self):
        if not self.is_running:
            return
        self._stop_event.set()
        self.is_running = False
        
        # If in hold mode, release the key
        if self.config.mode == "single" and self.config.action == "hold":
            key_up(self.config.key)
            
        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=0.3)
        self._thread = None
        
        if self.on_update:
            self.on_update(self.total_presses, 0.0)

    def _worker(self):
        last_callback_time = time.perf_counter()
        
        # Single key hold mode
        if self.config.mode == "single" and self.config.action == "hold":
            key_down(self.config.key)
            while not self._stop_event.is_set():
                if self.config.stop_condition == "time":
                    if time.perf_counter() - self.start_time >= self.config.max_time_seconds:
                        break
                precision_sleep(0.01)
            key_up(self.config.key)
            self.is_running = False
            return

        while not self._stop_event.is_set():
            # Stop condition checks
            if self.config.stop_condition == "count" and self.total_presses >= self.config.repeat_count:
                break
            if self.config.stop_condition == "time":
                if time.perf_counter() - self.start_time >= self.config.max_time_seconds:
                    break

            if self.config.mode == "single":
                key_press(self.config.key)
                self.total_presses += 1
                self.freq_meter.record_hit()
                
            elif self.config.mode == "combo":
                key_combo(self.config.combo_keys)
                self.total_presses += 1
                self.freq_meter.record_hit()
                
            elif self.config.mode == "sequence":
                for step in self.config.sequence_steps:
                    if self._stop_event.is_set():
                        break
                    k = step.get("key", "")
                    delay = step.get("delay_ms", 50.0) / 1000.0
                    if k:
                        key_press(k)
                        self.total_presses += 1
                        self.freq_meter.record_hit()
                    precision_sleep(delay)
                    
            elif self.config.mode == "text":
                send_text_unicode(self.config.text_content, char_delay=self.config.char_delay_ms / 1000.0)
                self.total_presses += len(self.config.text_content)
                self.freq_meter.record_hit()

            now = time.perf_counter()
            if now - last_callback_time >= 0.05:
                last_callback_time = now
                if self.on_update:
                    self.on_update(self.total_presses, self.freq_meter.get_current_frequency())

            # Interval delay between cycles
            base_delay = self.config.interval_ms / 1000.0
            if self.config.random_delay_ms > 0:
                jitter_sec = random.uniform(-self.config.random_delay_ms, self.config.random_delay_ms) / 1000.0
                delay = max(0.001, base_delay + jitter_sec)
            else:
                delay = max(0.001, base_delay)

            precision_sleep(delay)

        self.is_running = False
        if self.on_update:
            self.on_update(self.total_presses, 0.0)
