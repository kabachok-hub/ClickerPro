use eframe::egui::{self, Color32, RichText, Rounding, Stroke, Vec2};
use winapi::um::winuser::GetAsyncKeyState;

use crate::engine::{
    ActiveTab, AdvancedConfig, EngineState, KeyMode, KeyboardConfig, MouseConfig, RoutePoint,
    ScrollConfig, ScrollDirection,
};
use crate::hotkeys::{HotkeyAction, HotkeyEvent, HotkeyManager};
use crate::screen_capture::ScreenCapture;
use crate::vision::{ButtonTemplate, PopupColorFilter, VisionConfig, VisionMode};
use crate::win_input::{ClickType, MouseButton, WinInput};

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
    pub file_import_path: String,
    pub file_import_error: Option<String>,
    pub template_textures: std::collections::HashMap<u64, egui::TextureHandle>,

    // Embedded icon texture
    pub icon_texture: Option<egui::TextureHandle>,

    // Target window / background clicking state
    pub picking_window_target: bool,
    pub picking_window_point: bool,
    pub open_windows: Vec<crate::win_input::WindowInfo>,
    pub show_window_picker: bool,
    pub window_search_query: String,
    pub window_status_msg: Option<String>,

    pub unified_cfg: crate::engine::UnifiedConfig,

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

        Self {
            engine: EngineState::new(),
            hotkeys: HotkeyManager::new(),
            active_tab: ActiveTab::Mouse,

            capturing_hotkey: None,
            toggle_key_code: winapi::um::winuser::VK_F6 as u16,
            toggle_key_name: "F6".to_string(),
            pick_key_code: winapi::um::winuser::VK_F8 as u16,
            pick_key_name: "F8".to_string(),
            panic_key_code: winapi::um::winuser::VK_F12 as u16,
            panic_key_name: "F12".to_string(),

            advanced_cfg: AdvancedConfig::default(),
            unified_cfg: crate::engine::UnifiedConfig::default(),

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
                let mut v = VisionConfig::default();
                if let Ok(img) = image::load_from_memory(include_bytes!("../../assets/roblox_button.png")) {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    let mut bgra = Vec::with_capacity((w * h * 4) as usize);
                    for pixel in rgba.pixels() {
                        bgra.push(pixel[2]);
                        bgra.push(pixel[1]);
                        bgra.push(pixel[0]);
                        bgra.push(pixel[3]);
                    }
                    let tmpl = ButtonTemplate::new(
                        1001,
                        "🟣 Кружок Roblox (x2 / Тап)".to_string(),
                        w,
                        h,
                        bgra,
                        0.80,
                    );
                    v.templates.push(tmpl);
                }
                v
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
            window_status_msg: None,

            cursor_x: cx,
            cursor_y: cy,
        }
    }

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
        self.unified_cfg.window_target = self.advanced_cfg.window_target.clone();
        self.engine.toggle_unified(self.unified_cfg.clone(), self.advanced_cfg.clone());
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

    fn handle_vision_snip_hotkey(&mut self, cx: i32, cy: i32) {
        match self.vision_snip_mode {
            VisionSnipMode::CursorBox => {
                let half_w = self.snip_box_w / 2;
                let half_h = self.snip_box_h / 2;
                let x = (cx - half_w).max(0);
                let y = (cy - half_h).max(0);

                if let Some(cap) = ScreenCapture::capture_rect(x, y, self.snip_box_w, self.snip_box_h) {
                    let id = rand::random::<u64>();
                    let name = format!("Кнопка #{}", self.vision_cfg.templates.len() + 1);
                    let tmpl = ButtonTemplate::from_capture(id, name, &cap, 0.82);
                    self.vision_cfg.templates.push(tmpl);
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
                        if let Some(cap) = ScreenCapture::capture_rect(x, y, w, h) {
                            let id = rand::random::<u64>();
                            let name = format!("Кнопка #{}", self.vision_cfg.templates.len() + 1);
                            let tmpl = ButtonTemplate::from_capture(id, name, &cap, 0.82);
                            self.vision_cfg.templates.push(tmpl);
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
            ui.horizontal(|ui| {
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
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61))) // Border default #30363D
        .show(ui, |ui| {
            // Box Header
            egui::Frame::none()
                .fill(Color32::from_rgb(26, 31, 39))
                .rounding(Rounding {
                    nw: 8.0,
                    ne: 8.0,
                    sw: 0.0,
                    se: 0.0,
                })
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61)))
                .inner_margin(egui::Margin::symmetric(16.0, 10.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(title)
                                .size(13.5)
                                .strong()
                                .color(Color32::from_rgb(240, 246, 252)),
                        );
                        if let Some(desc) = description {
                            ui.label(
                                RichText::new(format!("— {}", desc))
                                    .size(11.5)
                                    .color(Color32::from_rgb(139, 148, 158)),
                            );
                        }
                    });
                });

            // Box Body
            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(16.0, 14.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    add_contents(ui);
                });
        });
    ui.add_space(14.0);
}

impl eframe::App for ClickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(30));

        let (cx, cy) = WinInput::get_cursor_pos();
        self.cursor_x = cx;
        self.cursor_y = cy;

        while let Some(event) = self.hotkeys.poll_event() {
            match event {
                HotkeyEvent::ToggleStartStop => {
                    self.toggle_active_engine();
                }
                HotkeyEvent::PickCoordinate => {
                    if self.picking_window_target {
                        if let Some(win) = WinInput::get_window_at_point(cx, cy) {
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

        // Send Always On Top viewport command
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            if self.advanced_cfg.always_on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            },
        ));

        let alpha = (255.0 * self.advanced_cfg.window_opacity.clamp(0.35, 1.0)) as u8;

        // GitHub Primer Dark Theme Tokens
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgba_unmultiplied(13, 17, 23, alpha); // #0D1117
        visuals.window_fill = Color32::from_rgba_unmultiplied(13, 17, 23, alpha);
        visuals.override_text_color = Some(Color32::from_rgb(230, 237, 243)); // #E6EDF3

        visuals.widgets.noninteractive.bg_fill = Color32::from_rgba_unmultiplied(22, 27, 34, alpha); // #161B22
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61)); // #30363D
        visuals.widgets.noninteractive.rounding = Rounding::same(6.0);

        visuals.widgets.inactive.bg_fill = Color32::from_rgba_unmultiplied(33, 38, 45, alpha); // #21262D
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61));
        visuals.widgets.inactive.rounding = Rounding::same(6.0);

        visuals.widgets.hovered.bg_fill = Color32::from_rgba_unmultiplied(48, 54, 61, alpha); // #30363D
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(139, 148, 158));
        visuals.widgets.hovered.rounding = Rounding::same(6.0);

        visuals.widgets.active.bg_fill = Color32::from_rgb(31, 111, 235); // #1F6FEB
        visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(88, 166, 255));
        visuals.widgets.active.rounding = Rounding::same(6.0);

        ctx.set_visuals(visuals);

        // Top Brand Header Panel
        egui::TopBottomPanel::top("top_brand_bar")
            .exact_height(54.0)
            .frame(egui::Frame::none().fill(Color32::from_rgba_unmultiplied(16, 20, 27, alpha)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(16.0);

                    if let Some(tex) = &self.icon_texture {
                        ui.add(
                            egui::Image::new(tex)
                                .fit_to_exact_size(Vec2::new(30.0, 30.0))
                                .rounding(Rounding::same(6.0)),
                        );
                    } else {
                        ui.label(RichText::new("⚡").size(22.0).color(Color32::from_rgb(240, 136, 62)));
                    }
                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("ClickerPro")
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::from_rgb(240, 246, 252)),
                            );
                            egui::Frame::none()
                                .fill(Color32::from_rgb(33, 38, 45))
                                .rounding(Rounding::same(4.0))
                                .inner_margin(egui::Margin::symmetric(5.0, 1.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new("Pro Rust")
                                            .size(9.5)
                                            .color(Color32::from_rgb(88, 166, 255)),
                                    );
                                });
                        });
                        ui.label(
                            RichText::new("Игровой автокликер + Авто-поиск всплывающих кнопок")
                                .size(11.0)
                                .color(Color32::from_rgb(139, 148, 158)),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);

                        // Always On Top Checkbox
                        ui.checkbox(
                            &mut self.advanced_cfg.always_on_top,
                            RichText::new("📌 Поверх окон").size(11.5).color(Color32::from_rgb(201, 209, 217)),
                        );

                        ui.add_space(12.0);

                        // Cursor coordinates
                        egui::Frame::none()
                            .fill(Color32::from_rgb(22, 27, 34))
                            .rounding(Rounding::same(6.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 5.0))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61)))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "Курсор: X: {:<4} Y: {:<4}",
                                        self.cursor_x, self.cursor_y
                                    ))
                                    .size(11.0)
                                    .color(Color32::from_rgb(139, 148, 158)),
                                );
                            });
                    });
                });
            });

        // Main Unified Central Panel
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(13, 17, 23)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            ui.vertical(|ui| {
                                ui.set_max_width(ui.available_width() - 20.0);
                                self.render_unified_view(ui);
                            });
                        });
                        ui.add_space(20.0);
                    });
            });
    }
}

