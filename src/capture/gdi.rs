use anyhow::{anyhow, Result};
use std::mem::size_of;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC, HGDIOBJ,
    SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

pub struct GdiScreenCapture {
    width: u32,
    height: u32,
    h_screen_dc: HDC,
    h_mem_dc: HDC,
    h_bitmap: windows::Win32::Graphics::Gdi::HBITMAP,
    bmi: BITMAPINFO,
}

// Safety: Windows GDI HDC handles should only be called on the thread or appropriately managed.
unsafe impl Send for GdiScreenCapture {}

impl GdiScreenCapture {
    pub fn new() -> Result<Self> {
        unsafe {
            let width = GetSystemMetrics(SM_CXSCREEN) as u32;
            let height = GetSystemMetrics(SM_CYSCREEN) as u32;

            if width == 0 || height == 0 {
                return Err(anyhow!("Invalid desktop resolution reported by Windows"));
            }

            let h_screen_dc = GetDC(HWND(std::ptr::null_mut()));
            if h_screen_dc.0.is_null() {
                return Err(anyhow!("Failed to acquire screen DC"));
            }

            let h_mem_dc = CreateCompatibleDC(h_screen_dc);
            if h_mem_dc.0.is_null() {
                ReleaseDC(HWND(std::ptr::null_mut()), h_screen_dc);
                return Err(anyhow!("Failed to create compatible memory DC"));
            }

            let h_bitmap = CreateCompatibleBitmap(h_screen_dc, width as i32, height as i32);
            if h_bitmap.0.is_null() {
                let _ = DeleteDC(h_mem_dc);
                let _ = ReleaseDC(HWND(std::ptr::null_mut()), h_screen_dc);
                return Err(anyhow!("Failed to create compatible bitmap"));
            }

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32), // Negative height specifies top-down DIB
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    biSizeImage: width * height * 4,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [windows::Win32::Graphics::Gdi::RGBQUAD {
                    rgbBlue: 0,
                    rgbGreen: 0,
                    rgbRed: 0,
                    rgbReserved: 0,
                }],
            };

            Ok(Self {
                width,
                height,
                h_screen_dc,
                h_mem_dc,
                h_bitmap,
                bmi,
            })
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Capture desktop into BGRA buffer (32-bit: 4 bytes per pixel).
    pub fn capture_frame(&mut self, buffer: &mut Vec<u8>) -> Result<()> {
        let expected_size = (self.width * self.height * 4) as usize;
        buffer.resize(expected_size, 0);

        unsafe {
            let old_obj = SelectObject(self.h_mem_dc, HGDIOBJ(self.h_bitmap.0));

            let blt_res = BitBlt(
                self.h_mem_dc,
                0,
                0,
                self.width as i32,
                self.height as i32,
                self.h_screen_dc,
                0,
                0,
                SRCCOPY,
            );

            if blt_res.is_err() {
                SelectObject(self.h_mem_dc, old_obj);
                return Err(anyhow!("GDI BitBlt failed"));
            }

            let dib_res = GetDIBits(
                self.h_mem_dc,
                self.h_bitmap,
                0,
                self.height,
                Some(buffer.as_mut_ptr() as *mut _),
                &mut self.bmi,
                DIB_RGB_COLORS,
            );

            SelectObject(self.h_mem_dc, old_obj);

            if dib_res == 0 {
                return Err(anyhow!("GDI GetDIBits failed"));
            }
        }

        Ok(())
    }
}

impl Drop for GdiScreenCapture {
    fn drop(&mut self) {
        unsafe {
            if !self.h_bitmap.0.is_null() {
                let _ = DeleteObject(HGDIOBJ(self.h_bitmap.0));
            }
            if !self.h_mem_dc.0.is_null() {
                let _ = DeleteDC(self.h_mem_dc);
            }
            if !self.h_screen_dc.0.is_null() {
                let _ = ReleaseDC(HWND(std::ptr::null_mut()), self.h_screen_dc);
            }
        }
    }
}
