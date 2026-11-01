//! Shared capture and matching pipeline for the running hunter and its preview.
use crate::engine::AdvancedConfig;
use crate::screen_capture::ScreenCapture;
use crate::vision::{ButtonTemplate, PopupDetector, VisionConfig, VisionEngine, VisionMode};
use crate::win_input::{ClickType, MouseButton, WinInput};
use std::time::Duration;

pub struct HunterFrame {
    pub image: ScreenCapture,
    pub origin: (i32, i32),
    pub hwnd: usize,
}

pub struct HunterTarget {
    pub x: i32,
    pub y: i32,
    pub button: MouseButton,
    pub click_type: ClickType,
    pub hold_ms: f64,
    pub method: String,
}

/// A session continues observing frames while input is in flight or cooling down.
/// Only input is throttled; target visibility and identity are never frozen by it.
pub struct HunterSession {
    config: VisionConfig,
    detector: PopupDetector,
    pending: Option<crate::vision::InputAttempt>,
    next_input_at: Duration,
    geometry: Option<(usize, i32, i32, i32, i32)>,
}

pub struct Observation {
    pub target: Option<HunterTarget>,
    pub start_input: bool,
}

impl HunterSession {
    pub fn new(config: VisionConfig) -> Self {
        Self { config, detector: PopupDetector::new(), pending: None,
            next_input_at: Duration::ZERO, geometry: None }
    }

    pub fn observe(&mut self, frame: &HunterFrame, now: Duration,
        running: Option<&std::sync::atomic::AtomicBool>) -> Observation {
        let geometry = (frame.hwnd, frame.origin.0, frame.origin.1, frame.image.width, frame.image.height);
        if self.geometry != Some(geometry) {
            self.detector = PopupDetector::new();
            self.geometry = Some(geometry);
        }
        let target = find_cancellable(frame, &self.config, &mut self.detector, running);
        let start_input = target.is_some() && self.pending.is_none() && now >= self.next_input_at
            && !running.is_some_and(|r| !r.load(std::sync::atomic::Ordering::Relaxed));
        if start_input {
            let t = target.as_ref().unwrap();
            self.pending = Some(self.detector.begin_attempt(t.x - frame.origin.0, t.y - frame.origin.1));
        }
        Observation { target, start_input }
    }

    pub fn complete_input(&mut self, succeeded: bool, now: Duration) {
        if let Some(attempt) = self.pending.take() {
            if !succeeded { self.detector.input_failed(&attempt); }
        }
        self.next_input_at = now + Duration::from_secs_f64(
            self.config.cooldown_ms.max(if succeeded { 50.0 } else { 1000.0 }) / 1000.0);
    }

    pub fn input_pending(&self) -> bool { self.pending.is_some() }
}

pub fn intersect_rect(
    a: (i32, i32, i32, i32),
    b: (i32, i32, i32, i32),
) -> Option<(i32, i32, i32, i32)> {
    let r = (a.0.max(b.0), a.1.max(b.1), a.2.min(b.2), a.3.min(b.3));
    (r.0 < r.2 && r.1 < r.3).then_some(r)
}

