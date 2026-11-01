use std::path::Path;
use crate::screen_capture::ScreenCapture;
use crate::win_input::{ClickType, MouseButton};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionMode {
    AutoPopup,     // Smart Pop-up Hunter (Screen Diff & Blob Detection)
    TemplateMatch, // Traditional template matching
    Hybrid,        // Hybrid: matches snipped templates first, then auto-popups
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupColorFilter {
    UniversalCircle,    // 🎯 Авто-охотник (Roblox x2 + Любые цветные кружки с иконкой)
    PurpleViolet,       // 🟣 Фиолетовый / Пурпурный кружок (Roblox x2 / x3 / x5 / Тап)
    WhitePointerInside, // 👆 Белая рука / тап внутри кружка
    AnyVibrant,         // 🌈 Любой цветной контрастный кружок / кнопка
    GoldYellow,         // 🟡 Золотые / жёлтые монетки
    Green,              // 🟢 Зелёные кнопки
    Red,                // 🔴 Красные кнопки
    CyanBlue,           // 🔵 Синие / голубые кнопки
    MotionDiff,         // ⚡ По движению / разнице кадров
}

#[allow(dead_code)]
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
                    if idx + 3 < bgra.len() && bgra[idx + 3] >= 128 {
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

        if sample_points.is_empty() && width > 0 {
            let visible: Vec<_> = bgra.chunks_exact(4).enumerate().filter(|(_,p)| p[3] >= 128).collect();
            for &(i,p) in visible.iter().step_by((visible.len()/49).max(1)).take(64) {
                sample_points.push((i as u32 % width, i as u32 / width, p[2], p[1], p[0]));
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

    #[allow(dead_code)]
    pub fn from_file<P: AsRef<Path>>(
        id: u64,
        name: String,
        path: P,
        threshold: f32,
    ) -> Result<Self, String> {
        let img = image::open(path).map_err(|e| format!("Ошибка загрузки: {}", e))?;
        let mut rgba = img.to_rgba8();
        let visible: Vec<_> = rgba.enumerate_pixels().filter(|(_,_,p)| p[3] >= 128).map(|(x,y,_)| (x,y)).collect();
        if visible.is_empty() { return Err("В картинке нет видимых пикселей".into()); }
        let x1 = visible.iter().map(|p| p.0).min().unwrap();
        let y1 = visible.iter().map(|p| p.1).min().unwrap();
        let x2 = visible.iter().map(|p| p.0).max().unwrap();
        let y2 = visible.iter().map(|p| p.1).max().unwrap();
        rgba = image::imageops::crop_imm(&rgba, x1, y1, x2-x1+1, y2-y1+1).to_image();
        let (w, h) = rgba.dimensions();

        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for pixel in rgba.pixels() {
            bgra.push(pixel[2]); // B
            bgra.push(pixel[1]); // G
            bgra.push(pixel[0]); // R
            bgra.push(pixel[3]); // A
        }

        let (w,h,bgra) = if bgra.chunks_exact(4).all(|p| p[3] == 255) {
            isolate_circle_sample(w,h,bgra)
        } else { (w,h,bgra) };
        Ok(Self::new(id, name, w, h, bgra, threshold))
    }

    pub fn from_capture(
        id: u64,
        name: String,
        cap: &ScreenCapture,
        threshold: f32,
    ) -> Self {
        let mut pixels = cap.buffer.clone();
        // Screen captures have no meaningful transparency, unlike imported PNG images.
        for pixel in pixels.chunks_exact_mut(4) { pixel[3] = 255; }
        let (width, height, pixels) = isolate_circle_sample(cap.width as u32, cap.height as u32, pixels);
        Self::new(id, name, width, height, pixels, threshold)
    }
}

// A cut-out of a round bonus should not encode the moving scenery behind its corners.
fn isolate_circle_sample(w: u32, h: u32, pixels: Vec<u8>) -> (u32, u32, Vec<u8>) {
    if w < 20 || h < 20 || w > 512 || h > 512 || pixels.len() != (w*h*4) as usize { return (w,h,pixels); }
    let rgb = |x: u32, y: u32| { let i=((y*w+x)*4) as usize; [pixels[i+2],pixels[i+1],pixels[i]] };
    let vibrant = |p: [u8;3]| p.iter().max().unwrap()-p.iter().min().unwrap() > 60;
    let ring: Vec<_> = (0..16).map(|n| {
        let a=n as f32*std::f32::consts::TAU/16.0;
        rgb((w as f32*(0.5+0.30*a.cos())) as u32,(h as f32*(0.5+0.30*a.sin())) as u32)
    }).filter(|&p| vibrant(p)).collect();
    let dominant = ring.iter().max_by_key(|&&p| ring.iter().filter(|&&q| same_color_family(p,q)).count()).copied();
    let Some(color) = dominant else { return (w,h,pixels); };
    let matches = |p| vibrant(p) && same_color_family(color,p);
    if ring.iter().filter(|&&p| matches(p)).count() < 10 { return (w,h,pixels); }
    let (mut x1,mut y1,mut x2,mut y2)=(w,h,0,0);
    for y in 0..h { for x in 0..w { if matches(rgb(x,y)) { x1=x1.min(x);y1=y1.min(y);x2=x2.max(x);y2=y2.max(y); } } }
    if x1>x2 || y1>y2 { return (w,h,pixels); }
    let (bw,bh)=(x2-x1+1,y2-y1+1);
    let ratio=bw as f32/bh as f32;
    if bw<w/2 || bh<h/2 || !(0.80..=1.25).contains(&ratio) { return (w,h,pixels); }
    let clear_corners=[(x1,y1),(x2,y1),(x1,y2),(x2,y2)].iter().filter(|&&(x,y)| !matches(rgb(x,y))).count();
    let white = (h/3..h*2/3).flat_map(|y| (w/3..w*2/3).map(move |x| (x,y)))
        .filter(|&(x,y)| { let p=rgb(x,y); p.iter().all(|&v|v>185) || (p[0]>175 && p[1]>150 && p[2]<160) }).count();
    if clear_corners<3 || white<8 { return (w,h,pixels); }
    let mut result=Vec::with_capacity((bw*bh*4) as usize);
    for y in y1..=y2 { for x in x1..=x2 {
        let i=((y*w+x)*4) as usize;
        let dx=(x as f32-(x1+x2) as f32/2.0)/(bw as f32/2.0);
        let dy=(y as f32-(y1+y2) as f32/2.0)/(bh as f32/2.0);
        result.extend_from_slice(&pixels[i..i+4]);
        if dx*dx+dy*dy>0.98 { *result.last_mut().unwrap()=0; }
    } }
    (bw,bh,result)
}

#[derive(Debug, Clone)]
pub struct VisionConfig {
    pub mode: VisionMode,
    pub only_new_targets: bool,
    pub roblox_profile: bool,
    pub round_only: bool,
    pub strict_circles: bool,
    pub require_white_icon: bool,
    pub exclude_hud: bool,
    pub multiscale: bool,
    pub background_input: bool,

    // Auto Pop-up Hunter parameters
    pub popup_min_size: i32,     // default: 20 px
    pub popup_max_size: i32,     // default: 350 px
    pub popup_sensitivity: u32,  // default: 45
    pub popup_filter: PopupColorFilter,
    pub popup_click_button: MouseButton,
    #[allow(dead_code)]
    pub popup_click_type: ClickType,
    pub popup_hold_ms: f64,

    // Template match parameters
    pub templates: Vec<ButtonTemplate>,

    // General engine parameters
    pub scan_interval_ms: f64, // e.g. 35.0 ms
    pub cooldown_ms: f64,      // delay after clicking before scanning again (e.g. 250 ms)
    pub return_cursor: bool,   // move cursor back to original place after clicking
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
            only_new_targets: true,
            roblox_profile: false,
            round_only: true,
            strict_circles: false,
            require_white_icon: false,
            exclude_hud: false,
            multiscale: true,
            background_input: false,

            popup_min_size: 25,
            popup_max_size: 240,
            popup_sensitivity: 40,
            popup_filter: PopupColorFilter::AnyVibrant,
            popup_click_button: MouseButton::Left,
            popup_click_type: ClickType::Single,
            popup_hold_ms: 45.0, // Critical for Roblox GUI event triggers!

            templates: Vec::new(),

            scan_interval_ms: 30.0,
            cooldown_ms: 220.0,
            return_cursor: true,
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
    #[cfg(test)]
    pub fn find_template(
        screen: &ScreenCapture,
        template: &ButtonTemplate,
        region_opt: Option<(i32, i32, i32, i32)>,
    ) -> Option<MatchResult> {
        Self::find_template_cancellable(screen, template, region_opt, None)
    }

    #[cfg(test)]
    pub fn find_template_cancellable(
        screen: &ScreenCapture, template: &ButtonTemplate,
        region_opt: Option<(i32, i32, i32, i32)>,
        running: Option<&std::sync::atomic::AtomicBool>,
    ) -> Option<MatchResult> {
        Self::find_template_candidates(screen, template, region_opt, running).into_iter().next()
    }

    pub fn find_template_candidates(
        screen: &ScreenCapture,
        template: &ButtonTemplate,
        region_opt: Option<(i32, i32, i32, i32)>,
        running: Option<&std::sync::atomic::AtomicBool>,
    ) -> Vec<MatchResult> {
        if screen.width <= 0 || screen.height <= 0 || screen.buffer.len() != screen.width as usize * screen.height as usize * 4 { return Vec::new(); }
        let sw = screen.width as u32;
        let sh = screen.height as u32;
        let tw = template.width;
        let th = template.height;

        if tw == 0 || th == 0 || sw < tw || sh < th || template.sample_points.is_empty() {
            return Vec::new();
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
            return Vec::new();
        }

        let num_samples = template.sample_points.len();
        let max_coarse_diff = (num_samples * 3 * 255) as f32;
        let coarse_tolerance = (1.0 - template.threshold) * 1.5;
        let max_allowed_coarse_diff = (max_coarse_diff * coarse_tolerance) as u32;

        let screen_buf = &screen.buffer;
        let tmpl_buf = &template.bgra;

        let mut matches: Vec<MatchResult> = Vec::new();
        let best_conf = template.threshold;

        let stride_y = 1;
        let stride_x = 1;

        let total_pixels = template.bgra.chunks_exact(4).filter(|p| p[3] >= 128).count() as f32;
        let max_total_diff = total_pixels * 3.0 * 255.0;
        let mut y = min_y;
        while y <= max_y {
            if running.is_some_and(|r| !r.load(std::sync::atomic::Ordering::Relaxed)) { return Vec::new(); }
            let mut x = min_x;
            while x <= max_x {
                let mut coarse_diff: u32 = 0;
                let mut early_reject = false;

                for &(dx, dy, tr, tg, tb) in &template.sample_points {
                    let s_x = x + dx;
                    let s_y = y + dy;
                    let s_idx = ((s_y * sw + s_x) * 4) as usize;

                    if s_idx + 3 < screen_buf.len() && screen_buf[s_idx + 3] == 0 {
                        early_reject = true; break;
                    }
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
                    let mut total_diff: u64 = 0;
                    let max_allowed_total_diff = (max_total_diff * (1.0 - best_conf)) as u64;

                    let mut full_reject = false;

                    for ty in 0..th {
                        let s_row_idx = (((y + ty) * sw + x) * 4) as usize;
                        let t_row_idx = ((ty * tw) * 4) as usize;

                        for tx in 0..tw {
                            let s_i = s_row_idx + (tx * 4) as usize;
                            let t_i = t_row_idx + (tx * 4) as usize;

                            if s_i + 2 < screen_buf.len() && t_i + 3 < tmpl_buf.len() && tmpl_buf[t_i + 3] >= 128 {
                                if screen_buf[s_i + 3] == 0 { full_reject = true; break; }
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
                            let candidate = MatchResult {
                                x: x as i32,
                                y: y as i32,
                                center_x: (x + tw / 2) as i32,
                                center_y: (y + th / 2) as i32,
                                confidence: conf,
                            };
                            let duplicate = matches.iter_mut().find(|m|
                                (m.center_x - candidate.center_x).abs() < (tw as i32 * 3 / 4).max(2)
                                && (m.center_y - candidate.center_y).abs() < (th as i32 * 3 / 4).max(2));
                            if let Some(m) = duplicate {
                                if candidate.confidence > m.confidence { *m = candidate; }
                            } else if matches.len() < 64 { matches.push(candidate); }
                        }
                    }
                }

                x += stride_x;
            }
            y += stride_y;
        }

        matches.sort_by(|a,b| b.confidence.total_cmp(&a.confidence));
        matches
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddedRobloxTemplate {
    pub width: u32,
    pub height: u32,
    pub sample_points: Vec<(u32, u32, u8, u8, u8)>, // (dx, dy, r, g, b)
}

impl EmbeddedRobloxTemplate {
    pub fn load() -> Option<Self> {
        let bytes = include_bytes!("../../assets/roblox_button.png");
        let img = image::load_from_memory(bytes).ok()?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        if w == 0 || h == 0 {
            return None;
        }

        let step_x = (w / 8).max(2);
        let step_y = (h / 8).max(2);
        let mut sample_points = Vec::new();

        let mut y = step_y / 2;
        while y < h {
            let mut x = step_x / 2;
            while x < w {
                let pixel = rgba.get_pixel(x, y);
                // Only sample non-transparent pixels
                if pixel[3] > 64 {
                    sample_points.push((x, y, pixel[0], pixel[1], pixel[2]));
                }
                x += step_x;
            }
            y += step_y;
        }

        Some(Self {
            width: w,
            height: h,
            sample_points,
        })
    }
}

/// Smart Pop-up Hunter Engine
/// Automatically detects newly appeared buttons and interactive elements without manual templates!
#[derive(Default)]
pub struct AppearanceTracker {
    frame: u64,
    tracks: Vec<Appearance>,
}

struct Appearance {
    id: u64,
    x: i32,
    y: i32,
    radius: i32,
    color: Option<[u8; 3]>,
    last_seen: u64,
    missed: u32,
    confirmations: u32,
    consumed: bool,
    attempts: u8,
}

// Identify hue, not brightness: hover/press lighting is not another popup.
fn same_color_family(a: [u8; 3], b: [u8; 3]) -> bool {
    fn hue(rgb: [u8; 3]) -> Option<f32> {
        let [r,g,b] = rgb.map(|v| v as f32);
        let max = r.max(g).max(b); let min = r.min(g).min(b);
        let delta = max - min;
        if delta < 40.0 { return None; }
        let h = if max == r { ((g-b)/delta).rem_euclid(6.0) }
            else if max == g { (b-r)/delta + 2.0 } else { (r-g)/delta + 4.0 };
        Some(h * 60.0)
    }
    match (hue(a), hue(b)) {
        (Some(a), Some(b)) => { let d = (a-b).abs(); d.min(360.0-d) < 30.0 }
        _ => true,
    }
}

impl AppearanceTracker {
    pub fn begin_frame(&mut self) { self.frame += 1; }

    pub fn observe(&mut self, x: i32, y: i32, radius: i32) -> bool {
        self.observe_color(x, y, radius, None)
    }

    pub fn observe_color(&mut self, x: i32, y: i32, radius: i32, color: Option<[u8; 3]>) -> bool {
        let frame = self.frame;
        let matching = self.tracks.iter_mut().filter(|t| {
            let dx = i64::from(t.x) - i64::from(x);
            let dy = i64::from(t.y) - i64::from(y);
            let same_color = match (t.color, color) {
                (Some(a), Some(b)) => same_color_family(a, b),
                _ => true,
            };
            same_color && dx * dx + dy * dy <= i64::from(t.radius.max(radius)).pow(2)
        }).min_by_key(|t| (i64::from(t.x)-i64::from(x)).pow(2) + (i64::from(t.y)-i64::from(y)).pow(2));
        if let Some(t) = matching {
            if t.last_seen != frame {
                t.confirmations = if t.last_seen + 1 == frame { t.confirmations + 1 } else { 1 };
            }
            t.last_seen = frame;
            t.missed = 0;
            t.x = x; t.y = y;
            !t.consumed && t.confirmations >= 2
        } else {
            static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
            let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.tracks.push(Appearance { id, x, y, radius, color, last_seen: frame,
                missed: 0, confirmations: 1, consumed: frame == 1, attempts: 0 });
            false
        }
    }

    pub fn finish_frame(&mut self, image: &ScreenCapture) {
        for t in &mut self.tracks {
            if t.last_seen == self.frame { continue; }
            // Missing pixels under PiP/overlays are unknown, not proof of disappearance.
            let visible = [(0,0), (-t.radius,0), (t.radius,0), (0,-t.radius), (0,t.radius)].iter().all(|&(dx,dy)| {
                let (x,y) = (t.x + dx, t.y + dy);
                x >= 0 && y >= 0 && x < image.width && y < image.height
                    && image.buffer[((y * image.width + x) * 4 + 3) as usize] != 0
            });
            if visible { t.missed += 1; }
        }
        // Two visible negative observations distinguish disappearance from a one-frame dropout.
        self.tracks.retain(|t| t.missed < 2);
    }

    pub fn consume(&mut self, x: i32, y: i32) -> Vec<u64> {
        let frame = self.frame;
        let nearest = self.tracks.iter_mut().filter(|t| t.last_seen == frame && !t.consumed
            && (t.x - x).abs() <= t.radius && (t.y - y).abs() <= t.radius)
            .min_by_key(|t| (i64::from(t.x)-i64::from(x)).pow(2)+(i64::from(t.y)-i64::from(y)).pow(2));
        if let Some(t) = nearest {
            t.consumed = true;
            t.attempts += 1;
            vec![t.id]
        } else { vec![] }
    }

    pub fn retry_failed(&mut self, ids: &[u64]) {
        for t in &mut self.tracks {
            if ids.contains(&t.id) && t.attempts < 2 { t.consumed = false; }
        }
    }
}

pub struct InputAttempt {
    color_ids: Vec<u64>,
    template_ids: Vec<u64>,
}

pub struct PopupDetector {
    appearances: AppearanceTracker,
    pub template_appearances: AppearanceTracker,
    prev_grid: Option<Vec<(u8, u8, u8)>>,
    grid_w: usize,
    grid_h: usize,
    cell_size: usize,
    pub embedded_roblox: Option<EmbeddedRobloxTemplate>,
    pub recent_targets: Vec<(i32, i32, std::time::Instant)>,
}

impl PopupDetector {
    pub fn new() -> Self {
        Self {
            appearances: AppearanceTracker::default(),
            template_appearances: AppearanceTracker::default(),
            prev_grid: None,
            grid_w: 0,
            grid_h: 0,
            cell_size: 4, // 4x4 px grid for high precision (<0.3ms scan speed)
            embedded_roblox: EmbeddedRobloxTemplate::load(),
            recent_targets: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.prev_grid = None;
    }

    #[cfg(test)]
    pub fn record_click(&mut self, x: i32, y: i32) {
        let _ = self.begin_attempt(x, y);
    }

    pub fn begin_attempt(&mut self, x: i32, y: i32) -> InputAttempt {
        let attempt = InputAttempt { color_ids: self.appearances.consume(x, y),
            template_ids: self.template_appearances.consume(x, y) };
        let now = std::time::Instant::now();
        self.recent_targets.push((x, y, now));
        self.recent_targets.retain(|&(_, _, t)| now.duration_since(t).as_secs_f32() < 5.0);
        attempt
    }

    pub fn input_failed(&mut self, attempt: &InputAttempt) {
        // A transient cancelled/rejected injection must not permanently consume the popup.
        self.appearances.retry_failed(&attempt.color_ids);
        self.template_appearances.retry_failed(&attempt.template_ids);
    }

    pub fn is_in_cooldown(&self, x: i32, y: i32, radius: i32, cooldown_secs: f32) -> bool {
        let now = std::time::Instant::now();
        for &(cx, cy, t) in &self.recent_targets {
            if now.duration_since(t).as_secs_f32() < cooldown_secs {
                let dx = cx - x;
                let dy = cy - y;
                if dx * dx + dy * dy <= radius * radius {
                    return true;
                }
            }
        }
        false
    }

    /// Scans the screen, detects target buttons / popups, and returns center (X, Y).
    pub fn detect_popup(
        &mut self,
        screen: &ScreenCapture,
        config: &VisionConfig,
    ) -> Option<(i32, i32)> {
        if screen.width <= 0 || screen.height <= 0 || screen.buffer.len() != screen.width as usize * screen.height as usize * 4 { return None; }
        self.appearances.begin_frame();
        let sw = screen.width as usize;
        let sh = screen.height as usize;
        let cell_size = self.cell_size;

        let (min_x, min_y, max_x, max_y) = if config.use_custom_region {
            let x1 = config.region_x1.min(config.region_x2).clamp(0, sw as i32) as usize;
            let y1 = config.region_y1.min(config.region_y2).clamp(0, sh as i32) as usize;
            let x2 = config.region_x1.max(config.region_x2).clamp(0, sw as i32) as usize;
            let y2 = config.region_y1.max(config.region_y2).clamp(0, sh as i32) as usize;
            (x1, y1, x2, y2)
        } else {
            // Safe playfield bounds: strictly excludes left static HUD (Shop, Rebirth, Index),
            // top bar (Y < 130: currency, timers, stats), right leaderboard (X > sw - 150), bottom bar (Y > sh - 110).
            if config.exclude_hud && sw >= 600 && sh >= 400 {
                let safe_min_x = ((sw as f32) * 0.20).max(280.0) as usize;
                let safe_max_x = ((sw as f32) * 0.84).min(sw as f32 - 150.0) as usize;
                let safe_min_y = ((sh as f32) * 0.13).max(130.0) as usize;
                let safe_max_y = ((sh as f32) * 0.86).min(sh as f32 - 110.0) as usize;
                if safe_min_x < safe_max_x && safe_min_y < safe_max_y {
                    (safe_min_x, safe_min_y, safe_max_x, safe_max_y)
                } else {
                    (0, 0, sw, sh)
                }
            } else {
                (0, 0, sw, sh)
            }
        };

        if min_x >= max_x || min_y >= max_y {
            return None;
        }

        // =========================================================================
        // PASS 1: High-Precision Template Match for Roblox Multiplier Button (Purple)
        // =========================================================================
        let mut embedded_target = None;
        if config.roblox_profile && config.popup_filter == PopupColorFilter::PurpleViolet {
            if let Some(ref tmpl) = self.embedded_roblox {
                let tw = tmpl.width as usize;
                let th = tmpl.height as usize;
                let region_w = max_x.saturating_sub(min_x);
                let region_h = max_y.saturating_sub(min_y);
                if region_w >= tw && region_h >= th && !tmpl.sample_points.is_empty() {
                    let screen_buf = &screen.buffer;
                    let num_samples = tmpl.sample_points.len();
                    let max_allowed_coarse = (num_samples * 3 * 255) as f32 * 0.18; // Strict 82% match tolerance
                    let max_possible = (num_samples * 3 * 255) as f32;

                    let stride = 4;
                    let start_y = min_y;
                    let end_y = max_y - th;
                    let start_x = min_x;
                    let end_x = max_x - tw;

                    let mut best_diff = max_allowed_coarse as u32;
                    let mut best_target = None;

                    let mut y = start_y;
                    while y <= end_y {
                        let mut x = start_x;
                        while x <= end_x {
                            let mut coarse_diff = 0u32;
                            let mut reject = false;

                            for &(dx, dy, tr, tg, tb) in &tmpl.sample_points {
                                let sx = x + dx as usize;
                                let sy = y + dy as usize;
                                let s_idx = (sy * sw + sx) * 4;

                                if s_idx + 2 < screen_buf.len() {
                                    let sb = screen_buf[s_idx];
                                    let sg = screen_buf[s_idx + 1];
                                    let sr = screen_buf[s_idx + 2];

                                    let diff = (sr as i32 - tr as i32).abs() as u32
                                        + (sg as i32 - tg as i32).abs() as u32
                                        + (sb as i32 - tb as i32).abs() as u32;

                                    coarse_diff += diff;
                                    if coarse_diff > best_diff {
                                        reject = true;
                                        break;
                                    }
                                }
                            }

                            if !reject && coarse_diff < best_diff {
                                best_diff = coarse_diff;
                                best_target = Some((
                                    (x + tw / 2) as i32,
                                    (y + th / 2) as i32,
                                ));
                            }

                            x += stride;
                        }
                        y += stride;
                    }

                    if let Some(target) = best_target {
                        let conf = 1.0 - (best_diff as f32 / max_possible);
                        if conf >= 0.82
                            && (!config.only_new_targets || self.appearances.observe(target.0, target.1, 24))
                            && !self.is_in_cooldown(target.0, target.1, 45, 1.8) {
                            embedded_target = Some(target);
                        }
                    }
                }
            }
        }

        // =========================================================================
        // PASS 2: Universal Smart Circle & Icon Badge Detector
        // =========================================================================
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

        let is_motion_diff = config.popup_filter == PopupColorFilter::MotionDiff;

        let prev_opt = if is_motion_diff {
            match &self.prev_grid {
                Some(p) if p.len() == curr_grid.len() => Some(p),
                _ => {
                    self.prev_grid = Some(curr_grid);
                    self.grid_w = grid_w;
                    self.grid_h = grid_h;
                    return None;
                }
            }
        } else {
            None
        };

        let mut color_mask = vec![false; grid_w * grid_h];
        let mut white_mask = vec![false; grid_w * grid_h];
        let sens = config.popup_sensitivity;

        for i in 0..curr_grid.len() {
            let (cr, cg, cb) = curr_grid[i];

            // White icon mask (interior hand pointer, multiplier text)
            if (cr >= 195 && cg >= 195 && cb >= 195
                && (cr as i32 - cg as i32).abs() < 30
                && (cr as i32 - cb as i32).abs() < 30) || (cr > 175 && cg > 150 && cb < 160)
            {
                white_mask[i] = true;
            }

            let matches = if let Some(prev) = prev_opt {
                let (pr, pg, pb) = prev[i];
                let delta = (cr as i32 - pr as i32).abs() as u32
                    + (cg as i32 - pg as i32).abs() as u32
                    + (cb as i32 - pb as i32).abs() as u32;
                delta >= sens
            } else {
                match config.popup_filter {
                    PopupColorFilter::UniversalCircle => {
                        // 🎯 Matches ONLY true multiplier popups: Purple/Magenta x2, Sky Blue/Cyan x4, Orange x3
                        let is_purple = cr >= 95 && cb >= 120 && cg <= 105
                            && (cb as i32 - cg as i32) >= 20
                            && (cr as i32 - cg as i32) >= 15;
                        let is_orange = cr >= 190 && cg >= 80 && cg <= 165 && cb <= 70 && (cr as i32 - cg as i32) >= 25;
                        let is_cyan_or_blue = cb >= 130 && ((cb as i32 - cr as i32) >= 20 || (cg >= 105 && cr <= 105));

                        is_purple || is_cyan_or_blue || is_orange
                    }
                    PopupColorFilter::PurpleViolet => {
                        // 🟣 Violet / Purple circle button (Roblox x2 / x3 / multiplier / tap)
                        cr >= 90 && cb >= 125 && cg <= 95
                            && (cb as i32 - cg as i32) >= 25
                            && (cr as i32 - cg as i32) >= 15
                    }
                    PopupColorFilter::WhitePointerInside => {
                        // 👆 White hand cursor / pointer or white text badge inside a button
                        cr > 200
                            && cg > 200
                            && cb > 200
                            && (cr as i32 - cg as i32).abs() < 25
                            && (cr as i32 - cb as i32).abs() < 25
                    }
                    PopupColorFilter::AnyVibrant => {
                        // 🌈 Any vibrant colored button / circle (high color saturation)
                        let max_c = cr.max(cg).max(cb);
                        let min_c = cr.min(cg).min(cb);
                        max_c > 85 && (max_c - min_c) > 60
                    }
                    PopupColorFilter::GoldYellow => {
                        // 🟡 Gold / Yellow coins or stars
                        cr > 140 && cg > 110 && cb < 120 && (cr as i32 - cb as i32) >= 35 && (cg as i32 - cb as i32) >= 25
                    }
                    PopupColorFilter::Green => {
                        // 🟢 Green buttons
                        cg > 120 && (cg as i32 - cr as i32) >= 25 && (cg as i32 - cb as i32) >= 25
                    }
                    PopupColorFilter::Red => {
                        // 🔴 Red buttons
                        cr > 130 && (cr as i32 - cg as i32) >= 30 && (cr as i32 - cb as i32) >= 30
                    }
                    PopupColorFilter::CyanBlue => {
                        // 🔵 Cyan / Blue buttons
                        cb > 130 && (cg > 95 || (cb as i32 - cr as i32) >= 25) && (cb as i32 - cr as i32) > 15
                    }
                    PopupColorFilter::MotionDiff => false,
                }
            };

            if matches {
                color_mask[i] = true;
            }
        }

        // BFS Connected Components to identify target button bounding box
        let mut visited = vec![false; grid_w * grid_h];
        let mut component_labels = vec![0usize; grid_w * grid_h];
        let mut best_target: Option<(i32, i32)> = None;
        let mut best_score: i32 = 0;

        let min_pixel_size = config.popup_min_size.max(10);
        let max_pixel_size = config.popup_max_size.max(min_pixel_size);

        let min_w_cells = (min_pixel_size as usize / cell_size).max(2);
        let min_cells_count = (min_w_cells * min_w_cells) / 4;

        for gy in 0..grid_h {
            for gx in 0..grid_w {
                let idx = gy * grid_w + gx;
                if color_mask[idx] && !visited[idx] {
                    let mut queue = std::collections::VecDeque::new();
                    queue.push_back((gx, gy));
                    visited[idx] = true;

                    let mut blob_min_x = gx;
                    let mut blob_max_x = gx;
                    let mut blob_min_y = gy;
                    let mut blob_max_y = gy;
                    let mut cell_count = 0;
                    let mut color_sum = [0u64; 3];

                    while let Some((cx, cy)) = queue.pop_front() {
                        cell_count += 1;
                        component_labels[cy * grid_w + cx] = idx + 1;
                        let (r,g,b) = curr_grid[cy * grid_w + cx];
                        color_sum[0] += r as u64; color_sum[1] += g as u64; color_sum[2] += b as u64;
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
                                if color_mask[n_idx] && !visited[n_idx] {
                                    visited[n_idx] = true;
                                    queue.push_back((nx, ny));
                                }
                            }
                        }
                    }

                    let blob_w = blob_max_x - blob_min_x + 1;
                    let blob_h = blob_max_y - blob_min_y + 1;
                    let blob_pixel_w = (blob_w * cell_size) as i32;
                    let blob_pixel_h = (blob_h * cell_size) as i32;

                    // Aspect ratio filter: round buttons / badges have aspect ratio near 1.0 (0.60..1.65)
                    let aspect = blob_w as f32 / blob_h as f32;
                    let is_button_aspect = if config.round_only || config.roblox_profile { aspect >= 0.60 && aspect <= 1.65 } else { aspect >= 0.15 && aspect <= 6.0 };

                    // Density filter: solid shapes have density >= 0.15
                    let density = cell_count as f32 / (blob_w * blob_h) as f32;

                    // Count white icon cells inside the bounding box and inside the core
                    let cy = (blob_min_y + blob_max_y) / 2;
                    let cx = (blob_min_x + blob_max_x) / 2;
                    let r_core = (blob_w.min(blob_h) / 4).max(1);

                    let mut white_cells = 0;
                    let mut core_white = 0;
                    for wy in blob_min_y..=blob_max_y {
                        for wx in blob_min_x..=blob_max_x {
                            if white_mask[wy * grid_w + wx] {
                                white_cells += 1;
                                let dy = wy as i32 - cy as i32;
                                let dx = wx as i32 - cx as i32;
                                if (dx * dx + dy * dy) <= ((r_core + 1) * (r_core + 1)) as i32 {
                                    core_white += 1;
                                }
                            }
                        }
                    }

                    // Multiplier buttons (x2, x3, x4) or interactive circles:
                    // MUST have interior white pointer/text (white_cells >= 5, core_white >= 4) and round circular aspect (0.76..1.32) and density >= 0.35.
                    let is_valid_shape = if !config.roblox_profile {
                        density >= (if config.round_only { 0.45 } else { 0.25 })
                            && (!config.require_white_icon || white_cells >= 3)
                    } else if config.popup_filter == PopupColorFilter::WhitePointerInside {
                        white_cells >= 3 && core_white >= 2 && density >= 0.25
                    } else if config.popup_filter == PopupColorFilter::UniversalCircle || config.popup_filter == PopupColorFilter::PurpleViolet {
                        white_cells >= 5 && core_white >= 4 && aspect >= 0.76 && aspect <= 1.32 && density >= 0.35
                    } else if white_cells >= 4 {
                        core_white >= 3 && aspect >= 0.65 && aspect <= 1.55 && density >= 0.30
                    } else {
                        // Strict filter for solid bubbles to prevent false positives on scenery/pets
                        aspect >= 0.80 && aspect <= 1.25 && density >= 0.50 && cell_count >= 16
                    };

                    let circle_evidence = if config.strict_circles && config.round_only {
                        let mut ring_hits = 0;
                        for n in 0..16 {
                            let angle = n as f32 * std::f32::consts::TAU / 16.0;
                            let sx = blob_min_x + ((blob_w - 1) as f32 * (0.5 + 0.35 * angle.cos())).round() as usize;
                            let sy = blob_min_y + ((blob_h - 1) as f32 * (0.5 + 0.35 * angle.sin())).round() as usize;
                            if component_labels[sy * grid_w + sx] == idx + 1 { ring_hits += 1; }
                        }
                        let mut clear_corners = 0;
                        for fx in [0.08, 0.92] { for fy in [0.08, 0.92] {
                            let sx = blob_min_x + ((blob_w - 1) as f32 * fx).round() as usize;
                            let sy = blob_min_y + ((blob_h - 1) as f32 * fy).round() as usize;
                            if component_labels[sy * grid_w + sx] != idx + 1 { clear_corners += 1; }
                        } }
                        let center_x = min_x as f32 + (blob_min_x as f32 + blob_w as f32/2.0)*cell_size as f32;
                        let center_y = min_y as f32 + (blob_min_y as f32 + blob_h as f32/2.0)*cell_size as f32;
                        let mut dark_rim = 0;
                        for n in 0..32 {
                            let a=n as f32*std::f32::consts::TAU/32.0;
                            let found=(1..=10).any(|offset| {
                                let x=(center_x+(blob_pixel_w as f32/2.0+offset as f32)*a.cos()).round() as i32;
                                let y=(center_y+(blob_pixel_h as f32/2.0+offset as f32)*a.sin()).round() as i32;
                                if x<0 || y<0 || x>=sw as i32 || y>=sh as i32 { return false; }
                                let i=((y as usize*sw+x as usize)*4) as usize;
                                screen_buf[i]<85 && screen_buf[i+1]<85 && screen_buf[i+2]<85 && screen_buf[i+3]!=0
                            });
                            if found {dark_rim+=1;}
                        }
                        let body=color_sum.map(|v| (v/cell_count as u64) as u8);
                        let mut contrasting_icon=0;
                        for yy in cy.saturating_sub(r_core)..=(cy+r_core).min(grid_h-1) {
                            for xx in cx.saturating_sub(r_core)..=(cx+r_core).min(grid_w-1) {
                                let (r,g,b)=curr_grid[yy*grid_w+xx];
                                let p=[r,g,b];
                                let contrast=p.iter().zip(body).map(|(a,b)| (*a as i32-b as i32).abs()).sum::<i32>();
                                if (r>195 && g>195 && b>195 && contrast>180)
                                    || (r>175 && g>150 && b<160 && !same_color_family(body,p)) {contrasting_icon+=1;}
                            }
                        }
                        aspect >= 0.80 && aspect <= 1.25 && density >= 0.35 && density <= 0.94
                            && ring_hits >= 10 && clear_corners >= 2 && core_white >= 2 && contrasting_icon >= 2 && dark_rim >= 24
                    } else { true };

                    if circle_evidence && cell_count >= min_cells_count
                        && blob_pixel_w >= if config.roblox_profile { min_pixel_size.max(45) } else { min_pixel_size }
                        && blob_pixel_w <= max_pixel_size
                        && blob_pixel_h >= if config.roblox_profile { min_pixel_size.max(45) } else { min_pixel_size }
                        && blob_pixel_h <= max_pixel_size
                        && is_button_aspect
                        && is_valid_shape
                    {
                        let center_gx = (blob_min_x + blob_max_x) / 2;
                        let center_gy = (blob_min_y + blob_max_y) / 2;
                        let real_x = (min_x + center_gx * cell_size + cell_size / 2) as i32;
                        let real_y = (min_y + center_gy * cell_size + cell_size / 2) as i32;

                        // High-confidence pixel verification of button core:
                        // Real Roblox multiplier circles ("x2", "x4", hand pointer) have bright white center lettering/icon.
                        // Scenery, benches, pets, and terrain have NO bright white center.
                        let mut core_white_px = 0;
                        let core_radius = 12; // 24x24 core
                        let screen_buf = &screen.buffer;
                        let y_min_core = (real_y - core_radius).max(0) as usize;
                        let y_max_core = (real_y + core_radius).min(sh as i32 - 1) as usize;
                        let x_min_core = (real_x - core_radius).max(0) as usize;
                        let x_max_core = (real_x + core_radius).min(sw as i32 - 1) as usize;

                        for py in y_min_core..=y_max_core {
                            let row_off = py * sw * 4;
                            for px in x_min_core..=x_max_core {
                                let idx = row_off + px * 4;
                                if idx + 2 < screen_buf.len() {
                                    let b = screen_buf[idx] as i32;
                                    let g = screen_buf[idx + 1] as i32;
                                    let r = screen_buf[idx + 2] as i32;
                                    if r >= 180 && g >= 180 && b >= 180 && (r - g).abs() < 40 && (r - b).abs() < 40 {
                                        core_white_px += 1;
                                    }
                                }
                            }
                        }

                        let newly_appeared = !config.only_new_targets || self.appearances.observe_color(real_x, real_y,
                            (blob_pixel_w.min(blob_pixel_h) / 3).clamp(8, 40),
                            Some(color_sum.map(|sum| (sum / cell_count as u64) as u8)));
                        if newly_appeared && (!config.roblox_profile || core_white_px >= 45) && (config.only_new_targets || !self.is_in_cooldown(real_x, real_y, (blob_pixel_w.min(blob_pixel_h) / 2).max(8), (config.cooldown_ms / 1000.0) as f32)) {
                            let score = (white_cells as i32 * 15) + (core_white_px as i32) + cell_count as i32;
                            if score > best_score {
                                best_score = score;
                                best_target = Some((real_x, real_y));
                            }
                        }
                    }
                }
            }
        }

        if is_motion_diff {
            self.prev_grid = Some(curr_grid);
        }

        self.appearances.finish_frame(screen);
        embedded_target.or(best_target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_roblox_button() {
        let img_bytes = include_bytes!("../../assets/roblox_button.png");
        let img = image::load_from_memory(img_bytes).expect("Failed to load roblox_button.png");
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for p in rgba.pixels() {
            bgra.push(p[2]); // B
            bgra.push(p[1]); // G
            bgra.push(p[0]); // R
            bgra.push(p[3]); // A
        }

        let cap = ScreenCapture {
            width: w as i32,
            height: h as i32,
            buffer: bgra,
        };

        let mut detector = PopupDetector::new();
        let config = VisionConfig {
            only_new_targets: false,
            roblox_profile: true,
            exclude_hud: true,
            popup_filter: PopupColorFilter::PurpleViolet,
            popup_min_size: 20,
            popup_max_size: 300,
            ..Default::default()
        };

        let target = detector.detect_popup(&cap, &config);
        println!("detect_popup result: {:?}", target);
        assert!(target.is_some(), "Roblox button was NOT detected!");
        let (tx, ty) = target.unwrap();
        println!("Detected Roblox button center at: ({}, {})", tx, ty);
    }

    #[test]
    fn test_detect_roblox_button_on_full_screen() {
        let img_bytes = include_bytes!("../../assets/roblox_button.png");
        let img = image::load_from_memory(img_bytes).expect("Failed to load roblox_button.png");
        let rgba = img.to_rgba8();
        let (bw, bh) = rgba.dimensions();

        // Create 1920x1080 screen with noise/background
        let sw = 1920;
        let sh = 1080;
        let mut screen_bgra = vec![30u8; sw * sh * 4]; // Dark background

        // Paste button at (700, 350)
        let paste_x = 700;
        let paste_y = 350;
        for py in 0..bh {
            for px in 0..bw {
                let p = rgba.get_pixel(px, py);
                if p[3] > 64 {
                    let s_idx = ((paste_y + py as usize) * sw + (paste_x + px as usize)) * 4;
                    screen_bgra[s_idx] = p[2];     // B
                    screen_bgra[s_idx + 1] = p[1]; // G
                    screen_bgra[s_idx + 2] = p[0]; // R
                    screen_bgra[s_idx + 3] = p[3]; // A
                }
            }
        }

        let cap = ScreenCapture {
            width: sw as i32,
            height: sh as i32,
            buffer: screen_bgra,
        };

        let mut detector = PopupDetector::new();
        let config = VisionConfig {
            only_new_targets: false,
            roblox_profile: true,
            exclude_hud: true,
            popup_filter: PopupColorFilter::UniversalCircle,
            popup_min_size: 20,
            popup_max_size: 300,
            ..Default::default()
        };

        let target = detector.detect_popup(&cap, &config);
        println!("Full-screen detect_popup result: {:?}", target);
        assert!(target.is_some(), "Roblox button was NOT detected on full screen!");
        let (tx, ty) = target.unwrap();
        let expected_x = paste_x as i32 + bw as i32 / 2;
        let expected_y = paste_y as i32 + bh as i32 / 2;
        println!("Detected at ({}, {}), expected near ({}, {})", tx, ty, expected_x, expected_y);
        assert!((tx - expected_x).abs() <= 15, "X coordinate offset too large");
        assert!((ty - expected_y).abs() <= 15, "Y coordinate offset too large");
    }

    #[test]
    fn test_hud_exclusion_ignores_rebirth_area() {
        let img_bytes = include_bytes!("../../assets/roblox_button.png");
        let img = image::load_from_memory(img_bytes).expect("Failed to load roblox_button.png");
        let rgba = img.to_rgba8();
        let (bw, bh) = rgba.dimensions();

        let sw = 1920;
        let sh = 1080;
        let mut screen_bgra = vec![30u8; sw * sh * 4];

        // Paste button inside HUD area: x = 100, y = 550 (where «Перерождение» is located)
        let paste_x = 100;
        let paste_y = 550;
        for py in 0..bh {
            for px in 0..bw {
                let p = rgba.get_pixel(px, py);
                if p[3] > 64 {
                    let s_idx = ((paste_y + py as usize) * sw + (paste_x + px as usize)) * 4;
                    screen_bgra[s_idx] = p[2];     // B
                    screen_bgra[s_idx + 1] = p[1]; // G
                    screen_bgra[s_idx + 2] = p[0]; // R
                    screen_bgra[s_idx + 3] = p[3]; // A
                }
            }
        }

        let cap = ScreenCapture {
            width: sw as i32,
            height: sh as i32,
            buffer: screen_bgra,
        };

        let mut detector = PopupDetector::new();
        let config = VisionConfig {
            only_new_targets: false,
            roblox_profile: true,
            exclude_hud: true,
            popup_filter: PopupColorFilter::UniversalCircle,
            use_custom_region: false,
            ..Default::default()
        };

        let target = detector.detect_popup(&cap, &config);
        assert!(target.is_none(), "HUD element in Rebirth area (x < 280) MUST NOT be detected!");
    }

    #[test]
    fn test_detect_circle_on_live_game_screenshot() {
        let path = r"C:\Users\kabachok\.gemini\antigravity\brain\11a4bce7-d340-4854-afd4-557019596be0\background_mode_verified.png";
        if let Ok(img) = image::open(path) {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let mut bgra = Vec::with_capacity((w * h * 4) as usize);
            for pixel in rgba.pixels() {
                bgra.push(pixel[2]); // B
                bgra.push(pixel[1]); // G
                bgra.push(pixel[0]); // R
                bgra.push(pixel[3]); // A
            }
            let cap = ScreenCapture {
                width: w as i32,
                height: h as i32,
                buffer: bgra,
            };

            let mut detector = PopupDetector::new();
            let config = VisionConfig {
            only_new_targets: false,
            roblox_profile: true,
            exclude_hud: true,
                popup_filter: PopupColorFilter::PurpleViolet,
                use_custom_region: false,
                ..Default::default()
            };

            let target = detector.detect_popup(&cap, &config);
            println!("Target detected with PurpleViolet filter: {:?}", target);
            assert!(target.is_some(), "Purple circle must be detected on live screen!");
            let (tx, ty) = target.unwrap();
            println!("Target coordinates: ({}, {})", tx, ty);
            assert!((tx - 1162).abs() < 50, "Target X should be around 1162, got {}", tx);
            assert!((ty - 320).abs() < 50, "Target Y should be around 320, got {}", ty);

            // Test UniversalCircle filter
            let mut detector2 = PopupDetector::new();
            let config2 = VisionConfig {
            only_new_targets: false,
            roblox_profile: true,
            exclude_hud: true,
                popup_filter: PopupColorFilter::UniversalCircle,
                use_custom_region: false,
                ..Default::default()
            };
            let target2 = detector2.detect_popup(&cap, &config2);
            println!("Target detected with UniversalCircle filter: {:?}", target2);
            assert!(target2.is_some(), "Circle must also be detected by UniversalCircle filter!");
            let (t2x, t2y) = target2.unwrap();
            assert!((t2x - 1162).abs() < 50, "UniversalCircle target X should be near 1162, got {}", t2x);
            assert!((t2y - 339).abs() < 50, "UniversalCircle target Y should be near 339, got {}", t2y);
        }

        // Test the new blue circle on roblox_after_bg_click.png
        let path_after = r"C:\Users\kabachok\.gemini\antigravity\brain\11a4bce7-d340-4854-afd4-557019596be0\roblox_after_bg_click.png";
        if let Ok(img) = image::open(path_after) {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let mut bgra = Vec::with_capacity((w * h * 4) as usize);
            for pixel in rgba.pixels() {
                bgra.push(pixel[2]);
                bgra.push(pixel[1]);
                bgra.push(pixel[0]);
                bgra.push(pixel[3]);
            }
            let cap = ScreenCapture {
                width: w as i32,
                height: h as i32,
                buffer: bgra,
            };

            let mut detector = PopupDetector::new();
            let config = VisionConfig {
            only_new_targets: false,
            roblox_profile: true,
            exclude_hud: true,
                popup_filter: PopupColorFilter::UniversalCircle,
                use_custom_region: false,
                ..Default::default()
            };

            let target = detector.detect_popup(&cap, &config);
            println!("Blue circle detected with UniversalCircle filter: {:?}", target);
            assert!(target.is_some(), "Blue x4 circle must be detected by UniversalCircle!");
            let (tx, ty) = target.unwrap();
            println!("Blue circle coordinates: ({}, {})", tx, ty);
            assert!((tx - 1222).abs() < 50, "Blue target X should be around 1222, got {}", tx);
            assert!((ty - 683).abs() < 50, "Blue target Y should be around 683, got {}", ty);
        }

        // Test that on live_now.png (where there is no active circle, only scenery and HUD), it returns None!
        let path_live = r"d:\коды\кликер\live_now.png";
        if let Ok(img) = image::open(path_live) {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let mut bgra = Vec::with_capacity((w * h * 4) as usize);
            for pixel in rgba.pixels() {
                bgra.push(pixel[2]);
                bgra.push(pixel[1]);
                bgra.push(pixel[0]);
                bgra.push(pixel[3]);
            }
            let cap = ScreenCapture {
                width: w as i32,
                height: h as i32,
                buffer: bgra,
            };

            let mut detector = PopupDetector::new();
            let config = VisionConfig {
            only_new_targets: false,
            roblox_profile: true,
            exclude_hud: true,
                popup_filter: PopupColorFilter::UniversalCircle,
                use_custom_region: false,
                ..Default::default()
            };

            let target = detector.detect_popup(&cap, &config);
            println!("live_now.png scan result (expecting None): {:?}", target);
            assert!(target.is_none(), "Must NOT detect any false targets on live_now screenshot!");
        }
    }

    #[test]
    fn test_template_matching_and_snipping() {
        // Create synthetic screen 1280x720 (dark gray background)
        let sw = 1280;
        let sh = 720;
        let mut screen_bgra = vec![30u8; sw * sh * 4];

        // Draw a distinct button at (400, 300) with size 50x50 (button body + white border + center emblem)
        let bx = 400;
        let by = 300;
        let bw = 50;
        let bh = 50;
        for y in by..(by + bh) {
            for x in bx..(bx + bw) {
                let idx = ((y * sw + x) * 4) as usize;
                let is_border = x == bx || x == bx + bw - 1 || y == by || y == by + bh - 1;
                let is_center = (x as i32 - (bx as i32 + 25)).abs() < 8 && (y as i32 - (by as i32 + 25)).abs() < 8;

                if is_border {
                    screen_bgra[idx] = 255;     // B
                    screen_bgra[idx + 1] = 255; // G
                    screen_bgra[idx + 2] = 255; // R
                } else if is_center {
                    screen_bgra[idx] = 20;      // B
                    screen_bgra[idx + 1] = 230; // G
                    screen_bgra[idx + 2] = 250; // R
                } else {
                    screen_bgra[idx] = 220;     // B
                    screen_bgra[idx + 1] = 120; // G
                    screen_bgra[idx + 2] = 40;  // R
                }
                screen_bgra[idx + 3] = 255; // A
            }
        }

        let screen = ScreenCapture {
            width: sw as i32,
            height: sh as i32,
            buffer: screen_bgra.clone(),
        };

        // Extract a snipped template of that button
        let mut tmpl_bgra = Vec::with_capacity(bw * bh * 4);
        for y in by..(by + bh) {
            for x in bx..(bx + bw) {
                let idx = ((y * sw + x) * 4) as usize;
                tmpl_bgra.push(screen_bgra[idx]);
                tmpl_bgra.push(screen_bgra[idx + 1]);
                tmpl_bgra.push(screen_bgra[idx + 2]);
                tmpl_bgra.push(screen_bgra[idx + 3]);
            }
        }

        let template = ButtonTemplate::new(1, "Тестовая Кнопка".to_string(), bw as u32, bh as u32, tmpl_bgra, 0.85);

        // 1. Full-screen search
        let match_res = VisionEngine::find_template(&screen, &template, None);
        assert!(match_res.is_some(), "Template matching must find the button on full screen!");
        let m = match_res.unwrap();
        assert_eq!(m.x, bx as i32);
        assert_eq!(m.y, by as i32);
        assert_eq!(m.center_x, (bx + bw / 2) as i32);
        assert_eq!(m.center_y, (by + bh / 2) as i32);
        assert!(m.confidence > 0.95);

        // 2. Search inside matching region
        let region_match = VisionEngine::find_template(&screen, &template, Some((350, 250, 500, 400)));
        assert!(region_match.is_some(), "Must find button within bounding region!");

        // 3. Search outside matching region
        let region_miss = VisionEngine::find_template(&screen, &template, Some((0, 0, 200, 200)));
        assert!(region_miss.is_none(), "Must NOT find button outside its region!");
    }
}
