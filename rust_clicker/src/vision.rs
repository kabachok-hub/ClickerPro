use std::path::Path;
use crate::screen_capture::ScreenCapture;
use crate::win_input::{ClickType, MouseButton};

#[derive(Debug, Clone)]
pub struct ButtonTemplate {
    pub id: u64,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
    pub sample_points: Vec<(u32, u32, u8, u8, u8)>, // (dx, dy, r, g, b)
    pub threshold: f32, // 0.60 to 0.99 (default 0.82)
    pub click_button: MouseButton,
    pub click_type: ClickType,
    pub hold_ms: f64,
    pub enabled: bool,
}

impl ButtonTemplate {
    pub fn new(
        id: u64,
        name: String,
        width: u32,
        height: u32,
        bgra: Vec<u8>,
        threshold: f32,
    ) -> Self {
        let mut sample_points = Vec::new();
        if width > 0 && height > 0 {
            // Pick a grid of up to 48 sample points across the template
            let step_x = (width / 7).max(2);
            let step_y = (height / 7).max(2);

            let mut y = step_y / 2;
            while y < height {
                let mut x = step_x / 2;
                while x < width {
                    let idx = ((y * width + x) * 4) as usize;
                    if idx + 2 < bgra.len() {
                        let b = bgra[idx];
                        let g = bgra[idx + 1];
                        let r = bgra[idx + 2];
                        sample_points.push((x, y, r, g, b));
                    }
                    x += step_x;
                }
                y += step_y;
            }
        }

        Self {
            id,
            name,
            width,
            height,
            bgra,
            sample_points,
            threshold,
            click_button: MouseButton::Left,
            click_type: ClickType::Single,
            hold_ms: 100.0,
            enabled: true,
        }
    }

    /// Load template from an image file (PNG / JPG / BMP)
    pub fn from_file<P: AsRef<Path>>(
        id: u64,
        name: String,
        path: P,
        threshold: f32,
    ) -> Result<Self, String> {
        let img = image::open(path).map_err(|e| format!("Ошибка загрузки: {}", e))?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();

        // Convert RGBA to BGRA
        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for pixel in rgba.pixels() {
            bgra.push(pixel[2]); // B
            bgra.push(pixel[1]); // G
            bgra.push(pixel[0]); // R
            bgra.push(pixel[3]); // A
        }

        Ok(Self::new(id, name, w, h, bgra, threshold))
    }

    /// Create template from captured screen region
    pub fn from_capture(
        id: u64,
        name: String,
        cap: &ScreenCapture,
        threshold: f32,
    ) -> Self {
        Self::new(
            id,
            name,
            cap.width as u32,
            cap.height as u32,
            cap.buffer.clone(),
            threshold,
        )
    }
}

#[derive(Debug, Clone)]
pub struct VisionConfig {
    pub templates: Vec<ButtonTemplate>,
    pub scan_interval_ms: f64, // e.g. 40.0 ms
    pub cooldown_ms: f64,      // delay after clicking before scanning again (e.g. 250 ms)
    pub return_cursor: bool,   // move cursor back to original place after clicking
    pub click_jitter_px: i32,  // small random offset from exact center (+-px)
    pub use_custom_region: bool,
    pub region_x1: i32,
    pub region_y1: i32,
    pub region_x2: i32,
    pub region_y2: i32,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            templates: Vec::new(),
            scan_interval_ms: 50.0,
            cooldown_ms: 250.0,
            return_cursor: true,
            click_jitter_px: 2,
            use_custom_region: false,
            region_x1: 0,
            region_y1: 0,
            region_x2: 1920,
            region_y2: 1080,
        }
    }
}

#[allow(dead_code)]
pub struct MatchResult {
    pub x: i32,
    pub y: i32,
    pub center_x: i32,
    pub center_y: i32,
    pub confidence: f32,
}

pub struct VisionEngine;

