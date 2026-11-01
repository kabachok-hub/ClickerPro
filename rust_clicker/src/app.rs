use eframe::egui::{self, Color32, RichText, Rounding, Stroke, Vec2};
use winapi::um::winuser::GetAsyncKeyState;

use crate::engine::{
    ActiveTab, AdvancedConfig, EngineState, KeyMode, KeyboardConfig, MouseConfig, RoutePoint,
    ScrollConfig, ScrollDirection,
};
use crate::hotkeys::{HotkeyAction, HotkeyEvent, HotkeyManager};
use crate::screen_capture::ScreenCapture;
use crate::vision::{ButtonTemplate, PopupColorFilter, PopupDetector, VisionConfig, VisionMode};
use crate::win_input::{ClickType, MouseButton, WinInput};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionSnipMode {
    None,
    CursorBox,
    TwoPointRect,
    RegionPoint,
}

pub struct ClickerApp {
    pub engine: EngineState,
    pub hotkeys: HotkeyManager,
    pub active_tab: ActiveTab,

    // Hotkey rebinding state
    pub capturing_hotkey: Option<HotkeyAction>,
    pub toggle_key_code: u16,
    pub toggle_key_name: String,
    pub pick_key_code: u16,
    pub pick_key_name: String,
    pub panic_key_code: u16,
    pub panic_key_name: String,

    // Advanced configuration
    pub advanced_cfg: AdvancedConfig,

    // Mouse tab state
    pub mouse_cfg: MouseConfig,

    // Keyboard tab state
    pub kb_cfg: KeyboardConfig,
    pub capturing_key: bool,

    // Scroll tab state
    pub scroll_cfg: ScrollConfig,

    // Points tab state
    pub points: Vec<RoutePoint>,
    pub new_pt_x: i32,
    pub new_pt_y: i32,
    pub new_pt_delay: f64,
    pub new_pt_hold: f64,
    pub new_pt_btn: MouseButton,

    // Vision tab state
    pub vision_cfg: VisionConfig,
    pub vision_snip_mode: VisionSnipMode,
    pub snip_box_w: i32,
    pub snip_box_h: i32,
    pub snip_p1: Option<(i32, i32)>,
    pub vision_status_msg: Option<String>,
    #[allow(dead_code)]
    pub file_import_path: String,
    #[allow(dead_code)]
    pub file_import_error: Option<String>,
    #[allow(dead_code)]
    pub template_textures: std::collections::HashMap<u64, egui::TextureHandle>,
    pub test_scan_result: Option<(bool, String)>,
    pending_test: Option<std::time::Instant>,
    ui_snapshot: Option<std::path::PathBuf>,
    snapshot_requested: bool,
    hunter_preview: Option<egui::TextureHandle>,

    // Embedded icon texture
    pub icon_texture: Option<egui::TextureHandle>,

    // Target window / background clicking state
    pub picking_window_target: bool,
    pub picking_window_point: bool,
    pub open_windows: Vec<crate::win_input::WindowInfo>,
    pub show_window_picker: bool,
    pub window_search_query: String,
    pub window_status_msg: Option<String>,

    // Live info
    pub cursor_x: i32,
    pub cursor_y: i32,
}

