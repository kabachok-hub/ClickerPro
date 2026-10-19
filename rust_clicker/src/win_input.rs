use std::mem::size_of;
use winapi::shared::minwindef::{BOOL, LPARAM, WPARAM};
use winapi::shared::windef::{HWND, POINT, RECT};
use winapi::um::winuser::{
    ClientToScreen, EnumWindows, GetAncestor, GetClientRect, GetCursorPos, GetForegroundWindow,
    GetWindowRect, GetWindowTextW, IsWindow, IsWindowVisible, MapWindowPoints, PostMessageW,
    RealChildWindowFromPoint, ScreenToClient, SendInput, SetCursorPos, WindowFromPoint, GA_ROOT,
    INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    MK_LBUTTON, MK_MBUTTON, MK_RBUTTON, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT,
    WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP,
    WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP, XBUTTON1, XBUTTON2,
};

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
        }
    }

    pub fn mouse_click(btn: MouseButton) {
        Self::mouse_down(btn);
        Self::mouse_up(btn);
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

            // Inform window of mouse hover
            PostMessageW(target_hwnd, WM_MOUSEMOVE, 0, lparam as LPARAM);

            match click_type {
                ClickType::Single => {
                    PostMessageW(target_hwnd, msg_down, wparam_down, lparam as LPARAM);
                    if hold_ms > 0.0 {
                        crate::precision_timer::PrecisionTimer::sleep_ms(hold_ms);
                    }
                    PostMessageW(target_hwnd, msg_up, 0, lparam as LPARAM);
                }
                ClickType::Double => {
                    PostMessageW(target_hwnd, msg_down, wparam_down, lparam as LPARAM);
                    PostMessageW(target_hwnd, msg_up, 0, lparam as LPARAM);
                    crate::precision_timer::PrecisionTimer::sleep_ms(25.0);
                    PostMessageW(target_hwnd, msg_down, wparam_down, lparam as LPARAM);
                    PostMessageW(target_hwnd, msg_up, 0, lparam as LPARAM);
                }
                ClickType::Triple => {
                    for i in 0..3 {
                        PostMessageW(target_hwnd, msg_down, wparam_down, lparam as LPARAM);
                        PostMessageW(target_hwnd, msg_up, 0, lparam as LPARAM);
                        if i < 2 {
                            crate::precision_timer::PrecisionTimer::sleep_ms(25.0);
                        }
                    }
                }
                ClickType::Hold => {
                    PostMessageW(target_hwnd, msg_down, wparam_down, lparam as LPARAM);
                    if hold_ms > 0.0 {
                        crate::precision_timer::PrecisionTimer::sleep_ms(hold_ms);
                    }
                    PostMessageW(target_hwnd, msg_up, 0, lparam as LPARAM);
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

