use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use winapi::um::winuser::{GetAsyncKeyState, VK_F12, VK_F6, VK_F8};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    ToggleStartStop,
    PickCoordinate,
    EmergencyStop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    ToggleStartStop,
    PickCoordinate,
    EmergencyStop,
}

pub struct HotkeyManager {
    running: Arc<AtomicBool>,
    rx: Receiver<HotkeyEvent>,
    pub key_toggle: Arc<AtomicU16>,
    pub key_pick: Arc<AtomicU16>,
    pub key_panic: Arc<AtomicU16>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let (tx, rx) = channel();

        let key_toggle = Arc::new(AtomicU16::new(VK_F6 as u16));
        let key_pick = Arc::new(AtomicU16::new(VK_F8 as u16));
        let key_panic = Arc::new(AtomicU16::new(VK_F12 as u16));

        let running_clone = running.clone();
        let toggle_k = key_toggle.clone();
        let pick_k = key_pick.clone();
        let panic_k = key_panic.clone();

        thread::spawn(move || {
            let mut toggle_was_down = false;
            let mut pick_was_down = false;
            let mut panic_was_down = false;

            while running_clone.load(Ordering::Relaxed) {
                unsafe {
                    let k_toggle = toggle_k.load(Ordering::Relaxed);
                    let k_pick = pick_k.load(Ordering::Relaxed);
                    let k_panic = panic_k.load(Ordering::Relaxed);

                    // Toggle Start/Stop
                    if k_toggle > 0 {
                        let toggle_state = (GetAsyncKeyState(k_toggle as i32) as u16 & 0x8000) != 0;
                        if toggle_state && !toggle_was_down {
                            let _ = tx.send(HotkeyEvent::ToggleStartStop);
                        }
                        toggle_was_down = toggle_state;
                    }

                    // Pick Coordinate
                    if k_pick > 0 {
                        let pick_state = (GetAsyncKeyState(k_pick as i32) as u16 & 0x8000) != 0;
                        if pick_state && !pick_was_down {
                            let _ = tx.send(HotkeyEvent::PickCoordinate);
                        }
                        pick_was_down = pick_state;
                    }

                    // Emergency Panic Stop
                    if k_panic > 0 {
                        let panic_state = (GetAsyncKeyState(k_panic as i32) as u16 & 0x8000) != 0;
                        if panic_state && !panic_was_down {
                            let _ = tx.send(HotkeyEvent::EmergencyStop);
                        }
                        panic_was_down = panic_state;
                    }
                }

                thread::sleep(Duration::from_millis(20));
            }
        });

        Self {
            running,
            rx,
            key_toggle,
            key_pick,
            key_panic,
        }
    }

    pub fn set_key(&self, action: HotkeyAction, vk_code: u16) {
        match action {
            HotkeyAction::ToggleStartStop => self.key_toggle.store(vk_code, Ordering::Relaxed),
            HotkeyAction::PickCoordinate => self.key_pick.store(vk_code, Ordering::Relaxed),
            HotkeyAction::EmergencyStop => self.key_panic.store(vk_code, Ordering::Relaxed),
        }
    }

    pub fn poll_event(&self) -> Option<HotkeyEvent> {
        self.rx.try_recv().ok()
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