impl ClickerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (cx, cy) = WinInput::get_cursor_pos();

        // Load embedded minimalist icon into texture
        let icon_texture = if let Ok(img) = image::load_from_memory(include_bytes!("../../assets/icon.png")) {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let color_image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            Some(cc.egui_ctx.load_texture("app_logo", color_image, egui::TextureOptions::LINEAR))
        } else {
            None
        };

        let roblox_target = crate::engine::WindowTargetConfig::default();
        let roblox_status = None;

        let mut advanced_cfg = AdvancedConfig::default();
        advanced_cfg.window_target = roblox_target;

        Self {
            engine: EngineState::new(),
            hotkeys: HotkeyManager::new(cc.egui_ctx.clone()),
            active_tab: ActiveTab::Vision,
            test_scan_result: None,
            pending_test: None,
            ui_snapshot: std::env::args().find_map(|a| a.strip_prefix("--ui-snapshot=").map(std::path::PathBuf::from)),
            snapshot_requested: false,
            hunter_preview: None,

            capturing_hotkey: None,
            toggle_key_code: winapi::um::winuser::VK_F6 as u16,
            toggle_key_name: "F6".to_string(),
            pick_key_code: winapi::um::winuser::VK_F8 as u16,
            pick_key_name: "F8".to_string(),
            panic_key_code: winapi::um::winuser::VK_F12 as u16,
            panic_key_name: "F12".to_string(),

            advanced_cfg,

            mouse_cfg: MouseConfig::default(),

            kb_cfg: KeyboardConfig::default(),
            capturing_key: false,

            scroll_cfg: ScrollConfig::default(),

            points: vec![RoutePoint {
                x: cx,
                y: cy,
                button: MouseButton::Left,
                delay_after_ms: 100.0,
                hold_ms: 0.0,
            }],
            new_pt_x: cx,
            new_pt_y: cy,
            new_pt_delay: 150.0,
            new_pt_hold: 0.0,
            new_pt_btn: MouseButton::Left,

            vision_cfg: {
                let mut config = VisionConfig { only_new_targets: false, strict_circles: true, require_white_icon: true, exclude_hud: true, ..VisionConfig::default() };
                if let Some(root) = std::env::var_os("LOCALAPPDATA") {
                    let path = std::path::PathBuf::from(root).join("ClickerPro/last-template.png");
                    if let Ok(template) = ButtonTemplate::from_file(rand::random(), "Последний образец".into(), path, 0.90) {
                        config.templates.push(template);
                        config.mode = VisionMode::TemplateMatch;
                    }
                }
                config
            },
            vision_snip_mode: VisionSnipMode::None,
            snip_box_w: 60,
            snip_box_h: 60,
            snip_p1: None,
            vision_status_msg: None,
            file_import_path: String::new(),
            file_import_error: None,
            template_textures: std::collections::HashMap::new(),

            icon_texture,

            picking_window_target: false,
            picking_window_point: false,
            open_windows: Vec::new(),
            show_window_picker: false,
            window_search_query: String::new(),
            window_status_msg: roblox_status,

            cursor_x: cx,
            cursor_y: cy,
        }
    }

    #[allow(dead_code)]
    pub fn apply_preset(&mut self, preset_id: &str) {
        match preset_id {
            "minecraft_pvp" => {
                self.active_tab = ActiveTab::Mouse;
                self.mouse_cfg.button = MouseButton::Left;
                self.mouse_cfg.click_type = ClickType::Single;
                self.mouse_cfg.use_cps = true;
                self.mouse_cfg.cps = 16.0;
                self.mouse_cfg.jitter_ms = 3.0;
                self.mouse_cfg.fixed_pos = false;
                self.mouse_cfg.stop_after_count = 0;
            }
            "fast_100_cps" => {
                self.active_tab = ActiveTab::Mouse;
                self.mouse_cfg.button = MouseButton::Left;
                self.mouse_cfg.click_type = ClickType::Single;
                self.mouse_cfg.use_cps = true;
                self.mouse_cfg.cps = 100.0;
                self.mouse_cfg.jitter_ms = 0.0;
                self.mouse_cfg.fixed_pos = false;
                self.mouse_cfg.stop_after_count = 0;
            }
            "hold_attack" => {
                self.active_tab = ActiveTab::Mouse;
                self.mouse_cfg.button = MouseButton::Left;
                self.mouse_cfg.click_type = ClickType::Hold;
                self.mouse_cfg.hold_duration_ms = 350.0;
                self.mouse_cfg.use_cps = false;
                self.mouse_cfg.custom_interval_ms = 40.0;
                self.mouse_cfg.fixed_pos = false;
            }
            "space_spammer" => {
                self.active_tab = ActiveTab::Keyboard;
                self.kb_cfg.vk_code = 0x20;
                self.kb_cfg.key_name = "Space".to_string();
                self.kb_cfg.mode = KeyMode::Tap;
                self.kb_cfg.interval_ms = 25.0;
                self.kb_cfg.jitter_ms = 0.0;
            }
            "hold_e_key" => {
                self.active_tab = ActiveTab::Keyboard;
                self.kb_cfg.vk_code = 0x45;
                self.kb_cfg.key_name = "E".to_string();
                self.kb_cfg.mode = KeyMode::Hold;
                self.kb_cfg.hold_ms = 500.0;
                self.kb_cfg.interval_ms = 50.0;
            }
            "afk_scroll" => {
                self.active_tab = ActiveTab::Scroll;
                self.scroll_cfg.direction = ScrollDirection::Down;
                self.scroll_cfg.ticks = 2;
                self.scroll_cfg.interval_ms = 60.0;
            }
            "vision_fast" => {
                self.active_tab = ActiveTab::Vision;
                self.vision_cfg.mode = VisionMode::AutoPopup;
                self.vision_cfg.scan_interval_ms = 30.0;
                self.vision_cfg.cooldown_ms = 220.0;
                self.vision_cfg.return_cursor = true;
            }
            "roblox_circle" => {
                self.vision_cfg.roblox_profile = true;
                self.vision_cfg.exclude_hud = true;
                self.vision_cfg.round_only = true;
                self.vision_cfg.require_white_icon = true;
                self.active_tab = ActiveTab::Vision;
                self.vision_cfg.mode = VisionMode::AutoPopup;
                self.vision_cfg.popup_filter = PopupColorFilter::PurpleViolet;
                self.vision_cfg.popup_min_size = 25;
                self.vision_cfg.popup_max_size = 240;
                self.vision_cfg.popup_sensitivity = 40;
                self.vision_cfg.popup_hold_ms = 45.0;
                self.vision_cfg.scan_interval_ms = 30.0;
                self.vision_cfg.cooldown_ms = 220.0;
                self.vision_cfg.return_cursor = true;
                self.vision_status_msg = Some("✓ Пресет «Roblox: Фиолетовый кружок x2» активирован!".to_string());
            }
            _ => {}
        }
    }

    fn toggle_active_engine(&mut self) {
        match self.active_tab {
            ActiveTab::Mouse => {
                self.engine.toggle_mouse(self.mouse_cfg.clone(), self.advanced_cfg.clone());
            }
            ActiveTab::Keyboard => {
                self.engine.toggle_keyboard(self.kb_cfg.clone(), self.advanced_cfg.clone());
            }
            ActiveTab::Scroll => {
                self.engine.toggle_scroll(self.scroll_cfg.clone(), self.advanced_cfg.clone());
            }
            ActiveTab::Points => {
                self.engine.toggle_route(self.points.clone(), self.advanced_cfg.clone());
            }
            ActiveTab::Vision => {
                self.engine.toggle_vision(self.vision_cfg.clone(), self.advanced_cfg.clone());
            }
            ActiveTab::Settings => {
                self.engine.toggle_mouse(self.mouse_cfg.clone(), self.advanced_cfg.clone());
            }
        }
    }

    fn check_key_capture(&mut self) {
        if !self.capturing_key {
            return;
        }

        unsafe {
            for vk in 0x08..=0xFE {
                if vk == 0x01 || vk == 0x02 || vk == 0x04 || vk == 0x05 || vk == 0x06 {
                    continue;
                }
                if vk == self.toggle_key_code as i32
                    || vk == self.pick_key_code as i32
                    || vk == self.panic_key_code as i32
                {
                    continue;
                }

                if (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 {
                    let name = vk_to_human_name(vk as u16);
                    self.kb_cfg.vk_code = vk as u16;
                    self.kb_cfg.key_name = name;
                    self.capturing_key = false;
                    break;
                }
            }
        }
    }

    fn check_hotkey_capture(&mut self) {
        let action = match self.capturing_hotkey {
            Some(a) => a,
            None => return,
        };

        unsafe {
            for vk in 0x08..=0xFE {
                // Ignore mouse buttons
                if vk == 0x01 || vk == 0x02 || vk == 0x04 || vk == 0x05 || vk == 0x06 {
                    continue;
                }

                if (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 {
                    let name = vk_to_human_name(vk as u16);
                    match action {
                        HotkeyAction::ToggleStartStop => {
                            self.toggle_key_code = vk as u16;
                            self.toggle_key_name = name;
                            self.hotkeys.set_key(HotkeyAction::ToggleStartStop, vk as u16);
                        }
                        HotkeyAction::PickCoordinate => {
                            self.pick_key_code = vk as u16;
                            self.pick_key_name = name;
                            self.hotkeys.set_key(HotkeyAction::PickCoordinate, vk as u16);
                        }
                        HotkeyAction::EmergencyStop => {
                            self.panic_key_code = vk as u16;
                            self.panic_key_name = name;
                            self.hotkeys.set_key(HotkeyAction::EmergencyStop, vk as u16);
                        }
                    }
                    self.capturing_hotkey = None;
                    break;
                }
            }
        }
    }

    fn install_hunter_template(&mut self, template: ButtonTemplate) {
        let mut pixels = template.bgra.clone();
        for p in pixels.chunks_exact_mut(4) { p.swap(0,2); }
        if let Some(root) = std::env::var_os("LOCALAPPDATA") {
            let directory = std::path::PathBuf::from(root).join("ClickerPro");
            let _ = std::fs::create_dir_all(&directory);
            if let Some(image) = image::RgbaImage::from_raw(template.width, template.height, pixels) {
                let _ = image.save(directory.join("last-template.png"));
            }
        }
        self.vision_cfg.mode = VisionMode::TemplateMatch;
        self.vision_cfg.only_new_targets = false;
        self.vision_cfg.templates.clear();
        self.template_textures.clear();
        self.vision_status_msg = Some(format!("Образец {}×{} принят. Нажмите «Проверить без клика» или F6 в игре", template.width, template.height));
        self.vision_cfg.templates.push(template);
    }

    fn load_hunter_image(&mut self, path: &std::path::Path) {
        self.engine.stop();
        self.vision_cfg.mode = VisionMode::TemplateMatch;
        self.vision_cfg.templates.clear();
        self.template_textures.clear();
        match ButtonTemplate::from_file(rand::random(), "Картинка".into(), path, 0.90) {
            Ok(template) => self.install_hunter_template(template),
            Err(error) => self.vision_status_msg = Some(error),
        }
    }

    fn capture_game_sample(&mut self, x: i32, y: i32, w: i32, h: i32) -> Option<ScreenCapture> {
        let mut config = self.vision_cfg.clone();
        config.use_custom_region = true;
        config.region_x1 = x; config.region_y1 = y;
        config.region_x2 = x + w; config.region_y2 = y + h;
        match crate::hunter::capture(&config, &self.advanced_cfg) {
            Ok(frame) if frame.origin == (x, y) && frame.image.width == w && frame.image.height == h
                && frame.image.buffer.chunks_exact(4).all(|p| p[3] != 0) => Some(frame.image),
            _ => { self.vision_status_msg = Some("Образец должен целиком находиться в игре и не перекрываться другим окном".into()); None }
        }
    }

    fn handle_vision_snip_hotkey(&mut self, cx: i32, cy: i32) {
        if self.advanced_cfg.window_target.hwnd == 0 && self.vision_snip_mode != VisionSnipMode::None {
            if let Some(window) = WinInput::get_window_at_point(cx, cy) {
                if !["Картинка в картинке", "Picture in picture", "Picture-in-Picture"].contains(&window.title.as_str())
                    && !window.title.contains("Автокликер Pro") {
                    self.advanced_cfg.window_target.hwnd = window.hwnd;
                    self.advanced_cfg.window_target.title = window.title;
                    self.advanced_cfg.window_target.enabled = true;
                }
            }
        }
        let selected = self.advanced_cfg.window_target.hwnd;
        if self.vision_snip_mode != VisionSnipMode::None && (selected == 0
            || !WinInput::get_window_at_point(cx, cy).is_some_and(|w| w.hwnd == selected)) {
            self.vision_status_msg = Some("Выделяйте цель внутри выбранной игры, вне плавающего видео".into());
            return;
        }
        match self.vision_snip_mode {
            VisionSnipMode::CursorBox => {
                let half_w = self.snip_box_w / 2;
                let half_h = self.snip_box_h / 2;
                let x = cx - half_w;
                let y = cy - half_h;

                if let Some(cap) = self.capture_game_sample(x, y, self.snip_box_w, self.snip_box_h) {
                    let id = rand::random::<u64>();
                    let name = format!("Кнопка #{}", self.vision_cfg.templates.len() + 1);
                    let tmpl = ButtonTemplate::from_capture(id, name, &cap, 0.90);
                    self.install_hunter_template(tmpl);
                    self.vision_snip_mode = VisionSnipMode::None;
                    self.vision_status_msg = Some(format!(
                        "✓ Захвачена кнопка {}×{} px!",
                        self.snip_box_w, self.snip_box_h
                    ));
                }
            }
            VisionSnipMode::TwoPointRect => {
                if let Some((p1_x, p1_y)) = self.snip_p1 {
                    let x = p1_x.min(cx);
                    let y = p1_y.min(cy);
                    let w = (cx - p1_x).abs();
                    let h = (cy - p1_y).abs();

                    if w > 5 && h > 5 {
                        if let Some(cap) = self.capture_game_sample(x, y, w, h) {
                            let id = rand::random::<u64>();
                            let name = format!("Кнопка #{}", self.vision_cfg.templates.len() + 1);
                            let tmpl = ButtonTemplate::from_capture(id, name, &cap, 0.90);
                            self.install_hunter_template(tmpl);
                            self.vision_status_msg = Some(format!(
                                "✓ Кнопка {}×{} px успешно добавлена!",
                                w, h
                            ));
                        }
                    }
                    self.snip_p1 = None;
                    self.vision_snip_mode = VisionSnipMode::None;
                } else {
                    self.snip_p1 = Some((cx, cy));
                    self.vision_status_msg = Some(format!(
                        "Точка 1: ({}, {}). Наведите на противоположный угол кнопки и нажмите [{}]",
                        cx, cy, self.pick_key_name
                    ));
                }
            }
            VisionSnipMode::RegionPoint => {
                if let Some((p1_x, p1_y)) = self.snip_p1 {
                    self.vision_cfg.region_x1 = p1_x.min(cx);
                    self.vision_cfg.region_y1 = p1_y.min(cy);
                    self.vision_cfg.region_x2 = p1_x.max(cx);
                    self.vision_cfg.region_y2 = p1_y.max(cy);
                    self.vision_cfg.use_custom_region = true;
                    self.snip_p1 = None;
                    self.vision_snip_mode = VisionSnipMode::None;
                    self.vision_status_msg = Some(format!(
                        "✓ Область поиска: ({}, {}) - ({}, {})",
                        self.vision_cfg.region_x1,
                        self.vision_cfg.region_y1,
                        self.vision_cfg.region_x2,
                        self.vision_cfg.region_y2
                    ));
                } else {
                    self.snip_p1 = Some((cx, cy));
                    self.vision_status_msg = Some(format!(
                        "Угол 1: ({}, {}). Наведите на противоположный угол зоны поиска и нажмите [{}]",
                        cx, cy, self.pick_key_name
                    ));
                }
            }
            VisionSnipMode::None => {}
        }
    }
}

pub fn vk_to_human_name(vk: u16) -> String {
    match vk {
        0x20 => "Space".to_string(),
        0x0D => "Enter".to_string(),
        0x1B => "Esc".to_string(),
        0x09 => "Tab".to_string(),
        0x10 => "Shift".to_string(),
        0x11 => "Ctrl".to_string(),
        0x12 => "Alt".to_string(),
        0x14 => "CapsLock".to_string(),
        0x08 => "Backspace".to_string(),
        0x2D => "Insert".to_string(),
        0x2E => "Delete".to_string(),
        0x24 => "Home".to_string(),
        0x23 => "End".to_string(),
        0x21 => "PageUp".to_string(),
        0x22 => "PageDown".to_string(),
        0x25 => "Left".to_string(),
        0x26 => "Up".to_string(),
        0x27 => "Right".to_string(),
        0x28 => "Down".to_string(),
        0x30..=0x39 => format!("{}", (vk as u8 as char)),
        0x41..=0x5A => format!("{}", (vk as u8 as char)),
        0x70..=0x87 => format!("F{}", vk - 0x6F),
        0x60..=0x69 => format!("Num {}", vk - 0x60),
        0x6A => "Num *".to_string(),
        0x6B => "Num +".to_string(),
        0x6D => "Num -".to_string(),
        0x6E => "Num .".to_string(),
        0x6F => "Num /".to_string(),
        0xC0 => "` (Tilde)".to_string(),
        _ => format!("Key 0x{:02X}", vk),
    }
}

// GitHub Primer Design System: Segmented Control
fn primer_segmented_control<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    current: &mut T,
    options: &[(T, &str)],
) {
    egui::Frame::none()
        .fill(Color32::from_rgb(13, 17, 23)) // Inset background #0D1117
        .rounding(Rounding::same(6.0))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61))) // #30363D
        .inner_margin(egui::Margin::same(2.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
                for (val, label) in options {
                    let is_selected = *current == *val;
                    let text = RichText::new(*label)
                        .size(12.0)
                        .color(if is_selected {
                            Color32::from_rgb(240, 246, 252)
                        } else {
                            Color32::from_rgb(139, 148, 158)
                        });

                    let btn = egui::Button::new(text)
                        .min_size(Vec2::new(0.0, 28.0))
                        .rounding(Rounding::same(5.0))
                        .fill(if is_selected {
                            Color32::from_rgb(33, 38, 45) // Elevated card #21262D
                        } else {
                            Color32::TRANSPARENT
                        })
                        .stroke(if is_selected {
                            Stroke::new(1.0_f32, Color32::from_rgb(56, 139, 253)) // GitHub active border
                        } else {
                            Stroke::NONE
                        });

                    if ui.add(btn).clicked() {
                        *current = *val;
                    }
                }
            });
        });
}

