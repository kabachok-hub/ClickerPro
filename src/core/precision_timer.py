import ctypes
import time

try:
    winmm = ctypes.windll.winmm
    # Set Windows timer resolution to 1ms
    winmm.timeBeginPeriod(1)
except Exception:
    winmm = None

def cleanup_timer():
    if winmm:
        try:
            winmm.timeEndPeriod(1)
        except Exception:
            pass

def precision_sleep(duration_seconds: float):
    """
    High-precision hybrid sleep.
    Uses time.sleep() for bulk duration and busy-waits for the last few milliseconds.
    """
    if duration_seconds <= 0:
        return
    
    start = time.perf_counter()
    end = start + duration_seconds
    
    # If sleep time is larger than 2ms, sleep for most of it
    if duration_seconds > 0.002:
        time.sleep(duration_seconds - 0.0015)
        
    # Busy wait for remainder with microsecond precision
    while time.perf_counter() < end:
        pass

class FrequencyMeter:
    """Calculates actual CPS (clicks per second) in real-time."""
    def __init__(self, window_size: float = 1.0):
        self.window_size = window_size
        self.timestamps = []
        
    def record_hit(self):
        now = time.perf_counter()
        self.timestamps.append(now)
        # Drop timestamps older than window_size
        cutoff = now - self.window_size
        while self.timestamps and self.timestamps[0] < cutoff:
            self.timestamps.pop(0)
            
    def get_current_frequency(self) -> float:
        now = time.perf_counter()
        cutoff = now - self.window_size
        while self.timestamps and self.timestamps[0] < cutoff:
            self.timestamps.pop(0)
        return len(self.timestamps) / self.window_size
