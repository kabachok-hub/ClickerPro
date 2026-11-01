//! Regressions use generated pixels and never send input to the desktop.
use crate::engine::AdvancedConfig;
use crate::hunter::{find, preview, HunterFrame};
use crate::screen_capture::ScreenCapture;
use crate::vision::{ButtonTemplate, PopupColorFilter, PopupDetector, VisionConfig, VisionMode};

fn blank(w: i32, h: i32) -> ScreenCapture {
    let mut buffer = vec![0; w as usize * h as usize * 4];
    for p in buffer.chunks_exact_mut(4) {
        p.copy_from_slice(&[25, 25, 25, 255]);
    }
    ScreenCapture {
        width: w,
        height: h,
        buffer,
    }
}

fn rectangle(image: &mut ScreenCapture, x: i32, y: i32, w: i32, h: i32, rgb: [u8; 3]) {
    for py in y..y + h {
        for px in x..x + w {
            let i = ((py * image.width + px) * 4) as usize;
            image.buffer[i..i + 4].copy_from_slice(&[rgb[2], rgb[1], rgb[0], 255]);
        }
    }
}

#[test]
fn generic_finds_green_yellow_red_without_white_icon_at_screen_edges() {
    for color in [[20, 220, 30], [240, 220, 20], [230, 30, 25], [30, 170, 240]] {
        let mut image = blank(900, 600);
        rectangle(&mut image, 8, 8, 36, 36, color);
        let target = PopupDetector::new()
            .detect_popup(&image, &snapshot_config())
            .expect("Generic search must include edges and all colors");
        assert!((target.0 - 26).abs() < 7 && (target.1 - 26).abs() < 7);
    }
}

#[test]
fn generic_honors_large_targets_and_optional_rectangular_shapes() {
    let mut image = blank(800, 600);
    rectangle(&mut image, 100, 100, 260, 80, [230, 170, 20]);
    let mut config = VisionConfig {
        round_only: false,
        popup_max_size: 300,
        ..snapshot_config()
    };
    assert!(PopupDetector::new().detect_popup(&image, &config).is_some());
    config.round_only = true;
    assert!(PopupDetector::new().detect_popup(&image, &config).is_none());
    config.round_only = false;
    config.popup_max_size = 130;
    assert!(PopupDetector::new().detect_popup(&image, &config).is_none());
}

#[test]
fn icon_requirement_and_color_filter_are_optional_but_effective() {
    let mut image = blank(300, 200);
    rectangle(&mut image, 80, 60, 60, 60, [20, 220, 30]);
    let config = VisionConfig {
        require_white_icon: true,
        ..snapshot_config()
    };
    assert!(PopupDetector::new().detect_popup(&image, &config).is_none());
    let config = VisionConfig {
        popup_filter: PopupColorFilter::Red,
        ..snapshot_config()
    };
    assert!(PopupDetector::new().detect_popup(&image, &config).is_none());
}

#[test]
fn already_cropped_roi_gets_origin_added_once_including_second_monitor() {
    let mut image = blank(180, 140);
    rectangle(&mut image, 20, 30, 40, 40, [20, 220, 30]);
    let frame = HunterFrame {
        image,
        origin: (-1200, 400),
        hwnd: 0,
    };
    let config = VisionConfig {
        use_custom_region: true,
        region_x1: -1200,
        region_y1: 400,
        region_x2: -1020,
        region_y2: 540,
        ..snapshot_config()
    };
    let target = find(&frame, &config, &mut PopupDetector::new()).unwrap();
    assert!((target.x - -1160).abs() < 6 && (target.y - 450).abs() < 6);
    let (pixels, w, h) = preview(&frame, Some(&target));
    assert_eq!(pixels.len(), w * h * 4);
    assert!(pixels.chunks_exact(4).any(|p| p == [255, 40, 40, 255]));
}

#[test]
fn cooldown_prevents_repeat_without_masking_distant_target() {
    let mut image = blank(300, 200);
    rectangle(&mut image, 20, 20, 40, 40, [20, 220, 30]);
    let config = snapshot_config();
    let mut detector = PopupDetector::new();
    let target = detector.detect_popup(&image, &config).unwrap();
    detector.record_click(target.0, target.1);
    assert!(detector.detect_popup(&image, &config).is_none());
    rectangle(&mut image, 200, 120, 40, 40, [20, 220, 30]);
    assert!(detector.detect_popup(&image, &config).unwrap().0 > 180);
}