// GitHub Primer Design System: Topic Badge for Presets
fn primer_topic_badge(ui: &mut egui::Ui, text: &str) -> bool {
    let btn_text = RichText::new(text)
        .size(11.5)
        .color(Color32::from_rgb(88, 166, 255)); // GitHub blue link

    let btn = egui::Button::new(btn_text)
        .min_size(Vec2::new(0.0, 24.0))
        .rounding(Rounding::same(12.0)) // Pill badge
        .fill(Color32::from_rgb(22, 27, 34))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61)));

    ui.add(btn).clicked()
}

// GitHub Primer Design System: Primer Box Container
fn primer_box<R>(
    ui: &mut egui::Ui,
    title: &str,
    description: Option<&str>,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) {
    egui::Frame::none()
        .fill(Color32::from_rgb(22, 27, 34)) // Canvas subtle #161B22
        .rounding(Rounding::same(6.0))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61))) // Border default #30363D
        .show(ui, |ui| {
            // Box Header
            egui::Frame::none()
                .fill(Color32::from_rgb(26, 31, 39))
                .rounding(Rounding {
                    nw: 6.0,
                    ne: 6.0,
                    sw: 0.0,
                    se: 0.0,
                })
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61)))
                .inner_margin(egui::Margin::symmetric(12.0, 5.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(title)
                                .size(12.5)
                                .strong()
                                .color(Color32::from_rgb(240, 246, 252)),
                        );
                        if let Some(desc) = description {
                            ui.label(
                                RichText::new(format!("— {}", desc))
                                    .size(11.0)
                                    .color(Color32::from_rgb(139, 148, 158)),
                            );
                        }
                    });
                });

            // Box Body
            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    add_contents(ui);
                });
        });
    ui.add_space(8.0);
}