impl VisionEngine {
    /// Fast template matching across the screen capture
    pub fn find_template(
        screen: &ScreenCapture,
        template: &ButtonTemplate,
        region_opt: Option<(i32, i32, i32, i32)>,
    ) -> Option<MatchResult> {
        let sw = screen.width as u32;
        let sh = screen.height as u32;
        let tw = template.width;
        let th = template.height;

        if tw == 0 || th == 0 || sw < tw || sh < th || template.sample_points.is_empty() {
            return None;
        }

        // Determine bounding box to scan
        let (min_x, min_y, max_x, max_y) = if let Some((r_x1, r_y1, r_x2, r_y2)) = region_opt {
            let start_x = (r_x1.min(r_x2)).max(0) as u32;
            let start_y = (r_y1.min(r_y2)).max(0) as u32;
            let end_x = (r_x1.max(r_x2)).min(sw as i32) as u32;
            let end_y = (r_y1.max(r_y2)).min(sh as i32) as u32;

            (
                start_x,
                start_y,
                if end_x >= tw { end_x - tw } else { 0 },
                if end_y >= th { end_y - th } else { 0 },
            )
        } else {
            (0, 0, sw - tw, sh - th)
        };

        if min_x > max_x || min_y > max_y {
            return None;
        }

        let num_samples = template.sample_points.len();
        let max_coarse_diff = (num_samples * 3 * 255) as f32;
        let coarse_tolerance = (1.0 - template.threshold) * 1.5; // generous coarse tolerance
        let max_allowed_coarse_diff = (max_coarse_diff * coarse_tolerance) as u32;

        let screen_buf = &screen.buffer;
        let tmpl_buf = &template.bgra;

        // Step 1: Scan with stride of 2 for maximum speed
        let mut best_match: Option<MatchResult> = None;
        let mut best_conf = template.threshold;

        let stride_y = if th > 40 { 2 } else { 1 };
        let stride_x = if tw > 40 { 2 } else { 1 };

        let mut y = min_y;
        while y <= max_y {
            let mut x = min_x;
            while x <= max_x {
                // Coarse check on sample points
                let mut coarse_diff: u32 = 0;
                let mut early_reject = false;

                for &(dx, dy, tr, tg, tb) in &template.sample_points {
                    let s_x = x + dx;
                    let s_y = y + dy;
                    let s_idx = ((s_y * sw + s_x) * 4) as usize;

                    if s_idx + 2 < screen_buf.len() {
                        let sb = screen_buf[s_idx];
                        let sg = screen_buf[s_idx + 1];
                        let sr = screen_buf[s_idx + 2];

                        let diff = (sr as i32 - tr as i32).abs() as u32
                            + (sg as i32 - tg as i32).abs() as u32
                            + (sb as i32 - tb as i32).abs() as u32;

                        coarse_diff += diff;
                        if coarse_diff > max_allowed_coarse_diff {
                            early_reject = true;
                            break;
                        }
                    }
                }

                if !early_reject {
                    // Candidate found! Perform full pixel verification
                    let total_pixels = (tw * th) as f32;
                    let max_total_diff = total_pixels * 3.0 * 255.0;
                    let mut total_diff: u64 = 0;
                    let max_allowed_total_diff = (max_total_diff * (1.0 - best_conf)) as u64;

                    let mut full_reject = false;

                    for ty in 0..th {
                        let s_row_idx = (((y + ty) * sw + x) * 4) as usize;
                        let t_row_idx = ((ty * tw) * 4) as usize;

                        for tx in 0..tw {
                            let s_i = s_row_idx + (tx * 4) as usize;
                            let t_i = t_row_idx + (tx * 4) as usize;

                            if s_i + 2 < screen_buf.len() && t_i + 2 < tmpl_buf.len() {
                                let sb = screen_buf[s_i];
                                let sg = screen_buf[s_i + 1];
                                let sr = screen_buf[s_i + 2];

                                let tb = tmpl_buf[t_i];
                                let tg = tmpl_buf[t_i + 1];
                                let tr = tmpl_buf[t_i + 2];

                                let d = (sr as i32 - tr as i32).abs() as u64
                                    + (sg as i32 - tg as i32).abs() as u64
                                    + (sb as i32 - tb as i32).abs() as u64;

                                total_diff += d;
                                if total_diff > max_allowed_total_diff {
                                    full_reject = true;
                                    break;
                                }
                            }
                        }
                        if full_reject {
                            break;
                        }
                    }

                    if !full_reject {
                        let conf = 1.0 - (total_diff as f32 / max_total_diff);
                        if conf >= best_conf {
                            best_conf = conf;
                            best_match = Some(MatchResult {
                                x: x as i32,
                                y: y as i32,
                                center_x: (x + tw / 2) as i32,
                                center_y: (y + th / 2) as i32,
                                confidence: conf,
                            });

                            // If we found an extremely high confidence match (> 94%), stop scanning immediately
                            if conf > 0.94 {
                                return best_match;
                            }
                        }
                    }
                }

                x += stride_x;
            }
            y += stride_y;
        }

        best_match
    }
}
