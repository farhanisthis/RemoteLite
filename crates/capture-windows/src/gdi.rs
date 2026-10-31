//! Windows GDI / BitBlt desktop screen capturer.
//!
//! Provides a reliable, universal capture mechanism that works on all Windows systems,
//! virtual desktops, and multi-monitor setups.
//!
//! Automatically ensures the capture thread is attached to the interactive "Default" desktop
//! if running inside an isolated runner or service desktop context.

use crate::error::CaptureError;
use std::time::Instant;
use windows::core::s;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC,
    HGDIOBJ, SRCCOPY,
};
use windows::Win32::System::StationsAndDesktops::{
    OpenDesktopA, SetThreadDesktop, DESKTOP_CONTROL_FLAGS,
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

/// Captured screen frame in 32-bit BGRA format.
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub data: Vec<u8>,
    pub timestamp: Instant,
}

/// GDI-based screen capturer.
pub struct GdiCapturer {
    width: u32,
    height: u32,
    hdc_mem: HDC,
    hbitmap: HBITMAP,
    old_obj: HGDIOBJ,
}

impl GdiCapturer {
    pub fn new() -> Result<Self, CaptureError> {
        unsafe {
            // Attempt to ensure current thread is attached to the interactive "Default" desktop.
            Self::ensure_default_desktop();

            let width = GetSystemMetrics(SM_CXSCREEN) as u32;
            let height = GetSystemMetrics(SM_CYSCREEN) as u32;

            if width == 0 || height == 0 {
                return Err(CaptureError::CaptureFailed {
                    reason: "invalid screen dimensions from GetSystemMetrics".into(),
                });
            }

            let hdc_screen = GetDC(HWND(std::ptr::null_mut()));
            if hdc_screen.is_invalid() {
                return Err(CaptureError::CaptureFailed {
                    reason: "failed to acquire desktop window DC".into(),
                });
            }

            let hdc_mem = CreateCompatibleDC(hdc_screen);
            if hdc_mem.is_invalid() {
                ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
                return Err(CaptureError::CaptureFailed {
                    reason: "failed to create compatible memory DC".into(),
                });
            }

            let hbitmap = CreateCompatibleBitmap(hdc_screen, width as i32, height as i32);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);

            if hbitmap.is_invalid() {
                let _ = DeleteDC(hdc_mem);
                return Err(CaptureError::CaptureFailed {
                    reason: "failed to create compatible bitmap".into(),
                });
            }

            let old_obj = SelectObject(hdc_mem, hbitmap);

            tracing::info!(
                width = width,
                height = height,
                "GdiCapturer initialized successfully"
            );

            Ok(Self {
                width,
                height,
                hdc_mem,
                hbitmap,
                old_obj,
            })
        }
    }

    /// Ensure the calling thread has access to the interactive desktop.
    fn ensure_default_desktop() {
        unsafe {
            let h_desk = OpenDesktopA(s!("Default"), DESKTOP_CONTROL_FLAGS(0), false, 0x01FF);
            if let Ok(h_desk) = h_desk {
                if !h_desk.is_invalid() {
                    let _ = SetThreadDesktop(h_desk);
                }
            }
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Capture a single frame from the primary display.
    pub fn capture_frame(&self) -> Result<CapturedFrame, CaptureError> {
        let timestamp = Instant::now();
        unsafe {
            Self::ensure_default_desktop();

            let hdc_screen = GetDC(HWND(std::ptr::null_mut()));
            if hdc_screen.is_invalid() {
                return Err(CaptureError::CaptureFailed {
                    reason: "failed to acquire screen DC".into(),
                });
            }

            // BitBlt desktop to memory DC
            let blt_res = BitBlt(
                self.hdc_mem,
                0,
                0,
                self.width as i32,
                self.height as i32,
                hdc_screen,
                0,
                0,
                SRCCOPY,
            );

            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);

            if let Err(e) = blt_res {
                return Err(CaptureError::CaptureFailed {
                    reason: format!("BitBlt failed: {e}"),
                });
            }

            let stride = self.width * 4;
            let buffer_size = (stride * self.height) as usize;
            let mut data = vec![0u8; buffer_size];

            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: self.width as i32,
                    // Negative biHeight specifies top-down DIB (row 0 is the top row)
                    biHeight: -(self.height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [windows::Win32::Graphics::Gdi::RGBQUAD::default(); 1],
            };

            let lines = GetDIBits(
                self.hdc_mem,
                self.hbitmap,
                0,
                self.height,
                Some(data.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            if lines == 0 {
                return Err(CaptureError::CaptureFailed {
                    reason: "GetDIBits failed to copy frame bits".into(),
                });
            }

            Ok(CapturedFrame {
                width: self.width,
                height: self.height,
                stride,
                data,
                timestamp,
            })
        }
    }
}

impl Drop for GdiCapturer {
    fn drop(&mut self) {
        unsafe {
            if !self.old_obj.is_invalid() {
                SelectObject(self.hdc_mem, self.old_obj);
            }
            if !self.hbitmap.is_invalid() {
                let _ = DeleteObject(self.hbitmap);
            }
            if !self.hdc_mem.is_invalid() {
                let _ = DeleteDC(self.hdc_mem);
            }
        }
    }
}

// Safety: The capturer owns its GDI resources and can be used across threads with appropriate synchronization.
unsafe impl Send for GdiCapturer {}
