use std::time::{Duration, Instant};

#[link(name = "winmm")]
extern "system" {
    fn timeBeginPeriod(uPeriod: u32) -> u32;
    fn timeEndPeriod(uPeriod: u32) -> u32;
}

pub struct PrecisionTimer;

impl PrecisionTimer {
    pub fn init() {
        unsafe {
            timeBeginPeriod(1);
        }
    }

    pub fn cleanup() {
        unsafe {
            timeEndPeriod(1);
        }
    }

    /// High-precision sleep combining thread::sleep with spin-lock for the last 1.5ms
    pub fn sleep_ms(ms: f64) {
        if ms <= 0.0 {
            return;
        }

        let start = Instant::now();
        let target = Duration::from_secs_f64(ms / 1000.0);

        // Sleep coarse duration if target > 2.5ms
        if ms > 2.5 {
            let coarse_sleep = Duration::from_secs_f64((ms - 1.5) / 1000.0);
            std::thread::sleep(coarse_sleep);
        }

        // Spin-wait for sub-millisecond precision
        while start.elapsed() < target {
            std::hint::spin_loop();
        }
    }
}
