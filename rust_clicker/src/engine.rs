use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

use rand::Rng;

use crate::precision_timer::PrecisionTimer;
use crate::screen_capture::ScreenCapture;
use crate::vision::{VisionConfig, VisionEngine};
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
            hold_duration_ms: 200.0,
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

    // Target process/window lock
    pub target_window_enabled: bool,
    pub target_window_title: String,
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

pub struct EngineState {
    pub running: Arc<AtomicBool>,
    pub total_clicks: Arc<AtomicU64>,
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            total_clicks: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn toggle_mouse(&self, config: MouseConfig, adv: AdvancedConfig) {
        if self.is_running() {
            self.stop();
        } else {
            self.running.store(true, Ordering::Relaxed);
            let running = self.running.clone();
            let total = self.total_clicks.clone();

            thread::spawn(move || {
                let mut rng = rand::thread_rng();
                let mut clicks_done: u64 = 0;
                let session_start = std::time::Instant::now();
                let mut last_pos = WinInput::get_cursor_pos();

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

                    // 3. Screen corner panic (0,0)
                    let (cur_x, cur_y) = WinInput::get_cursor_pos();
                    if adv.corner_panic && cur_x <= 5 && cur_y <= 5 {
                        break;
                    }

                    // 4. Mouse movement failsafe (user physically grabbed mouse)
                    if adv.failsafe_mouse_move && clicks_done > 0 {
                        let dx = (cur_x - last_pos.0).abs();
                        let dy = (cur_y - last_pos.1).abs();
                        if dx > adv.mouse_move_threshold || dy > adv.mouse_move_threshold {
                            break;
                        }
                    }

                    // 5. Target active window filter
                    if adv.target_window_enabled && !adv.target_window_title.trim().is_empty() {
                        let active_title = WinInput::get_active_window_title();
                        if !active_title
                            .to_lowercase()
                            .contains(&adv.target_window_title.to_lowercase())
                        {
                            PrecisionTimer::sleep_ms(100.0);
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
                            let hold_ms = if adv.human_hold_duration {
                                let j = calculate_jitter(&mut rng, 8.0, adv.gaussian_jitter);
                                (adv.natural_hold_ms + j).clamp(10.0, 95.0)
                            } else {
                                0.0
                            };

                            WinInput::mouse_down(config.button);
                            if hold_ms > 0.0 {
                                PrecisionTimer::sleep_ms(hold_ms);
                            }
                            WinInput::mouse_up(config.button);
                        }
                        ClickType::Double => {
                            WinInput::mouse_click(config.button);
                            PrecisionTimer::sleep_ms(25.0);
                            WinInput::mouse_click(config.button);
                        }
                        ClickType::Triple => {
                            WinInput::mouse_click(config.button);
                            PrecisionTimer::sleep_ms(25.0);
                            WinInput::mouse_click(config.button);
                            PrecisionTimer::sleep_ms(25.0);
                            WinInput::mouse_click(config.button);
                        }
                        ClickType::Hold => {
                            // Exact hold mode
                            WinInput::mouse_down(config.button);
                            PrecisionTimer::sleep_ms(config.hold_duration_ms);
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

                    PrecisionTimer::sleep_ms(delay);
                }

                running.store(false, Ordering::Relaxed);
            });
        }
    }

    pub fn toggle_keyboard(&self, config: KeyboardConfig, adv: AdvancedConfig) {
        if self.is_running() {
            self.stop();
        } else {
            self.running.store(true, Ordering::Relaxed);
            let running = self.running.clone();
            let total = self.total_clicks.clone();

            thread::spawn(move || {
                let mut rng = rand::thread_rng();
                let mut count = 0;
                let session_start = std::time::Instant::now();

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

                    // Screen corner panic (0,0)
                    let (cur_x, cur_y) = WinInput::get_cursor_pos();
                    if adv.corner_panic && cur_x <= 5 && cur_y <= 5 {
                        break;
                    }

                    // Target window filter
                    if adv.target_window_enabled && !adv.target_window_title.trim().is_empty() {
                        let active_title = WinInput::get_active_window_title();
                        if !active_title
                            .to_lowercase()
                            .contains(&adv.target_window_title.to_lowercase())
                        {
                            PrecisionTimer::sleep_ms(100.0);
                            continue;
                        }
                    }

                    match config.mode {
                        KeyMode::Tap => {
                            WinInput::key_tap(config.vk_code);
                        }
                        KeyMode::Hold => {
                            WinInput::key_down(config.vk_code);
                            PrecisionTimer::sleep_ms(config.hold_ms);
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

                    PrecisionTimer::sleep_ms(delay);
                }

                running.store(false, Ordering::Relaxed);
            });
        }
    }

    pub fn toggle_scroll(&self, config: ScrollConfig, adv: AdvancedConfig) {
        if self.is_running() {
            self.stop();
        } else {
            self.running.store(true, Ordering::Relaxed);
            let running = self.running.clone();
            let total = self.total_clicks.clone();

            thread::spawn(move || {
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
                            PrecisionTimer::sleep_ms(100.0);
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
                    PrecisionTimer::sleep_ms(config.interval_ms);
                }

                running.store(false, Ordering::Relaxed);
            });
        }
    }

    pub fn toggle_route(&self, points: Vec<RoutePoint>, adv: AdvancedConfig) {
        if self.is_running() {
            self.stop();
        } else if !points.is_empty() {
            self.running.store(true, Ordering::Relaxed);
            let running = self.running.clone();
            let total = self.total_clicks.clone();

            thread::spawn(move || {
                let mut idx = 0;
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
                            PrecisionTimer::sleep_ms(100.0);
                            continue;
                        }
                    }

                    let pt = &points[idx];
                    WinInput::set_cursor_pos(pt.x, pt.y);

                    if pt.hold_ms > 0.0 {
                        WinInput::mouse_down(pt.button);
                        PrecisionTimer::sleep_ms(pt.hold_ms);
                        WinInput::mouse_up(pt.button);
                    } else {
                        WinInput::mouse_click(pt.button);
                    }

                    total.fetch_add(1, Ordering::Relaxed);
                    PrecisionTimer::sleep_ms(pt.delay_after_ms.max(10.0));

                    idx = (idx + 1) % points.len();
                }

                running.store(false, Ordering::Relaxed);
            });
        }
    }

    pub fn toggle_vision(&self, config: VisionConfig, adv: AdvancedConfig) {
        if self.is_running() {
            self.stop();
        } else if !config.templates.is_empty() {
            self.running.store(true, Ordering::Relaxed);
            let running = self.running.clone();
            let total = self.total_clicks.clone();

            thread::spawn(move || {
                let session_start = std::time::Instant::now();
                let mut rng = rand::thread_rng();

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
                            PrecisionTimer::sleep_ms(150.0);
                            continue;
                        }
                    }

                    if let Some(screen) = ScreenCapture::capture_screen() {
                        let region_opt = if config.use_custom_region {
                            Some((
                                config.region_x1,
                                config.region_y1,
                                config.region_x2,
                                config.region_y2,
                            ))
                        } else {
                            None
                        };

                        let mut clicked = false;
                        for template in &config.templates {
                            if !template.enabled {
                                continue;
                            }

                            if let Some(m) = VisionEngine::find_template(&screen, template, region_opt) {
                                let orig_pos = WinInput::get_cursor_pos();

                                let jx = if config.click_jitter_px > 0 {
                                    rng.gen_range(-config.click_jitter_px..=config.click_jitter_px)
                                } else {
                                    0
                                };
                                let jy = if config.click_jitter_px > 0 {
                                    rng.gen_range(-config.click_jitter_px..=config.click_jitter_px)
                                } else {
                                    0
                                };

                                let target_x = m.center_x + jx;
                                let target_y = m.center_y + jy;

                                WinInput::set_cursor_pos(target_x, target_y);

                                match template.click_type {
                                    ClickType::Single => {
                                        WinInput::mouse_click(template.click_button);
                                    }
                                    ClickType::Double => {
                                        WinInput::mouse_click(template.click_button);
                                        PrecisionTimer::sleep_ms(35.0);
                                        WinInput::mouse_click(template.click_button);
                                    }
                                    ClickType::Triple => {
                                        for _ in 0..3 {
                                             WinInput::mouse_click(template.click_button);
                                             PrecisionTimer::sleep_ms(30.0);
                                        }
                                    }
                                    ClickType::Hold => {
                                        WinInput::mouse_down(template.click_button);
                                        PrecisionTimer::sleep_ms(template.hold_ms.max(10.0));
                                        WinInput::mouse_up(template.click_button);
                                    }
                                }

                                total.fetch_add(1, Ordering::Relaxed);

                                if config.return_cursor {
                                    WinInput::set_cursor_pos(orig_pos.0, orig_pos.1);
                                }

                                clicked = true;
                                PrecisionTimer::sleep_ms(config.cooldown_ms.max(20.0));
                                break;
                            }
                        }

                        if !clicked {
                            PrecisionTimer::sleep_ms(config.scan_interval_ms.max(10.0));
                        }
                    } else {
                        PrecisionTimer::sleep_ms(50.0);
                    }
                }

                running.store(false, Ordering::Relaxed);
            });
        }
    }
}
