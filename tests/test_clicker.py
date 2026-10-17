import time
import threading
from src.core.win_input import get_mouse_position, set_mouse_position, mouse_click, key_press
from src.core.precision_timer import precision_sleep, FrequencyMeter
from src.core.mouse_engine import MouseEngine, MouseConfig
from src.core.keyboard_engine import KeyboardEngine, KeyboardConfig
from src.core.scroll_engine import ScrollEngine, ScrollConfig
from src.core.multipoint_engine import MultiPointEngine, MultiPointConfig
from src.core.profile_manager import ProfileManager

def test_precision_timer():
    print("[TEST] Testing precision timer for 10ms delay...")
    start = time.perf_counter()
    for _ in range(10):
        precision_sleep(0.01) # 10ms
    duration = time.perf_counter() - start
    avg = (duration / 10) * 1000
    print(f"[PASS] Precision Timer avg sleep: {avg:.2f} ms (expected ~10.0ms)")
    assert 8.0 <= avg <= 15.0, f"Sleep precision outside acceptable range: {avg}ms"

def test_mouse_position():
    print("[TEST] Testing mouse coordinates retrieval...")
    x, y = get_mouse_position()
    print(f"[PASS] Current mouse position: ({x}, {y})")
    assert isinstance(x, int) and isinstance(y, int)

def test_mouse_engine_count_limit():
    print("[TEST] Testing Mouse Engine with 20 clicks count limit...")
    updates = []
    def on_upd(count, cps):
        updates.append((count, cps))
        
    engine = MouseEngine(on_update_callback=on_upd)
    cfg = MouseConfig(
        button="left",
        click_type="single",
        mode="cps",
        cps=100.0,
        stop_condition="count",
        repeat_count=20
    )
    engine.start(cfg)
    time.sleep(0.4) # Wait for completion
    engine.stop()
    print(f"[PASS] Mouse Engine total clicks: {engine.total_clicks}")
    assert engine.total_clicks == 20

def test_keyboard_engine_count_limit():
    print("[TEST] Testing Keyboard Engine with 10 count limit...")
    engine = KeyboardEngine()
    cfg = KeyboardConfig(
        mode="single",
        key="space",
        interval_ms=10.0,
        stop_condition="count",
        repeat_count=10
    )
    engine.start(cfg)
    time.sleep(0.3)
    engine.stop()
    print(f"[PASS] Keyboard Engine total presses: {engine.total_presses}")
    assert engine.total_presses == 10

def test_profiles():
    print("[TEST] Testing Profile Manager...")
    pm = ProfileManager()
    profiles = pm.list_profiles()
    print(f"[PASS] Found profiles: {profiles}")
    assert len(profiles) > 0

if __name__ == "__main__":
    test_precision_timer()
    test_mouse_position()
    test_mouse_engine_count_limit()
    test_keyboard_engine_count_limit()
    test_profiles()
    print("\n>>> ALL TESTS PASSED SUCCESSFULLY! <<<")
