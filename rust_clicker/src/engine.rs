use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

use rand::Rng;

use crate::screen_capture::ScreenCapture;
use crate::vision::{PopupColorFilter, PopupDetector, VisionConfig, VisionMode};
use crate::win_input::{ClickType, MouseButton, WinInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Mouse,
    Keyboard,
    Scroll,
    Points,
    Vision,
    Settings,
}

#[derive(Debug, Clone)]
pub struct MouseConfig {
    pub button: MouseButton,
    pub click_type: ClickType,
    pub cps: f64,
    pub custom_interval_ms: f64,
    pub use_cps: bool,
    pub jitter_ms: f64,
    pub continuous_hold: bool,
    pub hold_duration_ms: f64,
    pub fixed_pos: bool,
    pub target_x: i32,
    pub target_y: i32,
    pub jitter_px: i32,
    pub stop_after_count: u64, // 0 = infinite
}

impl Default for MouseConfig {
    fn default() -> Self {
        Self {
            button: MouseButton::Left,
            click_type: ClickType::Single,
            cps: 50.0,
            custom_interval_ms: 20.0,
            use_cps: true,
            jitter_ms: 0.0,
            continuous_hold: true,
            hold_duration_ms: 0.0,
            fixed_pos: false,
            target_x: 0,
            target_y: 0,
            jitter_px: 0,
            stop_after_count: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMode {
    Tap,
    Hold,
}

#[derive(Debug, Clone)]
pub struct KeyboardConfig {
    pub vk_code: u16,
    pub key_name: String,
    pub mode: KeyMode,
    pub hold_ms: f64,
    pub interval_ms: f64,
    pub jitter_ms: f64,
    pub stop_after_count: u64,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            vk_code: 0x20, // Spacebar
            key_name: "Space".to_string(),
            mode: KeyMode::Tap,
            hold_ms: 150.0,
            interval_ms: 50.0,
            jitter_ms: 0.0,
            stop_after_count: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Down,
    Up,
    Right,
    Left,
}

#[derive(Debug, Clone)]
pub struct ScrollConfig {
    pub direction: ScrollDirection,
    pub ticks: i32,
    pub interval_ms: f64,
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            direction: ScrollDirection::Down,
            ticks: 1,
            interval_ms: 50.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoutePoint {
    pub x: i32,
    pub y: i32,
    pub button: MouseButton,
    pub delay_after_ms: f64,
    pub hold_ms: f64,
}

/// Comprehensive Advanced Settings (without sound feedback)
#[derive(Debug, Clone)]
pub struct AdvancedConfig {
    // Window behavior
    pub always_on_top: bool,
    pub window_opacity: f32, // 0.35 .. 1.0

    // Safety & Failsafes
    pub failsafe_mouse_move: bool,
    pub mouse_move_threshold: i32, // e.g. 60 px
    pub corner_panic: bool,        // stop if mouse is in (0,0) corner
    pub session_limit_enabled: bool,
    pub session_limit_seconds: u64, // default 1800s (30m)

    // Anti-detection & Human Simulation
    pub gaussian_jitter: bool,
    pub human_hold_duration: bool,
    pub natural_hold_ms: f64,
    pub micro_drift: bool,
    pub micro_drift_px: i32,

    // Target process/window lock & background clicking
    pub window_target: WindowTargetConfig,
    pub target_window_enabled: bool,
    pub target_window_title: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowTargetConfig {
    pub enabled: bool,
    pub hwnd: usize,
    pub title: String,
    pub click_center: bool,
    pub custom_x: i32,
    pub custom_y: i32,
}

impl Default for WindowTargetConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            hwnd: 0,
            title: String::new(),
            click_center: true,
            custom_x: 0,
            custom_y: 0,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct UnifiedConfig {
    // 1. Popup Hunter
    pub hunter_enabled: bool,
    pub popup_filter: PopupColorFilter,
    pub popup_min_size: i32,
    pub popup_max_size: i32,
    pub popup_hold_ms: f64,
    pub return_cursor: bool,
    pub popup_cooldown_ms: f64,

    // 2. Continuous Farm Clicking
    pub farm_enabled: bool,
    pub farm_cps: f64,
    pub farm_button: MouseButton,
    pub farm_hold_ms: f64,

    // 3. Background Game Window Lock (PostMessage)
    pub window_target: WindowTargetConfig,

    // 4. Scan frequency
    pub scan_interval_ms: f64,
}

impl Default for UnifiedConfig {
    fn default() -> Self {
        Self {
            hunter_enabled: true,
            popup_filter: PopupColorFilter::PurpleViolet,
            popup_min_size: 25,
            popup_max_size: 240,
            popup_hold_ms: 45.0,
            return_cursor: true,
            popup_cooldown_ms: 220.0,

            farm_enabled: true,
            farm_cps: 12.0,
            farm_button: MouseButton::Left,
            farm_hold_ms: 25.0,

            window_target: WindowTargetConfig::default(),

            scan_interval_ms: 25.0,
        }
    }
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            always_on_top: false,
            window_opacity: 1.0,

            failsafe_mouse_move: true,
            mouse_move_threshold: 60,
            corner_panic: true,
            session_limit_enabled: false,
            session_limit_seconds: 1800, // 30 min

            gaussian_jitter: true,
            human_hold_duration: true,
            natural_hold_ms: 35.0,
            micro_drift: false,
            micro_drift_px: 2,

            window_target: WindowTargetConfig::default(),
            target_window_enabled: false,
            target_window_title: String::new(),
        }
    }
}

/// Helper function to calculate jitter with either Gaussian or Uniform distribution
pub fn calculate_jitter(rng: &mut impl Rng, max_jitter: f64, gaussian: bool) -> f64 {
    if max_jitter <= 0.0 {
        return 0.0;
    }
    if gaussian {
        let u1: f64 = rng.gen_range(0.0001..1.0);
        let u2: f64 = rng.gen_range(0.0001..1.0);
        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        let sigma = max_jitter / 2.5;
        (z0 * sigma).clamp(-max_jitter, max_jitter)
    } else {
        rng.gen_range(-max_jitter..=max_jitter)
    }
}

#[derive(Clone, Default)]
pub struct HunterStatus {
    pub message: String,
    pub last_action: String,
    pub input_error: Option<String>,
    pub scans: u64,
    pub scan_ms: f64,
    pub frame_size: (i32, i32),
    pub preview: Option<(Vec<u8>, usize, usize)>,
}

pub struct EngineState {
    pub running: Arc<AtomicBool>,
    pub hunter_status: Arc<std::sync::Mutex<HunterStatus>>,
    worker: std::sync::Mutex<Option<thread::JoinHandle<()>>>,
    pub total_clicks: Arc<AtomicU64>,
    pub last_target: Arc<std::sync::Mutex<Option<(i32, i32)>>>,
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            hunter_status: Arc::new(std::sync::Mutex::new(HunterStatus::default())),
            worker: std::sync::Mutex::new(None),
            total_clicks: Arc::new(AtomicU64::new(0)),
            last_target: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    fn join_previous_worker(&self) {
        if let Ok(mut slot) = self.worker.lock() {
            if let Some(worker) = slot.take() { let _ = worker.join(); }
        }
    }

    pub fn toggle_mouse(&self, config: MouseConfig, adv: AdvancedConfig) {
        if self.is_running() {
            self.stop();
        } else {
            self.join_previous_worker();
            self.running.store(true, Ordering::Relaxed);
            let running = self.running.clone();
            let total = self.total_clicks.clone();

            let worker = thread::spawn(move || {
                let mut rng = rand::thread_rng();
                let mut clicks_done: u64 = 0;
                let session_start = std::time::Instant::now();
                let mut last_pos = WinInput::get_cursor_pos();

                // True Continuous Hold Mode: Press down once and keep holding until F6/Stop!
                if config.click_type == ClickType::Hold && config.continuous_hold {
                    if config.fixed_pos {
                        WinInput::set_cursor_pos(config.target_x, config.target_y);
                        interruptible_wait(&running, 15.0);
                    }

                    if adv.window_target.enabled && adv.window_target.hwnd != 0 {
                        let hwnd = adv.window_target.hwnd;
                        if WinInput::is_window_valid(hwnd) {
                            let (px, py) = if adv.window_target.click_center {
                                WinInput::get_window_client_rect(hwnd)
                                    .map(|(_, _, w, h)| (w / 2, h / 2))
                                    .unwrap_or((100, 100))
                            } else if config.fixed_pos {
                                (config.target_x, config.target_y)
                            } else {
                                (adv.window_target.custom_x, adv.window_target.custom_y)
                            };
                            let lparam = (((py as u32) & 0xFFFF) << 16) | ((px as u32) & 0xFFFF);
                            let (msg_down, msg_up, wparam_down) = match config.button {
                                MouseButton::Left => (winapi::um::winuser::WM_LBUTTONDOWN, winapi::um::winuser::WM_LBUTTONUP, winapi::um::winuser::MK_LBUTTON),
                                MouseButton::Right => (winapi::um::winuser::WM_RBUTTONDOWN, winapi::um::winuser::WM_RBUTTONUP, winapi::um::winuser::MK_RBUTTON),
                                MouseButton::Middle => (winapi::um::winuser::WM_MBUTTONDOWN, winapi::um::winuser::WM_MBUTTONUP, winapi::um::winuser::MK_MBUTTON),
                                MouseButton::X1 => (0x020B, 0x020C, 0x0020),
                                MouseButton::X2 => (0x020B, 0x020C, 0x0040),
                            };
                            unsafe {
                                winapi::um::winuser::PostMessageW(hwnd as winapi::shared::windef::HWND, msg_down, wparam_down, lparam as winapi::shared::minwindef::LPARAM);
                            }
                            total.fetch_add(1, Ordering::Relaxed);
                            while running.load(Ordering::Relaxed) {
                                if adv.session_limit_enabled && session_start.elapsed().as_secs() >= adv.session_limit_seconds {
                                    break;
                                }
                                interruptible_wait(&running, 20.0);
                            }
                            unsafe {
                                winapi::um::winuser::PostMessageW(hwnd as winapi::shared::windef::HWND, msg_up, 0, lparam as winapi::shared::minwindef::LPARAM);
                            }
                        }
                    } else {
                        WinInput::mouse_down(config.button);
                        total.fetch_add(1, Ordering::Relaxed);
                        while running.load(Ordering::Relaxed) {
                            let (cur_x, cur_y) = WinInput::get_cursor_pos();
                            if adv.corner_panic && cur_x <= 5 && cur_y <= 5 {
                                break;
                            }
                            if adv.session_limit_enabled && session_start.elapsed().as_secs() >= adv.session_limit_seconds {
                                break;
                            }
                            interruptible_wait(&running, 20.0);
                        }
                        WinInput::mouse_up(config.button);
                    }
                    running.store(false, Ordering::Relaxed);
                    return;
                }

                while running.load(Ordering::Relaxed) {
                    // 1. Session duration limit
                    if adv.session_limit_enabled
                        && session_start.elapsed().as_secs() >= adv.session_limit_seconds
                    {
                        break;
                    }

                    // 2. Count limit
                    if config.stop_after_count > 0 && clicks_done >= config.stop_after_count {
                        break;
                    }

                    // 3. Screen corner panic (0,0) - only for global cursor mode
                    let (cur_x, cur_y) = WinInput::get_cursor_pos();
                    if !adv.window_target.enabled && adv.corner_panic && cur_x <= 5 && cur_y <= 5 {
                        break;
                    }

                    // 4. Mouse movement failsafe (user physically grabbed mouse) - only for global cursor mode
                    if !adv.window_target.enabled && adv.failsafe_mouse_move && clicks_done > 0 {
                        let dx = (cur_x - last_pos.0).abs();
                        let dy = (cur_y - last_pos.1).abs();
                        if dx > adv.mouse_move_threshold || dy > adv.mouse_move_threshold {
                            break;
                        }
                    }

                    // 5. Background Window Mode (isolated to game window via PostMessageW)
                    if adv.window_target.enabled && adv.window_target.hwnd != 0 {
                        let hwnd = adv.window_target.hwnd;
                        if !WinInput::is_window_valid(hwnd) {
                            interruptible_wait(&running, 150.0);
                            continue;
                        }

                        let (mut px, mut py) = if adv.window_target.click_center {
                            if let Some((_, _, w, h)) = WinInput::get_window_client_rect(hwnd) {
                                (w / 2, h / 2)
                            } else {
                                (100, 100)
                            }
                        } else if config.fixed_pos {
                            (config.target_x, config.target_y)
                        } else {
                            (adv.window_target.custom_x, adv.window_target.custom_y)
                        };

                        if config.jitter_px > 0 {
                            px += rng.gen_range(-config.jitter_px..=config.jitter_px);
                            py += rng.gen_range(-config.jitter_px..=config.jitter_px);
                        }

                        let hold_ms = if config.click_type == ClickType::Hold {
                            config.hold_duration_ms.max(15.0)
                        } else if adv.human_hold_duration {
                            let j = calculate_jitter(&mut rng, 8.0, adv.gaussian_jitter);
                            (adv.natural_hold_ms + j).clamp(10.0, 95.0)
                        } else {
                            0.0
                        };

                        WinInput::post_mouse_click_to_window(
                            hwnd,
                            px,
                            py,
                            config.button,
                            config.click_type,
                            hold_ms,
                        );

                        clicks_done += 1;
                        total.fetch_add(1, Ordering::Relaxed);

                        let base_delay = if config.use_cps {
                            if config.cps > 0.0 {
                                1000.0 / config.cps
                            } else {
                                50.0
                            }
                        } else {
                            config.custom_interval_ms
                        };

                        let mut delay = base_delay;
                        if config.jitter_ms > 0.0 {
                            let j = calculate_jitter(&mut rng, config.jitter_ms, adv.gaussian_jitter);
                            delay = (delay + j).max(1.0);
                        }

                        interruptible_wait(&running, delay);
                        continue;
                    }

                    // 6. Target active window filter (for global cursor mode)
                    if adv.target_window_enabled && !adv.target_window_title.trim().is_empty() {
                        let active_title = WinInput::get_active_window_title();
                        if !active_title
                            .to_lowercase()
                            .contains(&adv.target_window_title.to_lowercase())
                        {
                            interruptible_wait(&running, 100.0);
                            last_pos = WinInput::get_cursor_pos();
                            continue;
                        }
                    }

                    // 6. Positioning
                    if config.fixed_pos {
                        let mut px = config.target_x;
                        let mut py = config.target_y;
                        if config.jitter_px > 0 {
                            px += rng.gen_range(-config.jitter_px..=config.jitter_px);
                            py += rng.gen_range(-config.jitter_px..=config.jitter_px);
                        }
                        WinInput::set_cursor_pos(px, py);
                    } else if adv.micro_drift && adv.micro_drift_px > 0 {
                        let mx = rng.gen_range(-adv.micro_drift_px..=adv.micro_drift_px);
                        let my = rng.gen_range(-adv.micro_drift_px..=adv.micro_drift_px);
                        WinInput::set_cursor_pos(cur_x + mx, cur_y + my);
                    }

                    // 7. Click execution
                    match config.click_type {
                        ClickType::Single => {
                            let hold_ms = if config.use_cps {
                                if config.cps >= 25.0 {
                                    1.5
                                } else {
                                    (1000.0 / config.cps * 0.25).clamp(2.0, 15.0)
                                }
                            } else if adv.human_hold_duration {
                                let j = calculate_jitter(&mut rng, 8.0, adv.gaussian_jitter);
                                (adv.natural_hold_ms + j).clamp(10.0, 95.0)
                            } else {
                                1.0
                            };

                            WinInput::mouse_down(config.button);
                            if hold_ms > 0.0 {
                                interruptible_wait(&running, hold_ms);
                            }
                            WinInput::mouse_up(config.button);
                        }
                        ClickType::Double => {
                            WinInput::mouse_click(config.button);
                            interruptible_wait(&running, 25.0);
                            WinInput::mouse_click(config.button);
                        }
                        ClickType::Triple => {
                            WinInput::mouse_click(config.button);
                            interruptible_wait(&running, 25.0);
                            WinInput::mouse_click(config.button);
                            interruptible_wait(&running, 25.0);
                            WinInput::mouse_click(config.button);
                        }
                        ClickType::Hold => {
                            // Exact hold mode
                            WinInput::mouse_down(config.button);
                            interruptible_wait(&running, config.hold_duration_ms);
                            WinInput::mouse_up(config.button);
                        }
                    }

                    clicks_done += 1;
                    total.fetch_add(1, Ordering::Relaxed);
                    last_pos = WinInput::get_cursor_pos();

                    // 8. Delay calculation
                    let base_delay = if config.use_cps {
                        if config.cps > 0.0 {
                            1000.0 / config.cps
                        } else {
                            50.0
                        }
                    } else {
                        config.custom_interval_ms
                    };

                    let mut delay = base_delay;
                    if config.jitter_ms > 0.0 {
                        let j = calculate_jitter(&mut rng, config.jitter_ms, adv.gaussian_jitter);
                        delay = (delay + j).max(0.5);
                    }

                    interruptible_wait(&running, delay);
                }

                running.store(false, Ordering::Relaxed);
            });
            if let Ok(mut slot) = self.worker.lock() { *slot = Some(worker); }
        }
    }

    pub fn toggle_keyboard(&self, config: KeyboardConfig, adv: AdvancedConfig) {
        if self.is_running() {
            self.stop();
        } else {
            self.join_previous_worker();
            self.running.store(true, Ordering::Relaxed);
            let running = self.running.clone();
            let total = self.total_clicks.clone();

            let worker = thread::spawn(move || {
                let mut rng = rand::thread_rng();
                let mut count = 0;
                let session_start = std::time::Instant::now();

                // True Continuous Key Hold Mode
                if config.mode == KeyMode::Hold && config.hold_ms <= 0.0 {
                    if adv.window_target.enabled && adv.window_target.hwnd != 0 {
                        let hwnd = adv.window_target.hwnd;
                        if WinInput::is_window_valid(hwnd) {
                            unsafe {
                                winapi::um::winuser::PostMessageW(hwnd as winapi::shared::windef::HWND, winapi::um::winuser::WM_KEYDOWN, config.vk_code as winapi::shared::minwindef::WPARAM, 1 as winapi::shared::minwindef::LPARAM);
                            }
                            total.fetch_add(1, Ordering::Relaxed);
                            while running.load(Ordering::Relaxed) {
                                if adv.session_limit_enabled && session_start.elapsed().as_secs() >= adv.session_limit_seconds {
                                    break;
                                }
                                interruptible_wait(&running, 20.0);
                            }
                            unsafe {
                                let up_lparam = 1 | (1 << 30) | (1 << 31);
                                winapi::um::winuser::PostMessageW(hwnd as winapi::shared::windef::HWND, winapi::um::winuser::WM_KEYUP, config.vk_code as winapi::shared::minwindef::WPARAM, up_lparam as winapi::shared::minwindef::LPARAM);
                            }
                        }
                    } else {
                        WinInput::key_down(config.vk_code);
                        total.fetch_add(1, Ordering::Relaxed);
                        while running.load(Ordering::Relaxed) {
                            let (cur_x, cur_y) = WinInput::get_cursor_pos();
                            if adv.corner_panic && cur_x <= 5 && cur_y <= 5 {
                                break;
                            }
                            if adv.session_limit_enabled && session_start.elapsed().as_secs() >= adv.session_limit_seconds {
                                break;
                            }
                            interruptible_wait(&running, 20.0);
                        }
                        WinInput::key_up(config.vk_code);
                    }
                    running.store(false, Ordering::Relaxed);
                    return;
                }

                while running.load(Ordering::Relaxed) {
                    // Session duration limit
                    if adv.session_limit_enabled
                        && session_start.elapsed().as_secs() >= adv.session_limit_seconds
                    {
                        break;
                    }

                    // Count limit
                    if config.stop_after_count > 0 && count >= config.stop_after_count {
                        break;
                    }

                    // Screen corner panic (0,0) - only for global mode
                    let (cur_x, cur_y) = WinInput::get_cursor_pos();
                    if !adv.window_target.enabled && adv.corner_panic && cur_x <= 5 && cur_y <= 5 {
                        break;
                    }

                    // Background window key dispatch
                    if adv.window_target.enabled && adv.window_target.hwnd != 0 {
                        let hwnd = adv.window_target.hwnd;
                        if !WinInput::is_window_valid(hwnd) {
                            interruptible_wait(&running, 150.0);
                            continue;
                        }
                        let hold_ms = if config.mode == KeyMode::Hold { config.hold_ms } else { 0.0 };
                        WinInput::post_key_tap_to_window(hwnd, config.vk_code, hold_ms);

                        count += 1;
                        total.fetch_add(1, Ordering::Relaxed);

                        let mut delay = config.interval_ms;
                        if config.jitter_ms > 0.0 {
                            let j = calculate_jitter(&mut rng, config.jitter_ms, adv.gaussian_jitter);
                            delay = (delay + j).max(1.0);
                        }

                        interruptible_wait(&running, delay);
                        continue;
                    }

                    // Target window filter (for global mode)
                    if adv.target_window_enabled && !adv.target_window_title.trim().is_empty() {
                        let active_title = WinInput::get_active_window_title();
                        if !active_title
                            .to_lowercase()
                            .contains(&adv.target_window_title.to_lowercase())
                        {
                            interruptible_wait(&running, 100.0);
                            continue;
                        }
                    }

                    match config.mode {
                        KeyMode::Tap => {
                            WinInput::key_tap(config.vk_code);
                        }
                        KeyMode::Hold => {
                            WinInput::key_down(config.vk_code);
                            interruptible_wait(&running, config.hold_ms);
                            WinInput::key_up(config.vk_code);
                        }
                    }

                    count += 1;
                    total.fetch_add(1, Ordering::Relaxed);

                    let mut delay = config.interval_ms;
                    if config.jitter_ms > 0.0 {
                        let j = calculate_jitter(&mut rng, config.jitter_ms, adv.gaussian_jitter);
                        delay = (delay + j).max(1.0);
                    }

                    interruptible_wait(&running, delay);
                }

                running.store(false, Ordering::Relaxed);
            });
            if let Ok(mut slot) = self.worker.lock() { *slot = Some(worker); }
        }
    }

    pub fn toggle_scroll(&self, config: ScrollConfig, adv: AdvancedConfig) {
        if self.is_running() {
            self.stop();
        } else {
            self.join_previous_worker();
            self.running.store(true, Ordering::Relaxed);
            let running = self.running.clone();
            let total = self.total_clicks.clone();

            let worker = thread::spawn(move || {
                let wheel_delta = 120;
                let session_start = std::time::Instant::now();

                while running.load(Ordering::Relaxed) {
                    if adv.session_limit_enabled
                        && session_start.elapsed().as_secs() >= adv.session_limit_seconds
                    {
                        break;
                    }

                    let (cur_x, cur_y) = WinInput::get_cursor_pos();
                    if adv.corner_panic && cur_x <= 5 && cur_y <= 5 {
                        break;
                    }

                    if adv.target_window_enabled && !adv.target_window_title.trim().is_empty() {
                        let active_title = WinInput::get_active_window_title();
                        if !active_title
                            .to_lowercase()
                            .contains(&adv.target_window_title.to_lowercase())
                        {
                            interruptible_wait(&running, 100.0);
                            continue;
                        }
                    }

                    match config.direction {
                        ScrollDirection::Down => {
                            WinInput::mouse_scroll(-wheel_delta * config.ticks, false);
                        }
                        ScrollDirection::Up => {
                            WinInput::mouse_scroll(wheel_delta * config.ticks, false);
                        }
                        ScrollDirection::Right => {
                            WinInput::mouse_scroll(wheel_delta * config.ticks, true);
                        }
                        ScrollDirection::Left => {
                            WinInput::mouse_scroll(-wheel_delta * config.ticks, true);
                        }
                    }

                    total.fetch_add(1, Ordering::Relaxed);
                    interruptible_wait(&running, config.interval_ms);
                }

                running.store(false, Ordering::Relaxed);
            });
            if let Ok(mut slot) = self.worker.lock() { *slot = Some(worker); }
        }
    }

    pub fn toggle_route(&self, points: Vec<RoutePoint>, adv: AdvancedConfig) {
        if self.is_running() {
            self.stop();
        } else if !points.is_empty() {
            self.join_previous_worker();
            self.running.store(true, Ordering::Relaxed);
            let running = self.running.clone();
            let total = self.total_clicks.clone();

            let worker = thread::spawn(move || {
                let mut idx = 0;
                let session_start = std::time::Instant::now();

                while running.load(Ordering::Relaxed) {
                    if adv.session_limit_enabled
                        && session_start.elapsed().as_secs() >= adv.session_limit_seconds
                    {
                        break;
                    }

                    let (cur_x, cur_y) = WinInput::get_cursor_pos();
                    if !adv.window_target.enabled && adv.corner_panic && cur_x <= 5 && cur_y <= 5 {
                        break;
                    }

                    // Background window mode for route points
                    if adv.window_target.enabled && adv.window_target.hwnd != 0 {
                        let hwnd = adv.window_target.hwnd;
                        if !WinInput::is_window_valid(hwnd) {
                            interruptible_wait(&running, 150.0);
                            continue;
                        }
                        let pt = &points[idx];
                        WinInput::post_mouse_click_to_window(
                            hwnd,
                            pt.x,
                            pt.y,
                            pt.button,
                            ClickType::Single,
                            pt.hold_ms,
                        );
                        total.fetch_add(1, Ordering::Relaxed);
                        interruptible_wait(&running, pt.delay_after_ms.max(10.0));
                        idx = (idx + 1) % points.len();
                        continue;
                    }

                    if adv.target_window_enabled && !adv.target_window_title.trim().is_empty() {
                        let active_title = WinInput::get_active_window_title();
                        if !active_title
                            .to_lowercase()
                            .contains(&adv.target_window_title.to_lowercase())
                        {
                            interruptible_wait(&running, 100.0);
                            continue;
                        }
                    }

                    let pt = &points[idx];
                    WinInput::set_cursor_pos(pt.x, pt.y);

                    if pt.hold_ms > 0.0 {
                        WinInput::mouse_down(pt.button);
                        interruptible_wait(&running, pt.hold_ms);
                        WinInput::mouse_up(pt.button);
                    } else {
                        WinInput::mouse_click(pt.button);
                    }

                    total.fetch_add(1, Ordering::Relaxed);
                    interruptible_wait(&running, pt.delay_after_ms.max(10.0));

                    idx = (idx + 1) % points.len();
                }

                running.store(false, Ordering::Relaxed);
            });
            if let Ok(mut slot) = self.worker.lock() { *slot = Some(worker); }
        }
    }

    pub fn toggle_vision(&self, config: VisionConfig, adv: AdvancedConfig) {
        if self.is_running() { self.stop(); return; }
        self.join_previous_worker();
        if adv.window_target.hwnd == 0 || !WinInput::is_window_valid(adv.window_target.hwnd) {
            if let Ok(mut s) = self.hunter_status.lock() { s.message = "Сначала выберите окно игры".into(); }
            return;
        }
        if config.mode == VisionMode::TemplateMatch && !config.templates.iter().any(|t| t.enabled) {
            if let Ok(mut status) = self.hunter_status.lock() { status.message = "Сначала захватите или загрузите образец цели".into(); }
            return;
        }
        self.running.store(true, Ordering::Relaxed);
        self.total_clicks.store(0, Ordering::Relaxed);
        let running = self.running.clone();
        let total = self.total_clicks.clone();
        let last_target = self.last_target.clone();
        let status = self.hunter_status.clone();
        if let Ok(mut s) = status.lock() { *s = HunterStatus { message: "Ожидание окна игры…".into(), ..Default::default() }; }
        let worker = thread::spawn(move || {
            let started = std::time::Instant::now();
            let mut log = crate::diagnostics::HunterLog::new();
            log.event(&format!("START target={:#x} title={} mode={:?}", adv.window_target.hwnd, adv.window_target.title, config.mode));
            let mut last_capture_error: Option<String> = None;
            let mut stop_reason = "User stop";
            let mut session = crate::hunter::HunterSession::new(config.clone());
            let mut input_job: Option<(thread::JoinHandle<Result<(), String>>, String)> = None;
            let mut last_preview = std::time::Instant::now() - std::time::Duration::from_secs(1);
            while running.load(Ordering::Relaxed) {
                if adv.session_limit_enabled && started.elapsed().as_secs() >= adv.session_limit_seconds { stop_reason = "Session time limit"; break; }
                if input_job.as_ref().is_some_and(|(job,_)| job.is_finished()) {
                    let (job, description) = input_job.take().unwrap();
                    let result = job.join().unwrap_or_else(|_| Err("Ошибка потока ввода".into()));
                    log.event(&format!("INPUT result={:?} {}", result, description));
                    session.complete_input(result.is_ok(), started.elapsed());
                    if let Ok(mut s) = status.lock() {
                        s.input_error = result.as_ref().err().cloned();
                        s.last_action = match result {
                            Ok(()) => { total.fetch_add(1, Ordering::Relaxed); description }
                            Err(error) => format!("Клик не выполнен: {}", error),
                        };
                    }
                }
                let (cx, cy) = WinInput::get_cursor_pos();
                if adv.corner_panic && cx <= 5 && cy <= 5 { stop_reason = "Cursor corner failsafe"; break; }
                let frame = match crate::hunter::capture(&config, &adv) {
                    Ok(frame) => {
                        if last_capture_error.take().is_some() { log.event("CAPTURE resumed"); }
                        frame
                    },
                    Err(message) => {
                        if last_capture_error.as_deref() != Some(message.as_str()) {
                            log.event(&format!("CAPTURE paused: {} foreground={:#x}", message, WinInput::get_foreground_window()));
                            last_capture_error = Some(message.clone());
                        }
                        if let Ok(mut s) = status.lock() { s.message = message; s.frame_size = (0, 0); }
                        interruptible_wait(&running, 100.0);
                        continue;
                    }
                };
                let scan_start = std::time::Instant::now();
                let observation = session.observe(&frame, started.elapsed(), Some(&running));
                if let Ok(mut s) = status.lock() {
                    s.scans += 1;
                    s.scan_ms = scan_start.elapsed().as_secs_f64() * 1000.0;
                    s.frame_size = (frame.image.width, frame.image.height);
                    s.message = if session.input_pending() { "Нажимаю цель; продолжаю следить за следующей".into() }
                        else { "Поиск работает: ожидаю следующую цель".into() };
                    if last_preview.elapsed().as_millis() >= 500 {
                        s.preview = Some(crate::hunter::preview(&frame, observation.target.as_ref()));
                        last_preview = std::time::Instant::now();
                    }
                }
                if observation.start_input {
                    let target = observation.target.unwrap();
                    if !running.load(Ordering::Relaxed) || WinInput::get_foreground_window() != frame.hwnd
                        || !WinInput::get_window_at_point(target.x, target.y).is_some_and(|w| w.hwnd == frame.hwnd) {
                        let point_window = WinInput::get_window_at_point(target.x, target.y).map(|w| w.hwnd).unwrap_or(0);
                        log.event(&format!("INPUT skipped: focus/overlay guard target={:#x} foreground={:#x} point={:#x}",
                            frame.hwnd, WinInput::get_foreground_window(), point_window));
                        if let Ok(mut s) = status.lock() { s.input_error = Some("Клик отложен: цель перекрыта или игра неактивна".into()); }
                        session.complete_input(false, started.elapsed());
                        continue;
                    }
                    let original = WinInput::get_cursor_pos();
                    if let Ok(mut last) = last_target.lock() { *last = Some((target.x, target.y)); }
                    let description = format!("Клик: ({}, {}) · {}", target.x, target.y, target.method);
                    log.event(&format!("INPUT begin {} scans={}", description, status.lock().map(|s| s.scans).unwrap_or(0)));
                    let input_running = running.clone();
                    let return_cursor = config.return_cursor;
                    let background_input = config.background_input;
                    let hwnd = frame.hwnd;
                    // Input and its hold/release delays must not hide the next popup from capture.
                    let job = thread::spawn(move || {
                        if !input_running.load(Ordering::Relaxed) || WinInput::get_foreground_window() != hwnd
                            || !WinInput::get_window_at_point(target.x, target.y).is_some_and(|w| w.hwnd == hwnd) {
                            return Err("Цель перекрыта или игра неактивна".into());
                        }
                        if background_input {
                            match WinInput::screen_to_client(hwnd, target.x, target.y) {
                                Some((x,y)) => { WinInput::post_mouse_click_to_window(hwnd, x, y, target.button, target.click_type, target.hold_ms); Ok(()) }
                                None => Err("Не удалось преобразовать координаты окна".into()),
                            }
                        } else {
                            WinInput::send_hardware_click(target.x, target.y, target.button, target.hold_ms,
                                return_cursor, original.0, original.1, hwnd, &input_running)
                        }
                    });
                    input_job = Some((job, description));
                }
                // Always capture again at the scan interval, including during input/cooldown.
                interruptible_wait(&running, config.scan_interval_ms.max(10.0));
            }
            running.store(false, Ordering::Relaxed);
            if let Some((job,_)) = input_job { let _ = job.join(); }
            log.event(&format!("STOP reason={} clicks={}", stop_reason, total.load(Ordering::Relaxed)));
            if let Ok(mut s) = status.lock() {
                s.message = if stop_reason == "Cursor corner failsafe" {
                    "Остановлен: курсор оказался в верхнем левом углу. F6 — запустить снова".into()
                } else { "Остановлен".into() };
            }
        });
        if let Ok(mut slot) = self.worker.lock() { *slot = Some(worker); }
    }

    #[allow(dead_code)]
    pub fn toggle_unified(&self, config: UnifiedConfig, adv: AdvancedConfig) {
        if self.is_running() {
            self.stop();
        } else {
            self.join_previous_worker();
            self.running.store(true, Ordering::Relaxed);
            let running = self.running.clone();
            let total = self.total_clicks.clone();

            let worker = thread::spawn(move || {
                let session_start = std::time::Instant::now();
                let mut popup_detector = PopupDetector::new();
                let mut last_farm_time = std::time::Instant::now();

                while running.load(Ordering::Relaxed) {
                    if adv.session_limit_enabled
                        && session_start.elapsed().as_secs() >= adv.session_limit_seconds
                    {
                        break;
                    }

                    let is_bg = adv.window_target.enabled && adv.window_target.hwnd != 0;
                    let target_hwnd = if is_bg { adv.window_target.hwnd } else { 0 };

                    let (cur_x, cur_y) = WinInput::get_cursor_pos();
                    if !is_bg && adv.corner_panic && cur_x <= 5 && cur_y <= 5 {
                        break;
                    }

                    let mut popup_clicked = false;

                    // 1. Popup Hunter Step (Direct Real-time Screen Target Recognition)
                    if config.hunter_enabled {
                        let (screen_opt, offset_x, offset_y) = if is_bg {
                            match ScreenCapture::capture_window(target_hwnd) {
                                Some(cap) if cap.buffer.iter().take(20000).any(|&b| b > 0) => (Some(cap), 0, 0),
                                _ => {
                                    if let Some((l, t, r, b)) = WinInput::get_window_screen_rect(target_hwnd) {
                                        let w = (r - l).max(1);
                                        let h = (b - t).max(1);
                                        (ScreenCapture::capture_rect(l, t, w, h), l, t)
                                    } else {
                                        (ScreenCapture::capture_screen(), 0, 0)
                                    }
                                }
                            }
                        } else {
                            (ScreenCapture::capture_screen(), 0, 0)
                        };

                        if let Some(screen) = screen_opt {
                            let vision_cfg = VisionConfig {
                                mode: VisionMode::AutoPopup,
                                popup_min_size: config.popup_min_size,
                                popup_max_size: config.popup_max_size,
                                popup_sensitivity: 40,
                                popup_filter: config.popup_filter,
                                ..Default::default()
                            };

                            if let Some((target_x, target_y)) =
                                popup_detector.detect_popup(&screen, &vision_cfg)
                            {
                                let orig_pos = WinInput::get_cursor_pos();
                                let final_x = target_x + offset_x;
                                let final_y = target_y + offset_y;

                                if is_bg {
                                    let (client_x, client_y) = if offset_x == 0 && offset_y == 0 {
                                        (final_x, final_y)
                                    } else {
                                        WinInput::screen_to_client(target_hwnd, final_x, final_y)
                                            .unwrap_or((final_x, final_y))
                                    };

                                    WinInput::post_mouse_click_to_window(
                                        target_hwnd,
                                        client_x,
                                        client_y,
                                        MouseButton::Left,
                                        ClickType::Single,
                                        config.popup_hold_ms,
                                    );
                                } else {
                                    let _ = WinInput::send_hardware_click(
                                        final_x,
                                        final_y,
                                        MouseButton::Left,
                                        config.popup_hold_ms,
                                        config.return_cursor,
                                        orig_pos.0,
                                        orig_pos.1,
                                        WinInput::get_foreground_window(),
                                        &running,
                                    );
                                }

                                total.fetch_add(1, Ordering::Relaxed);
                                popup_clicked = true;
                                popup_detector.reset();
                                interruptible_wait(&running, config.popup_cooldown_ms.max(50.0));
                            }
                        }
                    }

                    // 2. Continuous Farm Clicking Step (when no pop-up button is active)
                    if !popup_clicked && config.farm_enabled {
                        let farm_delay_ms = if config.farm_cps > 0.0 {
                            1000.0 / config.farm_cps
                        } else {
                            80.0
                        };

                        if last_farm_time.elapsed().as_secs_f64() * 1000.0 >= farm_delay_ms {
                            if is_bg {
                                let (client_x, client_y) = if config.window_target.click_center {
                                    WinInput::get_window_client_rect(target_hwnd)
                                        .map(|(_, _, w, h)| (w / 2, h / 2))
                                        .unwrap_or((100, 100))
                                } else {
                                    (config.window_target.custom_x, config.window_target.custom_y)
                                };

                                WinInput::post_mouse_click_to_window(
                                    target_hwnd,
                                    client_x,
                                    client_y,
                                    config.farm_button,
                                    ClickType::Single,
                                    config.farm_hold_ms,
                                );
                            } else {
                                WinInput::mouse_down(config.farm_button);
                                interruptible_wait(&running, config.farm_hold_ms.max(15.0));
                                WinInput::mouse_up(config.farm_button);
                            }

                            total.fetch_add(1, Ordering::Relaxed);
                            last_farm_time = std::time::Instant::now();
                        }
                    }

                    interruptible_wait(&running, config.scan_interval_ms.max(10.0));
                }

                running.store(false, Ordering::Relaxed);
            });
            if let Ok(mut slot) = self.worker.lock() { *slot = Some(worker); }
        }
    }
}

fn interruptible_wait(running: &AtomicBool, ms: f64) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs_f64(ms.max(0.0) / 1000.0);
    while running.load(Ordering::Relaxed) && std::time::Instant::now() < deadline {
        thread::sleep(std::time::Duration::from_millis(5));
    }
}

impl Drop for EngineState {
    fn drop(&mut self) {
        self.stop();
        self.join_previous_worker();
    }
}