impl ClickerApp {
    fn render_unified_view(&mut self, ui: &mut egui::Ui) {
        let running = self.engine.is_running();

        // 1. Status Alert
        if running {
            egui::Frame::none()
                .fill(Color32::from_rgb(15, 45, 30))
                .rounding(Rounding::same(8.0))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(46, 160, 67)))
                .inner_margin(egui::Margin::symmetric(16.0, 10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("● КЛИКЕР АКТИВЕН И РАБОТАЕТ В ИГРЕ")
                                .size(13.5)
                                .strong()
                                .color(Color32::from_rgb(63, 185, 80)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let total = self.engine.total_clicks.load(std::sync::atomic::Ordering::Relaxed);
                            ui.label(
                                RichText::new(format!("Всего кликов: {}", total))
                                    .size(12.5)
                                    .strong()
                                    .color(Color32::from_rgb(126, 231, 135)),
                            );
                            ui.label(
                                RichText::new(format!("Пауза: [{}]  •  Стоп: [{}]  |  ", self.toggle_key_name, self.panic_key_name))
                                    .size(12.0)
                                    .color(Color32::from_rgb(139, 148, 158)),
                            );
                        });
                    });
                });
            ui.add_space(10.0);
        }

        // 2. Giant Main Launch Button
        let (btn_text, btn_fill, border_color) = if running {
            (
                format!("⏹  ОСТАНОВИТЬ КЛИКЕР [{}]", self.toggle_key_name),
                Color32::from_rgb(218, 54, 51),
                Color32::from_rgb(248, 81, 73),
            )
        } else {
            (
                format!("▶  ЗАПУСТИТЬ КЛИКЕР [{}]", self.toggle_key_name),
                Color32::from_rgb(35, 134, 54),
                Color32::from_rgb(46, 160, 67),
            )
        };

        let main_btn = egui::Button::new(
            RichText::new(btn_text)
                .size(15.0)
                .strong()
                .color(Color32::WHITE),
        )
        .min_size(Vec2::new(ui.available_width(), 46.0))
        .rounding(Rounding::same(8.0))
        .fill(btn_fill)
        .stroke(Stroke::new(1.0_f32, border_color));

        if ui.add(main_btn).clicked() {
            self.toggle_active_engine();
        }

        ui.add_space(14.0);

        // Box 1: 🎯 Авто-поиск всплывающих кнопок
        primer_box(
            ui,
            "🎯 Авто-поиск и клик по всплывающим кнопкам (Roblox x2 / Кружки)",
            Some("кликер сам непрерывно сканирует экран и мгновенно кликает по кружкам"),
            |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.unified_cfg.hunter_enabled,
                        RichText::new("Включить авто-поиск кнопок на экране (Roblox x2, бонусы, тапы)")
                            .size(13.0)
                            .strong()
                            .color(Color32::from_rgb(240, 246, 252)),
                    );
                });

                if self.unified_cfg.hunter_enabled {
                    ui.add_space(10.0);
                    ui.label(RichText::new("Что именно искать на экране:").size(12.5).color(Color32::from_rgb(139, 148, 158)));
                    ui.add_space(6.0);

                    ui.horizontal_wrapped(|ui| {
                        let targets = [
                            (PopupColorFilter::PurpleViolet, "🟣 Кружок Roblox (Фиолетовый x2 / Тап)"),
                            (PopupColorFilter::AnyVibrant, "🌈 Любой яркий кружок / кнопка"),
                            (PopupColorFilter::GoldYellow, "🟡 Золотые монеты / звёзды"),
                            (PopupColorFilter::Green, "🟢 Зелёные кнопки"),
                            (PopupColorFilter::Red, "🔴 Красные кнопки"),
                            (PopupColorFilter::WhitePointerInside, "👆 Белая рука-указатель"),
                        ];

                        for (filter, label) in targets {
                            let is_sel = self.unified_cfg.popup_filter == filter;
                            let btn = egui::Button::new(
                                RichText::new(label)
                                    .size(12.0)
                                    .color(if is_sel { Color32::WHITE } else { Color32::from_rgb(139, 148, 158) }),
                            )
                            .min_size(Vec2::new(0.0, 30.0))
                            .rounding(Rounding::same(6.0))
                            .fill(if is_sel { Color32::from_rgb(31, 111, 235) } else { Color32::from_rgb(33, 38, 45) })
                            .stroke(Stroke::new(1.0_f32, if is_sel { Color32::from_rgb(88, 166, 255) } else { Color32::from_rgb(48, 54, 61) }));

                            if ui.add(btn).clicked() {
                                self.unified_cfg.popup_filter = filter;
                            }
                        }
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.unified_cfg.return_cursor, "");
                        ui.label(
                            RichText::new("↩️ Возвращать курсор на исходное место после клика (прицел и обзор в игре не сбиваются)")
                                .size(12.0)
                                .color(Color32::from_rgb(201, 209, 217)),
                        );
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Время зажатия кнопки:").size(12.0).color(Color32::from_rgb(139, 148, 158)));
                        ui.add(egui::DragValue::new(&mut self.unified_cfg.popup_hold_ms).speed(1.0).range(25.0..=200.0).suffix(" мс"));
                        ui.label(
                            RichText::new("(45 мс гарантирует 100% регистрацию клика в Roblox)")
                                .size(11.0)
                                .color(Color32::from_rgb(139, 148, 158)),
                        );
                    });
                }
            },
        );

        // Box 2: ⚡ Обычный авто-кликер
        primer_box(
            ui,
            "⚡ Обычный авто-кликер в игре (пока нет всплывающих кнопок)",
            Some("непрерывный фарм кликов в игре"),
            |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.unified_cfg.farm_enabled,
                        RichText::new("Постоянно кликать в игре (авто-фарм)")
                            .size(13.0)
                            .strong()
                            .color(Color32::from_rgb(240, 246, 252)),
                    );
                });

                if self.unified_cfg.farm_enabled {
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Скорость кликов:").size(12.5).color(Color32::from_rgb(201, 209, 217)));
                        ui.add(egui::Slider::new(&mut self.unified_cfg.farm_cps, 1.0..=50.0).text("CPS (кликов в сек)"));
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Кнопка мыши:").size(12.0).color(Color32::from_rgb(139, 148, 158)));
                        let is_left = self.unified_cfg.farm_button == MouseButton::Left;
                        if ui.button(if is_left { "Левая (ЛКМ)" } else { "Правая (ПКМ)" }).clicked() {
                            self.unified_cfg.farm_button = if is_left { MouseButton::Right } else { MouseButton::Left };
                        }
                    });
                }
            },
        );

        // Box 3: 🪟 Фоновый режим (привязка к окну)
        self.render_window_target_card(ui);

        // Box 4: ⌨️ Горячие клавиши
        primer_box(
            ui,
            "⌨️ Горячие клавиши (настраиваемые)",
            Some("управление кликером из любой игры"),
            |ui| {
                let rows: [(HotkeyAction, &str, &str, String, Color32); 2] = [
                    (
                        HotkeyAction::ToggleStartStop,
                        "Запуск / Остановка (Toggle)",
                        "Включает или ставит на паузу автокликер поверх любого окна или игры",
                        self.toggle_key_name.clone(),
                        Color32::from_rgb(88, 166, 255),
                    ),
                    (
                        HotkeyAction::EmergencyStop,
                        "Экстренная остановка (Panic)",
                        "Немедленно глушит все процессы кликера",
                        self.panic_key_name.clone(),
                        Color32::from_rgb(248, 81, 73),
                    ),
                ];

                for (action, title, desc, key_name, badge_color) in rows {
                    let is_capturing = self.capturing_hotkey == Some(action);

                    ui.horizontal(|ui| {
                        egui::Frame::none()
                            .fill(if is_capturing {
                                Color32::from_rgb(210, 153, 34)
                            } else {
                                Color32::from_rgb(33, 38, 45)
                            })
                            .rounding(Rounding::same(6.0))
                            .stroke(if is_capturing {
                                Stroke::new(1.0_f32, Color32::from_rgb(240, 136, 62))
                            } else {
                                Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61))
                            })
                            .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                            .show(ui, |ui| {
                                let label_text = if is_capturing {
                                    "Нажмите клавишу...".to_string()
                                } else {
                                    key_name
                                };
                                ui.label(
                                    RichText::new(label_text)
                                        .size(13.0)
                                        .strong()
                                        .color(if is_capturing { Color32::BLACK } else { badge_color }),
                                );
                            });

                        let btn_label = if is_capturing { "Отмена" } else { "Изменить" };
                        let rebind_btn = egui::Button::new(RichText::new(btn_label).size(11.5))
                            .min_size(Vec2::new(75.0, 26.0))
                            .rounding(Rounding::same(5.0));

                        if ui.add(rebind_btn).clicked() {
                            if is_capturing {
                                self.capturing_hotkey = None;
                            } else {
                                self.capturing_hotkey = Some(action);
                            }
                        }

                        ui.vertical(|ui| {
                            ui.label(RichText::new(title).size(12.5).strong().color(Color32::from_rgb(240, 246, 252)));
                            ui.label(RichText::new(desc).size(11.0).color(Color32::from_rgb(139, 148, 158)));
                        });
                    });
                    ui.add_space(8.0);
                }
            },
        );
    }

    fn render_window_target_card(&mut self, ui: &mut egui::Ui) {
        primer_box(
            ui,
            "🪟 Фоновый режим: Клики строго в окно игры (Background Game Lock)",
            Some("клики происходят только внутри выбранной игры через Win32 API — мышь свободна, окно может быть перекрыто"),
            |ui| {
                let mut bg_enabled = self.advanced_cfg.window_target.enabled;
                if ui
                    .checkbox(
                        &mut bg_enabled,
                        RichText::new("Включить фоновый режим (клики строго в окно игры, курсор мыши свободен)")
                            .strong()
                            .color(Color32::from_rgb(240, 246, 252)),
                    )
                    .changed()
                {
                    self.advanced_cfg.window_target.enabled = bg_enabled;
                    if bg_enabled && self.open_windows.is_empty() {
                        self.open_windows = WinInput::list_open_windows();
                    }
                }

                ui.add_space(4.0);
                ui.label(
                    RichText::new("💡 Если включено: автокликер отправляет клики напрямую в игру через Win32 API. Вы можете положить поверх любое другое окно (браузер, видео, чат), печатать или скроллить — клики не попадут в другие приложения!")
                        .size(11.5)
                        .color(Color32::from_rgb(139, 148, 158)),
                );

                if self.advanced_cfg.window_target.enabled {
                    ui.add_space(10.0);

                    // Notification banner if any
                    if let Some(ref msg) = self.window_status_msg {
                        egui::Frame::none()
                            .fill(Color32::from_rgb(15, 35, 25))
                            .rounding(Rounding::same(6.0))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(46, 160, 67)))
                            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(msg)
                                            .size(12.0)
                                            .color(Color32::from_rgb(63, 185, 80)),
                                    );
                                });
                            });
                        ui.add_space(8.0);
                    }

                    // Window binding indicator
                    let hwnd = self.advanced_cfg.window_target.hwnd;
                    let is_bound = hwnd != 0 && WinInput::is_window_valid(hwnd);

                    egui::Frame::none()
                        .fill(if is_bound {
                            Color32::from_rgb(13, 27, 42)
                        } else {
                            Color32::from_rgb(38, 26, 12)
                        })
                        .rounding(Rounding::same(6.0))
                        .stroke(Stroke::new(
                            1.0_f32,
                            if is_bound {
                                Color32::from_rgb(31, 111, 235)
                            } else {
                                Color32::from_rgb(210, 153, 34)
                            },
                        ))
                        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if is_bound {
                                    ui.label(
                                        RichText::new("🟢 Привязано к:")
                                            .size(13.0)
                                            .strong()
                                            .color(Color32::from_rgb(88, 166, 255)),
                                    );
                                    ui.label(
                                        RichText::new(&self.advanced_cfg.window_target.title)
                                            .size(13.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    );
                                    if let Some((_, _, w, h)) = WinInput::get_window_client_rect(hwnd) {
                                        ui.label(
                                            RichText::new(format!("({}×{})", w, h))
                                                .size(12.0)
                                                .color(Color32::from_rgb(139, 148, 158)),
                                        );
                                    }
                                } else {
                                    ui.label(
                                        RichText::new("⚠️ Игра не выбрана!")
                                            .size(13.0)
                                            .strong()
                                            .color(Color32::from_rgb(227, 179, 65)),
                                    );
                                    ui.label(
                                        RichText::new("Нажмите «Захватить окно» или выберите из списка запущенных окон ниже.")
                                            .size(12.0)
                                            .color(Color32::from_rgb(201, 209, 217)),
                                    );
                                }
                            });
                        });

                    ui.add_space(10.0);

                    // Action Buttons
                    ui.horizontal(|ui| {
                        let pick_btn_text = if self.picking_window_target {
                            format!("⏳ Наведите мышь на окно игры и нажмите [{}]", self.pick_key_name)
                        } else {
                            format!("🎯 Захватить окно игры [{}]", self.pick_key_name)
                        };

                        let pick_btn = egui::Button::new(
                            RichText::new(pick_btn_text)
                                .size(12.5)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .min_size(Vec2::new(0.0, 32.0))
                        .fill(if self.picking_window_target {
                            Color32::from_rgb(187, 128, 9)
                        } else {
                            Color32::from_rgb(35, 134, 54)
                        })
                        .rounding(Rounding::same(6.0));

                        if ui.add(pick_btn).clicked() {
                            if self.picking_window_target {
                                self.picking_window_target = false;
                            } else {
                                self.picking_window_target = true;
                                self.window_status_msg = Some(format!(
                                    "Наведите курсор на окно вашей игры и нажмите клавишу [{}]",
                                    self.pick_key_name
                                ));
                            }
                        }

                        ui.add_space(6.0);

                        let list_btn = egui::Button::new(
                            RichText::new(if self.show_window_picker {
                                "✕ Скрыть список"
                            } else {
                                "📋 Выбрать из списка окон"
                            })
                            .size(12.0)
                            .color(Color32::from_rgb(201, 209, 217)),
                        )
                        .min_size(Vec2::new(0.0, 32.0))
                        .fill(Color32::from_rgb(33, 38, 45))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61)))
                        .rounding(Rounding::same(6.0));

                        if ui.add(list_btn).clicked() {
                            self.show_window_picker = !self.show_window_picker;
                            if self.show_window_picker {
                                self.open_windows = WinInput::list_open_windows();
                            }
                        }

                        if is_bound {
                            ui.add_space(6.0);
                            if ui
                                .button(
                                    RichText::new("Сбросить")
                                        .size(11.5)
                                        .color(Color32::from_rgb(248, 81, 73)),
                                )
                                .clicked()
                            {
                                self.advanced_cfg.window_target.hwnd = 0;
                                self.advanced_cfg.window_target.title.clear();
                                self.window_status_msg = Some("Привязка к окну сброшена.".to_string());
                            }
                        }
                    });

                    // Dropdown list of running windows
                    if self.show_window_picker {
                        ui.add_space(8.0);
                        egui::Frame::none()
                            .fill(Color32::from_rgb(22, 27, 34))
                            .rounding(Rounding::same(6.0))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61)))
                            .inner_margin(10.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("Поиск:").size(12.0).color(Color32::from_rgb(139, 148, 158)),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.window_search_query)
                                            .hint_text("Название игры или программы...")
                                            .desired_width(220.0),
                                    );
                                    if ui.button("🔄 Обновить").clicked() {
                                        self.open_windows = WinInput::list_open_windows();
                                    }
                                });

                                ui.add_space(6.0);
                                let query = self.window_search_query.to_lowercase();
                                let filtered: Vec<_> = self
                                    .open_windows
                                    .iter()
                                    .filter(|w| query.is_empty() || w.title.to_lowercase().contains(&query))
                                    .cloned()
                                    .collect();

                                if filtered.is_empty() {
                                    ui.label(
                                        RichText::new("Окна не найдены. Убедитесь, что игра запущена и открыта на панели задач.")
                                            .size(11.5)
                                            .color(Color32::from_rgb(139, 148, 158)),
                                    );
                                } else {
                                    egui::ScrollArea::vertical()
                                        .max_height(170.0)
                                        .show(ui, |ui| {
                                            for win in filtered {
                                                ui.horizontal(|ui| {
                                                    let is_current =
                                                        win.hwnd == self.advanced_cfg.window_target.hwnd;
                                                    let title_display = if win.title.len() > 46 {
                                                        format!("{}...", &win.title[..43])
                                                    } else {
                                                        win.title.clone()
                                                    };

                                                    let pick_btn = ui.button(
                                                        RichText::new(if is_current {
                                                            "✓ Выбрано"
                                                        } else {
                                                            "Выбрать"
                                                        })
                                                        .size(11.5)
                                                        .color(if is_current {
                                                            Color32::from_rgb(63, 185, 80)
                                                        } else {
                                                            Color32::from_rgb(88, 166, 255)
                                                        }),
                                                    );

                                                    if pick_btn.clicked() {
                                                        self.advanced_cfg.window_target.hwnd = win.hwnd;
                                                        self.advanced_cfg.window_target.title = win.title.clone();
                                                        self.advanced_cfg.window_target.enabled = true;
                                                        self.show_window_picker = false;
                                                        self.window_status_msg = Some(format!(
                                                            "✓ Выбрано окно: «{}» ({}×{})",
                                                            win.title, win.width, win.height
                                                        ));
                                                    }

                                                    ui.label(
                                                        RichText::new(title_display)
                                                            .size(12.0)
                                                            .strong()
                                                            .color(if is_current {
                                                                Color32::WHITE
                                                            } else {
                                                                Color32::from_rgb(201, 209, 217)
                                                            }),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!("({}×{})", win.width, win.height))
                                                            .size(11.0)
                                                            .color(Color32::from_rgb(139, 148, 158)),
                                                    );
                                                });
                                                ui.separator();
                                            }
                                        });
                                }
                            });
                    }

                    // Click point inside the window
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Точка клика в окне:")
                                .size(12.5)
                                .color(Color32::from_rgb(201, 209, 217)),
                        );
                        ui.radio_value(
                            &mut self.advanced_cfg.window_target.click_center,
                            true,
                            "🎯 В центр окна игры",
                        );
                        ui.radio_value(
                            &mut self.advanced_cfg.window_target.click_center,
                            false,
                            "📍 В точные координаты окна",
                        );
                    });

                    if !self.advanced_cfg.window_target.click_center {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("X (от левого края окна):").size(11.5));
                            ui.add(
                                egui::DragValue::new(&mut self.advanced_cfg.window_target.custom_x)
                                    .speed(1)
                                    .range(0..=4000)
                                    .suffix(" px"),
                            );
                            ui.label(RichText::new("Y (от верхнего края окна):").size(11.5));
                            ui.add(
                                egui::DragValue::new(&mut self.advanced_cfg.window_target.custom_y)
                                    .speed(1)
                                    .range(0..=4000)
                                    .suffix(" px"),
                            );

                            ui.add_space(8.0);
                            let pt_text = if self.picking_window_point {
                                format!("⏳ Наведите в игре и нажмите [{}]", self.pick_key_name)
                            } else {
                                format!("📍 Указать точку в игре [{}]", self.pick_key_name)
                            };

                            let pt_btn = egui::Button::new(
                                RichText::new(pt_text)
                                    .size(11.5)
                                    .color(Color32::from_rgb(88, 166, 255)),
                            )
                            .fill(Color32::from_rgb(33, 38, 45))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61)))
                            .rounding(Rounding::same(5.0));

                            if ui.add(pt_btn).clicked() {
                                if self.picking_window_point {
                                    self.picking_window_point = false;
                                } else {
                                    self.picking_window_point = true;
                                    self.window_status_msg = Some(format!(
                                        "Наведите курсор на желаемую точку внутри игры и нажмите [{}]",
                                        self.pick_key_name
                                    ));
                                }
                            }
                        });
                    }
                }
            },
        );
        ui.add_space(14.0);
    }

    fn render_mouse_tab(&mut self, ui: &mut egui::Ui) {
        // Фоновый режим: привязка кликов к игре
        self.render_window_target_card(ui);

        // Box 1: Кнопка и действие
        primer_box(
            ui,
            "Кнопка мыши и тип клика",
            Some("выберите клавишу и вариант срабатывания"),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Кнопка:").size(12.5));
                    let btn_options = [
                        (MouseButton::Left, "Левая (LMB)"),
                        (MouseButton::Right, "Правая (RMB)"),
                        (MouseButton::Middle, "Колесико"),
                        (MouseButton::X1, "X1"),
                        (MouseButton::X2, "X2"),
                    ];
                    primer_segmented_control(ui, &mut self.mouse_cfg.button, &btn_options);
                });

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Действие:").size(12.5));
                    let click_options = [
                        (ClickType::Single, "Одиночный"),
                        (ClickType::Double, "Двойной"),
                        (ClickType::Triple, "Тройной"),
                        (ClickType::Hold, "Удержание (Hold)"),
                    ];
                    primer_segmented_control(ui, &mut self.mouse_cfg.click_type, &click_options);
                });

                if self.mouse_cfg.click_type == ClickType::Hold {
                    ui.add_space(12.0);
                    egui::Frame::none()
                        .fill(Color32::from_rgb(13, 17, 23))
                        .rounding(Rounding::same(6.0))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(31, 111, 235)))
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Время зажатия кнопки:")
                                        .size(12.5)
                                        .color(Color32::from_rgb(88, 166, 255)),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut self.mouse_cfg.hold_duration_ms)
                                        .speed(5.0)
                                        .range(1.0..=100000.0)
                                        .suffix(" мс"),
                                );
                                ui.label(
                                    RichText::new("(кнопка будет непрерывно удерживаться)")
                                        .size(11.0)
                                        .color(Color32::from_rgb(139, 148, 158)),
                                );
                            });
                        });
                }
            },
        );

        // Box 2: Скорость
        primer_box(
            ui,
            "Частота и интервал",
            Some("настройка скорости кликов и рандомизации"),
            |ui| {
                ui.horizontal(|ui| {
                    let mode_options = [
                        (true, "Режим CPS (кликов/сек)"),
                        (false, "Пауза между кликами (мс)"),
                    ];
                    primer_segmented_control(ui, &mut self.mouse_cfg.use_cps, &mode_options);
                });

                ui.add_space(12.0);

                if self.mouse_cfg.use_cps {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Скорость:").size(12.5));
                        ui.add(
                            egui::Slider::new(&mut self.mouse_cfg.cps, 1.0..=1000.0)
                                .text("CPS")
                                .logarithmic(true),
                        );
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Часто используемые:")
                                .size(11.5)
                                .color(Color32::from_rgb(139, 148, 158)),
                        );
                        let presets = [
                            ("10 CPS", 10.0),
                            ("16 CPS", 16.0),
                            ("25 CPS", 25.0),
                            ("50 CPS", 50.0),
                            ("100 CPS", 100.0),
                            ("1000 MAX", 1000.0),
                        ];
                        for (title, val) in presets {
                            if primer_topic_badge(ui, title) {
                                self.mouse_cfg.cps = val;
                            }
                        }
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Интервал:").size(12.5));
                        ui.add(
                            egui::DragValue::new(&mut self.mouse_cfg.custom_interval_ms)
                                .speed(1.0)
                                .range(0.1..=60000.0)
                                .suffix(" мс"),
                        );
                    });
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Разброс задержки (Jitter):").size(12.5));
                    ui.add(
                        egui::DragValue::new(&mut self.mouse_cfg.jitter_ms)
                            .speed(0.5)
                            .range(0.0..=500.0)
                            .suffix(" ±мс"),
                    );
                    ui.label(
                        RichText::new("(защита от обнаружения ботов)")
                            .size(11.0)
                            .color(Color32::from_rgb(139, 148, 158)),
                    );
                });

                ui.add_space(8.0);
                let ms_per_click = if self.mouse_cfg.use_cps {
                    1000.0 / self.mouse_cfg.cps.max(0.1)
                } else {
                    self.mouse_cfg.custom_interval_ms
                };
                let estimated_cps = if self.mouse_cfg.use_cps {
                    self.mouse_cfg.cps
                } else {
                    1000.0 / self.mouse_cfg.custom_interval_ms.max(0.1)
                };

                ui.label(
                    RichText::new(format!(
                        "Расчет: 1 клик каждые {:.1} мс  •  {:.1} кликов в секунду",
                        ms_per_click, estimated_cps
                    ))
                    .size(11.0)
                    .color(Color32::from_rgb(110, 118, 129)),
                );
            },
        );

        // Box 3: Позиция
        primer_box(
            ui,
            "Позиционирование курсора",
            Some("точка на экране, куда отправлять клики"),
            |ui| {
                let pos_options = [
                    (false, "Текущее положение курсора"),
                    (true, "Фиксированные координаты (X, Y)"),
                ];
                primer_segmented_control(ui, &mut self.mouse_cfg.fixed_pos, &pos_options);

                if self.mouse_cfg.fixed_pos {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("X:").size(12.5));
                        ui.add(egui::DragValue::new(&mut self.mouse_cfg.target_x).speed(1));

                        ui.add_space(10.0);
                        ui.label(RichText::new("Y:").size(12.5));
                        ui.add(egui::DragValue::new(&mut self.mouse_cfg.target_y).speed(1));

                        ui.add_space(14.0);
                        let pick_btn = egui::Button::new(format!("Захватить курсор [{}]", self.pick_key_name))
                            .fill(Color32::from_rgb(31, 111, 235))
                            .rounding(Rounding::same(6.0));

                        if ui.add(pick_btn).clicked() {
                            self.mouse_cfg.target_x = self.cursor_x;
                            self.mouse_cfg.target_y = self.cursor_y;
                        }
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Разброс позиции (Jitter):").size(12.5));
                        ui.add(
                            egui::DragValue::new(&mut self.mouse_cfg.jitter_px)
                                .speed(1)
                                .range(0..=100)
                                .suffix(" ±px"),
                        );
                    });
                }
            },
        );

        // Box 4: Остановка
        primer_box(
            ui,
            "Условие завершения",
            Some("автоматическая остановка по числу кликов"),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Остановиться после:").size(12.5));
                    ui.add(
                        egui::DragValue::new(&mut self.mouse_cfg.stop_after_count)
                            .speed(10)
                            .range(0..=1000000),
                    );
                    ui.label(
                        RichText::new("кликов (0 = без ограничений)")
                            .size(11.5)
                            .color(Color32::from_rgb(139, 148, 158)),
                    );
                });
            },
        );
    }

    fn render_keyboard_tab(&mut self, ui: &mut egui::Ui) {
        primer_box(
            ui,
            "Клавиша и режим работы",
            Some("автоматический спам или удержание клавиши"),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Выбранная клавиша:").size(12.5));

                    let key_text = if self.capturing_key {
                        RichText::new("Нажмите клавишу на клавиатуре...")
                            .size(12.5)
                            .strong()
                            .color(Color32::from_rgb(240, 136, 62))
                    } else {
                        RichText::new(format!("[  {}  ]", self.kb_cfg.key_name))
                            .size(13.5)
                            .strong()
                            .color(Color32::WHITE)
                    };

                    let pick_key_btn = egui::Button::new(key_text)
                        .min_size(Vec2::new(140.0, 32.0))
                        .fill(if self.capturing_key {
                            Color32::from_rgb(158, 66, 17)
                        } else {
                            Color32::from_rgb(33, 38, 45)
                        })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if self.capturing_key {
                                Color32::from_rgb(240, 136, 62)
                            } else {
                                Color32::from_rgb(48, 54, 61)
                            },
                        ))
                        .rounding(Rounding::same(6.0));

                    if ui.add(pick_key_btn).clicked() {
                        self.capturing_key = !self.capturing_key;
                    }

                    if !self.capturing_key {
                        let change_btn = egui::Button::new("Изменить")
                            .fill(Color32::from_rgb(31, 111, 235))
                            .rounding(Rounding::same(6.0));
                        if ui.add(change_btn).clicked() {
                            self.capturing_key = true;
                        }
                    }
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Быстрый выбор:")
                            .size(11.5)
                            .color(Color32::from_rgb(139, 148, 158)),
                    );
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

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Режим:").size(12.5));
                    let key_modes = [
                        (KeyMode::Tap, "Тап (нажал-отпустил)"),
                        (KeyMode::Hold, "Удержание клавиши (Hold)"),
                    ];
                    primer_segmented_control(ui, &mut self.kb_cfg.mode, &key_modes);
                });

                if self.kb_cfg.mode == KeyMode::Hold {
                    ui.add_space(12.0);
                    egui::Frame::none()
                        .fill(Color32::from_rgb(13, 17, 23))
                        .rounding(Rounding::same(6.0))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(31, 111, 235)))
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Время удержания клавиши:")
                                        .size(12.5)
                                        .color(Color32::from_rgb(88, 166, 255)),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut self.kb_cfg.hold_ms)
                                        .speed(5.0)
                                        .range(1.0..=60000.0)
                                        .suffix(" мс"),
                                );
                            });
                        });
                }

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Интервал:").size(12.5));
                    ui.add(
                        egui::DragValue::new(&mut self.kb_cfg.interval_ms)
                            .speed(2.0)
                            .range(1.0..=60000.0)
                            .suffix(" мс"),
                    );

                    ui.add_space(14.0);
                    ui.label(RichText::new("Разброс:").size(12.5));
                    ui.add(
                        egui::DragValue::new(&mut self.kb_cfg.jitter_ms)
                            .speed(1.0)
                            .range(0.0..=500.0)
                            .suffix(" ±мс"),
                    );
                });
            },
        );
    }

    fn render_scroll_tab(&mut self, ui: &mut egui::Ui) {
        primer_box(
            ui,
            "Авто-скроллинг колесиком",
            Some("автоматическая прокрутка мыши в любом направлении"),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Направление:").size(12.5));
                    let directions = [
                        (ScrollDirection::Down, "Вниз"),
                        (ScrollDirection::Up, "Вверх"),
                        (ScrollDirection::Right, "Вправо"),
                        (ScrollDirection::Left, "Влево"),
                    ];
                    primer_segmented_control(ui, &mut self.scroll_cfg.direction, &directions);
                });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Шагов за раз (тиков):").size(12.5));
                    ui.add(
                        egui::DragValue::new(&mut self.scroll_cfg.ticks)
                            .speed(1)
                            .range(1..=50),
                    );

                    ui.add_space(14.0);
                    ui.label(RichText::new("Интервал:").size(12.5));
                    ui.add(
                        egui::DragValue::new(&mut self.scroll_cfg.interval_ms)
                            .speed(5.0)
                            .range(5.0..=10000.0)
                            .suffix(" мс"),
                    );
                });
            },
        );
    }

    fn render_points_tab(&mut self, ui: &mut egui::Ui) {
        primer_box(
            ui,
            "Добавление точек маршрута",
            Some("клик по заранее заданным координатам"),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("X:").size(12.5));
                    ui.add(egui::DragValue::new(&mut self.new_pt_x).speed(1));

                    ui.add_space(8.0);
                    ui.label(RichText::new("Y:").size(12.5));
                    ui.add(egui::DragValue::new(&mut self.new_pt_y).speed(1));

                    ui.add_space(12.0);
                    let cur_btn = egui::Button::new(format!("Взять текущий курсор [{}]", self.pick_key_name))
                        .fill(Color32::from_rgb(31, 111, 235))
                        .rounding(Rounding::same(6.0));
                    if ui.add(cur_btn).clicked() {
                        self.new_pt_x = self.cursor_x;
                        self.new_pt_y = self.cursor_y;
                    }
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Пауза после:").size(12.5));
                    ui.add(
                        egui::DragValue::new(&mut self.new_pt_delay)
                            .speed(5.0)
                            .range(10.0..=60000.0)
                            .suffix(" мс"),
                    );

                    ui.add_space(12.0);
                    ui.label(RichText::new("Удержание:").size(12.5));
                    ui.add(
                        egui::DragValue::new(&mut self.new_pt_hold)
                            .speed(5.0)
                            .range(0.0..=10000.0)
                            .suffix(" мс"),
                    );
                });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let add_btn = egui::Button::new(
                        RichText::new("Добавить точку")
                            .size(12.5)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .min_size(Vec2::new(0.0, 32.0))
                    .fill(Color32::from_rgb(35, 134, 54))
                    .rounding(Rounding::same(6.0));

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
                        ui.add_space(10.0);
                        let clear_btn = egui::Button::new("Очистить все")
                            .min_size(Vec2::new(0.0, 32.0))
                            .fill(Color32::from_rgb(218, 54, 51))
                            .rounding(Rounding::same(6.0));

                        if ui.add(clear_btn).clicked() {
                            self.points.clear();
                        }
                    }
                });
            },
        );

        primer_box(
            ui,
            "Список точек маршрута",
            Some(format!("всего точек: {}", self.points.len()).as_str()),
            |ui| {
                if self.points.is_empty() {
                    ui.label(
                        RichText::new(format!(
                            "Точек нет. Наведите курсор на нужную позицию и нажмите [{}].",
                            self.pick_key_name
                        ))
                        .color(Color32::from_rgb(139, 148, 158)),
                    );
                } else {
                    let mut remove_idx = None;
                    for (i, pt) in self.points.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{}.", i + 1))
                                    .strong()
                                    .color(Color32::from_rgb(88, 166, 255)),
                            );
                            ui.label(format!("(X: {}, Y: {})", pt.x, pt.y));
                            ui.label(format!("пауза: {} мс", pt.delay_after_ms));
                            if pt.hold_ms > 0.0 {
                                ui.label(
                                    RichText::new(format!("удержание: {} мс", pt.hold_ms))
                                        .color(Color32::from_rgb(63, 185, 80)),
                                );
                            }
                            if ui.button("Удалить").clicked() {
                                remove_idx = Some(i);
                            }
                        });
                        ui.add_space(4.0);
                    }
                    if let Some(i) = remove_idx {
                        self.points.remove(i);
                    }
                }
            },
        );
    }

    fn render_vision_tab(&mut self, ui: &mut egui::Ui) {
        // Mode Switcher Header
        ui.horizontal(|ui| {
            let is_auto = self.vision_cfg.mode == VisionMode::AutoPopup;

            let auto_btn = egui::Button::new(
                RichText::new("⚡ Авто-охотник (Pop-up Hunter)")
                    .size(13.0)
                    .strong()
                    .color(if is_auto { Color32::WHITE } else { Color32::from_rgb(139, 148, 158) }),
            )
            .min_size(Vec2::new(230.0, 36.0))
            .rounding(Rounding::same(6.0))
            .fill(if is_auto { Color32::from_rgb(31, 111, 235) } else { Color32::from_rgb(22, 27, 34) })
            .stroke(Stroke::new(1.0_f32, if is_auto { Color32::from_rgb(88, 166, 255) } else { Color32::from_rgb(48, 54, 61) }));

            if ui.add(auto_btn).clicked() {
                self.vision_cfg.mode = VisionMode::AutoPopup;
            }

            let is_tmpl = self.vision_cfg.mode == VisionMode::TemplateMatch;
            let tmpl_btn = egui::Button::new(
                RichText::new("🖼️ По образцу (Шаблоны кнопок)")
                    .size(13.0)
                    .strong()
                    .color(if is_tmpl { Color32::WHITE } else { Color32::from_rgb(139, 148, 158) }),
            )
            .min_size(Vec2::new(230.0, 36.0))
            .rounding(Rounding::same(6.0))
            .fill(if is_tmpl { Color32::from_rgb(31, 111, 235) } else { Color32::from_rgb(22, 27, 34) })
            .stroke(Stroke::new(1.0_f32, if is_tmpl { Color32::from_rgb(88, 166, 255) } else { Color32::from_rgb(48, 54, 61) }));

            if ui.add(tmpl_btn).clicked() {
                self.vision_cfg.mode = VisionMode::TemplateMatch;
            }
        });

        if self.advanced_cfg.window_target.enabled && self.advanced_cfg.window_target.hwnd != 0 {
            ui.add_space(8.0);
            egui::Frame::none()
                .fill(Color32::from_rgb(13, 27, 42))
                .rounding(Rounding::same(6.0))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(31, 111, 235)))
                .inner_margin(egui::Margin::symmetric(14.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("🪟 Фоновый режим активен:")
                                .size(12.0)
                                .strong()
                                .color(Color32::from_rgb(88, 166, 255)),
                        );
                        ui.label(
                            RichText::new(format!(
                                "«{}» — поиск и клики изолированы внутри окна игры, мышь свободна!",
                                self.advanced_cfg.window_target.title
                            ))
                            .size(12.0)
                            .color(Color32::WHITE),
                        );
                    });
                });
        }

        ui.add_space(12.0);

        if self.vision_cfg.mode == VisionMode::AutoPopup {
            self.render_auto_popup_view(ui);
        } else {
            self.render_template_match_view(ui);
        }
    }

    fn render_auto_popup_view(&mut self, ui: &mut egui::Ui) {
        // Hero info card
        egui::Frame::none()
            .fill(Color32::from_rgb(15, 23, 42))
            .rounding(Rounding::same(8.0))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(56, 139, 253)))
            .inner_margin(egui::Margin::symmetric(16.0, 12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("⚡ Авто-охотник за всплывающими кнопками")
                            .size(14.0)
                            .strong()
                            .color(Color32::from_rgb(88, 166, 255)),
                    );
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Кликер сам непрерывно сравнивает кадры экрана и МГНОВЕННО кликает по любому новому объекту или кнопке, которая выскочила в случайном месте экрана. Вам вообще не нужно вырезать картинки или создавать шаблоны!")
                        .size(12.0)
                        .color(Color32::from_rgb(201, 209, 217)),
                );
            });
        ui.add_space(14.0);

        // Status alert message
        if let Some(ref msg) = self.vision_status_msg {
            egui::Frame::none()
                .fill(Color32::from_rgb(15, 35, 25))
                .rounding(Rounding::same(6.0))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(46, 160, 67)))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(msg)
                                .size(12.0)
                                .color(Color32::from_rgb(63, 185, 80)),
                        );
                    });
                });
            ui.add_space(10.0);
        }

        // Box 1: Тип объекта и фильтрация
        primer_box(
            ui,
            "🎯 Тип всплывающего объекта и цвет",
            Some("выберите, на что именно должен реагировать авто-охотник"),
            |ui| {
                // Quick 1-click Preset for Roblox
                egui::Frame::none()
                    .fill(Color32::from_rgb(35, 18, 55))
                    .rounding(Rounding::same(6.0))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(163, 113, 247)))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let roblox_btn = egui::Button::new(
                                RichText::new("🟣 1-Клик Настройка для Roblox (Фиолетовый кружок x2 / Тап)")
                                    .size(12.5)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(110, 45, 175))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(180, 110, 255)))
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(0.0, 30.0));

                            if ui.add(roblox_btn).clicked() {
                                self.apply_preset("roblox_circle");
                            }

                            ui.label(
                                RichText::new("— мгновенная настройка цвета, размера и задержки 45мс для Roblox!")
                                    .size(11.5)
                                    .color(Color32::from_rgb(210, 185, 255)),
                            );
                        });
                    });

                ui.add_space(10.0);
                ui.label(RichText::new("Цветовой фильтр всплывашек:").size(12.5).strong().color(Color32::from_rgb(240, 246, 252)));
                ui.add_space(6.0);

                ui.horizontal_wrapped(|ui| {
                    let filters = [
                        (PopupColorFilter::PurpleViolet, "🟣 Фиолетовый кружок (Roblox x2 / Тап)"),
                        (PopupColorFilter::WhitePointerInside, "👆 Белая рука-указатель / Тап"),
                        (PopupColorFilter::AnyVibrant, "🌈 Любой яркий кружок / кнопка"),
                        (PopupColorFilter::GoldYellow, "🟡 Золотые / жёлтые монетки"),
                        (PopupColorFilter::Green, "🟢 Зелёные кнопки"),
                        (PopupColorFilter::Red, "🔴 Красные кнопки"),
                        (PopupColorFilter::CyanBlue, "🔵 Синие / голубые кнопки"),
                        (PopupColorFilter::MotionDiff, "⚡ По движению / разнице кадров"),
                    ];

                    for (filter, label) in filters {
                        let is_active = self.vision_cfg.popup_filter == filter;
                        let btn = egui::Button::new(
                            RichText::new(label)
                                .size(12.0)
                                .color(if is_active { Color32::WHITE } else { Color32::from_rgb(139, 148, 158) }),
                        )
                        .min_size(Vec2::new(0.0, 28.0))
                        .rounding(Rounding::same(6.0))
                        .fill(if is_active { Color32::from_rgb(31, 111, 235) } else { Color32::from_rgb(33, 38, 45) })
                        .stroke(Stroke::new(1.0_f32, if is_active { Color32::from_rgb(88, 166, 255) } else { Color32::from_rgb(48, 54, 61) }));

                        if ui.add(btn).clicked() {
                            self.vision_cfg.popup_filter = filter;
                        }
                    }
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(10.0);

                // Sensitivity slider
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Чувствительность к изменениям:").size(12.5).color(Color32::from_rgb(201, 209, 217)));
                    let mut sens = self.vision_cfg.popup_sensitivity as i32;
                    if ui.add(egui::Slider::new(&mut sens, 15..=90)).changed() {
                        self.vision_cfg.popup_sensitivity = sens as u32;
                    }
                    ui.label(
                        RichText::new("(меньше = острее реакция; 40–50 идеально для игр)")
                            .size(11.0)
                            .color(Color32::from_rgb(139, 148, 158)),
                    );
                });

                ui.add_space(10.0);

                // Size bounds (noise filter)
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Размер искомой кнопки:").size(12.5).color(Color32::from_rgb(201, 209, 217)));
                    ui.label("от:");
                    ui.add(egui::DragValue::new(&mut self.vision_cfg.popup_min_size).speed(1).range(10..=150).suffix(" px"));
                    ui.label("до:");
                    ui.add(egui::DragValue::new(&mut self.vision_cfg.popup_max_size).speed(2).range(50..=600).suffix(" px"));

                    ui.add_space(8.0);
                    if ui.button("Монеты (15-90px)").clicked() {
                        self.vision_cfg.popup_min_size = 15;
                        self.vision_cfg.popup_max_size = 90;
                    }
                    if ui.button("Кнопки (30-220px)").clicked() {
                        self.vision_cfg.popup_min_size = 30;
                        self.vision_cfg.popup_max_size = 220;
                    }
                    if ui.button("Плашки (60-450px)").clicked() {
                        self.vision_cfg.popup_min_size = 60;
                        self.vision_cfg.popup_max_size = 450;
                    }
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(10.0);

                // Click Action
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Действие при клике:").size(12.5).color(Color32::from_rgb(201, 209, 217)));

                    let btn_label = match self.vision_cfg.popup_click_button {
                        MouseButton::Left => "ЛКМ",
                        MouseButton::Right => "ПКМ",
                        MouseButton::Middle => "СКМ",
                        _ => "ЛКМ",
                    };
                    if ui.button(btn_label).clicked() {
                        self.vision_cfg.popup_click_button = match self.vision_cfg.popup_click_button {
                            MouseButton::Left => MouseButton::Right,
                            MouseButton::Right => MouseButton::Middle,
                            MouseButton::Middle => MouseButton::Left,
                            _ => MouseButton::Left,
                        };
                    }

                    let type_label = match self.vision_cfg.popup_click_type {
                        ClickType::Single => "Одиночный клик",
                        ClickType::Double => "Двойной клик",
                        ClickType::Triple => "Тройной клик",
                        ClickType::Hold => "Зажатие",
                    };
                    if ui.button(type_label).clicked() {
                        self.vision_cfg.popup_click_type = match self.vision_cfg.popup_click_type {
                            ClickType::Single => ClickType::Double,
                            ClickType::Double => ClickType::Hold,
                            ClickType::Hold => ClickType::Single,
                            _ => ClickType::Single,
                        };
                    }

                    if self.vision_cfg.popup_click_type == ClickType::Hold {
                        ui.label("Время зажатия:");
                        ui.add(egui::DragValue::new(&mut self.vision_cfg.popup_hold_ms).speed(5.0).range(10.0..=2000.0).suffix(" мс"));
                    }
                });
            },
        );

        // Box 2: Скорость и поведение мыши
        primer_box(
            ui,
            "⚙️ Скорость сканирования и поведение мыши",
            Some("настройка частоты проверок и возврата курсора"),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚡ Частота сканирования:").size(12.5).color(Color32::from_rgb(201, 209, 217)));
                    ui.add(
                        egui::Slider::new(&mut self.vision_cfg.scan_interval_ms, 15.0..=200.0)
                            .step_by(5.0)
                            .suffix(" мс"),
                    );
                    ui.label(
                        RichText::new("(30 мс = ~33 кадра в секунду)")
                            .size(11.0)
                            .color(Color32::from_rgb(139, 148, 158)),
                    );
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⏱️ Cooldown (пауза после клика):").size(12.5).color(Color32::from_rgb(201, 209, 217)));
                    ui.add(
                        egui::Slider::new(&mut self.vision_cfg.cooldown_ms, 50.0..=1000.0)
                            .step_by(10.0)
                            .suffix(" мс"),
                    );
                    ui.label(
                        RichText::new("(защита от повторных кликов по исчезающей кнопке)")
                            .size(11.0)
                            .color(Color32::from_rgb(139, 148, 158)),
                    );
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.vision_cfg.return_cursor, "");
                    ui.label(
                        RichText::new("↩️ Возвращать курсор на исходное место после клика")
                            .size(12.5)
                            .strong()
                            .color(Color32::from_rgb(240, 246, 252)),
                    );
                    ui.label(
                        RichText::new("(не сбивает ваш прицел и обзор в игре!)")
                            .size(11.0)
                            .color(Color32::from_rgb(139, 148, 158)),
                    );
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🎲 Разброс клика (Jitter):").size(12.5).color(Color32::from_rgb(201, 209, 217)));
                    ui.add(
                        egui::Slider::new(&mut self.vision_cfg.click_jitter_px, 0..=10)
                            .suffix(" px"),
                    );
                    ui.label(
                        RichText::new("(микро-смещение от точного центра)")
                            .size(11.0)
                            .color(Color32::from_rgb(139, 148, 158)),
                    );
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.vision_cfg.use_custom_region, "");
                    ui.label(
                        RichText::new("🔲 Ограничить зону поиска (сканировать только часть экрана/окно игры)")
                            .size(12.5)
                            .color(Color32::from_rgb(201, 209, 217)),
                    );
                });

                if self.vision_cfg.use_custom_region {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Текущая зона: ({}, {}) — ({}, {})",
                                self.vision_cfg.region_x1,
                                self.vision_cfg.region_y1,
                                self.vision_cfg.region_x2,
                                self.vision_cfg.region_y2
                            ))
                            .size(11.5)
                            .color(Color32::from_rgb(88, 166, 255)),
                        );

                        ui.add_space(10.0);
                        let is_reg_snip = self.vision_snip_mode == VisionSnipMode::RegionPoint;
                        let reg_btn = egui::Button::new(if is_reg_snip {
                            format!("⏳ Задать углы зоны [{}]...", self.pick_key_name)
                        } else {
                            format!("📐 Задать зону по 2 точкам [{}]", self.pick_key_name)
                        })
                        .fill(Color32::from_rgb(33, 38, 45))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61)))
                        .rounding(Rounding::same(5.0));

                        if ui.add(reg_btn).clicked() {
                            if is_reg_snip {
                                self.vision_snip_mode = VisionSnipMode::None;
                                self.snip_p1 = None;
                            } else {
                                self.vision_snip_mode = VisionSnipMode::RegionPoint;
                                self.snip_p1 = None;
                                self.vision_status_msg = Some(format!(
                                    "Наведите курсор на угол 1 зоны поиска и нажмите [{}]",
                                    self.pick_key_name
                                ));
                            }
                        }

                        if ui.button("Сбросить на весь экран").clicked() {
                            self.vision_cfg.use_custom_region = false;
                        }
                    });
                }
            },
        );
    }

    fn render_template_match_view(&mut self, ui: &mut egui::Ui) {
        // Snipping Banner if in snip mode
        if self.vision_snip_mode != VisionSnipMode::None {
            egui::Frame::none()
                .fill(Color32::from_rgb(38, 26, 12))
                .rounding(Rounding::same(6.0))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(210, 153, 34)))
                .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let text = match self.vision_snip_mode {
                            VisionSnipMode::CursorBox => format!(
                                "🎯 РЕЖИМ ЗАХВАТА: Наведите мышь на центр кнопки в игре и нажмите [{}]",
                                self.pick_key_name
                            ),
                            VisionSnipMode::TwoPointRect => {
                                if let Some((x1, y1)) = self.snip_p1 {
                                    format!(
                                        "📐 Точка 1 ({}, {}) зафиксирована! Наведите на второй угол кнопки и нажмите [{}]",
                                        x1, y1, self.pick_key_name
                                    )
                                } else {
                                    format!(
                                        "📐 РЕЖИМ 2 ТОЧЕК: Наведите мышь на угол 1 кнопки и нажмите [{}]",
                                        self.pick_key_name
                                    )
                                }
                            }
                            VisionSnipMode::RegionPoint => {
                                if let Some((x1, y1)) = self.snip_p1 {
                                    format!(
                                        "🔲 Угол 1 зоны ({}, {}) зафиксирован! Наведите на второй угол зоны и нажмите [{}]",
                                        x1, y1, self.pick_key_name
                                    )
                                } else {
                                    format!(
                                        "🔲 ОГРАНИЧЕНИЕ ЗОНЫ: Наведите курсор на угол 1 зоны поиска и нажмите [{}]",
                                        self.pick_key_name
                                    )
                                }
                            }
                            VisionSnipMode::None => String::new(),
                        };

                        ui.label(
                            RichText::new(text)
                                .size(13.0)
                                .strong()
                                .color(Color32::from_rgb(227, 179, 65)),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(
                                    RichText::new("Отмена")
                                        .size(11.5)
                                        .color(Color32::from_rgb(248, 81, 73)),
                                )
                                .clicked()
                            {
                                self.vision_snip_mode = VisionSnipMode::None;
                                self.snip_p1 = None;
                            }
                        });
                    });
                });
            ui.add_space(10.0);
        }

        // Status alert message
        if let Some(ref msg) = self.vision_status_msg {
            egui::Frame::none()
                .fill(Color32::from_rgb(15, 35, 25))
                .rounding(Rounding::same(6.0))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(46, 160, 67)))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(msg)
                                .size(12.0)
                                .color(Color32::from_rgb(63, 185, 80)),
                        );
                    });
                });
            ui.add_space(10.0);
        }

        // Box 1: Захват образца кнопки
        primer_box(
            ui,
            "📸 Захват новой кнопки-цели с экрана",
            Some("добавление изображения кнопки, на которую кликер должен реагировать"),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Размер рамки под курсором:").size(12.5).color(Color32::from_rgb(201, 209, 217)));
                    ui.add_space(6.0);
                    ui.label("Ширина:");
                    ui.add(egui::DragValue::new(&mut self.snip_box_w).speed(1).range(20..=400).suffix(" px"));
                    ui.label("Высота:");
                    ui.add(egui::DragValue::new(&mut self.snip_box_h).speed(1).range(20..=400).suffix(" px"));

                    ui.add_space(10.0);
                    if ui.button("60×60").clicked() {
                        self.snip_box_w = 60;
                        self.snip_box_h = 60;
                    }
                    if ui.button("80×40").clicked() {
                        self.snip_box_w = 80;
                        self.snip_box_h = 40;
                    }
                    if ui.button("100×50").clicked() {
                        self.snip_box_w = 100;
                        self.snip_box_h = 50;
                    }
                });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let is_cursor_snip = self.vision_snip_mode == VisionSnipMode::CursorBox;
                    let btn_text = if is_cursor_snip {
                        format!("⏳ Наведите на кнопку в игре и нажмите [{}]", self.pick_key_name)
                    } else {
                        format!("🎯 Захват под курсором [{}]", self.pick_key_name)
                    };

                    let btn = egui::Button::new(
                        RichText::new(btn_text)
                            .size(12.5)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .min_size(Vec2::new(0.0, 32.0))
                    .fill(if is_cursor_snip {
                        Color32::from_rgb(187, 128, 9)
                    } else {
                        Color32::from_rgb(35, 134, 54)
                    })
                    .rounding(Rounding::same(6.0));

                    if ui.add(btn).clicked() {
                        if is_cursor_snip {
                            self.vision_snip_mode = VisionSnipMode::None;
                        } else {
                            self.vision_snip_mode = VisionSnipMode::CursorBox;
                            self.vision_status_msg = Some(format!(
                                "Наведите курсор на кнопку в игре и нажмите [{}]",
                                self.pick_key_name
                            ));
                        }
                    }

                    ui.add_space(8.0);
                    let is_2pt = self.vision_snip_mode == VisionSnipMode::TwoPointRect;
                    let btn_2pt = egui::Button::new(
                        RichText::new(if is_2pt {
                            "⏳ Выделение рамкой..."
                        } else {
                            "📐 Выделить рамкой (2 точки)"
                        })
                        .size(12.0)
                        .color(Color32::from_rgb(201, 209, 217)),
                    )
                    .min_size(Vec2::new(0.0, 32.0))
                    .fill(Color32::from_rgb(33, 38, 45))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61)))
                    .rounding(Rounding::same(6.0));

                    if ui.add(btn_2pt).clicked() {
                        if is_2pt {
                            self.vision_snip_mode = VisionSnipMode::None;
                            self.snip_p1 = None;
                        } else {
                            self.vision_snip_mode = VisionSnipMode::TwoPointRect;
                            self.snip_p1 = None;
                            self.vision_status_msg = Some(format!(
                                "Наведите на верхний левый угол кнопки и нажмите [{}]",
                                self.pick_key_name
                            ));
                        }
                    }
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Или путь к файлу картинки:").size(12.0).color(Color32::from_rgb(139, 148, 158)));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.file_import_path)
                            .hint_text("C:\\path\\to\\button.png")
                            .desired_width(260.0),
                    );

                    if ui.button("Загрузить").clicked() && !self.file_import_path.trim().is_empty() {
                        let path = self.file_import_path.trim().to_string();
                        let id = rand::random::<u64>();
                        let name = format!("Кнопка #{}", self.vision_cfg.templates.len() + 1);
                        match ButtonTemplate::from_file(id, name, &path, 0.82) {
                            Ok(tmpl) => {
                                self.vision_cfg.templates.push(tmpl);
                                self.file_import_error = None;
                                self.vision_status_msg = Some(format!("✓ Файл {} успешно загружен!", path));
                            }
                            Err(e) => {
                                self.file_import_error = Some(e);
                            }
                        }
                    }
                });

                if let Some(ref err) = self.file_import_error {
                    ui.label(RichText::new(err).size(11.5).color(Color32::from_rgb(248, 81, 73)));
                }
            },
        );

        // Box 2: Список отслеживаемых кнопок
        primer_box(
            ui,
            "🎯 Список целей (Кнопки для авто-клика)",
            Some(format!("активных кнопок: {}", self.vision_cfg.templates.iter().filter(|t| t.enabled).count()).as_str()),
            |ui| {
                if self.vision_cfg.templates.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new("Цели пока не добавлены.")
                                .size(13.0)
                                .color(Color32::from_rgb(139, 148, 158)),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Наведите курсор на нужную кнопку в игре и нажмите [{}] для захвата!",
                                self.pick_key_name
                            ))
                            .size(12.0)
                            .color(Color32::from_rgb(88, 166, 255)),
                        );
                        ui.add_space(10.0);
                    });
                } else {
                    let mut remove_idx = None;
                    let template_count = self.vision_cfg.templates.len();

                    for (idx, tmpl) in self.vision_cfg.templates.iter_mut().enumerate() {
                        let tex_id = tmpl.id;
                        let texture = self.template_textures.entry(tex_id).or_insert_with(|| {
                            let mut rgba = Vec::with_capacity(tmpl.bgra.len());
                            for chunk in tmpl.bgra.chunks_exact(4) {
                                rgba.push(chunk[2]); // R
                                rgba.push(chunk[1]); // G
                                rgba.push(chunk[0]); // B
                                rgba.push(chunk[3]); // A
                            }
                            let img = egui::ColorImage::from_rgba_unmultiplied(
                                [tmpl.width as usize, tmpl.height as usize],
                                &rgba,
                            );
                            ui.ctx().load_texture(
                                format!("tmpl_{}", tex_id),
                                img,
                                egui::TextureOptions::NEAREST,
                            )
                        });

                        egui::Frame::none()
                            .fill(Color32::from_rgb(26, 31, 39))
                            .rounding(Rounding::same(6.0))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61)))
                            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut tmpl.enabled, "");
                                    ui.add(egui::TextEdit::singleline(&mut tmpl.name).desired_width(120.0));

                                    // Thumbnail preview
                                    let aspect = tmpl.width as f32 / tmpl.height.max(1) as f32;
                                    let preview_h = 32.0;
                                    let preview_w = (preview_h * aspect).clamp(24.0, 72.0);
                                    ui.image((texture.id(), Vec2::new(preview_w, preview_h)));

                                    ui.label(
                                        RichText::new(format!("{}×{} px", tmpl.width, tmpl.height))
                                            .size(11.0)
                                            .color(Color32::from_rgb(139, 148, 158)),
                                    );

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let del_btn = egui::Button::new("🗑")
                                            .fill(Color32::from_rgb(33, 38, 45))
                                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(218, 54, 51)))
                                            .rounding(Rounding::same(5.0));

                                        if ui.add(del_btn).clicked() {
                                            remove_idx = Some(idx);
                                        }

                                        // Threshold slider
                                        ui.add_space(8.0);
                                        let mut pct = (tmpl.threshold * 100.0) as i32;
                                        ui.label(RichText::new("Точность:").size(11.5).color(Color32::from_rgb(201, 209, 217)));
                                        if ui.add(egui::Slider::new(&mut pct, 60..=98).suffix("%")).changed() {
                                            tmpl.threshold = pct as f32 / 100.0;
                                        }

                                        // Action button
                                        ui.add_space(8.0);
                                        let btn_label = match tmpl.click_button {
                                            MouseButton::Left => "ЛКМ",
                                            MouseButton::Right => "ПКМ",
                                            MouseButton::Middle => "СКМ",
                                            MouseButton::X1 => "X1",
                                            MouseButton::X2 => "X2",
                                        };
                                        if ui.button(btn_label).clicked() {
                                            tmpl.click_button = match tmpl.click_button {
                                                MouseButton::Left => MouseButton::Right,
                                                MouseButton::Right => MouseButton::Middle,
                                                MouseButton::Middle => MouseButton::Left,
                                                _ => MouseButton::Left,
                                            };
                                        }

                                        let type_label = match tmpl.click_type {
                                            ClickType::Single => "Клик",
                                            ClickType::Double => "2x Клик",
                                            ClickType::Triple => "3x Клик",
                                            ClickType::Hold => "Зажатие",
                                        };
                                        if ui.button(type_label).clicked() {
                                            tmpl.click_type = match tmpl.click_type {
                                                ClickType::Single => ClickType::Double,
                                                ClickType::Double => ClickType::Hold,
                                                ClickType::Hold => ClickType::Single,
                                                _ => ClickType::Single,
                                            };
                                        }
                                    });
                                });
                            });

                        if idx + 1 < template_count {
                            ui.add_space(6.0);
                        }
                    }

                    if let Some(i) = remove_idx {
                        let tmpl = self.vision_cfg.templates.remove(i);
                        self.template_textures.remove(&tmpl.id);
                    }
                }
            },
        );

        // Box 3: Параметры сканирования и отклика
        primer_box(
            ui,
            "⚙️ Скорость сканирования и поведение мыши",
            Some("настройка частоты проверок и возврата курсора"),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚡ Интервал проверки:").size(12.5).color(Color32::from_rgb(201, 209, 217)));
                    ui.add(
                        egui::Slider::new(&mut self.vision_cfg.scan_interval_ms, 15.0..=300.0)
                            .step_by(5.0)
                            .suffix(" мс"),
                    );
                    ui.label(
                        RichText::new("(40 мс = 25 сканов в сек)")
                            .size(11.0)
                            .color(Color32::from_rgb(139, 148, 158)),
                    );
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⏱️ Cooldown (пауза после клика):").size(12.5).color(Color32::from_rgb(201, 209, 217)));
                    ui.add(
                        egui::Slider::new(&mut self.vision_cfg.cooldown_ms, 50.0..=1000.0)
                            .step_by(10.0)
                            .suffix(" мс"),
                    );
                    ui.label(
                        RichText::new("(защита от повторных кликов)")
                            .size(11.0)
                            .color(Color32::from_rgb(139, 148, 158)),
                    );
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.vision_cfg.return_cursor, "");
                    ui.label(
                        RichText::new("↩️ Возвращать курсор на исходное место после клика")
                            .size(12.5)
                            .strong()
                            .color(Color32::from_rgb(240, 246, 252)),
                    );
                    ui.label(
                        RichText::new("(не сбивает ваш прицел и обзор в игре)")
                            .size(11.0)
                            .color(Color32::from_rgb(139, 148, 158)),
                    );
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🎲 Разброс клика (Jitter):").size(12.5).color(Color32::from_rgb(201, 209, 217)));
                    ui.add(
                        egui::Slider::new(&mut self.vision_cfg.click_jitter_px, 0..=10)
                            .suffix(" px"),
                    );
                    ui.label(
                        RichText::new("(рандомизация точки клика)")
                            .size(11.0)
                            .color(Color32::from_rgb(139, 148, 158)),
                    );
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.vision_cfg.use_custom_region, "");
                    ui.label(
                        RichText::new("🔲 Ограничить зону поиска (сканировать только часть экрана)")
                            .size(12.5)
                            .color(Color32::from_rgb(201, 209, 217)),
                    );
                });

                if self.vision_cfg.use_custom_region {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Текущая зона: ({}, {}) — ({}, {})",
                                self.vision_cfg.region_x1,
                                self.vision_cfg.region_y1,
                                self.vision_cfg.region_x2,
                                self.vision_cfg.region_y2
                            ))
                            .size(11.5)
                            .color(Color32::from_rgb(88, 166, 255)),
                        );

                        ui.add_space(10.0);
                        let is_reg_snip = self.vision_snip_mode == VisionSnipMode::RegionPoint;
                        let reg_btn = egui::Button::new(if is_reg_snip {
                            format!("⏳ Задать углы зоны [{}]...", self.pick_key_name)
                        } else {
                            format!("📐 Задать зону по 2 точкам [{}]", self.pick_key_name)
                        })
                        .fill(Color32::from_rgb(33, 38, 45))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61)))
                        .rounding(Rounding::same(5.0));

                        if ui.add(reg_btn).clicked() {
                            if is_reg_snip {
                                self.vision_snip_mode = VisionSnipMode::None;
                                self.snip_p1 = None;
                            } else {
                                self.vision_snip_mode = VisionSnipMode::RegionPoint;
                                self.snip_p1 = None;
                                self.vision_status_msg = Some(format!(
                                    "Наведите курсор на угол 1 зоны поиска и нажмите [{}]",
                                    self.pick_key_name
                                ));
                            }
                        }

                        if ui.button("Сбросить на весь экран").clicked() {
                            self.vision_cfg.use_custom_region = false;
                        }
                    });
                }
            },
        );
    }

    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        // Section 1: Failsafe & Safety
        primer_box(
            ui,
            "🛡️ Аварийная защита и безопасность (Failsafes)",
            Some("предотвращает потерю контроля над курсором и системой"),
            |ui| {
                ui.checkbox(
                    &mut self.advanced_cfg.failsafe_mouse_move,
                    RichText::new("Авто-пауза при физическом движении мыши (Mouse Failsafe)")
                        .strong()
                        .color(Color32::from_rgb(240, 246, 252)),
                );
                ui.label(
                    RichText::new("Если вы берете мышь в руку и двигаете курсор, кликер мгновенно встает на паузу, возвращая вам полный контроль.")
                        .size(11.5)
                        .color(Color32::from_rgb(139, 148, 158)),
                );
                if self.advanced_cfg.failsafe_mouse_move {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Порог смещения:");
                        ui.add(egui::Slider::new(&mut self.advanced_cfg.mouse_move_threshold, 20..=150).suffix(" px"));
                    });
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.checkbox(
                    &mut self.advanced_cfg.corner_panic,
                    RichText::new("Экстренная остановка в углу экрана (Corner Panic)")
                        .strong()
                        .color(Color32::from_rgb(240, 246, 252)),
                );
                ui.label(
                    RichText::new("Резкое перемещение курсора в левый верхний угол (координаты 0, 0) немедленно глушит кликер.")
                        .size(11.5)
                        .color(Color32::from_rgb(139, 148, 158)),
                );

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.checkbox(
                    &mut self.advanced_cfg.session_limit_enabled,
                    RichText::new("Таймер авто-отключения (AFK Limiter)")
                        .strong()
                        .color(Color32::from_rgb(240, 246, 252)),
                );
                ui.label(
                    RichText::new("Автоматически завершает кликанье по истечении заданного времени (удобно для AFK фарма).")
                        .size(11.5)
                        .color(Color32::from_rgb(139, 148, 158)),
                );
                if self.advanced_cfg.session_limit_enabled {
                    ui.add_space(4.0);
                    let mut minutes = (self.advanced_cfg.session_limit_seconds / 60) as u32;
                    if minutes == 0 {
                        minutes = 1;
                    }
                    ui.horizontal(|ui| {
                        ui.label("Максимальное время работы:");
                        if ui.add(egui::Slider::new(&mut minutes, 1..=180).suffix(" мин")).changed() {
                            self.advanced_cfg.session_limit_seconds = (minutes as u64) * 60;
                        }
                    });
                }
            },
        );

        // Section 2: Window Behavior & Overlay
        primer_box(
            ui,
            "🪟 Поведение окна и оверлей",
            Some("управление отображением поверх полноэкранных игр и приложений"),
            |ui| {
                ui.checkbox(
                    &mut self.advanced_cfg.always_on_top,
                    RichText::new("Закрепить поверх всех окон (Always on Top)")
                        .strong()
                        .color(Color32::from_rgb(240, 246, 252)),
                );
                ui.label(
                    RichText::new("Окно кликера всегда остается на виду поверх вашей игры, браузера или рабочих программ.")
                        .size(11.5)
                        .color(Color32::from_rgb(139, 148, 158)),
                );

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Прозрачность интерфейса (Opacity):");
                    ui.add(
                        egui::Slider::new(&mut self.advanced_cfg.window_opacity, 0.40..=1.0)
                            .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                    );
                });
                ui.label(
                    RichText::new("Позволяет видеть происходящее в игре сквозь полупрозрачный интерфейс автокликера.")
                        .size(11.5)
                        .color(Color32::from_rgb(139, 148, 158)),
                );
            },
        );

        // Section 3: Background Window Mode & Game Lock
        self.render_window_target_card(ui);

        // Section 4: Human Simulation & Anti-Detection
        primer_box(
            ui,
            "🤖 Человекоподобное поведение и анти-детект",
            Some("обход поведенческого анализа и античит систем"),
            |ui| {
                ui.checkbox(
                    &mut self.advanced_cfg.gaussian_jitter,
                    RichText::new("Гауссово нормальное распределение задержки (Gaussian Jitter)")
                        .strong()
                        .color(Color32::from_rgb(240, 246, 252)),
                );
                ui.label(
                    RichText::new("В отличие от примитивного генератора случайных чисел, гауссово распределение воспроизводит естественную колоколообразную кривую реакции человеческой руки.")
                        .size(11.5)
                        .color(Color32::from_rgb(139, 148, 158)),
                );

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.checkbox(
                    &mut self.advanced_cfg.human_hold_duration,
                    RichText::new("Естественное удержание клика мыши (Hold Duration)")
                        .strong()
                        .color(Color32::from_rgb(240, 246, 252)),
                );
                ui.label(
                    RichText::new("Удерживает кнопку мыши нажатой перед отпусканием (вместо мгновенного 0 мс спама, легко выявляемого античитами).")
                        .size(11.5)
                        .color(Color32::from_rgb(139, 148, 158)),
                );
                if self.advanced_cfg.human_hold_duration {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Среднее время зажатия клавиши:");
                        ui.add(egui::Slider::new(&mut self.advanced_cfg.natural_hold_ms, 15.0..=80.0).suffix(" мс"));
                    });
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.checkbox(
                    &mut self.advanced_cfg.micro_drift,
                    RichText::new("Микро-дрейф позиции курсора (Natural Sub-pixel Drift)")
                        .strong()
                        .color(Color32::from_rgb(240, 246, 252)),
                );
                ui.label(
                    RichText::new("Случайно смещает курсор на 1-3 пикселя во время клика, имитируя микроколебания руки игрока.")
                        .size(11.5)
                        .color(Color32::from_rgb(139, 148, 158)),
                );
                if self.advanced_cfg.micro_drift {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Радиус микро-смещения:");
                        ui.add(egui::Slider::new(&mut self.advanced_cfg.micro_drift_px, 1..=6).suffix(" px"));
                    });
                }
            },
        );

        // Section 5: Hotkeys (Rebindable)
        primer_box(
            ui,
            "⌨️ Горячие клавиши (настраиваемые)",
            Some("нажмите «Изменить» и любую клавишу на клавиатуре для переназначения"),
            |ui| {
                let rows: [(HotkeyAction, &str, &str, String, Color32); 3] = [
                    (
                        HotkeyAction::ToggleStartStop,
                        "Запуск / Остановка (Toggle)",
                        "Включает или ставит на паузу кликер поверх любого окна или игры",
                        self.toggle_key_name.clone(),
                        Color32::from_rgb(88, 166, 255),
                    ),
                    (
                        HotkeyAction::PickCoordinate,
                        "Захват координат (Pick)",
                        "Мгновенно сохраняет экранные координаты под курсором",
                        self.pick_key_name.clone(),
                        Color32::from_rgb(63, 185, 80),
                    ),
                    (
                        HotkeyAction::EmergencyStop,
                        "Экстренная остановка (Panic)",
                        "Немедленно глушит все процессы и отпускает зажатые кнопки",
                        self.panic_key_name.clone(),
                        Color32::from_rgb(248, 81, 73),
                    ),
                ];

                for (action, title, desc, key_name, badge_color) in rows {
                    let is_capturing = self.capturing_hotkey == Some(action);

                    ui.horizontal(|ui| {
                        // Key Badge or Capturing State
                        egui::Frame::none()
                            .fill(if is_capturing {
                                Color32::from_rgb(210, 153, 34) // Amber warning
                            } else {
                                Color32::from_rgb(33, 38, 45)
                            })
                            .rounding(Rounding::same(6.0))
                            .stroke(if is_capturing {
                                Stroke::new(1.0_f32, Color32::from_rgb(240, 136, 62))
                            } else {
                                Stroke::new(1.0_f32, Color32::from_rgb(48, 54, 61))
                            })
                            .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                            .show(ui, |ui| {
                                let label_text = if is_capturing {
                                    "Нажмите клавишу...".to_string()
                                } else {
                                    key_name
                                };
                                ui.label(
                                    RichText::new(label_text)
                                        .size(13.0)
                                        .strong()
                                        .color(if is_capturing { Color32::BLACK } else { badge_color }),
                                );
                            });

                        let btn_label = if is_capturing { "Отмена" } else { "Изменить" };
                        let rebind_btn = egui::Button::new(RichText::new(btn_label).size(11.5))
                            .min_size(Vec2::new(75.0, 26.0))
                            .rounding(Rounding::same(5.0));

                        if ui.add(rebind_btn).clicked() {
                            if is_capturing {
                                self.capturing_hotkey = None;
                            } else {
                                self.capturing_hotkey = Some(action);
                            }
                        }

                        ui.vertical(|ui| {
                            ui.label(RichText::new(title).size(12.5).strong().color(Color32::from_rgb(240, 246, 252)));
                            ui.label(RichText::new(desc).size(11.0).color(Color32::from_rgb(139, 148, 158)));
                        });
                    });
                    ui.add_space(8.0);
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(6.0);

                // Reset to Defaults Button
                let reset_btn = egui::Button::new("Вернуть по умолчанию (F6, F8, F12)")
                    .min_size(Vec2::new(0.0, 28.0))
                    .rounding(Rounding::same(5.0));

                if ui.add(reset_btn).clicked() {
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
            },
        );

        // Section 7: About
        primer_box(
            ui,
            "ℹ️ Архитектура и технологии",
            Some("Rust Edition Pro"),
            |ui| {
                ui.label("• Написано на Rust: нативный исполняемый файл без Python рантайма и задержек.");
                ui.add_space(4.0);
                ui.label("• Эмуляция Win32 API SendInput: отправляет реальные низкоуровневые аппаратные события.");
                ui.add_space(4.0);
                ui.label("• Высокоточный спин-лок таймер winmm.dll с шагом суб-миллисекунды.");
                ui.add_space(4.0);
                ui.label("• Автономный компактный .exe (14 МБ) — запускается мгновенно без батников и консолей.");
            },
        );
    }
}