#[test]
fn template_matching_respects_transparency_scale_and_screen_origin() {
    let mut template = vec![0u8; 20 * 20 * 4];
    for y in 4..16 {
        for x in 4..16 {
            let i = (y * 20 + x) * 4;
            template[i..i + 4].copy_from_slice(&[30 + x as u8 * 5, 80, 230 - y as u8 * 4, 255]);
        }
    }
    let mut image = blank(160, 120);
    for y in 0..40 {
        for x in 0..40 {
            let ti = ((y / 2) * 20 + x / 2) * 4;
            if template[ti + 3] == 255 {
                let si = ((31 + y) * 160 + 51 + x) * 4;
                image.buffer[si..si + 4].copy_from_slice(&template[ti..ti + 4]);
            }
        }
    }
    let config = VisionConfig {
        mode: VisionMode::TemplateMatch,
        templates: vec![ButtonTemplate::new(
            1,
            "badge".into(),
            20,
            20,
            template,
            0.999,
        )],
        multiscale: true,
        ..snapshot_config()
    };
    let frame = HunterFrame {
        image,
        origin: (300, 200),
        hwnd: 0,
    };
    let target = find(&frame, &config, &mut PopupDetector::new())
        .expect("Scaled transparent template must match");
    assert!(
        (target.x - 371).abs() <= 1 && (target.y - 251).abs() <= 1,
        "{}, {}",
        target.x,
        target.y
    );
}

#[test]
fn invalid_and_empty_frames_or_regions_do_not_panic() {
    let invalid = ScreenCapture {
        width: -1,
        height: 100,
        buffer: vec![],
    };
    assert!(PopupDetector::new()
        .detect_popup(&invalid, &snapshot_config())
        .is_none());
    let config = VisionConfig {
        use_custom_region: true,
        region_x1: -500,
        region_y1: -500,
        region_x2: -100,
        region_y2: -100,
        ..snapshot_config()
    };
    assert!(PopupDetector::new()
        .detect_popup(&blank(200, 200), &config)
        .is_none());
    // Keep the test independent of whatever game or desktop is currently open.
    let _ = AdvancedConfig::default();
}

#[test]
fn stopped_search_does_not_scan_templates_or_click() {
    use std::sync::atomic::AtomicBool;
    let frame = HunterFrame { image: blank(800, 600), origin: (0, 0), hwnd: 0 };
    let config = VisionConfig { mode: VisionMode::TemplateMatch,
        templates: vec![ButtonTemplate::new(1, "test".into(), 20, 20, vec![255; 20 * 20 * 4], 0.99)],
        ..snapshot_config() };
    let stopped = AtomicBool::new(false);
    assert!(crate::hunter::find_cancellable(&frame, &config, &mut PopupDetector::new(), Some(&stopped)).is_none());
    let engine = crate::engine::EngineState::new();
    engine.toggle_vision(VisionConfig { mode: VisionMode::TemplateMatch, ..snapshot_config() }, AdvancedConfig::default());
    assert!(!engine.is_running());
    assert!(engine.hunter_status.lock().unwrap().message.contains("окно игры"));
}

fn snapshot_config() -> VisionConfig {
    VisionConfig { only_new_targets: false, ..Default::default() }
}

#[test]
fn static_background_never_triggers_a_click_even_after_many_scans() {
    let mut scene = blank(400, 240);
    rectangle(&mut scene, 100, 60, 60, 60, [30, 220, 40]);
    let mut detector = PopupDetector::new();
    for _ in 0..200 {
        assert!(detector.detect_popup(&scene, &VisionConfig::default()).is_none());
    }
}

