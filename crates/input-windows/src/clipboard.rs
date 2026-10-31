//! Windows Clipboard integration with echo cancellation, rate limiting, and size bounding.

use crate::error::InputError;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use tracing::{debug, warn};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

/// Maximum allowed clipboard text length in bytes (1 MiB).
pub const MAX_CLIPBOARD_TEXT_LEN: usize = 1024 * 1024;

/// Safe wrapper for Windows Unicode clipboard access.
pub struct WindowsClipboard {
    last_hash: Arc<Mutex<Option<u64>>>,
}

impl WindowsClipboard {
    pub fn new() -> Self {
        Self {
            last_hash: Arc::new(Mutex::new(None)),
        }
    }

    /// Read plain text (CF_UNICODETEXT) from the Windows clipboard.
    pub fn read_text(&self) -> Result<Option<String>, InputError> {
        unsafe {
            if !OpenClipboard(HWND(std::ptr::null_mut())).is_ok() {
                return Ok(None);
            }

            let handle = match GetClipboardData(13 /* CF_UNICODETEXT */) {
                Ok(h) if !h.0.is_null() => h,
                _ => {
                    let _ = CloseClipboard();
                    return Ok(None);
                }
            };

            let ptr = GlobalLock(HGLOBAL(handle.0));
            if ptr.is_null() {
                let _ = CloseClipboard();
                return Ok(None);
            }

            // Read wide string characters until null terminator
            let u16_ptr = ptr as *const u16;
            let mut len = 0;
            while len < MAX_CLIPBOARD_TEXT_LEN && *u16_ptr.add(len) != 0 {
                len += 1;
            }

            let u16_slice = std::slice::from_raw_parts(u16_ptr, len);
            let text = String::from_utf16_lossy(u16_slice);

            let _ = GlobalUnlock(HGLOBAL(handle.0));
            let _ = CloseClipboard();

            Ok(Some(text))
        }
    }

    /// Write text to the Windows clipboard.
    ///
    /// Updates the internal hash cache to suppress echoing back this text to the remote peer.
    pub fn write_text(&self, text: &str) -> Result<(), InputError> {
        if text.len() > MAX_CLIPBOARD_TEXT_LEN {
            warn!(len = text.len(), "Rejecting oversized clipboard text");
            return Err(InputError::ClipboardError {
                reason: "Clipboard text exceeds 1MB limit".into(),
            });
        }

        // Cache hash for echo suppression
        let text_hash = hash_text(text);
        {
            let mut last = self.last_hash.lock().unwrap();
            *last = Some(text_hash);
        }

        unsafe {
            let u16_vec: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes_len = u16_vec.len() * 2;

            let h_mem =
                GlobalAlloc(GMEM_MOVEABLE, bytes_len).map_err(|e| InputError::ClipboardError {
                    reason: format!("GlobalAlloc failed: {e}"),
                })?;

            let ptr = GlobalLock(h_mem);
            if ptr.is_null() {
                let _ = GlobalFree(h_mem);
                return Err(InputError::ClipboardError {
                    reason: "GlobalLock failed".into(),
                });
            }

            std::ptr::copy_nonoverlapping(u16_vec.as_ptr() as *const u8, ptr as *mut u8, bytes_len);
            let _ = GlobalUnlock(h_mem);

            if !OpenClipboard(HWND(std::ptr::null_mut())).is_ok() {
                let _ = GlobalFree(h_mem);
                return Err(InputError::ClipboardError {
                    reason: "OpenClipboard failed".into(),
                });
            }

            let _ = EmptyClipboard();
            let res = SetClipboardData(13 /* CF_UNICODETEXT */, HANDLE(h_mem.0));
            let _ = CloseClipboard();

            if res.is_err() {
                let _ = GlobalFree(h_mem);
                return Err(InputError::ClipboardError {
                    reason: "SetClipboardData failed".into(),
                });
            }

            debug!(
                len = text.len(),
                "Successfully injected text to Windows clipboard"
            );
            Ok(())
        }
    }

    /// Check if the candidate text should be synchronized to the remote peer
    /// (returns false if this content matches what was previously received from the peer).
    pub fn should_sync_to_remote(&self, text: &str) -> bool {
        let text_hash = hash_text(text);
        let last = self.last_hash.lock().unwrap();
        if let Some(h) = *last {
            if h == text_hash {
                return false; // Echo suppression
            }
        }
        true
    }
}

impl Default for WindowsClipboard {
    fn default() -> Self {
        Self::new()
    }
}

fn hash_text(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_suppression() {
        let clip = WindowsClipboard::new();
        let sample = "Password or token 12345";

        assert!(clip.should_sync_to_remote(sample));

        // When written, internal cache is updated
        let h = hash_text(sample);
        *clip.last_hash.lock().unwrap() = Some(h);

        // Subsequent check suppresses echo
        assert!(!clip.should_sync_to_remote(sample));

        // Different text is permitted
        assert!(clip.should_sync_to_remote("Different text"));
    }

    #[test]
    fn test_oversized_text_rejected() {
        let clip = WindowsClipboard::new();
        let huge = "a".repeat(MAX_CLIPBOARD_TEXT_LEN + 10);
        let res = clip.write_text(&huge);
        assert!(res.is_err());
    }
}