pub fn capture(config: &VisionConfig, adv: &AdvancedConfig) -> Result<HunterFrame, String> {
    let active = WinInput::get_foreground_window();
    let hwnd = selected_window(adv.window_target.hwnd, active)?;
    if !WinInput::is_window_valid(hwnd) { return Err("Выбранная игра закрыта. Выберите её снова".into()); }
    let mut pid = 0;
    unsafe {
        winapi::um::winuser::GetWindowThreadProcessId(hwnd as _, &mut pid);
    }
    if pid == std::process::id() {
        return Err("Пауза: откройте игру. F6 — запуск / остановка".into());
    }
    if adv.target_window_enabled
        && !adv.target_window_title.trim().is_empty()
        && !WinInput::get_window_title_by_hwnd(hwnd)
            .to_lowercase()
            .contains(&adv.target_window_title.to_lowercase())
    {
        return Err("Пауза: активное окно не соответствует фильтру названия".into());
    }
    if unsafe { winapi::um::winuser::IsIconic(hwnd as _) } != 0 {
        return Err("Пауза: разверните окно игры".into());
    }
    let (_, _, w, h) =
        WinInput::get_window_client_rect(hwnd).ok_or("Не удалось получить размер окна")?;
    let (x, y) =
        WinInput::client_to_screen(hwnd, 0, 0).ok_or("Не удалось получить положение окна")?;
    let mut rect = (x, y, x + w, y + h);
    if config.use_custom_region {
        let roi = (
            config.region_x1.min(config.region_x2),
            config.region_y1.min(config.region_y2),
            config.region_x1.max(config.region_x2),
            config.region_y1.max(config.region_y2),
        );
        rect = intersect_rect(rect, roi).ok_or("Область поиска находится вне окна игры")?;
    }
    // Search visible pixels. PrintWindow often returns successful but black frames for games.
    let mut covered = WinInput::occluding_rects(hwnd).ok_or("Не удалось проверить перекрытие окна игры")?;
    let mut image = ScreenCapture::capture_rect(rect.0, rect.1, rect.2 - rect.0, rect.3 - rect.1)
        .ok_or("Ошибка захвата. Попробуйте оконный режим без рамки")?;
    // Some games do not expose their accelerated surface through desktop BitBlt.
    if !image.buffer.chunks_exact(4).any(|p| p[0] > 4 || p[1] > 4 || p[2] > 4) {
        if let Some(window) = ScreenCapture::capture_window(hwnd) {
            let dx = rect.0 - x; let dy = rect.1 - y;
            if dx >= 0 && dy >= 0 && dx + image.width <= window.width && dy + image.height <= window.height {
                for row in 0..image.height {
                    let source = (((dy + row) * window.width + dx) * 4) as usize;
                    let target = (row * image.width * 4) as usize;
                    let len = (image.width * 4) as usize;
                    image.buffer[target..target+len].copy_from_slice(&window.buffer[source..source+len]);
                }
            }
        }
    }
    let raw_nonblack = image.buffer.chunks_exact(4).any(|p| p[0] > 4 || p[1] > 4 || p[2] > 4);
    // GDI returns undefined alpha. Reserve alpha=0 exclusively for covered pixels.
    for p in image.buffer.chunks_exact_mut(4) { p[3] = 255; }
    covered.extend(WinInput::occluding_rects(hwnd).ok_or("Не удалось проверить перекрытие окна игры")?);
    mask_occlusions(&mut image, (rect.0, rect.1), &covered);
    if !image
        .buffer
        .chunks_exact(4)
        .any(|p| p[0] > 4 || p[1] > 4 || p[2] > 4)
    {
        return Err(if raw_nonblack { "Окно игры перекрыто другим окном. Освободите игровую область" }
            else { "Захват игры вернул чёрный кадр. Переключите игру в оконный режим без рамки" }.into());
    }
    Ok(HunterFrame {
        image,
        origin: (rect.0, rect.1),
        hwnd,
    })
}

pub fn selected_window(selected: usize, active: usize) -> Result<usize, String> {
    if selected == 0 { return Err("Сначала выберите окно игры".into()); }
    if selected != active { return Err("Пауза: вернитесь в выбранную игру".into()); }
    Ok(selected)
}

pub fn mask_occlusions(image: &mut ScreenCapture, origin: (i32, i32), covered: &[(i32, i32, i32, i32)]) {
    let bounds = (origin.0, origin.1, origin.0 + image.width, origin.1 + image.height);
    for &occlusion in covered {
        if let Some((x1, y1, x2, y2)) = intersect_rect(bounds, occlusion) {
            for y in y1..y2 {
                let start = (((y - origin.1) * image.width + x1 - origin.0) * 4) as usize;
                let end = start + ((x2 - x1) * 4) as usize;
                image.buffer[start..end].fill(0);
            }
        }
    }
}

pub fn find(
    frame: &HunterFrame,
    config: &VisionConfig,
    detector: &mut PopupDetector,
) -> Option<HunterTarget> {
    find_cancellable(frame, config, detector, None)
}