#[test]
fn new_popup_clicks_once_and_rearms_only_after_disappearing() {
    let empty = blank(400, 240);
    let mut scene = blank(400, 240);
    rectangle(&mut scene, 100, 60, 60, 60, [30, 220, 40]);
    let config = VisionConfig::default();
    let mut detector = PopupDetector::new();
    assert!(detector.detect_popup(&empty, &config).is_none());
    assert!(detector.detect_popup(&scene, &config).is_none());
    let (x, y) = detector.detect_popup(&scene, &config).expect("New popup confirmed on two frames");
    detector.record_click(x, y);
    // Even once the time-based cooldown expires, presence keeps the target disarmed.
    detector.recent_targets.clear();
    for _ in 0..100 { assert!(detector.detect_popup(&scene, &config).is_none()); }
    // One missing frame is only a recognition dropout, not a new appearance.
    assert!(detector.detect_popup(&empty, &config).is_none());
    assert!(detector.detect_popup(&scene, &config).is_none());
    assert!(detector.detect_popup(&scene, &config).is_none());
    for _ in 0..4 { assert!(detector.detect_popup(&empty, &config).is_none()); }
    assert!(detector.detect_popup(&scene, &config).is_none());
    assert!(detector.detect_popup(&scene, &config).is_some());
}

#[test]
fn static_high_score_object_does_not_hide_new_smaller_popup() {
    let mut scene = blank(400, 240);
    rectangle(&mut scene, 50, 50, 90, 90, [30, 220, 40]);
    let config = VisionConfig::default();
    let mut detector = PopupDetector::new();
    for _ in 0..3 { assert!(detector.detect_popup(&scene, &config).is_none()); }
    rectangle(&mut scene, 260, 100, 40, 40, [240, 220, 30]);
    assert!(detector.detect_popup(&scene, &config).is_none());
    let target = detector.detect_popup(&scene, &config).unwrap();
    assert!((target.0 - 280).abs() < 8);
}

#[test]
fn template_popup_also_requires_a_new_confirmed_appearance() {
    let empty = blank(220, 160);
    let mut scene = blank(220, 160);
    rectangle(&mut scene, 60, 60, 32, 32, [220, 40, 130]);
    let config = VisionConfig { mode: VisionMode::TemplateMatch, multiscale: false,
        templates: vec![ButtonTemplate::new(1, "popup".into(), 32, 32,
            [130u8, 40, 220, 255].repeat(32 * 32), 0.999)], ..Default::default() };
    let mut detector = PopupDetector::new();
    let frame = |image| HunterFrame { image, origin: (0, 0), hwnd: 0 };
    assert!(find(&frame(empty), &config, &mut detector).is_none());
    let frame = frame(scene);
    assert!(find(&frame, &config, &mut detector).is_none());
    let target = find(&frame, &config, &mut detector).unwrap();
    detector.record_click(target.x, target.y);
    for _ in 0..10 { assert!(find(&frame, &config, &mut detector).is_none()); }
}

#[test]
fn hunter_never_follows_an_unselected_foreground_window() {
    assert!(crate::hunter::selected_window(0, 900).is_err());
    assert!(crate::hunter::selected_window(123, 900).is_err());
    assert_eq!(crate::hunter::selected_window(123, 123).unwrap(), 123);
}

#[test]
fn floating_video_is_removed_from_pixels_and_detection() {
    let mut screen = blank(400, 240);
    rectangle(&mut screen, 280, 140, 60, 60, [20, 230, 30]);
    assert!(PopupDetector::new().detect_popup(&screen, &snapshot_config()).is_some());
    crate::hunter::mask_occlusions(&mut screen, (1000, 500), &[(1260, 620, 1400, 740)]);
    assert!(PopupDetector::new().detect_popup(&screen, &snapshot_config()).is_none());
    let index = ((170 * 400 + 310) * 4) as usize;
    assert_eq!(&screen.buffer[index..index + 4], &[0, 0, 0, 0]);
    rectangle(&mut screen, 40, 40, 40, 40, [20, 230, 30]);
    assert!(PopupDetector::new().detect_popup(&screen, &snapshot_config()).unwrap().0 < 100);
}

