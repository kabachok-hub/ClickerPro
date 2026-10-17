use std::mem::size_of;
use winapi::shared::windef::POINT;
use winapi::um::winuser::{
    GetCursorPos, GetForegroundWindow, GetWindowTextW, SendInput, SetCursorPos, INPUT,
    INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL,
    MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT, XBUTTON1, XBUTTON2,
};

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
}

