use std::mem::size_of;
use winapi::shared::minwindef::{BOOL, LPARAM, WPARAM};
use winapi::shared::windef::{HWND, POINT, RECT};
use winapi::um::winuser::{
    AttachThreadInput, ClientToScreen, EnumWindows, GetAncestor, GetClientRect, GetCursorPos,
    GetForegroundWindow, GetSystemMetrics, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId,
    IsWindow, IsWindowVisible, MapWindowPoints, PostMessageW, RealChildWindowFromPoint, ScreenToClient,
    SendInput, SetCursorPos, SetForegroundWindow, WindowFromPoint, GA_ROOT, INPUT, INPUT_KEYBOARD,
    INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MK_LBUTTON, MK_MBUTTON, MK_RBUTTON,
    MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL,
    MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT, WM_KEYDOWN, WM_KEYUP,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN,
    WM_RBUTTONUP, XBUTTON1, XBUTTON2,
};

#[link(name = "user32")]
extern "system" {
    fn mouse_event(dwFlags: u32, dx: u32, dy: u32, dwData: u32, dwExtraInfo: usize);
}

#[link(name = "shell32")]
extern "system" {
    fn IsUserAnAdmin() -> BOOL;
    fn ShellExecuteW(
        hwnd: HWND,
        lpOperation: *const u16,
        lpFile: *const u16,
        lpParameters: *const u16,
        lpDirectory: *const u16,
        nShowCmd: i32,
    ) -> winapi::shared::minwindef::HINSTANCE;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub hwnd: usize,
    pub title: String,
    pub width: i32,
    pub height: i32,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickType {
    Single,
    Double,
    Triple,
    Hold, // Hold down for specified duration
}

pub struct WinInput;

impl WinInput {
    pub fn pick_image_file() -> Option<std::path::PathBuf> {
        unsafe {
            let mut filename = vec![0u16; 32768];
            let filter: Vec<u16> = "Изображения PNG/JPG\0*.png;*.jpg;*.jpeg\0Все файлы\0*.*\0\0".encode_utf16().collect();
            let title: Vec<u16> = "Выберите картинку кнопки\0".encode_utf16().collect();
            let mut dialog: winapi::um::commdlg::OPENFILENAMEW = std::mem::zeroed();
            dialog.lStructSize = std::mem::size_of_val(&dialog) as u32;
            dialog.hwndOwner = GetForegroundWindow();
            dialog.lpstrFilter = filter.as_ptr();
            dialog.lpstrFile = filename.as_mut_ptr();
            dialog.nMaxFile = filename.len() as u32;
            dialog.lpstrTitle = title.as_ptr();
            dialog.Flags = winapi::um::commdlg::OFN_FILEMUSTEXIST | winapi::um::commdlg::OFN_PATHMUSTEXIST | winapi::um::commdlg::OFN_NOCHANGEDIR;
            if winapi::um::commdlg::GetOpenFileNameW(&mut dialog) == 0 { return None; }
            let len = filename.iter().position(|&c| c == 0).unwrap_or(filename.len());
            Some(std::path::PathBuf::from(String::from_utf16_lossy(&filename[..len])))
        }
    }

    /// Visible top-level windows above the game in Z order, including picture-in-picture.
    pub fn occluding_rects(target: usize) -> Option<Vec<(i32, i32, i32, i32)>> {
        unsafe {
            let mut hwnd = winapi::um::winuser::GetTopWindow(std::ptr::null_mut());
            let mut rects = Vec::new();
            for _ in 0..2048 {
                if hwnd.is_null() { break; }
                if hwnd as usize == target { return Some(rects); }
                if IsWindowVisible(hwnd) != 0 && winapi::um::winuser::IsIconic(hwnd) == 0 {
                    let mut rect: RECT = std::mem::zeroed();
                    if GetWindowRect(hwnd, &mut rect) != 0 && rect.right > rect.left && rect.bottom > rect.top {
                        // Visibility flags alone include cloaked UWP and transparent fullscreen
                        // helper windows. Mask only a window actually hit above the game.
                        let mut game: RECT = std::mem::zeroed();
                        if GetWindowRect(target as HWND, &mut game) == 0 { return None; }
                        let (l,t,r,b) = (rect.left.max(game.left), rect.top.max(game.top),
                            rect.right.min(game.right), rect.bottom.min(game.bottom));
                        if l < r && t < b {
                            let mut hit = false;
                            for fy in [0.15, 0.5, 0.85] { for fx in [0.15, 0.5, 0.85] {
                                let point = POINT { x: l + ((r-l) as f64 * fx) as i32,
                                    y: t + ((b-t) as f64 * fy) as i32 };
                                let top = WindowFromPoint(point);
                                if !top.is_null() && GetAncestor(top, GA_ROOT) == hwnd { hit = true; }
                            } }
                            if hit { rects.push((l,t,r,b)); }
                        }
                    }
                }
                hwnd = winapi::um::winuser::GetWindow(hwnd, winapi::um::winuser::GW_HWNDNEXT);
            }
            None
        }
    }

    pub fn get_cursor_pos() -> (i32, i32) {
        unsafe {
            let mut pt = POINT { x: 0, y: 0 };
            GetCursorPos(&mut pt);
            (pt.x, pt.y)
        }
    }

    pub fn set_cursor_pos(x: i32, y: i32) {
        unsafe {
            SetCursorPos(x, y);
        }
    }

    pub fn mouse_down(btn: MouseButton) {
        let (flags, data) = match btn {
            MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, 0),
            MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, 0),
            MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, 0),
            MouseButton::X1 => (MOUSEEVENTF_XDOWN, XBUTTON1 as u32),
            MouseButton::X2 => (MOUSEEVENTF_XDOWN, XBUTTON2 as u32),
        };

        unsafe {
            let mut input = INPUT {
                type_: INPUT_MOUSE,
                u: std::mem::zeroed(),
            };
            *input.u.mi_mut() = MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            };
            SendInput(1, &mut input, size_of::<INPUT>() as i32);
            mouse_event(flags, 0, 0, data, 0);
        }
    }

    pub fn mouse_up(btn: MouseButton) {
        let (flags, data) = match btn {
            MouseButton::Left => (MOUSEEVENTF_LEFTUP, 0),
            MouseButton::Right => (MOUSEEVENTF_RIGHTUP, 0),
            MouseButton::Middle => (MOUSEEVENTF_MIDDLEUP, 0),
            MouseButton::X1 => (MOUSEEVENTF_XUP, XBUTTON1 as u32),
            MouseButton::X2 => (MOUSEEVENTF_XUP, XBUTTON2 as u32),
        };

        unsafe {
            let mut input = INPUT {
                type_: INPUT_MOUSE,
                u: std::mem::zeroed(),
            };
            *input.u.mi_mut() = MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            };
            SendInput(1, &mut input, size_of::<INPUT>() as i32);
            mouse_event(flags, 0, 0, data, 0);
        }
    }

    pub fn mouse_click(btn: MouseButton) {
        Self::mouse_down(btn);
        Self::mouse_up(btn);
    }

    /// Rock-solid hardware mouse click for games like Roblox.
    /// Handles physical cursor movement, hover registration delay (25ms),
    /// hold duration (min 45ms), release confirmation (25ms), and safe return.
    /// Completely multi-monitor and DPI safe across any resolution.
    /// High-precision hardware click for games (Roblox, etc.).
    /// Uses both SetCursorPos and SendInput with absolute coordinates to ensure
    /// DirectInput/RawInput registers hover, Activated, and click events without dropping.
    pub fn send_hardware_click(
        x: i32,
        y: i32,
        button: MouseButton,
        hold_ms: f64,
        return_cursor: bool,
        orig_x: i32,
        orig_y: i32,
        expected_hwnd: usize,
        running: &std::sync::atomic::AtomicBool,
    ) -> Result<(), String> {
        unsafe {
            if !running.load(std::sync::atomic::Ordering::Relaxed)
                || Self::get_foreground_window() != expected_hwnd
                || !Self::get_window_at_point(x, y).is_some_and(|w| w.hwnd == expected_hwnd) {
                return Err("Клик отменён: цель перекрыта другим окном или игра неактивна".into());
            }
            let restore_cursor = || {
                // Do not pull the cursor into another application after Alt-Tab, or undo
                // physical mouse movement that happened while the injected click ran.
                let current = Self::get_cursor_pos();
                if return_cursor && Self::get_foreground_window() == expected_hwnd
                    && (current.0 - x).abs() <= 8 && (current.1 - y).abs() <= 8 {
                    SetCursorPos(orig_x, orig_y);
                }
            };
            let (down_flag, up_flag, data) = match button {
                MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, 0),
                MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, 0),
                MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, 0),
                MouseButton::X1 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, XBUTTON1 as u32),
                MouseButton::X2 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, XBUTTON2 as u32),
            };

            // 1. Physically move cursor to target position via SetCursorPos
            SetCursorPos(x, y);

            // Also send SendInput Move (0x0001 | 0x8000) so RawInput/DirectInput registers hardware cursor transition
            let sx = GetSystemMetrics(winapi::um::winuser::SM_XVIRTUALSCREEN);
            let sy = GetSystemMetrics(winapi::um::winuser::SM_YVIRTUALSCREEN);
            let sw = GetSystemMetrics(winapi::um::winuser::SM_CXVIRTUALSCREEN);
            let sh = GetSystemMetrics(winapi::um::winuser::SM_CYVIRTUALSCREEN);
            if sw > 0 && sh > 0 {
                let norm_x = (((x - sx) as f64 * 65535.0) / (sw - 1).max(1) as f64).round() as i32;
                let norm_y = (((y - sy) as f64 * 65535.0) / (sh - 1).max(1) as f64).round() as i32;
                let mut move_input = INPUT {
                    type_: INPUT_MOUSE,
                    u: std::mem::zeroed(),
                };
                *move_input.u.mi_mut() = MOUSEINPUT {
                    dx: norm_x,
                    dy: norm_y,
                    mouseData: 0,
                    dwFlags: 0x0001 | 0x8000 | 0x4000, // MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE
                    time: 0,
                    dwExtraInfo: 0,
                };
                if SendInput(1, &mut move_input, size_of::<INPUT>() as i32) != 1 { restore_cursor(); return Err("Windows отклонила перемещение мыши. Проверьте права игры и кликера".into()); }
            }

            // 2. Allow game window 35ms (2 frames at 60 FPS) to receive MouseEnter / Hover
            crate::precision_timer::PrecisionTimer::sleep_ms(35.0);

            if !running.load(std::sync::atomic::Ordering::Relaxed) || Self::get_foreground_window() != expected_hwnd
                || !Self::get_window_at_point(x, y).is_some_and(|w| w.hwnd == expected_hwnd) {
                restore_cursor();
                return Err("Клик отменён: остановка или смена активного окна".into());
            }

            // 3. A single MouseDown through SendInput
            let mut down_input = INPUT {
                type_: INPUT_MOUSE,
                u: std::mem::zeroed(),
            };
            *down_input.u.mi_mut() = MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: data,
                dwFlags: down_flag,
                time: 0,
                dwExtraInfo: 0,
            };
            if SendInput(1, &mut down_input, size_of::<INPUT>() as i32) != 1 {
                restore_cursor();
                return Err("Windows отклонила нажатие. Проверьте права игры и кликера".into());
            }

            // 4. Keep the configured hold duration, with a short minimum for game input
            let hold = hold_ms.max(15.0);
            crate::precision_timer::PrecisionTimer::sleep_ms(hold);

            // 5. Always release; legacy fallback is only used if SendInput fails
            let mut up_input = INPUT {
                type_: INPUT_MOUSE,
                u: std::mem::zeroed(),
            };
            *up_input.u.mi_mut() = MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: data,
                dwFlags: up_flag,
                time: 0,
                dwExtraInfo: 0,
            };
            let released = SendInput(1, &mut up_input, size_of::<INPUT>() as i32) == 1;
            if !released { mouse_event(up_flag, 0, 0, data, 0); }

            // 6. CRITICAL FOR ROBLOX: Wait 75ms after mouse up WHILE STILL OVER THE BUTTON
            // so Roblox's Lua engine processes the Activated event without MouseLeave canceling it!
            crate::precision_timer::PrecisionTimer::sleep_ms(75.0);

            // 7. Safely return cursor if enabled
            restore_cursor();
            if !released { return Err("Windows отклонила отпускание кнопки".into()); }
            Ok(())
        }
    }

    #[allow(dead_code)]
    pub fn play_sound_start() {
        // Sounds silenced per user request
    }

    #[allow(dead_code)]
    pub fn play_sound_stop() {
        // Sounds silenced per user request
    }

    #[allow(dead_code)]
    pub fn play_sound_click() {
        // Sounds silenced per user request
    }

    pub fn is_elevated() -> bool {
        unsafe {
            IsUserAnAdmin() != 0
        }
    }

    pub fn restart_as_admin() -> Result<(), String> {
        let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe_str = exe_path.to_str().ok_or("Invalid exe path")?;
        let exe_wide: Vec<u16> = exe_str.encode_utf16().chain(std::iter::once(0)).collect();
        let verb_wide: Vec<u16> = "runas".encode_utf16().chain(std::iter::once(0)).collect();

        unsafe {
            let res = ShellExecuteW(
                std::ptr::null_mut(),
                verb_wide.as_ptr(),
                exe_wide.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                winapi::um::winuser::SW_SHOWNORMAL,
            );
            if res as usize > 32 {
                std::process::exit(0);
            } else {
                Err(format!("Не удалось получить права Администратора (код {})", res as usize))
            }
        }
    }

    pub fn mouse_scroll(delta: i32, horizontal: bool) {
        let flags = if horizontal {
            MOUSEEVENTF_HWHEEL
        } else {
            MOUSEEVENTF_WHEEL
        };

        unsafe {
            let mut input = INPUT {
                type_: INPUT_MOUSE,
                u: std::mem::zeroed(),
            };
            *input.u.mi_mut() = MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: delta as u32,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            };
            SendInput(1, &mut input, size_of::<INPUT>() as i32);
        }
    }

    pub fn key_down(vk_code: u16) {
        unsafe {
            let mut input = INPUT {
                type_: INPUT_KEYBOARD,
                u: std::mem::zeroed(),
            };
            *input.u.ki_mut() = KEYBDINPUT {
                wVk: vk_code,
                wScan: 0,
                dwFlags: 0,
                time: 0,
                dwExtraInfo: 0,
            };
            SendInput(1, &mut input, size_of::<INPUT>() as i32);
        }
    }

    pub fn key_up(vk_code: u16) {
        unsafe {
            let mut input = INPUT {
                type_: INPUT_KEYBOARD,
                u: std::mem::zeroed(),
            };
            *input.u.ki_mut() = KEYBDINPUT {
                wVk: vk_code,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            SendInput(1, &mut input, size_of::<INPUT>() as i32);
        }
    }

    pub fn key_tap(vk_code: u16) {
        Self::key_down(vk_code);
        Self::key_up(vk_code);
    }

    #[allow(dead_code)]
    pub fn type_text(text: &str) {
        for ch in text.chars() {
            let mut utf16_buf = [0u16; 2];
            let encoded = ch.encode_utf16(&mut utf16_buf);
            for &mut code in encoded {
                unsafe {
                    // Down
                    let mut input = INPUT {
                        type_: INPUT_KEYBOARD,
                        u: std::mem::zeroed(),
                    };
                    *input.u.ki_mut() = KEYBDINPUT {
                        wVk: 0,
                        wScan: code,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: 0,
                    };
                    SendInput(1, &mut input, size_of::<INPUT>() as i32);

                    // Up
                    let mut up_input = input;
                    (*up_input.u.ki_mut()).dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;
                    SendInput(1, &mut up_input, size_of::<INPUT>() as i32);
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn force_foreground_window(hwnd_val: usize) {
        if hwnd_val == 0 {
            return;
        }
        unsafe {
            let hwnd = hwnd_val as HWND;
            let fg_hwnd = GetForegroundWindow();
            if fg_hwnd != hwnd {
                let cur_thread = GetWindowThreadProcessId(fg_hwnd, std::ptr::null_mut());
                let target_thread = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
                if cur_thread != 0 && target_thread != 0 && cur_thread != target_thread {
                    AttachThreadInput(cur_thread, target_thread, 1);
                    SetForegroundWindow(hwnd);
                    AttachThreadInput(cur_thread, target_thread, 0);
                } else {
                    SetForegroundWindow(hwnd);
                }
            }
        }
    }

    pub fn get_foreground_window() -> usize {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                0
            } else {
                hwnd as usize
            }
        }
    }

    pub fn get_active_window_title() -> String {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return String::new();
            }
            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), 512);
            if len > 0 {
                String::from_utf16_lossy(&buf[..len as usize])
            } else {
                String::new()
            }
        }
    }

    pub fn is_window_valid(hwnd_val: usize) -> bool {
        if hwnd_val == 0 {
            return false;
        }
        unsafe { IsWindow(hwnd_val as HWND) != 0 }
    }

    #[allow(dead_code)]
    pub fn get_window_title_by_hwnd(hwnd_val: usize) -> String {
        if hwnd_val == 0 {
            return String::new();
        }
        unsafe {
            let hwnd = hwnd_val as HWND;
            if IsWindow(hwnd) == 0 {
                return String::new();
            }
            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), 512);
            if len > 0 {
                String::from_utf16_lossy(&buf[..len as usize])
            } else {
                String::new()
            }
        }
    }

    pub fn get_window_client_rect(hwnd_val: usize) -> Option<(i32, i32, i32, i32)> {
        if hwnd_val == 0 {
            return None;
        }
        unsafe {
            let hwnd = hwnd_val as HWND;
            if IsWindow(hwnd) == 0 {
                return None;
            }
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetClientRect(hwnd, &mut rect) != 0 {
                Some((rect.left, rect.top, rect.right, rect.bottom))
            } else {
                None
            }
        }
    }

    pub fn get_window_screen_rect(hwnd_val: usize) -> Option<(i32, i32, i32, i32)> {
        if hwnd_val == 0 {
            return None;
        }
        unsafe {
            let hwnd = hwnd_val as HWND;
            if IsWindow(hwnd) == 0 {
                return None;
            }
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetWindowRect(hwnd, &mut rect) != 0 {
                Some((rect.left, rect.top, rect.right, rect.bottom))
            } else {
                None
            }
        }
    }

    pub fn screen_to_client(hwnd_val: usize, screen_x: i32, screen_y: i32) -> Option<(i32, i32)> {
        if hwnd_val == 0 {
            return None;
        }
        unsafe {
            let hwnd = hwnd_val as HWND;
            if IsWindow(hwnd) == 0 {
                return None;
            }
            let mut pt = POINT {
                x: screen_x,
                y: screen_y,
            };
            if ScreenToClient(hwnd, &mut pt) != 0 {
                Some((pt.x, pt.y))
            } else {
                None
            }
        }
    }

    #[allow(dead_code)]
    pub fn client_to_screen(hwnd_val: usize, client_x: i32, client_y: i32) -> Option<(i32, i32)> {
        if hwnd_val == 0 {
            return None;
        }
        unsafe {
            let hwnd = hwnd_val as HWND;
            if IsWindow(hwnd) == 0 {
                return None;
            }
            let mut pt = POINT {
                x: client_x,
                y: client_y,
            };
            if ClientToScreen(hwnd, &mut pt) != 0 {
                Some((pt.x, pt.y))
            } else {
                None
            }
        }
    }

    pub fn get_window_at_point(screen_x: i32, screen_y: i32) -> Option<WindowInfo> {
        unsafe {
            let pt = POINT {
                x: screen_x,
                y: screen_y,
            };
            let raw_hwnd = WindowFromPoint(pt);
            if raw_hwnd.is_null() {
                return None;
            }
            let root_hwnd = GetAncestor(raw_hwnd, GA_ROOT);
            let hwnd = if !root_hwnd.is_null() {
                root_hwnd
            } else {
                raw_hwnd
            };
            if IsWindow(hwnd) == 0 {
                return None;
            }

            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), 512);
            let title = if len > 0 {
                String::from_utf16_lossy(&buf[..len as usize])
            } else {
                format!("Окно (HWND: 0x{:X})", hwnd as usize)
            };

            let mut r = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            GetClientRect(hwnd, &mut r);
            let w = r.right - r.left;
            let h = r.bottom - r.top;

            Some(WindowInfo {
                hwnd: hwnd as usize,
                title,
                width: w,
                height: h,
            })
        }
    }

    pub fn list_open_windows() -> Vec<WindowInfo> {
        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            if IsWindowVisible(hwnd) != 0 {
                let mut rect = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                GetWindowRect(hwnd, &mut rect);
                let w = rect.right - rect.left;
                let h = rect.bottom - rect.top;
                if w >= 120 && h >= 80 {
                    let mut buf = [0u16; 512];
                    let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), 512);
                    if len > 0 {
                        let title = String::from_utf16_lossy(&buf[..len as usize]);
                        let t_trim = title.trim();
                        if !t_trim.is_empty()
                            && !t_trim.contains("⚡ Автокликер Pro")
                            && !["Картинка в картинке", "Picture in picture", "Picture-in-Picture"].contains(&t_trim)
                            && t_trim != "Program Manager"
                            && t_trim != "Default IME"
                            && t_trim != "MSCTFIME UI"
                        {
                            let list = &mut *(lparam as *mut Vec<WindowInfo>);
                            list.push(WindowInfo {
                                hwnd: hwnd as usize,
                                title: t_trim.to_string(),
                                width: w,
                                height: h,
                            });
                        }
                    }
                }
            }
            1
        }

        let mut list: Vec<WindowInfo> = Vec::new();
        unsafe {
            EnumWindows(Some(enum_proc), &mut list as *mut _ as LPARAM);
        }
        list
    }

    /// Automatically searches for an active Roblox client window (by title or WINDOWSCLIENT class).
    pub fn find_roblox_window() -> Option<(usize, String)> {
        unsafe {
            let h_input = winapi::um::winuser::OpenInputDesktop(0, 0, 0x01FF);
            if !h_input.is_null() {
                winapi::um::winuser::SetThreadDesktop(h_input);
            }

            struct SearchData {
                found: Option<(usize, String)>,
            }

            unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
                let data = &mut *(lparam as *mut SearchData);
                if IsWindowVisible(hwnd) != 0 {
                    let mut buf = [0u16; 512];
                    let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), 512);
                    let title = String::from_utf16_lossy(&buf[..len as usize]);

                    let mut cls_buf = [0u16; 256];
                    let cls_len = winapi::um::winuser::GetClassNameW(hwnd, cls_buf.as_mut_ptr(), 256);
                    let class_name = String::from_utf16_lossy(&cls_buf[..cls_len as usize]);

                    if title.to_lowercase().contains("roblox") || class_name == "WINDOWSCLIENT" {
                        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                        GetWindowRect(hwnd, &mut rect);
                        let w = rect.right - rect.left;
                        let h = rect.bottom - rect.top;
                        if w > 200 && h > 200 {
                            let display_title = if title.is_empty() { "Roblox".to_string() } else { title };
                            data.found = Some((hwnd as usize, display_title));
                            return 0;
                        }
                    }
                }
                1
            }

            let mut data = SearchData { found: None };
            EnumWindows(Some(enum_proc), &mut data as *mut _ as LPARAM);
            data.found
        }
    }

    /// Background Mouse Click directly into a target window via PostMessageW.
    /// Does NOT move the real hardware cursor and works even if the window is covered by other windows.
    pub fn post_mouse_click_to_window(
        hwnd_val: usize,
        client_x: i32,
        client_y: i32,
        btn: MouseButton,
        click_type: ClickType,
        hold_ms: f64,
    ) {
        if hwnd_val == 0 {
            return;
        }
        unsafe {
            let h_input = winapi::um::winuser::OpenInputDesktop(0, 0, 0x01FF);
            if !h_input.is_null() {
                winapi::um::winuser::SetThreadDesktop(h_input);
            }

            let hwnd = hwnd_val as HWND;
            if IsWindow(hwnd) == 0 {
                return;
            }

            // Find child control if any at this point
            let mut pt = POINT {
                x: client_x,
                y: client_y,
            };
            let child = RealChildWindowFromPoint(hwnd, pt);
            let (target_hwnd, target_x, target_y) = if !child.is_null() && child != hwnd {
                MapWindowPoints(hwnd, child, &mut pt, 1);
                (child, pt.x, pt.y)
            } else {
                (hwnd, client_x, client_y)
            };

            let lparam = (((target_y as u32) & 0xFFFF) << 16) | ((target_x as u32) & 0xFFFF);

            let (msg_down, msg_up, wparam_down) = match btn {
                MouseButton::Left => (WM_LBUTTONDOWN, WM_LBUTTONUP, MK_LBUTTON),
                MouseButton::Right => (WM_RBUTTONDOWN, WM_RBUTTONUP, MK_RBUTTON),
                MouseButton::Middle => (WM_MBUTTONDOWN, WM_MBUTTONUP, MK_MBUTTON),
                MouseButton::X1 => (0x020B, 0x020C, 0x0020),
                MouseButton::X2 => (0x020B, 0x020C, 0x0040),
            };

            // 1. Inform window of mouse hover (Roblox needs this for MouseEnter / Activated)
            PostMessageW(target_hwnd, WM_MOUSEMOVE, 0, lparam as LPARAM);
            crate::precision_timer::PrecisionTimer::sleep_ms(20.0);

            let hold = hold_ms.max(45.0);
            match click_type {
                ClickType::Single => {
                    PostMessageW(target_hwnd, msg_down, wparam_down, lparam as LPARAM);
                    crate::precision_timer::PrecisionTimer::sleep_ms(hold);
                    PostMessageW(target_hwnd, msg_up, 0, lparam as LPARAM);
                    crate::precision_timer::PrecisionTimer::sleep_ms(20.0);
                }
                ClickType::Double => {
                    PostMessageW(target_hwnd, msg_down, wparam_down, lparam as LPARAM);
                    crate::precision_timer::PrecisionTimer::sleep_ms(hold.min(35.0));
                    PostMessageW(target_hwnd, msg_up, 0, lparam as LPARAM);
                    crate::precision_timer::PrecisionTimer::sleep_ms(25.0);
                    PostMessageW(target_hwnd, msg_down, wparam_down, lparam as LPARAM);
                    crate::precision_timer::PrecisionTimer::sleep_ms(hold.min(35.0));
                    PostMessageW(target_hwnd, msg_up, 0, lparam as LPARAM);
                    crate::precision_timer::PrecisionTimer::sleep_ms(20.0);
                }
                ClickType::Triple => {
                    for i in 0..3 {
                        PostMessageW(target_hwnd, msg_down, wparam_down, lparam as LPARAM);
                        crate::precision_timer::PrecisionTimer::sleep_ms(hold.min(30.0));
                        PostMessageW(target_hwnd, msg_up, 0, lparam as LPARAM);
                        if i < 2 {
                            crate::precision_timer::PrecisionTimer::sleep_ms(25.0);
                        }
                    }
                    crate::precision_timer::PrecisionTimer::sleep_ms(20.0);
                }
                ClickType::Hold => {
                    PostMessageW(target_hwnd, msg_down, wparam_down, lparam as LPARAM);
                    crate::precision_timer::PrecisionTimer::sleep_ms(hold);
                    PostMessageW(target_hwnd, msg_up, 0, lparam as LPARAM);
                    crate::precision_timer::PrecisionTimer::sleep_ms(20.0);
                }
            }
        }
    }

    /// Background Key Tap directly to a target window via PostMessageW.
    pub fn post_key_tap_to_window(hwnd_val: usize, vk_code: u16, hold_ms: f64) {
        if hwnd_val == 0 {
            return;
        }
        unsafe {
            let hwnd = hwnd_val as HWND;
            if IsWindow(hwnd) == 0 {
                return;
            }
            PostMessageW(hwnd, WM_KEYDOWN, vk_code as WPARAM, 1 as LPARAM);
            if hold_ms > 0.0 {
                crate::precision_timer::PrecisionTimer::sleep_ms(hold_ms);
            }
            let up_lparam = 1 | (1 << 30) | (1 << 31);
            PostMessageW(hwnd, WM_KEYUP, vk_code as WPARAM, up_lparam as LPARAM);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_roblox() {
        let res = WinInput::find_roblox_window();
        println!("find_roblox_window returned: {:?}", res);
        if let Some((hwnd, title)) = res {
            println!("Found Roblox HWND 0x{:X} with title '{}'", hwnd, title);
            let rect = WinInput::get_window_client_rect(hwnd);
            println!("Client rect: {:?}", rect);

            if let Some(cap) = crate::screen_capture::ScreenCapture::capture_window(hwnd) {
                let non_zero = cap.buffer.iter().filter(|&&b| b > 0).count();
                println!("capture_window {}x{}, non_zero bytes: {} / {}", cap.width, cap.height, non_zero, cap.buffer.len());
            } else {
                println!("capture_window returned None!");
            }

            // Test sending click to the detected circle coordinate
            WinInput::post_mouse_click_to_window(hwnd, 1173, 342, MouseButton::Left, ClickType::Single, 45.0);
            println!("Successfully sent background click to Roblox at (1173, 342) without moving cursor!");
        }
    }
}