#[test]
fn templates_cannot_match_pixels_hidden_by_floating_windows() {
    let mut screen = blank(200, 160);
    rectangle(&mut screen, 40, 40, 32, 32, [230, 20, 30]);
    let template = ButtonTemplate::new(1, "video".into(), 32, 32,
        [30u8, 20, 230, 255].repeat(32 * 32), 0.999);
    assert!(crate::vision::VisionEngine::find_template(&screen, &template, None).is_some());
    crate::hunter::mask_occlusions(&mut screen, (-500, -200), &[(-470, -170, -400, -100)]);
    assert!(crate::vision::VisionEngine::find_template(&screen, &template, None).is_none());
}

#[test]
fn captured_samples_have_opaque_alpha_even_when_gdi_returns_zero() {
    let raw = ScreenCapture { width: 20, height: 20, buffer: [30u8, 90, 230, 0].repeat(400) };
    let sample = ButtonTemplate::from_capture(1, "capture".into(), &raw, 0.95);
    assert!(!sample.sample_points.is_empty());
    assert!(sample.bgra.chunks_exact(4).all(|p| p[3] == 255));
}

#[test]
fn session_keeps_observing_disappearance_during_input_and_cooldown() {
    use crate::hunter::HunterSession;
    use std::time::Duration;
    let mut session = HunterSession::new(VisionConfig::default());
    let frame = |present: bool| {
        let mut image = blank(320, 240);
        if present { rectangle(&mut image, 100, 80, 48, 48, [30, 220, 40]); }
        HunterFrame { image, origin: (0, 0), hwnd: 123 }
    };
    assert!(!session.observe(&frame(false), Duration::ZERO, None).start_input);
    let mut t = 0;
    let mut clicks = 0;
    for _ in 0..30 {
        // Two frames with the next popup. There is no focus change or detector reset.
        t += 30;
        assert!(!session.observe(&frame(true), Duration::from_millis(t), None).start_input);
        t += 30;
        assert!(session.observe(&frame(true), Duration::from_millis(t), None).start_input);
        clicks += 1;
        // The clicked circle disappears while the input worker is still holding/releasing.
        for _ in 0..2 { t += 30; assert!(!session.observe(&frame(false), Duration::from_millis(t), None).start_input); }
        // Its identical replacement appears at the SAME location during that input.
        for _ in 0..2 { t += 30; assert!(!session.observe(&frame(true), Duration::from_millis(t), None).start_input); }
        session.complete_input(true, Duration::from_millis(t));
        for _ in 0..7 { t += 30; assert!(!session.observe(&frame(true), Duration::from_millis(t), None).start_input); }
        // Next iteration crosses cooldown and clicks this already observed generation.
        // Clear it only after that click, not by recreating the session.
        t += 30;
        assert!(session.observe(&frame(true), Duration::from_millis(t), None).start_input);
        clicks += 1;
        session.complete_input(true, Duration::from_millis(t));
        for _ in 0..10 { t += 30; assert!(!session.observe(&frame(false), Duration::from_millis(t), None).start_input); }
    }
    assert_eq!(clicks, 60);
}

#[test]
fn different_colored_popup_at_same_location_is_a_new_identity() {
    let config = VisionConfig::default();
    let mut detector = PopupDetector::new();
    detector.detect_popup(&blank(300, 200), &config);
    for color in [[30, 220, 40], [230, 40, 180], [30, 180, 240], [240, 180, 30]] {
        let mut image = blank(300, 200);
        rectangle(&mut image, 100, 80, 48, 48, color);
        assert!(detector.detect_popup(&image, &config).is_none());
        let target = detector.detect_popup(&image, &config).expect("New color is not the previously consumed popup");
        detector.record_click(target.0, target.1);
    }
}

#[test]
fn template_search_does_not_get_stuck_on_static_best_match() {
    let config = VisionConfig { mode: VisionMode::TemplateMatch, multiscale: false,
        templates: vec![ButtonTemplate::new(1, "popup".into(), 24, 24,
            [130u8, 40, 220, 255].repeat(24 * 24), 0.999)], ..Default::default() };
    let mut detector = PopupDetector::new();
    let mut image = blank(260, 180);
    rectangle(&mut image, 30, 40, 24, 24, [220, 40, 130]);
    let mut frame = HunterFrame { image, origin: (0, 0), hwnd: 0 };
    for _ in 0..3 { assert!(find(&frame, &config, &mut detector).is_none()); }
    rectangle(&mut frame.image, 180, 100, 24, 24, [220, 40, 130]);
    assert!(find(&frame, &config, &mut detector).is_none());
    let target = find(&frame, &config, &mut detector).expect("Existing best match must not hide another instance");
    assert!(target.x > 160);
    detector.record_click(target.x, target.y);
    for _ in 0..10 { assert!(find(&frame, &config, &mut detector).is_none()); }
}