impl eframe::App for ClickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dropped = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|file| file.path.clone()).collect::<Vec<_>>());
        for path in dropped { self.load_hunter_image(&path); }
        // Optional rendering check: export our own framebuffer without desktop capture or input.
        if let Some(path) = &self.ui_snapshot {
            if !self.snapshot_requested {
                self.snapshot_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
            let screenshot = ctx.input(|i| i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()), _ => None,
            }));
            if let Some(image) = screenshot {
                let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                if let Some(buffer) = image::RgbaImage::from_raw(image.width() as u32, image.height() as u32, bytes) {
                    let _ = buffer.save(path);
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.pending_test.is_some_and(|deadline| std::time::Instant::now() >= deadline) {
            self.pending_test = None;
            self.run_hunter_test(ctx);
        }
        if let Ok(mut status) = self.engine.hunter_status.lock() {
            if let Some((pixels, w, h)) = status.preview.take() {
                self.hunter_preview = Some(ctx.load_texture("hunter_preview", egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels), egui::TextureOptions::LINEAR));
            }
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(30));

        let (cx, cy) = WinInput::get_cursor_pos();
        self.cursor_x = cx;
        self.cursor_y = cy;

        while let Some(event) = self.hotkeys.poll_event() {
            match event {
                HotkeyEvent::ToggleStartStop => {
                    self.toggle_active_engine();
                }
                HotkeyEvent::PickCoordinate(cx, cy) => {
                    if self.picking_window_target {
                        if let Some(win) = WinInput::get_window_at_point(cx, cy) {
                            if ["Картинка в картинке", "Picture in picture", "Picture-in-Picture"].contains(&win.title.as_str()) { continue; }
                            self.engine.stop();
                            self.clear_hunter_preview();
                            self.vision_cfg.use_custom_region = false;
                            self.advanced_cfg.window_target.hwnd = win.hwnd;
                            self.advanced_cfg.window_target.title = win.title.clone();
                            self.advanced_cfg.window_target.enabled = true;
                            self.picking_window_target = false;
                            self.window_status_msg = Some(format!(
                                "✓ Привязано к окну: «{}» ({}×{})",
                                win.title, win.width, win.height
                            ));
                        }
                    } else if self.picking_window_point {
                        if self.advanced_cfg.window_target.hwnd != 0 {
                            if let Some((client_x, client_y)) =
                                WinInput::screen_to_client(self.advanced_cfg.window_target.hwnd, cx, cy)
                            {
                                self.advanced_cfg.window_target.custom_x = client_x;
                                self.advanced_cfg.window_target.custom_y = client_y;
                                self.advanced_cfg.window_target.click_center = false;
                                self.picking_window_point = false;
                                self.window_status_msg = Some(format!(
                                    "✓ Точка внутри окна зафиксирована: ({}, {})",
                                    client_x, client_y
                                ));
                            }
                        }
                    } else if self.active_tab == ActiveTab::Mouse {
                        self.mouse_cfg.target_x = cx;
                        self.mouse_cfg.target_y = cy;
                        self.mouse_cfg.fixed_pos = true;
                    } else if self.active_tab == ActiveTab::Points {
                        self.points.push(RoutePoint {
                            x: cx,
                            y: cy,
                            button: self.new_pt_btn,
                            delay_after_ms: self.new_pt_delay,
                            hold_ms: self.new_pt_hold,
                        });
                    } else if self.active_tab == ActiveTab::Vision {
                        self.handle_vision_snip_hotkey(cx, cy);
                    }
                }
                HotkeyEvent::EmergencyStop => {
                    self.engine.stop();
                }
            }
        }

        self.check_key_capture();
        self.check_hotkey_capture();

        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if self.advanced_cfg.always_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else { egui::WindowLevel::Normal }));
        let mut style = (*ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.override_text_color = Some(Color32::from_rgb(225, 231, 239));
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
        style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        style.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
        style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(23.0));
        ctx.set_style(style);
        egui::SidePanel::left("navigation").exact_width(160.0).resizable(false).show(ctx, |ui| {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if let Some(icon) = &self.icon_texture { ui.image((icon.id(), Vec2::splat(22.0))); }
                ui.label(RichText::new("Кликер Pro").size(17.0).strong());
            });
            ui.add_space(20.0);
            for (tab, label) in [(ActiveTab::Vision, "Охотник"), (ActiveTab::Mouse, "Мышь"),
                (ActiveTab::Keyboard, "Клавиатура"), (ActiveTab::Scroll, "Прокрутка"),
                (ActiveTab::Points, "Маршрут"), (ActiveTab::Settings, "Настройки")] {
                if ui.add_sized([140.0, 36.0], egui::Button::new(label).selected(self.active_tab == tab)).clicked() {
                    self.active_tab = tab;
                }
            }
        });
        egui::TopBottomPanel::bottom("controls").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let running = self.engine.is_running();
                let ready = self.active_tab != ActiveTab::Vision || (self.advanced_cfg.window_target.hwnd != 0
                    && (self.vision_cfg.mode != VisionMode::TemplateMatch || self.vision_cfg.templates.iter().any(|t| t.enabled)));
                let text = if running { format!("Остановить [{}]", self.toggle_key_name) }
                    else { format!("Запустить [{}]", self.toggle_key_name) };
                let button = egui::Button::new(RichText::new(text).size(16.0).strong())
                    .fill(if running { Color32::from_rgb(170, 47, 51) } else { Color32::from_rgb(35, 134, 65) });
                if ui.add_enabled_ui(running || ready, |ui| ui.add_sized([220.0, 42.0], button)).inner.clicked() {
                    self.toggle_active_engine();
                }
                ui.label(if running { "Включён" } else { "Остановлен" });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(format!("Стоп [{}]", self.panic_key_name)).clicked() { self.engine.stop(); }
                });
            });
            ui.add_space(8.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(10.0);
                match self.active_tab {
                    ActiveTab::Vision => self.render_vision_tab(ui),
                    ActiveTab::Mouse => self.render_mouse_tab(ui),
                    ActiveTab::Keyboard => self.render_keyboard_tab(ui),
                    ActiveTab::Scroll => self.render_scroll_tab(ui),
                    ActiveTab::Points => self.render_points_tab(ui),
                    ActiveTab::Settings => self.render_settings_tab(ui),
                }
            });
        });
    }
}


impl ClickerApp {
    fn render_window_target_card(&mut self, ui: &mut egui::Ui) {
        primer_box(
            ui,
            "🪟 Окно поиска — любая игра",
            None,
            |ui| {
                let hwnd = self.advanced_cfg.window_target.hwnd;
                let is_bound = hwnd != 0 && WinInput::is_window_valid(hwnd);

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.advanced_cfg.window_target.enabled,
                        RichText::new("Только выбранное окно").strong(),
                    );

                    ui.add_space(8.0);

                    if is_bound {
                        let title_display = if self.advanced_cfg.window_target.title.len() > 24 {
                            format!("{}…", self.advanced_cfg.window_target.title.chars().take(21).collect::<String>())
                        } else {
                            self.advanced_cfg.window_target.title.clone()
                        };
                        ui.label(
                            RichText::new(format!("🟢 {}", title_display))
                                .size(11.5)
                                .strong()
                                .color(Color32::from_rgb(88, 166, 255)),
                        );
                    } else {
                        ui.label(
                            RichText::new("⚠️ Окно не выбрано")
                                .size(11.5)
                                .color(Color32::from_rgb(227, 179, 65)),
                        );
                    }

                    ui.add_space(6.0);

                    if ui.small_button("⚡ Roblox").clicked() {
                        if let Some((hwnd, title)) = WinInput::find_roblox_window() {
                            self.advanced_cfg.window_target.hwnd = hwnd;
                            self.advanced_cfg.window_target.title = title.clone();
                            self.advanced_cfg.window_target.enabled = true;
                            self.window_status_msg = Some(format!("🟢 Привязано: {}", title));
                        } else {
                            self.window_status_msg = Some("⚠️ Окно Roblox не найдено!".to_string());
                        }
                    }

                    let pick_btn_text = if self.picking_window_target {
                        format!("⏳ Нажмите [{}]", self.pick_key_name)
                    } else {
                        format!("🎯 Захват [{}]", self.pick_key_name)
                    };
                    if ui.small_button(pick_btn_text).clicked() {
                        self.picking_window_target = !self.picking_window_target;
                        if self.picking_window_target {
                            self.window_status_msg = Some(format!("Наведите на окно и нажмите [{}]", self.pick_key_name));
                        }
                    }

                    if ui.small_button(if self.show_window_picker { "✕ Скрыть" } else { "📋 Окна" }).clicked() {
                        self.show_window_picker = !self.show_window_picker;
                        if self.show_window_picker {
                            self.open_windows = WinInput::list_open_windows();
                        }
                    }

                    if is_bound && ui.small_button(RichText::new("✕").color(Color32::from_rgb(248, 81, 73))).clicked() {
                        self.advanced_cfg.window_target.hwnd = 0;
                        self.advanced_cfg.window_target.title.clear();
                    }
                });

