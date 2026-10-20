import time
import threading
import keyboard
from win_input import (
    hardware_click,
    get_cursor_pos,
    play_sound_start,
    play_sound_stop,
    play_sound_click,
    ensure_desktop_access,
)
from detector import PopupDetector

class ClickerEngine:
    def __init__(self):
        self.detector = PopupDetector()
        
        # State
        self.is_hunter_active = False
        self.is_mouse_active = False
        
        # Auto-hunter config
        self.hunter_mode = "roblox_auto"
        self.hunter_threshold = 0.70
        self.hunter_interval_ms = 40
        self.hunter_hold_ms = 45
        self.hunter_return_cursor = True
        self.hunter_sound = True
        self.hunter_region = None  # (x1, y1, x2, y2) or None
        self.hunter_cooldown_ms = 180  # Cooldown after clicking to allow popup to vanish
        
        # Normal mouse config
        self.mouse_button = "left"
        self.mouse_click_type = "single"
        self.mouse_interval_ms = 100
        self.mouse_fixed_pos = None  # (x, y) or None (follow cursor)
        
        # Stats
        self.total_hunter_clicks = 0
        self.last_target = None
        
        # Callbacks for UI updates
        self.on_state_changed = None
        self.on_target_found = None
        self.on_log = None
        
        # Worker threads
        self.stop_event = threading.Event()
        self.hunter_thread = None
        self.mouse_thread = None
        
        # Global hotkey bindings
        self.hotkey_hunter = "F6"
        self.hotkey_mouse = "F7"
        self.hotkey_panic = "F12"
        self.setup_hotkeys()

    def log(self, msg):
        print(f"[Engine] {msg}")
        if self.on_log:
            try:
                self.on_log(msg)
            except Exception:
                pass

    def setup_hotkeys(self):
        try:
            keyboard.unhook_all()
            keyboard.add_hotkey(self.hotkey_hunter, self.toggle_hunter)
            keyboard.add_hotkey(self.hotkey_mouse, self.toggle_mouse)
            keyboard.add_hotkey(self.hotkey_panic, self.panic_stop)
        except Exception as e:
            self.log(f"Hotkey setup warning: {e}")

    def update_hotkeys(self, hunter_key, mouse_key, panic_key):
        self.hotkey_hunter = hunter_key
        self.hotkey_mouse = mouse_key
        self.hotkey_panic = panic_key
        self.setup_hotkeys()

    def toggle_hunter(self):
        if self.is_hunter_active:
            self.stop_hunter()
        else:
            self.start_hunter()

    def start_hunter(self):
        if self.is_hunter_active:
            return
        self.is_hunter_active = True
        if self.hunter_sound:
            play_sound_start()
        self.log(f"🎯 Авто-охотник ЗАПУЩЕН [{self.hotkey_hunter}] (Режим: {self.hunter_mode})")
        if self.on_state_changed:
            self.on_state_changed()
            
        self.hunter_thread = threading.Thread(target=self._hunter_loop, daemon=True)
        self.hunter_thread.start()

    def stop_hunter(self):
        if not self.is_hunter_active:
            return
        self.is_hunter_active = False
        if self.hunter_sound:
            play_sound_stop()
        self.log(f"⏹️ Авто-охотник ОСТАНОВЛЕН [{self.hotkey_hunter}]")
        if self.on_state_changed:
            self.on_state_changed()

    def _hunter_loop(self):
        ensure_desktop_access()
        while self.is_hunter_active and not self.stop_event.is_set():
            t0 = time.perf_counter()
            try:
                result, _ = self.detector.detect_popup(
                    mode=self.hunter_mode,
                    threshold=self.hunter_threshold,
                    region=self.hunter_region,
                )
                
                if result is not None and self.is_hunter_active:
                    tx, ty = result["screen_center"]
                    conf = result["confidence"]
                    method = result["method"]
                    
                    self.last_target = (tx, ty)
                    self.total_hunter_clicks += 1
                    
                    self.log(f"🟢 ЦЕЛЬ НАЙДЕНА: ({tx}, {ty}) | Метод: {method} | Уверенность: {conf*100:.1f}%")
                    
                    # Hardware click in game
                    hardware_click(
                        x=tx,
                        y=ty,
                        button="left",
                        click_type="single",
                        hold_ms=self.hunter_hold_ms,
                        return_cursor=self.hunter_return_cursor,
                        pre_hover_ms=25,
                        post_release_ms=25,
                    )
                    
                    if self.hunter_sound:
                        play_sound_click()
                        
                    if self.on_target_found:
                        try:
                            self.on_target_found(tx, ty, method, conf)
                        except Exception:
                            pass
                            
                    # Cooldown so button has time to disappear
                    time.sleep(self.hunter_cooldown_ms / 1000.0)
                    continue

            except Exception as e:
                self.log(f"Ошибка цикла охотника: {e}")
                time.sleep(0.1)

            # Sleep remaining scan interval
            elapsed = (time.perf_counter() - t0) * 1000.0
            sleep_ms = max(5.0, self.hunter_interval_ms - elapsed)
            time.sleep(sleep_ms / 1000.0)

    def toggle_mouse(self):
        if self.is_mouse_active:
            self.stop_mouse()
        else:
            self.start_mouse()

    def start_mouse(self):
        if self.is_mouse_active:
            return
        self.is_mouse_active = True
        if self.hunter_sound:
            play_sound_start()
        self.log(f"🖱️ Обычный кликер ЗАПУЩЕН [{self.hotkey_mouse}] (Интервал: {self.mouse_interval_ms} мс)")
        if self.on_state_changed:
            self.on_state_changed()
            
        self.mouse_thread = threading.Thread(target=self._mouse_loop, daemon=True)
        self.mouse_thread.start()

    def stop_mouse(self):
        if not self.is_mouse_active:
            return
        self.is_mouse_active = False
        if self.hunter_sound:
            play_sound_stop()
        self.log(f"⏹️ Обычный кликер ОСТАНОВЛЕН [{self.hotkey_mouse}]")
        if self.on_state_changed:
            self.on_state_changed()

    def _mouse_loop(self):
        while self.is_mouse_active and not self.stop_event.is_set():
            t0 = time.perf_counter()
            try:
                if self.mouse_fixed_pos:
                    cx, cy = self.mouse_fixed_pos
                else:
                    cx, cy = get_cursor_pos()
                    
                hardware_click(
                    x=cx,
                    y=cy,
                    button=self.mouse_button,
                    click_type=self.mouse_click_type,
                    hold_ms=25,
                    return_cursor=False,
                    pre_hover_ms=0,
                    post_release_ms=0,
                )
            except Exception as e:
                self.log(f"Ошибка обычного клика: {e}")

            elapsed = (time.perf_counter() - t0) * 1000.0
            sleep_ms = max(1.0, self.mouse_interval_ms - elapsed)
            time.sleep(sleep_ms / 1000.0)

    def panic_stop(self):
        self.stop_hunter()
        self.stop_mouse()
        play_sound_stop()
        self.log(f"🚨 АВАРИЙНАЯ ОСТАНОВКА [{self.hotkey_panic}]")