#[test]
fn overlay_does_not_fake_disappearance_or_rearm_a_clicked_target() {
    let config = VisionConfig::default();
    let mut detector = PopupDetector::new();
    let mut visible = blank(300, 200);
    rectangle(&mut visible, 100, 80, 48, 48, [30, 220, 40]);
    detector.detect_popup(&blank(300, 200), &config);
    detector.detect_popup(&visible, &config);
    let target = detector.detect_popup(&visible, &config).unwrap();
    detector.record_click(target.0, target.1);
    let mut covered = blank(300, 200);
    crate::hunter::mask_occlusions(&mut covered, (0, 0), &[(80, 60, 170, 150)]);
    for _ in 0..20 { assert!(detector.detect_popup(&covered, &config).is_none()); }
    for _ in 0..20 { assert!(detector.detect_popup(&visible, &config).is_none()); }
}

#[test]
fn rejected_input_retries_once_then_stops_without_periodic_cursor_warps() {
    use crate::hunter::HunterSession;
    use std::time::Duration;
    let mut session = HunterSession::new(VisionConfig::default());
    let mut frame = HunterFrame { image: blank(300, 200), origin: (0, 0), hwnd: 123 };
    session.observe(&frame, Duration::ZERO, None);
    rectangle(&mut frame.image, 100, 80, 48, 48, [30, 220, 40]);
    session.observe(&frame, Duration::from_millis(30), None);
    assert!(session.observe(&frame, Duration::from_millis(60), None).start_input);
    session.complete_input(false, Duration::from_millis(100));
    for t in [130, 500, 1000] { assert!(!session.observe(&frame, Duration::from_millis(t), None).start_input); }
    assert!(session.observe(&frame, Duration::from_millis(1200), None).start_input);
    session.complete_input(false, Duration::from_millis(1300));
    for t in [1500, 3000, 10000] { assert!(!session.observe(&frame, Duration::from_millis(t), None).start_input); }
}

#[test]
fn late_failure_of_old_click_cannot_consume_its_replacement() {
    use crate::hunter::HunterSession;
    use std::time::Duration;
    let mut session = HunterSession::new(VisionConfig::default());
    let frame = |present| {
        let mut image = blank(300, 200);
        if present { rectangle(&mut image, 100, 80, 48, 48, [30, 220, 40]); }
        HunterFrame { image, origin: (0, 0), hwnd: 123 }
    };
    session.observe(&frame(false), Duration::ZERO, None);
    session.observe(&frame(true), Duration::from_millis(30), None);
    assert!(session.observe(&frame(true), Duration::from_millis(60), None).start_input);
    session.observe(&frame(false), Duration::from_millis(90), None);
    session.observe(&frame(false), Duration::from_millis(120), None);
    session.observe(&frame(true), Duration::from_millis(150), None);
    assert!(!session.observe(&frame(true), Duration::from_millis(180), None).start_input);
    session.complete_input(false, Duration::from_millis(200));
    assert!(session.observe(&frame(true), Duration::from_millis(1250), None).start_input);
    session.complete_input(true, Duration::from_millis(1300));
    assert!(!session.observe(&frame(true), Duration::from_millis(2500), None).start_input);
}