pub fn find_cancellable(
    frame: &HunterFrame,
    config: &VisionConfig,
    detector: &mut PopupDetector,
    running: Option<&std::sync::atomic::AtomicBool>,
) -> Option<HunterTarget> {
    detector.template_appearances.begin_frame();
    let mut selected = None;
    if config.mode != VisionMode::AutoPopup {
        for template in config.templates.iter().filter(|t| t.enabled) {
            let scales: &[f32] = if config.multiscale {
                &[1.0, 0.75, 1.25, 0.5, 1.5, 2.0]
            } else {
                &[1.0]
            };
            for &scale in scales {
                if running.is_some_and(|r| !r.load(std::sync::atomic::Ordering::Relaxed)) {
                    return None;
                }
                let scaled;
                let tmpl = if scale == 1.0 {
                    template
                } else {
                    let w = (template.width as f32 * scale).round() as u32;
                    let h = (template.height as f32 * scale).round() as u32;
                    if w == 0
                        || h == 0
                        || w > frame.image.width as u32
                        || h > frame.image.height as u32
                    {
                        continue;
                    }
                    let image = image::RgbaImage::from_raw(
                        template.width,
                        template.height,
                        template.bgra.clone(),
                    )?;
                    let pixels =
                        image::imageops::resize(&image, w, h, image::imageops::FilterType::Nearest)
                            .into_raw();
                    scaled = ButtonTemplate::new(
                        template.id,
                        template.name.clone(),
                        w,
                        h,
                        pixels,
                        template.threshold,
                    );
                    &scaled
                };
                for m in VisionEngine::find_template_candidates(&frame.image, tmpl, None, running) {
                    if config.only_new_targets && !detector.template_appearances.observe(m.center_x, m.center_y,
                        (tmpl.width.min(tmpl.height) as i32 / 3).clamp(8, 40)) { continue; }
                    if selected.is_none() { selected = Some(HunterTarget {
                        x: m.center_x + frame.origin.0,
                        y: m.center_y + frame.origin.1,
                        button: template.click_button,
                        click_type: template.click_type,
                        hold_ms: template.hold_ms,
                        method: format!(
                            "Образец «{}» · {:.0}% · ×{:.2}",
                            template.name,
                            m.confidence * 100.0,
                            scale
                        ),
                    }); }
                }
            }
        }
    }
    if config.mode != VisionMode::TemplateMatch {
        // The frame is already cropped to the ROI. All detector coordinates are local.
        let mut local = config.clone();
        local.use_custom_region = false;
        if config.use_custom_region {
            local.exclude_hud = false;
        }
        if let Some((x, y)) = detector.detect_popup(&frame.image, &local) {
            if selected.is_none() { selected = Some(HunterTarget {
                x: x + frame.origin.0,
                y: y + frame.origin.1,
                button: config.popup_click_button,
                click_type: config.popup_click_type,
                hold_ms: config.popup_hold_ms,
                method: "Цвет и форма".into(),
            }); }
        }
    }
    detector.template_appearances.finish_frame(&frame.image);
    selected
}

pub fn preview(frame: &HunterFrame, target: Option<&HunterTarget>) -> (Vec<u8>, usize, usize) {
    let mut rgba = frame.image.buffer.clone();
    for p in rgba.chunks_exact_mut(4) {
        p.swap(0, 2);
        p[3] = 255;
    }
    let source =
        image::RgbaImage::from_raw(frame.image.width as u32, frame.image.height as u32, rgba)
            .unwrap();
    let scale = (600.0 / source.width() as f32)
        .min(260.0 / source.height() as f32)
        .min(1.0);
    let mut thumb = image::imageops::resize(
        &source,
        (source.width() as f32 * scale).max(1.0) as u32,
        (source.height() as f32 * scale).max(1.0) as u32,
        image::imageops::FilterType::Triangle,
    );
    if let Some(t) = target {
        let x = ((t.x - frame.origin.0) as f32 * scale) as i32;
        let y = ((t.y - frame.origin.1) as f32 * scale) as i32;
        for d in -9..=9 {
            for (px, py) in [(x + d, y), (x, y + d)] {
                if px >= 0 && py >= 0 && px < thumb.width() as i32 && py < thumb.height() as i32 {
                    thumb.put_pixel(px as u32, py as u32, image::Rgba([255, 40, 40, 255]));
                }
            }
        }
    }
    let (w, h) = thumb.dimensions();
    (thumb.into_raw(), w as usize, h as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roi_intersection_handles_negative_monitor_coordinates() {
        assert_eq!(
            intersect_rect((-1200, 50, -200, 800), (-1000, 100, -600, 400)),
            Some((-1000, 100, -600, 400))
        );
        assert_eq!(intersect_rect((100, 100, 300, 300), (0, 0, 90, 90)), None);
    }
}