                if self.advanced_cfg.window_target.enabled {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Точка клика:").size(11.5).color(Color32::from_rgb(139, 148, 158)));
                        ui.radio_value(&mut self.advanced_cfg.window_target.click_center, true, "Центр окна");
                        ui.radio_value(&mut self.advanced_cfg.window_target.click_center, false, "Координаты");

                        if !self.advanced_cfg.window_target.click_center {
                            ui.label("X:");
                            ui.add(egui::DragValue::new(&mut self.advanced_cfg.window_target.custom_x).speed(1).range(0..=4000));
                            ui.label("Y:");
                            ui.add(egui::DragValue::new(&mut self.advanced_cfg.window_target.custom_y).speed(1).range(0..=4000));
                        }
                    });

                    if self.show_window_picker {
                        ui.add_space(6.0);
                        egui::Frame::none()
                            .fill(Color32::from_rgb(13, 17, 23))
                            .rounding(Rounding::same(5.0))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61)))
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Поиск:");
                                    ui.add(egui::TextEdit::singleline(&mut self.window_search_query).hint_text("Имя окна...").desired_width(180.0));
                                    if ui.small_button("🔄").clicked() {
                                        self.open_windows = WinInput::list_open_windows();
                                    }
                                });
                                ui.add_space(4.0);
                                let query = self.window_search_query.to_lowercase();
                                let filtered: Vec<_> = self.open_windows.iter().filter(|w| query.is_empty() || w.title.to_lowercase().contains(&query)).cloned().collect();
                                egui::ScrollArea::vertical().max_height(110.0).show(ui, |ui| {
                                    for win in filtered {
                                        ui.horizontal(|ui| {
                                            let is_cur = win.hwnd == self.advanced_cfg.window_target.hwnd;
                                            if ui.small_button(if is_cur { "✓" } else { "Выбрать" }).clicked() {
                                                self.advanced_cfg.window_target.hwnd = win.hwnd;
                                                self.advanced_cfg.window_target.title = win.title.clone();
                                                self.advanced_cfg.window_target.enabled = true;
                                                self.show_window_picker = false;
                                            }
                                            let display = if win.title.len() > 36 { format!("{}…", win.title.chars().take(33).collect::<String>()) } else { win.title.clone() };
                                            ui.label(RichText::new(display).size(11.5));
                                            ui.label(RichText::new(format!("({}×{})", win.width, win.height)).size(10.5).color(Color32::from_rgb(139, 148, 158)));
                                        });
                                    }
                                });
                            });
                    }
                }
            },
        );
    }

    fn render_mouse_tab(&mut self, ui: &mut egui::Ui) {
        // Быстрые пресеты
        ui.horizontal(|ui| {
            ui.label(RichText::new("Пресеты:").size(11.5).color(Color32::from_rgb(139, 148, 158)));
            if primer_topic_badge(ui, "⚡ Roblox AFK") {
                self.mouse_cfg.button = MouseButton::Left;
                self.mouse_cfg.click_type = ClickType::Single;
                self.mouse_cfg.use_cps = true;
                self.mouse_cfg.cps = 10.0;
                self.mouse_cfg.jitter_ms = 12.0;
            }
            if primer_topic_badge(ui, "🚀 Fast 25 CPS") {
                self.mouse_cfg.button = MouseButton::Left;
                self.mouse_cfg.click_type = ClickType::Single;
                self.mouse_cfg.use_cps = true;
                self.mouse_cfg.cps = 25.0;
                self.mouse_cfg.jitter_ms = 4.0;
            }
            if primer_topic_badge(ui, "⛏️ Hold LMB") {
                self.mouse_cfg.button = MouseButton::Left;
                self.mouse_cfg.click_type = ClickType::Hold;
                self.mouse_cfg.continuous_hold = true;
            }
            if primer_topic_badge(ui, "🎯 1000 MAX") {
                self.mouse_cfg.button = MouseButton::Left;
                self.mouse_cfg.click_type = ClickType::Single;
                self.mouse_cfg.use_cps = true;
                self.mouse_cfg.cps = 1000.0;
                self.mouse_cfg.jitter_ms = 0.0;
            }
        });
        ui.add_space(6.0);

        // Box 1: Кнопка и частота кликов
        primer_box(ui, "Параметры клика", None, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Кнопка:").size(12.0));
                let btn_options = [
                    (MouseButton::Left, "ЛКМ"),
                    (MouseButton::Right, "ПКМ"),
                    (MouseButton::Middle, "Колесико"),
                    (MouseButton::X1, "X1"),
                    (MouseButton::X2, "X2"),
                ];
                primer_segmented_control(ui, &mut self.mouse_cfg.button, &btn_options);

                ui.add_space(10.0);
                ui.label(RichText::new("Тип:").size(12.0));
                let click_options = [
                    (ClickType::Single, "Одиночный"),
                    (ClickType::Double, "Двойной"),
                    (ClickType::Triple, "Тройной"),
                    (ClickType::Hold, "Удержание"),
                ];
                primer_segmented_control(ui, &mut self.mouse_cfg.click_type, &click_options);
            });

            if self.mouse_cfg.click_type == ClickType::Hold {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.mouse_cfg.continuous_hold,
                        RichText::new("Постоянно зажато (до F6)").color(Color32::from_rgb(88, 166, 255)),
                    );
                    if !self.mouse_cfg.continuous_hold {
                        ui.add_space(8.0);
                        ui.label("Время зажатия:");
                        ui.add(
                            egui::DragValue::new(&mut self.mouse_cfg.hold_duration_ms)
                                .speed(5.0)
                                .range(10.0..=100000.0)
                                .suffix(" мс"),
                        );
                    }
                });
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let mode_options = [
                    (true, "CPS (в сек)"),
                    (false, "Интервал (мс)"),
                ];
                primer_segmented_control(ui, &mut self.mouse_cfg.use_cps, &mode_options);

                ui.add_space(8.0);
                if self.mouse_cfg.use_cps {
                    ui.add(
                        egui::Slider::new(&mut self.mouse_cfg.cps, 1.0..=1000.0)
                            .text("CPS")
                            .logarithmic(true),
                    );
                } else {
                    ui.add(
                        egui::DragValue::new(&mut self.mouse_cfg.custom_interval_ms)
                            .speed(1.0)
                            .range(0.1..=60000.0)
                            .suffix(" мс"),
                    );
                }

                ui.add_space(8.0);
                ui.label("Jitter:");
                ui.add(
                    egui::DragValue::new(&mut self.mouse_cfg.jitter_ms)
                        .speed(0.5)
                        .range(0.0..=500.0)
                        .suffix(" ±мс"),
                );
            });

            if self.mouse_cfg.use_cps {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Быстро:").size(11.0).color(Color32::from_rgb(139, 148, 158)));
                    let presets = [
                        ("10", 10.0),
                        ("16", 16.0),
                        ("25", 25.0),
                        ("50", 50.0),
                        ("100", 100.0),
                        ("1000 MAX", 1000.0),
                    ];
                    for (title, val) in presets {
                        if primer_topic_badge(ui, title) {
                            self.mouse_cfg.cps = val;
                        }
                    }
                    let ms_per_click = 1000.0 / self.mouse_cfg.cps.max(0.1);
                    ui.label(
                        RichText::new(format!("({:.1} мс/клик)", ms_per_click))
                            .size(10.5)
                            .color(Color32::from_rgb(110, 118, 129)),
                    );
                });
            }
        });

        // Box 2: Позиционирование и завершение
        primer_box(ui, "Позиция курсора и лимит", None, |ui| {
            ui.horizontal(|ui| {
                let pos_options = [
                    (false, "Текущий курсор"),
                    (true, "Координаты (X, Y)"),
                ];
                primer_segmented_control(ui, &mut self.mouse_cfg.fixed_pos, &pos_options);

                if self.mouse_cfg.fixed_pos {
                    ui.add_space(8.0);
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut self.mouse_cfg.target_x).speed(1));
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut self.mouse_cfg.target_y).speed(1));

                    ui.add_space(6.0);
                    if ui.small_button(format!("🎯 Захват [{}]", self.pick_key_name)).clicked() {
                        self.mouse_cfg.target_x = self.cursor_x;
                        self.mouse_cfg.target_y = self.cursor_y;
                    }

                    ui.add_space(6.0);
                    ui.label("Jitter:");
                    ui.add(
                        egui::DragValue::new(&mut self.mouse_cfg.jitter_px)
                            .speed(1)
                            .range(0..=100)
                            .suffix(" ±px"),
                    );
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Остановить после:").size(11.5).color(Color32::from_rgb(139, 148, 158)));
                ui.add(
                    egui::DragValue::new(&mut self.mouse_cfg.stop_after_count)
                        .speed(10)
                        .range(0..=1000000),
                );
                ui.label(RichText::new("кликов (0 = без ограничений)").size(11.0).color(Color32::from_rgb(110, 118, 129)));
            });
        });

        // Box 3: Фоновый режим (привязка к игре)
        self.render_window_target_card(ui);
    }

    fn render_keyboard_tab(&mut self, ui: &mut egui::Ui) {
        primer_box(ui, "Авто-нажатие клавиши", None, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Клавиша:").size(12.0));

                let key_text = if self.capturing_key {
                    RichText::new("Нажмите клавишу...").size(12.0).strong().color(Color32::from_rgb(240, 136, 62))
                } else {
                    RichText::new(format!("[ {} ]", self.kb_cfg.key_name)).size(12.5).strong().color(Color32::WHITE)
                };

                let pick_key_btn = egui::Button::new(key_text)
                    .min_size(Vec2::new(100.0, 26.0))
                    .fill(if self.capturing_key { Color32::from_rgb(158, 66, 17) } else { Color32::from_rgb(33, 38, 45) })
                    .stroke(Stroke::new(1.0_f32, if self.capturing_key { Color32::from_rgb(240, 136, 62) } else { Color32::from_rgb(48, 54, 61) }))
                    .rounding(Rounding::same(5.0));

                if ui.add(pick_key_btn).clicked() {
                    self.capturing_key = !self.capturing_key;
                }

                ui.add_space(8.0);
                ui.label(RichText::new("Быстро:").size(11.5).color(Color32::from_rgb(139, 148, 158)));
                let keys = [
                    ("Space", 0x20),
                    ("E", 0x45),
                    ("Q", 0x51),
                    ("F", 0x46),
                    ("1", 0x31),
                    ("2", 0x32),
                    ("Shift", 0x10),
                    ("Enter", 0x0D),
                ];
                for (name, code) in keys {
                    if primer_topic_badge(ui, name) {
                        self.kb_cfg.vk_code = code;
                        self.kb_cfg.key_name = name.to_string();
                    }
                }
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Режим:").size(12.0));
                let key_modes = [
                    (KeyMode::Tap, "Тап (нажал-отпустил)"),
                    (KeyMode::Hold, "Удержание (Hold)"),
                ];
                primer_segmented_control(ui, &mut self.kb_cfg.mode, &key_modes);

                if self.kb_cfg.mode == KeyMode::Hold {
                    ui.add_space(8.0);
                    ui.label("Удержание:");
                    ui.add(
                        egui::DragValue::new(&mut self.kb_cfg.hold_ms)
                            .speed(5.0)
                            .range(1.0..=60000.0)
                            .suffix(" мс"),
                    );
                }

                ui.add_space(8.0);
                ui.label("Интервал:");
                ui.add(
                    egui::DragValue::new(&mut self.kb_cfg.interval_ms)
                        .speed(2.0)
                        .range(1.0..=60000.0)
                        .suffix(" мс"),
                );

                ui.add_space(8.0);
                ui.label("Jitter:");
                ui.add(
                    egui::DragValue::new(&mut self.kb_cfg.jitter_ms)
                        .speed(1.0)
                        .range(0.0..=500.0)
                        .suffix(" ±мс"),
                );
            });
        });
    }

    fn render_scroll_tab(&mut self, ui: &mut egui::Ui) {
        primer_box(ui, "Авто-скроллинг колесиком", None, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Направление:").size(12.0));
                let directions = [
                    (ScrollDirection::Down, "Вниз"),
                    (ScrollDirection::Up, "Вверх"),
                    (ScrollDirection::Right, "Вправо"),
                    (ScrollDirection::Left, "Влево"),
                ];
                primer_segmented_control(ui, &mut self.scroll_cfg.direction, &directions);

                ui.add_space(10.0);
                ui.label("Шагов (тиков):");
                ui.add(
                    egui::DragValue::new(&mut self.scroll_cfg.ticks)
                        .speed(1)
                        .range(1..=50),
                );

                ui.add_space(10.0);
                ui.label("Интервал:");
                ui.add(
                    egui::DragValue::new(&mut self.scroll_cfg.interval_ms)
                        .speed(5.0)
                        .range(5.0..=10000.0)
                        .suffix(" мс"),
                );
            });
        });
    }

    fn render_points_tab(&mut self, ui: &mut egui::Ui) {
        primer_box(ui, "Маршрут кликов по точкам", None, |ui| {
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut self.new_pt_x).speed(1));
                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut self.new_pt_y).speed(1));

                ui.add_space(6.0);
                if ui.small_button(format!("🎯 Захват [{}]", self.pick_key_name)).clicked() {
                    self.new_pt_x = self.cursor_x;
                    self.new_pt_y = self.cursor_y;
                }

                ui.add_space(6.0);
                ui.label("Пауза:");
                ui.add(egui::DragValue::new(&mut self.new_pt_delay).speed(5.0).range(10.0..=60000.0).suffix(" мс"));

                ui.add_space(6.0);
                ui.label("Удержание:");
                ui.add(egui::DragValue::new(&mut self.new_pt_hold).speed(5.0).range(0.0..=10000.0).suffix(" мс"));

                ui.add_space(8.0);
                let add_btn = egui::Button::new(RichText::new("➕ Добавить").size(11.5).strong().color(Color32::WHITE))
                    .min_size(Vec2::new(0.0, 24.0))
                    .fill(Color32::from_rgb(35, 134, 54))
                    .rounding(Rounding::same(5.0));

                if ui.add(add_btn).clicked() {
                    self.points.push(RoutePoint {
                        x: self.new_pt_x,
                        y: self.new_pt_y,
                        button: self.new_pt_btn,
                        delay_after_ms: self.new_pt_delay,
                        hold_ms: self.new_pt_hold,
                    });
                }

                if !self.points.is_empty() {
                    ui.add_space(6.0);
                    if ui.small_button(RichText::new("🗑️").color(Color32::from_rgb(248, 81, 73))).clicked() {
                        self.points.clear();
                    }
                }
            });

            if !self.points.is_empty() {
                ui.add_space(6.0);
                let mut remove_idx = None;
                egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                    for (i, pt) in self.points.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{}.", i + 1)).strong().color(Color32::from_rgb(88, 166, 255)));
                            ui.label(format!("(X: {}, Y: {})", pt.x, pt.y));
                            ui.label(format!("пауза: {} мс", pt.delay_after_ms));
                            if pt.hold_ms > 0.0 {
                                ui.label(RichText::new(format!("удержание: {} мс", pt.hold_ms)).color(Color32::from_rgb(63, 185, 80)));
                            }
                            if ui.small_button("Удалить").clicked() {
                                remove_idx = Some(i);
                            }
                        });
                        ui.add_space(2.0);
                    }
                });
                if let Some(i) = remove_idx {
                    self.points.remove(i);
                }
            }
        });
    }

    fn run_hunter_test(&mut self, ctx: &egui::Context) {
        match crate::hunter::capture(&self.vision_cfg, &self.advanced_cfg) {
            Ok(frame) => {
                let mut detector = PopupDetector::new();
                let mut test_config = self.vision_cfg.clone();
                test_config.only_new_targets = false; // Preview checks appearance, without waiting for a new popup.
                let target = crate::hunter::find(&frame, &test_config, &mut detector);
                let message = if let Some(t) = &target {
                    format!("Найдена цель ({}, {}) · {} · тест без клика", t.x, t.y, t.method)
                } else if self.vision_cfg.mode == VisionMode::TemplateMatch {
                    if self.vision_cfg.templates.is_empty() { "Образец ещё не загружен или не вырезан".into() }
                    else {
                        let mut diagnostic = test_config.clone();
                        for template in &mut diagnostic.templates { template.threshold = 0.65; }
                        if let Some(closest) = crate::hunter::find(&frame, &diagnostic, &mut PopupDetector::new()) {
                            format!("Совпадение ниже заданной точности: {}. Вырежьте цель плотнее, без фона. Клик не отправлен", closest.method)
                        } else { "Картинка в кадре не найдена. Проверьте миниатюру, окно игры и размер цели".into() }
                    }
                } else { "Подтверждённого кружка со светлым значком нет. Клик не отправлен".into() };
                let (pixels, w, h) = crate::hunter::preview(&frame, target.as_ref());
                self.hunter_preview = Some(ctx.load_texture("hunter_preview", egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels), egui::TextureOptions::LINEAR));
                self.test_scan_result = Some((target.is_some(), message));
            }
            Err(message) => { self.test_scan_result = Some((false, message)); self.hunter_preview = None; }
        }
    }

    fn render_vision_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Охотник за бонусами");
        ui.label("Выберите игру и цель. Затем откройте игру и нажмите F6.");
        let running = self.engine.is_running();
        let previous_window = self.advanced_cfg.window_target.hwnd;
        ui.add_space(8.0);
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("1. Окно игры").strong().size(17.0));
            ui.add_enabled_ui(!running, |ui| {
                if self.open_windows.is_empty() { self.open_windows = WinInput::list_open_windows(); }
                ui.horizontal_wrapped(|ui| {
                    let title = if self.advanced_cfg.window_target.hwnd == 0 { "Выберите игру".to_string() }
                        else { self.advanced_cfg.window_target.title.chars().take(42).collect() };
                    egui::ComboBox::from_id_salt("hunter_game").width(280.0).selected_text(title).show_ui(ui, |ui| {
                        for win in &self.open_windows {
                            if ui.selectable_label(win.hwnd == self.advanced_cfg.window_target.hwnd, &win.title).clicked() {
                                self.advanced_cfg.window_target.hwnd = win.hwnd;
                                self.advanced_cfg.window_target.title = win.title.clone();
                            }
                        }
                    });
                    if ui.button("Обновить список").clicked() { self.open_windows = WinInput::list_open_windows(); }
                    if ui.button(format!("Указать окно [{}]", self.pick_key_name)).clicked() {
                        self.picking_window_target = true;
                    }
                });
                if self.picking_window_target { ui.colored_label(Color32::YELLOW,
                    format!("Наведите курсор на игру и нажмите {}", self.pick_key_name)); }
            });
            ui.small("Работает только в выбранной игре. Плавающее видео и другие окна исключены.");
        });
        if previous_window != self.advanced_cfg.window_target.hwnd {
            self.clear_hunter_preview();
            self.vision_cfg.use_custom_region = false;
        }
        self.advanced_cfg.window_target.enabled = self.advanced_cfg.window_target.hwnd != 0;
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("2. Цель").strong().size(17.0));
            ui.add_enabled_ui(!running, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.selectable_label(self.vision_cfg.mode == VisionMode::AutoPopup, "Авто: кружки").clicked() {
                        self.vision_cfg.mode = VisionMode::AutoPopup;
                        self.vision_cfg.exclude_hud = true;
                        self.vision_cfg.strict_circles = true;
                        self.vision_cfg.require_white_icon = true;
                        self.vision_cfg.round_only = true;
                        self.vision_cfg.only_new_targets = false;
                    }
                    if ui.selectable_label(self.vision_cfg.mode == VisionMode::TemplateMatch, "По картинке").clicked() {
                        self.vision_cfg.mode = VisionMode::TemplateMatch;
                        self.vision_cfg.only_new_targets = false;
                    }
                });
                ui.horizontal_wrapped(|ui| {
                        if ui.add_sized([240.0, 40.0], egui::Button::new(format!("Вырезать картинку [{}]", self.pick_key_name))).clicked() {
                            self.vision_cfg.mode = VisionMode::TemplateMatch;
                            self.vision_cfg.only_new_targets = false;
                            self.vision_snip_mode = VisionSnipMode::TwoPointRect;
                            self.snip_p1 = None;
                            self.vision_status_msg = Some(format!("В игре нажмите {} на двух углах нужной кнопки", self.pick_key_name));
                        }
                });
                if ui.button("Загрузить картинку…").clicked() {
                    if let Some(path) = WinInput::pick_image_file() { self.load_hunter_image(&path); }
                }
                ui.small("PNG/JPG можно перетащить в окно. Новый образец заменяет предыдущий.");
                if self.vision_cfg.mode != VisionMode::AutoPopup {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("Образцов: {}", self.vision_cfg.templates.len()));
                        if !self.vision_cfg.templates.is_empty() && ui.small_button("Удалить образцы").clicked() {
                            self.vision_cfg.templates.clear();
                        }
                    });
                    ui.small("Укажите два угла кнопки в игре клавишей F8. Затем включите поиск — найденная картинка будет нажиматься с паузой.");
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Или файл:");
                        ui.text_edit_singleline(&mut self.file_import_path);
                        if ui.button("Загрузить PNG / JPG").clicked() {
                            let path = std::path::PathBuf::from(self.file_import_path.trim().trim_matches('"'));
                            self.load_hunter_image(&path);
                        }
                    });
                    for template in &self.vision_cfg.templates {
                        let texture = self.template_textures.entry(template.id).or_insert_with(|| {
                            let mut pixels = template.bgra.clone();
                            for p in pixels.chunks_exact_mut(4) { p.swap(0,2); }
                            ui.ctx().load_texture(format!("sample_{}", template.id), egui::ColorImage::from_rgba_unmultiplied(
                                [template.width as usize, template.height as usize], &pixels), egui::TextureOptions::LINEAR)
                        });
                        let size = texture.size_vec2();
                        let scale = (120.0 / size.x).min(70.0 / size.y).min(1.0);
                        ui.image((texture.id(), size * scale));
                    }
                } else {
                    ui.checkbox(&mut self.vision_cfg.exclude_hud, "Не нажимать на меню и счётчики по краям экрана");
                    ui.small("Поиск в игровой области: боковые меню, верхняя панель и нижние счётчики исключены.");
                }
                if let Some(message) = &self.vision_status_msg { ui.colored_label(Color32::LIGHT_BLUE, message); }
            });
        });
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("Состояние").strong().size(17.0));
            if self.advanced_cfg.window_target.hwnd == 0 { ui.label("Выберите игру в списке выше или выделите образец в игре."); }
            else if self.vision_cfg.mode == VisionMode::TemplateMatch && self.vision_cfg.templates.is_empty() {
                ui.label("Вырежьте или загрузите картинку кнопки. Без образца поиск не запускается.");
            }
            else if let Ok(status) = self.engine.hunter_status.lock() {
                ui.label(if status.message.is_empty() { "Готов. Переключитесь в игру и нажмите F6." } else { &status.message });
                if let Some(error) = &status.input_error { ui.colored_label(Color32::LIGHT_RED, error); }
            }
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Кликов: {}", self.engine.total_clicks.load(std::sync::atomic::Ordering::Relaxed)));
                if ui.add_enabled(!running && self.advanced_cfg.window_target.hwnd != 0,
                    egui::Button::new("Проверить без клика")).clicked() {
                    self.pending_test = Some(std::time::Instant::now() + std::time::Duration::from_secs(3));
                    self.test_scan_result = None;
                }
            });
            if let Some(deadline) = self.pending_test {
                ui.colored_label(Color32::YELLOW, format!("Откройте игру. Проверка через {:.1} с",
                    deadline.saturating_duration_since(std::time::Instant::now()).as_secs_f32()));
            }
            if let Some((_, message)) = &self.test_scan_result { ui.label(message); }
        });
        egui::CollapsingHeader::new("Дополнительные настройки").show(ui, |ui| {
            ui.add_enabled_ui(!running, |ui| {
                ui.checkbox(&mut self.vision_cfg.only_new_targets, "Только новые появления");
                ui.checkbox(&mut self.vision_cfg.round_only, "Только круглые или квадратные объекты");
                ui.checkbox(&mut self.vision_cfg.require_white_icon, "Светлый значок внутри");
                if ui.checkbox(&mut self.vision_cfg.roblox_profile, "Строгий фильтр кружков Roblox").changed() {
                    self.vision_cfg.popup_filter = if self.vision_cfg.roblox_profile { PopupColorFilter::UniversalCircle } else { PopupColorFilter::AnyVibrant };
                    self.vision_cfg.exclude_hud = self.vision_cfg.roblox_profile;
                }
                ui.horizontal_wrapped(|ui| {
                    ui.label("Цвет:");
                    egui::ComboBox::from_id_salt("hunter_color").selected_text(match self.vision_cfg.popup_filter {
                        PopupColorFilter::PurpleViolet => "Фиолетовый", PopupColorFilter::Green => "Зелёный",
                        PopupColorFilter::Red => "Красный", PopupColorFilter::CyanBlue => "Синий",
                        PopupColorFilter::GoldYellow => "Жёлтый", PopupColorFilter::UniversalCircle => "Roblox", _ => "Любой"
                    }).show_ui(ui, |ui| {
                        for (color, label) in [(PopupColorFilter::AnyVibrant,"Любой"), (PopupColorFilter::PurpleViolet,"Фиолетовый"),
                            (PopupColorFilter::Green,"Зелёный"), (PopupColorFilter::Red,"Красный"),
                            (PopupColorFilter::CyanBlue,"Синий"), (PopupColorFilter::GoldYellow,"Жёлтый")] {
                            ui.selectable_value(&mut self.vision_cfg.popup_filter, color, label);
                        }
                    });
                    ui.label("Размер:");
                    ui.add(egui::DragValue::new(&mut self.vision_cfg.popup_min_size).range(10..=500));
                    ui.label("—");
                    ui.add(egui::DragValue::new(&mut self.vision_cfg.popup_max_size).range(10..=800).suffix(" px"));
                });
                ui.checkbox(&mut self.vision_cfg.return_cursor, "Возвращать курсор после клика");
                ui.checkbox(&mut self.vision_cfg.multiscale, "Искать образец в разных размерах");
                for template in &mut self.vision_cfg.templates {
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut template.enabled, &template.name);
                        ui.add(egui::Slider::new(&mut template.threshold, 0.7..=0.99).text("Точность"));
                    });
                }
                ui.horizontal_wrapped(|ui| {
                    ui.label("Пауза после клика:");
                    ui.add(egui::DragValue::new(&mut self.vision_cfg.cooldown_ms).range(50.0..=2000.0).suffix(" мс"));
                    if ui.button(format!("Ограничить область [{}]", self.pick_key_name)).clicked() {
                        self.vision_snip_mode = VisionSnipMode::RegionPoint;
                        self.snip_p1 = None;
                        self.vision_status_msg = Some(format!("Укажите два угла области в игре клавишей {}", self.pick_key_name));
                    }
                    if self.vision_cfg.use_custom_region && ui.button("Всё окно игры").clicked() { self.vision_cfg.use_custom_region = false; }
                });
            });
        });
        egui::CollapsingHeader::new("Последний кадр и диагностика").show(ui, |ui| {
            if let Some(texture) = &self.hunter_preview {
                let size = texture.size_vec2();
                let scale = (360.0 / size.x).min(180.0 / size.y).min(1.0);
                ui.image((texture.id(), size * scale));
                ui.small("Чёрные области — перекрывающие окна, они не участвуют в поиске.");
            }
            if let Ok(status) = self.engine.hunter_status.lock() {
                ui.small(format!("Кадр {}×{} · Проверок: {} · Поиск: {:.1} мс", status.frame_size.0, status.frame_size.1, status.scans, status.scan_ms));
                ui.small(&status.last_action);
            }
        });
    }

    fn clear_hunter_preview(&mut self) {
        self.hunter_preview = None;
        self.test_scan_result = None;
        self.pending_test = None;
        self.vision_status_msg = None;
        if let Ok(mut status) = self.engine.hunter_status.lock() { *status = Default::default(); }
    }

    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        if !WinInput::is_elevated() && ui.button("Перезапустить с правами администратора").clicked() {
            let _ = WinInput::restart_as_admin();
        }
        // Section 1: Failsafe & Overlay
        primer_box(ui, "🛡️ Безопасность и окно", None, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.advanced_cfg.failsafe_mouse_move, "Пауза при движении мыши");
                if self.advanced_cfg.failsafe_mouse_move {
                    ui.add_space(4.0);
                    ui.label("Порог:");
                    ui.add(egui::DragValue::new(&mut self.advanced_cfg.mouse_move_threshold).speed(1).range(10..=200).suffix(" px"));
                }

                ui.add_space(10.0);
                ui.checkbox(&mut self.advanced_cfg.corner_panic, "Остановка в углу (0,0)");
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.advanced_cfg.session_limit_enabled, "AFK таймер");
                if self.advanced_cfg.session_limit_enabled {
                    ui.add_space(4.0);
                    let mut minutes = (self.advanced_cfg.session_limit_seconds / 60) as u32;
                    if minutes == 0 { minutes = 1; }
                    if ui.add(egui::DragValue::new(&mut minutes).speed(1).range(1..=360).suffix(" мин")).changed() {
                        self.advanced_cfg.session_limit_seconds = (minutes as u64) * 60;
                    }
                }

                ui.add_space(10.0);
                ui.checkbox(&mut self.advanced_cfg.always_on_top, "Поверх всех окон");

                ui.add_space(10.0);
                ui.label("Прозрачность:");
                ui.add(
                    egui::Slider::new(&mut self.advanced_cfg.window_opacity, 0.40..=1.0)
                        .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                );
            });
        });

        // Section 2: Human Simulation & Anti-Detection
        primer_box(ui, "🤖 Человекоподобный клик и анти-детект", None, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.advanced_cfg.gaussian_jitter, "Гауссово распределение");

                ui.add_space(10.0);
                ui.checkbox(&mut self.advanced_cfg.human_hold_duration, "Естественное удержание");
                if self.advanced_cfg.human_hold_duration {
                    ui.add(egui::DragValue::new(&mut self.advanced_cfg.natural_hold_ms).speed(1.0).range(10.0..=100.0).suffix(" мс"));
                }

                ui.add_space(10.0);
                ui.checkbox(&mut self.advanced_cfg.micro_drift, "Микро-дрейф мыши");
                if self.advanced_cfg.micro_drift {
                    ui.add(egui::DragValue::new(&mut self.advanced_cfg.micro_drift_px).speed(1).range(1..=6).suffix(" px"));
                }
            });
        });

        // Section 3: Hotkeys
        primer_box(ui, "⌨️ Горячие клавиши", None, |ui| {
            let rows: [(HotkeyAction, &str, String, Color32); 3] = [
                (
                    HotkeyAction::ToggleStartStop,
                    "Запуск / Пауза кликера",
                    self.toggle_key_name.clone(),
                    Color32::from_rgb(88, 166, 255),
                ),
                (
                    HotkeyAction::PickCoordinate,
                    "Захват координат / ножницы",
                    self.pick_key_name.clone(),
                    Color32::from_rgb(63, 185, 80),
                ),
                (
                    HotkeyAction::EmergencyStop,
                    "Экстренная остановка (Panic)",
                    self.panic_key_name.clone(),
                    Color32::from_rgb(248, 81, 73),
                ),
            ];

            for (action, title, key_name, badge_color) in rows {
                let is_capturing = self.capturing_hotkey == Some(action);

                ui.horizontal(|ui| {
                    egui::Frame::none()
                        .fill(if is_capturing { Color32::from_rgb(210, 153, 34) } else { Color32::from_rgb(33, 38, 45) })
                        .rounding(Rounding::same(5.0))
                        .stroke(if is_capturing { Stroke::new(1.0_f32, Color32::from_rgb(240, 136, 62)) } else { Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61)) })
                        .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                        .show(ui, |ui| {
                            let label_text = if is_capturing { "Нажмите клавишу...".to_string() } else { key_name };
                            ui.label(
                                RichText::new(label_text)
                                    .size(12.0)
                                    .strong()
                                    .color(if is_capturing { Color32::BLACK } else { badge_color }),
                            );
                        });

                    let btn_label = if is_capturing { "Отмена" } else { "Изменить" };
                    if ui.small_button(btn_label).clicked() {
                        if is_capturing {
                            self.capturing_hotkey = None;
                        } else {
                            self.capturing_hotkey = Some(action);
                        }
                    }

                    ui.label(RichText::new(title).size(12.0).color(Color32::from_rgb(240, 246, 252)));
                });
                ui.add_space(4.0);
            }

            ui.add_space(2.0);
            if ui.small_button("Вернуть по умолчанию (F6, F8, F12)").clicked() {
                self.toggle_key_code = winapi::um::winuser::VK_F6 as u16;
                self.toggle_key_name = "F6".to_string();
                self.hotkeys.set_key(HotkeyAction::ToggleStartStop, winapi::um::winuser::VK_F6 as u16);

                self.pick_key_code = winapi::um::winuser::VK_F8 as u16;
                self.pick_key_name = "F8".to_string();
                self.hotkeys.set_key(HotkeyAction::PickCoordinate, winapi::um::winuser::VK_F8 as u16);

                self.panic_key_code = winapi::um::winuser::VK_F12 as u16;
                self.panic_key_name = "F12".to_string();
                self.hotkeys.set_key(HotkeyAction::EmergencyStop, winapi::um::winuser::VK_F12 as u16);
                self.capturing_hotkey = None;
            }
        });

        // Section 4: Compact About
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("⚡ Clicker Pro (Rust) • Win32 SendInput • winmm.dll таймер суб-мс • 14 МБ автономно")
                    .size(10.5)
                    .color(Color32::from_rgb(110, 118, 129)),
            );
        });
    }
}