#[test]
fn session_does_not_need_focus_toggle_to_resume_after_a_pause() {
    use crate::hunter::HunterSession;
    use std::time::Duration;
    let mut session = HunterSession::new(VisionConfig::default());
    let mut empty = HunterFrame { image: blank(300, 200), origin: (0, 0), hwnd: 123 };
    session.observe(&empty, Duration::ZERO, None);
    rectangle(&mut empty.image, 100, 80, 48, 48, [30, 220, 40]);
    session.observe(&empty, Duration::from_millis(30), None);
    assert!(session.observe(&empty, Duration::from_millis(60), None).start_input);
    session.complete_input(true, Duration::from_millis(200));
    // No observations during an inactive-window pause. Returning must not reset the session.
    empty.image = blank(300, 200);
    session.observe(&empty, Duration::from_secs(10), None);
    session.observe(&empty, Duration::from_millis(10030), None);
    rectangle(&mut empty.image, 100, 80, 48, 48, [30, 220, 40]);
    session.observe(&empty, Duration::from_millis(10060), None);
    assert!(session.observe(&empty, Duration::from_millis(10090), None).start_input);
}

#[test]
fn hover_brightness_change_does_not_create_another_popup() {
    let config = VisionConfig::default();
    let mut detector = PopupDetector::new();
    detector.detect_popup(&blank(300, 200), &config);
    let mut image = blank(300, 200);
    rectangle(&mut image, 100, 80, 48, 48, [30, 160, 40]);
    detector.detect_popup(&image, &config);
    let target = detector.detect_popup(&image, &config).unwrap();
    detector.record_click(target.0, target.1);
    for color in [[50, 250, 65], [30, 160, 40], [60, 245, 72], [30, 160, 40]] {
        rectangle(&mut image, 100, 80, 48, 48, color);
        for _ in 0..5 { assert!(detector.detect_popup(&image, &config).is_none()); }
    }
}

#[test]
fn simple_picture_mode_clicks_an_already_visible_button_and_repeats_after_pause() {
    use crate::hunter::HunterSession;
    use std::time::Duration;
    let config = VisionConfig { mode: VisionMode::TemplateMatch, only_new_targets: false, multiscale: false,
        cooldown_ms: 220.0, templates: vec![ButtonTemplate::new(1, "cutout".into(), 24, 24,
        [130u8, 40, 220, 255].repeat(24*24), 0.999)], ..Default::default() };
    let mut session = HunterSession::new(config);
    let mut frame = HunterFrame { image: blank(260,180), origin: (0,0), hwnd: 123 };
    rectangle(&mut frame.image, 90,60,24,24,[220,40,130]);
    assert!(session.observe(&frame, Duration::ZERO, None).start_input);
    session.complete_input(true, Duration::from_millis(100));
    assert!(!session.observe(&frame, Duration::from_millis(200), None).start_input);
    assert!(session.observe(&frame, Duration::from_millis(400), None).start_input);
    session.complete_input(true, Duration::from_millis(450));
    frame.image = blank(260,180);
    assert!(!session.observe(&frame, Duration::from_millis(1000), None).start_input);
}

fn paint_round_bonus(image: &mut ScreenCapture, cx: i32, cy: i32, radius: i32) {
    for y in cy-radius..=cy+radius { for x in cx-radius..=cx+radius {
        if (x-cx).pow(2)+(y-cy).pow(2)<=radius.pow(2) { rectangle(image,x,y,1,1,[190,30,220]); }
    } }
    rectangle(image,cx-4,cy-9,8,18,[245,245,245]);
    rectangle(image,cx-8,cy-3,16,6,[245,245,245]);
}

#[test]
fn strict_auto_rejects_colored_rectangles_text_and_bare_circles() {
    let config = VisionConfig { strict_circles: true, require_white_icon: true, only_new_targets: false, ..Default::default() };
    let mut background=blank(400,300);
    rectangle(&mut background,20,210,60,60,[190,30,220]);
    rectangle(&mut background,35,225,25,20,[245,245,245]);
    assert!(PopupDetector::new().detect_popup(&background,&config).is_none());
    let mut bare=blank(400,300);
    paint_round_bonus(&mut bare,200,130,32);
    rectangle(&mut bare,190,118,20,24,[190,30,220]);
    assert!(PopupDetector::new().detect_popup(&bare,&config).is_none());
    paint_round_bonus(&mut background,200,130,32);
    let found=PopupDetector::new().detect_popup(&background,&config).expect("Real round icon must still be recognized");
    assert!((found.0-200).abs()<8 && (found.1-130).abs()<8);
}

