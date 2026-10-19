use std::path::Path;
use crate::screen_capture::ScreenCapture;
use crate::win_input::{ClickType, MouseButton};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionMode {
    AutoPopup,     // Smart Pop-up Hunter (Screen Diff & Blob Detection) - 0 manual cropping!
    TemplateMatch, // Traditional template matching
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupColorFilter {
    AnyAppearance, // Любой новый объект / кнопка
    BrightOnly,    // Только яркие / светящиеся элементы
    GoldYellow,    // Золотые / жёлтые монетки и кнопки
    Green,         // Зелёные кнопки
    Red,           // Красные кнопки / мишени
    CyanBlue,      // Голубые / синие кнопки
}

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

    pub fn from_file<P: AsRef<Path>>(
        id: u64,
        name: String,
        path: P,
        threshold: f32,
    ) -> Result<Self, String> {
        let img = image::open(path).map_err(|e| format!("Ошибка загрузки: {}", e))?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();

        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for pixel in rgba.pixels() {
            bgra.push(pixel[2]); // B
            bgra.push(pixel[1]); // G
            bgra.push(pixel[0]); // R
            bgra.push(pixel[3]); // A
        }

        Ok(Self::new(id, name, w, h, bgra, threshold))
    }

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
    pub mode: VisionMode,

    // Auto Pop-up Hunter parameters
    pub popup_min_size: i32,     // default: 20 px
    pub popup_max_size: i32,     // default: 350 px
    pub popup_sensitivity: u32,  // default: 45
    pub popup_filter: PopupColorFilter,
    pub popup_click_button: MouseButton,
    pub popup_click_type: ClickType,
    pub popup_hold_ms: f64,

    // Template match parameters
    pub templates: Vec<ButtonTemplate>,

    // General engine parameters
    pub scan_interval_ms: f64, // e.g. 35.0 ms
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
            mode: VisionMode::AutoPopup, // Default to Smart Auto Pop-up Hunter

            popup_min_size: 20,
            popup_max_size: 350,
            popup_sensitivity: 45,
            popup_filter: PopupColorFilter::AnyAppearance,
            popup_click_button: MouseButton::Left,
            popup_click_type: ClickType::Single,
            popup_hold_ms: 100.0,

            templates: Vec::new(),

            scan_interval_ms: 35.0,
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
        let coarse_tolerance = (1.0 - template.threshold) * 1.5;
        let max_allowed_coarse_diff = (max_coarse_diff * coarse_tolerance) as u32;

        let screen_buf = &screen.buffer;
        let tmpl_buf = &template.bgra;

        let mut best_match: Option<MatchResult> = None;
        let mut best_conf = template.threshold;

        let stride_y = if th > 40 { 2 } else { 1 };
        let stride_x = if tw > 40 { 2 } else { 1 };

        let mut y = min_y;
        while y <= max_y {
            let mut x = min_x;
            while x <= max_x {
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

/// Smart Pop-up Hunter Engine
/// Automatically detects newly appeared buttons and interactive elements without manual templates!
pub struct PopupDetector {
    prev_grid: Option<Vec<(u8, u8, u8)>>,
    grid_w: usize,
    grid_h: usize,
    cell_size: usize,
}

impl PopupDetector {
    pub fn new() -> Self {
        Self {
            prev_grid: None,
            grid_w: 0,
            grid_h: 0,
            cell_size: 10, // 10x10 px cells for sub-millisecond scan speed
        }
    }

    pub fn reset(&mut self) {
        self.prev_grid = None;
    }

    /// Scans the screen, detects newly appeared objects / popups, and returns center (X, Y).
    pub fn detect_popup(
        &mut self,
        screen: &ScreenCapture,
        config: &VisionConfig,
    ) -> Option<(i32, i32)> {
        let sw = screen.width as usize;
        let sh = screen.height as usize;
        let cell_size = self.cell_size;

        let (min_x, min_y, max_x, max_y) = if config.use_custom_region {
            let x1 = (config.region_x1.min(config.region_x2)).max(0) as usize;
            let y1 = (config.region_y1.min(config.region_y2)).max(0) as usize;
            let x2 = (config.region_x1.max(config.region_x2)).min(sw as i32) as usize;
            let y2 = (config.region_y1.max(config.region_y2)).min(sh as i32) as usize;
            (x1, y1, x2, y2)
        } else {
            (0, 0, sw, sh)
        };

        if min_x >= max_x || min_y >= max_y {
            return None;
        }

        let region_w = max_x - min_x;
        let region_h = max_y - min_y;

        let grid_w = region_w / cell_size;
        let grid_h = region_h / cell_size;

        if grid_w == 0 || grid_h == 0 {
            return None;
        }

        // Sample current frame grid
        let mut curr_grid = Vec::with_capacity(grid_w * grid_h);
        let screen_buf = &screen.buffer;

        for gy in 0..grid_h {
            let sy = min_y + gy * cell_size + cell_size / 2;
            let row_offset = sy * sw * 4;

            for gx in 0..grid_w {
                let sx = min_x + gx * cell_size + cell_size / 2;
                let idx = row_offset + sx * 4;

                if idx + 2 < screen_buf.len() {
                    let b = screen_buf[idx];
                    let g = screen_buf[idx + 1];
                    let r = screen_buf[idx + 2];
                    curr_grid.push((r, g, b));
                } else {
                    curr_grid.push((0, 0, 0));
                }
            }
        }

        let prev = match &self.prev_grid {
            Some(p) if p.len() == curr_grid.len() => p,
            _ => {
                // Initialize baseline
                self.prev_grid = Some(curr_grid);
                self.grid_w = grid_w;
                self.grid_h = grid_h;
                return None;
            }
        };

        let mut mask = vec![false; grid_w * grid_h];
        let sens = config.popup_sensitivity;

        for i in 0..curr_grid.len() {
            let (cr, cg, cb) = curr_grid[i];
            let (pr, pg, pb) = prev[i];

            let delta = (cr as i32 - pr as i32).abs() as u32
                + (cg as i32 - pg as i32).abs() as u32
                + (cb as i32 - pb as i32).abs() as u32;

            let changed = delta >= sens;

            if changed {
                let passes_filter = match config.popup_filter {
                    PopupColorFilter::AnyAppearance => true,
                    PopupColorFilter::BrightOnly => {
                        let lum = (cr as u32 * 299 + cg as u32 * 587 + cb as u32 * 114) / 1000;
                        lum > 155
                    }
                    PopupColorFilter::GoldYellow => {
                        cr > 140 && cg > 105 && cb < 130 && cr >= cb + 40
                    }
                    PopupColorFilter::Green => {
                        cg > 120 && cg >= cr + 20 && cg >= cb + 20
                    }
                    PopupColorFilter::Red => {
                        cr > 130 && cr >= cg + 30 && cr >= cb + 30
                    }
                    PopupColorFilter::CyanBlue => {
                        cb > 130 && (cg > 95 || cb >= cr + 25)
                    }
                };

                if passes_filter {
                    mask[i] = true;
                }
            }
        }

        // BFS Connected Components to identify target button bounding box
        let mut visited = vec![false; grid_w * grid_h];
        let mut best_target: Option<(i32, i32)> = None;
        let mut best_size: usize = 0;

        let min_cells = ((config.popup_min_size / cell_size as i32).max(1) as usize).pow(2) / 3;
        let max_w_cells = (config.popup_max_size / cell_size as i32).max(1) as usize;
        let max_h_cells = max_w_cells;

        for gy in 0..grid_h {
            for gx in 0..grid_w {
                let idx = gy * grid_w + gx;
                if mask[idx] && !visited[idx] {
                    let mut queue = std::collections::VecDeque::new();
                    queue.push_back((gx, gy));
                    visited[idx] = true;

                    let mut blob_min_x = gx;
                    let mut blob_max_x = gx;
                    let mut blob_min_y = gy;
                    let mut blob_max_y = gy;
                    let mut cell_count = 0;

                    while let Some((cx, cy)) = queue.pop_front() {
                        cell_count += 1;
                        blob_min_x = blob_min_x.min(cx);
                        blob_max_x = blob_max_x.max(cx);
                        blob_min_y = blob_min_y.min(cy);
                        blob_max_y = blob_max_y.max(cy);

                        let neighbors = [
                            (cx.wrapping_sub(1), cy),
                            (cx + 1, cy),
                            (cx, cy.wrapping_sub(1)),
                            (cx, cy + 1),
                        ];

                        for &(nx, ny) in &neighbors {
                            if nx < grid_w && ny < grid_h {
                                let n_idx = ny * grid_w + nx;
                                if mask[n_idx] && !visited[n_idx] {
                                    visited[n_idx] = true;
                                    queue.push_back((nx, ny));
                                }
                            }
                        }
                    }

                    let blob_w = blob_max_x - blob_min_x + 1;
                    let blob_h = blob_max_y - blob_min_y + 1;

                    // Filter out single noise pixels and full-screen scene changes
                    if cell_count >= min_cells.max(2)
                        && blob_w <= max_w_cells
                        && blob_h <= max_h_cells
                    {
                        if cell_count > best_size {
                            best_size = cell_count;
                            let center_gx = (blob_min_x + blob_max_x) / 2;
                            let center_gy = (blob_min_y + blob_max_y) / 2;

                            let real_x = (min_x + center_gx * cell_size + cell_size / 2) as i32;
                            let real_y = (min_y + center_gy * cell_size + cell_size / 2) as i32;
                            best_target = Some((real_x, real_y));
                        }
                    }
                }
            }
        }

        // Always update baseline
        self.prev_grid = Some(curr_grid);

        best_target
    }
}
