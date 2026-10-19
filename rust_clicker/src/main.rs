#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod engine;
mod hotkeys;
mod precision_timer;
mod screen_capture;
mod vision;
mod win_input;

use app::ClickerApp;
use precision_timer::PrecisionTimer;

fn main() -> eframe::Result<()> {
    unsafe {
        winapi::um::winuser::SetProcessDPIAware();
    }
    PrecisionTimer::init();

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([920.0, 680.0])
        .with_min_inner_size([840.0, 600.0])
        .with_title("⚡ Автокликер Pro (Rust)");

    // Try to load embedded icon
    let icon_bytes = include_bytes!("../../assets/icon.png");
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let icon_data = eframe::egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        };
        viewport = viewport.with_icon(icon_data);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let result = eframe::run_native(
        "⚡ Автокликер Pro",
        native_options,
        Box::new(|cc| Ok(Box::new(ClickerApp::new(cc)))),
    );

    PrecisionTimer::cleanup();
    result
}