#[test]
fn cut_circle_sample_matches_when_the_background_changes() {
    let mut selected=blank(80,80);
    paint_round_bonus(&mut selected,40,40,30);
    let sample=ButtonTemplate::from_capture(1,"cut".into(),&selected,0.95);
    assert!(sample.bgra.chunks_exact(4).any(|p|p[3]==0),"Corners should not encode game scenery");
    let mut game=blank(320,220);
    rectangle(&mut game,0,0,320,220,[80,120,40]);
    paint_round_bonus(&mut game,160,110,30);
    let config=VisionConfig { mode: VisionMode::TemplateMatch, only_new_targets:false, multiscale:false,
        templates:vec![sample], ..Default::default() };
    let frame=HunterFrame {image:game,origin:(0,0),hwnd:123};
    let matched=find(&frame,&config,&mut PopupDetector::new()).expect("Cut sample should ignore the old background");
    assert!((matched.x-160).abs()<2 && (matched.y-110).abs()<2);
}

#[test]
fn sparse_transparent_png_can_be_loaded_and_matched() {
    let directory=std::env::temp_dir().join(format!("clicker-sample-test-{}",std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path=directory.join("образец.png");
    let mut png=image::RgbaImage::new(50,50);
    for y in 10..40 { png.put_pixel(17,y,image::Rgba([220,60,130,255])); }
    png.save(&path).unwrap();
    let template=ButtonTemplate::from_file(1,"png".into(),&path,0.99).unwrap();
    assert_eq!((template.width,template.height),(1,30));
    assert!(!template.sample_points.is_empty());
    let mut screen=blank(120,100);
    rectangle(&mut screen,50,30,1,30,[220,60,130]);
    assert!(crate::vision::VisionEngine::find_template(&screen,&template,None).is_some());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn actual_user_yellow_hand_sample_is_detected_and_ignores_background() {
    let path=std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("ClickerPro/last-template.png");
    if !path.exists() { return; }
    let img=image::open(path).unwrap().to_rgba8();
    let (w,h)=img.dimensions();
    let pixels:Vec<u8>=img.pixels().flat_map(|p|[p[2],p[1],p[0],255]).collect();
    let capture=ScreenCapture {width:w as i32,height:h as i32,buffer:pixels};
    let sample=ButtonTemplate::from_capture(1,"actual".into(),&capture,0.90);
    assert!(sample.bgra.chunks_exact(4).any(|p|p[3]==0),"Real sample must discard scenery corners");
    let strict=VisionConfig {strict_circles:true,require_white_icon:true,only_new_targets:false,..Default::default()};
    assert!(PopupDetector::new().detect_popup(&capture,&strict).is_some(),"Yellow hand is a valid icon");
    let mut game=blank(400,350);
    for y in 0..sample.height { for x in 0..sample.width {
        let a=((y*sample.width+x)*4) as usize;
        if sample.bgra[a+3]>0 { let b=(((y+30)*400+x+50)*4) as usize;game.buffer[b..b+4].copy_from_slice(&sample.bgra[a..a+4]); }
    } }
    assert!(crate::vision::VisionEngine::find_template(&game,&sample,None).is_some());
}

#[test]
fn user_hud_screenshot_without_popup_never_becomes_a_click_target() {
    let image=image::load_from_memory(include_bytes!("../../tests/fixtures/roblox-hud-no-popup.png")).unwrap().to_rgba8();
    let (width,height)=image.dimensions();
    let frame=HunterFrame {image:ScreenCapture {width:width as i32,height:height as i32,
        buffer:image.pixels().flat_map(|p|[p[2],p[1],p[0],255]).collect()},origin:(0,0),hwnd:123};
    let config=VisionConfig {strict_circles:true,require_white_icon:true,exclude_hud:true,only_new_targets:false,..Default::default()};
    let mut session=crate::hunter::HunterSession::new(config);
    for n in 0..30 {
        let observed=session.observe(&frame,std::time::Duration::from_millis(n*100),None);
        assert!(observed.target.is_none(),"False target: {:?}", observed.target.as_ref().map(|t|(t.x,t.y)));
        assert!(!observed.start_input);
    }
}
