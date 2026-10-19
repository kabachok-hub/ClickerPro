use winapi::shared::minwindef::{DWORD, LPVOID};
use winapi::shared::windef::{HBITMAP, HDC, HGDIOBJ};
use winapi::um::wingdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
    SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, RGBQUAD, SRCCOPY,
};
use winapi::um::winuser::{GetDC, GetSystemMetrics, ReleaseDC, SM_CXSCREEN, SM_CYSCREEN};

pub struct ScreenCapture {
    pub width: i32,
    pub height: i32,
    pub buffer: Vec<u8>, // BGRA format
}

impl ScreenCapture {
    /// Captures the primary screen (or full desktop) into a BGRA byte buffer.
    pub fn capture_screen() -> Option<Self> {
        unsafe {
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

            let hbm: HBITMAP = CreateCompatibleBitmap(hdc_screen, width, height);
            if hbm.is_null() {
                DeleteDC(hdc_mem);
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                return None;
            }

            let old_obj = SelectObject(hdc_mem, hbm as HGDIOBJ);

            // Copy screen contents to memory DC
            let blt_res = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY);

            let mut buffer = vec![0u8; (width * height * 4) as usize];

            if blt_res != 0 {
                let mut bi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as DWORD,
                        biWidth: width,
                        biHeight: -height, // negative for top-down DIB
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

                GetDIBits(
                    hdc_mem,
                    hbm,
                    0,
                    height as u32,
                    buffer.as_mut_ptr() as LPVOID,
                    &mut bi,
                    DIB_RGB_COLORS,
                );
            }

            // Cleanup GDI objects
            SelectObject(hdc_mem, old_obj);
            DeleteObject(hbm as HGDIOBJ);
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);

            if blt_res != 0 {
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

            let hdc_screen: HDC = GetDC(std::ptr::null_mut());
            if hdc_screen.is_null() {
                return None;
            }

            let hdc_mem: HDC = CreateCompatibleDC(hdc_screen);
            if hdc_mem.is_null() {
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                return None;
            }

            let hbm: HBITMAP = CreateCompatibleBitmap(hdc_screen, w, h);
            if hbm.is_null() {
                DeleteDC(hdc_mem);
                ReleaseDC(std::ptr::null_mut(), hdc_screen);
                return None;
            }

            let old_obj = SelectObject(hdc_mem, hbm as HGDIOBJ);

            let blt_res = BitBlt(hdc_mem, 0, 0, w, h, hdc_screen, x, y, SRCCOPY);

            let mut buffer = vec![0u8; (w * h * 4) as usize];

            if blt_res != 0 {
                let mut bi = BITMAPINFO {
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

                GetDIBits(
                    hdc_mem,
                    hbm,
                    0,
                    h as u32,
                    buffer.as_mut_ptr() as LPVOID,
                    &mut bi,
                    DIB_RGB_COLORS,
                );
            }

            SelectObject(hdc_mem, old_obj);
            DeleteObject(hbm as HGDIOBJ);
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);

            if blt_res != 0 {
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
}
