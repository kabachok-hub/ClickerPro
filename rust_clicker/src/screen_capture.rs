use winapi::shared::minwindef::{DWORD, LPVOID};
use winapi::shared::windef::{HBITMAP, HDC, HGDIOBJ, HWND, RECT};
use winapi::um::wingdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, RGBQUAD, SRCCOPY,
};
use winapi::um::winuser::{
    GetClientRect, GetDC, GetSystemMetrics, IsWindow, PrintWindow, ReleaseDC, SM_CXSCREEN,
    SM_CYSCREEN,
};

pub const CAPTUREBLT: DWORD = 0x40000000;

pub struct ScreenCapture {
    pub width: i32,
    pub height: i32,
    pub buffer: Vec<u8>, // BGRA format
}

#[inline]
pub unsafe fn ensure_input_desktop() {
    let h_input = winapi::um::winuser::OpenInputDesktop(0, 0, 0x01FF);
    if !h_input.is_null() {
        winapi::um::winuser::SetThreadDesktop(h_input);
    }
}

impl ScreenCapture {
    /// Captures the primary screen (or full desktop) into a BGRA byte buffer.
    pub fn capture_screen() -> Option<Self> {
        unsafe {
            ensure_input_desktop();

            let width = GetSystemMetrics(SM_CXSCREEN);
            let height = GetSystemMetrics(SM_CYSCREEN);

            if width <= 0 || height <= 0 {
                return None;
            }

            let hdc_screen: HDC = GetDC(std::ptr::null_mut());
            if hdc_screen.is_null() {
                return None;
            }

            let hdc_mem: HDC = CreateCompatibleDC(hdc_screen);
            if hdc_mem.is_null() {
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                return None;
            }

            let bi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as DWORD,
                    biWidth: width,
                    biHeight: -height, // top-down DIB
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    biSizeImage: (width * height * 4) as DWORD,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD {
                    rgbBlue: 0,
                    rgbGreen: 0,
                    rgbRed: 0,
                    rgbReserved: 0,
                }],
            };

            let mut bits: LPVOID = std::ptr::null_mut();
            let hbm = CreateDIBSection(
                hdc_mem,
                &bi,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            if hbm.is_null() || bits.is_null() {
                DeleteDC(hdc_mem);
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                return None;
            }

            let old_obj = SelectObject(hdc_mem, hbm as HGDIOBJ);
            let blt_res = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY);

            let buffer = if blt_res != 0 && !bits.is_null() {
                std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize).to_vec()
            } else {
                Vec::new()
            };

            SelectObject(hdc_mem, old_obj);
            DeleteObject(hbm as HGDIOBJ);
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);

            if blt_res != 0 && !buffer.is_empty() {
                Some(Self {
                    width,
                    height,
                    buffer,
                })
            } else {
                None
            }
        }
    }

    /// Captures a specific rectangle on screen (e.g. for creating a template)
    pub fn capture_rect(x: i32, y: i32, w: i32, h: i32) -> Option<Self> {
        unsafe {
            if w <= 0 || h <= 0 {
                return None;
            }

            ensure_input_desktop();

            let hdc_screen: HDC = GetDC(std::ptr::null_mut());
            if hdc_screen.is_null() {
                return None;
            }

            let hdc_mem: HDC = CreateCompatibleDC(hdc_screen);
            if hdc_mem.is_null() {
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                return None;
            }

            let bi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as DWORD,
                    biWidth: w,
                    biHeight: -h, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    biSizeImage: (w * h * 4) as DWORD,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD {
                    rgbBlue: 0,
                    rgbGreen: 0,
                    rgbRed: 0,
                    rgbReserved: 0,
                }],
            };

            let mut bits: LPVOID = std::ptr::null_mut();
            let hbm = CreateDIBSection(
                hdc_mem,
                &bi,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            if hbm.is_null() || bits.is_null() {
                DeleteDC(hdc_mem);
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                return None;
            }

            let old_obj = SelectObject(hdc_mem, hbm as HGDIOBJ);
            let blt_res = BitBlt(hdc_mem, 0, 0, w, h, hdc_screen, x, y, SRCCOPY);

            let buffer = if blt_res != 0 && !bits.is_null() {
                std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize).to_vec()
            } else {
                Vec::new()
            };

            SelectObject(hdc_mem, old_obj);
            DeleteObject(hbm as HGDIOBJ);
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);

            if blt_res != 0 && !buffer.is_empty() {
                Some(Self {
                    width: w,
                    height: h,
                    buffer,
                })
            } else {
                None
            }
        }
    }

    /// Captures the client area of a specific window even when obscured or behind other windows.
    pub fn capture_window(hwnd_val: usize) -> Option<Self> {
        if hwnd_val == 0 {
            return None;
        }
        unsafe {
            let hwnd = hwnd_val as HWND;
            if IsWindow(hwnd) == 0 {
                return None;
            }

            ensure_input_desktop();

            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            GetClientRect(hwnd, &mut rect);
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;

            if width <= 0 || height <= 0 {
                return None;
            }

            let hdc_screen: HDC = GetDC(hwnd);
            if hdc_screen.is_null() {
                return None;
            }

            let hdc_mem: HDC = CreateCompatibleDC(hdc_screen);
            if hdc_mem.is_null() {
                ReleaseDC(hwnd, hdc_screen);
                return None;
            }

            let bi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as DWORD,
                    biWidth: width,
                    biHeight: -height, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    biSizeImage: (width * height * 4) as DWORD,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD {
                    rgbBlue: 0,
                    rgbGreen: 0,
                    rgbRed: 0,
                    rgbReserved: 0,
                }],
            };

            let mut bits: LPVOID = std::ptr::null_mut();
            let hbm = CreateDIBSection(
                hdc_mem,
                &bi,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            if hbm.is_null() || bits.is_null() {
                DeleteDC(hdc_mem);
                ReleaseDC(hwnd, hdc_screen);
                return None;
            }

            let old_obj = SelectObject(hdc_mem, hbm as HGDIOBJ);

            // PW_RENDERFULLCONTENT = 2
            let pw_res = PrintWindow(hwnd, hdc_mem, 2);
            let blt_res = if pw_res != 0 {
                1
            } else {
                BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY)
            };

            let buffer = if blt_res != 0 && !bits.is_null() {
                std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize).to_vec()
            } else {
                Vec::new()
            };

            SelectObject(hdc_mem, old_obj);
            DeleteObject(hbm as HGDIOBJ);
            DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc_screen);

            if blt_res != 0 && !buffer.is_empty() {
                Some(Self {
                    width,
                    height,
                    buffer,
                })
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_screen() {
        let cap = ScreenCapture::capture_screen();
        assert!(cap.is_some(), "capture_screen returned None");
        let c = cap.unwrap();
        println!("Captured: {}x{}, buf len={}", c.width, c.height, c.buffer.len());
        let non_zero = c.buffer.iter().filter(|&&b| b > 0).count();
        println!("Non-zero bytes: {} / {}", non_zero, c.buffer.len());
        assert!(non_zero > 0, "Captured screen is completely black!");
    }
}
