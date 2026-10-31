//! Windows DXGI Desktop Duplication capture using Direct3D 11.
//!
//! Provides zero-copy GPU screen capture when hardware acceleration and
//! desktop session output duplication are supported by the display adapter.

use crate::error::CaptureError;
use crate::gdi::CapturedFrame;
use std::time::Instant;
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ,
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{
    IDXGIDevice, IDXGIOutput1, IDXGIOutputDuplication, DXGI_OUTDUPL_FRAME_INFO,
};

pub struct DxgiCapturer {
    #[allow(dead_code)]
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    duplication: IDXGIOutputDuplication,
    staging_texture: ID3D11Texture2D,
    width: u32,
    height: u32,
}

impl DxgiCapturer {
    pub fn new() -> Result<Self, CaptureError> {
        unsafe {
            let mut device = None;
            let mut context = None;

            // Try hardware Direct3D 11 device first, fallback to WARP (software rasterizer)
            let hr = D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            );

            if hr.is_err() {
                D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_WARP,
                    None,
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    None,
                    D3D11_SDK_VERSION,
                    Some(&mut device),
                    None,
                    Some(&mut context),
                )
                .map_err(|e| CaptureError::CaptureFailed {
                    reason: format!("failed to create Direct3D 11 device: {e}"),
                })?;
            }

            let device = device.ok_or_else(|| CaptureError::CaptureFailed {
                reason: "Direct3D 11 device is null".into(),
            })?;
            let context = context.ok_or_else(|| CaptureError::CaptureFailed {
                reason: "Direct3D 11 context is null".into(),
            })?;

            let dxgi_device: IDXGIDevice =
                device.cast().map_err(|e| CaptureError::CaptureFailed {
                    reason: format!("failed to cast to IDXGIDevice: {e}"),
                })?;

            let adapter = dxgi_device
                .GetAdapter()
                .map_err(|e| CaptureError::CaptureFailed {
                    reason: format!("failed to get DXGI adapter: {e}"),
                })?;

            let output = adapter
                .EnumOutputs(0)
                .map_err(|e| CaptureError::CaptureFailed {
                    reason: format!("failed to enumerate DXGI output 0: {e}"),
                })?;

            let output1: IDXGIOutput1 = output.cast().map_err(|e| CaptureError::CaptureFailed {
                reason: format!("failed to cast to IDXGIOutput1: {e}"),
            })?;

            let duplication =
                output1
                    .DuplicateOutput(&device)
                    .map_err(|e| CaptureError::BackendUnavailable {
                        backend: format!("DXGI DuplicateOutput failed: {e}"),
                    })?;

            let desc = output.GetDesc().map_err(|e| CaptureError::CaptureFailed {
                reason: format!("failed to get output desc: {e}"),
            })?;

            let width =
                (desc.DesktopCoordinates.right - desc.DesktopCoordinates.left).unsigned_abs();
            let height =
                (desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top).unsigned_abs();

            let texture_desc = D3D11_TEXTURE2D_DESC {
                Width: width,
                Height: height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };

            let mut staging_texture = None;
            device
                .CreateTexture2D(&texture_desc, None, Some(&mut staging_texture))
                .map_err(|e| CaptureError::CaptureFailed {
                    reason: format!("failed to create staging texture: {e}"),
                })?;

            let staging_texture = staging_texture.ok_or_else(|| CaptureError::CaptureFailed {
                reason: "staging texture is null".into(),
            })?;

            tracing::info!(
                width = width,
                height = height,
                "DxgiCapturer initialized successfully"
            );

            Ok(Self {
                device,
                context,
                duplication,
                staging_texture,
                width,
                height,
            })
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn capture_frame(&self) -> Result<CapturedFrame, CaptureError> {
        let timestamp = Instant::now();
        unsafe {
            let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut desktop_resource = None;

            // Wait up to 50ms for a new desktop frame
            let hr = self
                .duplication
                .AcquireNextFrame(50, &mut frame_info, &mut desktop_resource);

            if let Err(e) = hr {
                return Err(CaptureError::CaptureFailed {
                    reason: format!("AcquireNextFrame failed: {e}"),
                });
            }

            let desktop_resource = desktop_resource.ok_or_else(|| CaptureError::CaptureFailed {
                reason: "desktop resource is null".into(),
            })?;

            let desktop_texture: ID3D11Texture2D = desktop_resource.cast().map_err(|e| {
                let _ = self.duplication.ReleaseFrame();
                CaptureError::CaptureFailed {
                    reason: format!("failed to cast resource to ID3D11Texture2D: {e}"),
                }
            })?;

            // Copy captured GPU texture to CPU-readable staging texture
            self.context
                .CopyResource(&self.staging_texture, &desktop_texture);
            let _ = self.duplication.ReleaseFrame();

            // Map CPU staging texture
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            self.context
                .Map(
                    &self.staging_texture,
                    0,
                    D3D11_MAP_READ,
                    0,
                    Some(&mut mapped),
                )
                .map_err(|e| CaptureError::CaptureFailed {
                    reason: format!("failed to map staging texture: {e}"),
                })?;

            let stride = self.width * 4;
            let mut data = vec![0u8; (stride * self.height) as usize];

            let src_pitch = mapped.RowPitch as usize;
            let dst_pitch = stride as usize;
            let src_ptr = mapped.pData as *const u8;

            for row in 0..self.height as usize {
                let src_row = src_ptr.add(row * src_pitch);
                let dst_row = &mut data[row * dst_pitch..(row + 1) * dst_pitch];
                std::ptr::copy_nonoverlapping(src_row, dst_row.as_mut_ptr(), dst_pitch);
            }

            self.context.Unmap(&self.staging_texture, 0);

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

unsafe impl Send for DxgiCapturer {}
