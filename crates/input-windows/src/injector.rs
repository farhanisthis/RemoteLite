//! Win32 `SendInput` mouse and keyboard injection engine.
//!
//! Provides coordinate normalization, button dispatch, scroll wheel handling,
//! virtual keyboard mapping, Unicode character injection, and rate limiting.

use crate::error::InputError;
use crate::events::{
    InputEvent, KeyAction, KeyEvent, MouseAction, MouseButton, MouseEvent, ScrollEvent,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{trace, warn};

use windows::core::s;
use windows::Win32::System::StationsAndDesktops::{
    OpenDesktopA, OpenInputDesktop, SetThreadDesktop, DESKTOP_ACCESS_FLAGS, DESKTOP_CONTROL_FLAGS,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
    KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MAPVK_VK_TO_VSC,
    MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL, MOUSEINPUT, MOUSE_EVENT_FLAGS,
    VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN, SM_CYVIRTUALSCREEN,
};

/// Maximum allowed input events per second to prevent event flooding.
pub const DEFAULT_MAX_EVENTS_PER_SEC: u32 = 1_000;

/// Standard Windows wheel delta step.
pub const WHEEL_DELTA: f32 = 120.0;

/// Production input injector for Windows desktop.
pub struct InputInjector {
    is_active: Arc<AtomicBool>,
    max_events_per_sec: u32,
    rate_limiter: Mutex<RateLimiter>,
}

struct RateLimiter {
    window_start: Instant,
    event_count: u32,
}

impl InputInjector {
    /// Create a new input injector. By default, injection is inactive until activated.
    pub fn new() -> Self {
        Self::with_rate_limit(DEFAULT_MAX_EVENTS_PER_SEC)
    }

    /// Create an input injector with a custom events-per-second limit.
    pub fn with_rate_limit(max_events_per_sec: u32) -> Self {
        Self {
            is_active: Arc::new(AtomicBool::new(true)),
            max_events_per_sec,
            rate_limiter: Mutex::new(RateLimiter {
                window_start: Instant::now(),
                event_count: 0,
            }),
        }
    }

    /// Activate or deactivate injection (e.g. when session state changes).
    pub fn set_active(&self, active: bool) {
        self.is_active.store(active, Ordering::SeqCst);
    }

    /// Whether input injection is currently active.
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }

    /// Check rate limit. Returns Err if rate limit exceeded.
    fn check_rate_limit(&self) -> Result<(), InputError> {
        let mut limiter = self
            .rate_limiter
            .lock()
            .map_err(|_| InputError::InjectionFailed {
                reason: "Rate limiter lock poisoned".to_string(),
            })?;

        let now = Instant::now();
        if now.duration_since(limiter.window_start) >= Duration::from_secs(1) {
            limiter.window_start = now;
            limiter.event_count = 1;
            Ok(())
        } else {
            limiter.event_count += 1;
            if limiter.event_count > self.max_events_per_sec {
                Err(InputError::RateLimitExceeded {
                    count: limiter.event_count,
                    limit: self.max_events_per_sec,
                })
            } else {
                Ok(())
            }
        }
    }

    /// Query primary display resolution (width, height) in pixels.
    pub fn primary_display_resolution() -> (i32, i32) {
        unsafe {
            let w = GetSystemMetrics(SM_CXSCREEN);
            let h = GetSystemMetrics(SM_CYSCREEN);
            (w, h)
        }
    }

    /// Query full virtual desktop resolution (width, height) across all monitors.
    pub fn virtual_desktop_resolution() -> (i32, i32) {
        unsafe {
            let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            (w, h)
        }
    }

    /// Inject an arbitrary input event into the Windows desktop input queue.
    pub fn inject(&self, event: &InputEvent) -> Result<(), InputError> {
        if !self.is_active() {
            return Err(InputError::UnauthorizedSession {
                session_id: "inactive".to_string(),
            });
        }

        self.check_rate_limit()?;

        match event {
            InputEvent::Mouse(mouse) => self.inject_mouse(mouse),
            InputEvent::Key(key) => self.inject_key(key),
            InputEvent::Scroll(scroll) => self.inject_scroll(scroll),
            InputEvent::Unicode(unicode) => self.inject_unicode(&unicode.text),
        }
    }

    /// Inject mouse motion, button press, button release, or double click.
    pub fn inject_mouse(&self, mouse: &MouseEvent) -> Result<(), InputError> {
        validate_coordinates(mouse.x, mouse.y)?;

        let (dx, dy) = normalize_to_win32_coords(mouse.x, mouse.y);
        let base_flags = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK;

        let mut inputs = Vec::new();

        match (mouse.button, mouse.action) {
            (MouseButton::None, MouseAction::Move) => {
                inputs.push(make_mouse_input(dx, dy, base_flags, 0));
            }
            (MouseButton::Left, MouseAction::Move) => {
                inputs.push(make_mouse_input(dx, dy, base_flags, 0));
            }
            (MouseButton::Left, MouseAction::Down) => {
                inputs.push(make_mouse_input(
                    dx,
                    dy,
                    base_flags | MOUSEEVENTF_LEFTDOWN,
                    0,
                ));
            }
            (MouseButton::Left, MouseAction::Up) => {
                inputs.push(make_mouse_input(dx, dy, base_flags | MOUSEEVENTF_LEFTUP, 0));
            }
            (MouseButton::Left, MouseAction::DoubleClick) => {
                // First click
                inputs.push(make_mouse_input(
                    dx,
                    dy,
                    base_flags | MOUSEEVENTF_LEFTDOWN,
                    0,
                ));
                inputs.push(make_mouse_input(dx, dy, base_flags | MOUSEEVENTF_LEFTUP, 0));
                // Second click
                inputs.push(make_mouse_input(
                    dx,
                    dy,
                    base_flags | MOUSEEVENTF_LEFTDOWN,
                    0,
                ));
                inputs.push(make_mouse_input(dx, dy, base_flags | MOUSEEVENTF_LEFTUP, 0));
            }
            (MouseButton::Right, MouseAction::Down) => {
                inputs.push(make_mouse_input(
                    dx,
                    dy,
                    base_flags | MOUSEEVENTF_RIGHTDOWN,
                    0,
                ));
            }
            (MouseButton::Right, MouseAction::Up) => {
                inputs.push(make_mouse_input(
                    dx,
                    dy,
                    base_flags | MOUSEEVENTF_RIGHTUP,
                    0,
                ));
            }
            (MouseButton::Right, _) => {
                inputs.push(make_mouse_input(dx, dy, base_flags, 0));
            }
            (MouseButton::Middle, MouseAction::Down) => {
                inputs.push(make_mouse_input(
                    dx,
                    dy,
                    base_flags | MOUSEEVENTF_MIDDLEDOWN,
                    0,
                ));
            }
            (MouseButton::Middle, MouseAction::Up) => {
                inputs.push(make_mouse_input(
                    dx,
                    dy,
                    base_flags | MOUSEEVENTF_MIDDLEUP,
                    0,
                ));
            }
            (MouseButton::Middle, _) => {
                inputs.push(make_mouse_input(dx, dy, base_flags, 0));
            }
            _ => {
                inputs.push(make_mouse_input(dx, dy, base_flags, 0));
            }
        }

        send_inputs(&inputs)
    }

    /// Inject vertical or horizontal mouse wheel scroll.
    pub fn inject_scroll(&self, scroll: &ScrollEvent) -> Result<(), InputError> {
        validate_coordinates(scroll.x, scroll.y)?;
        let (dx, dy) = normalize_to_win32_coords(scroll.x, scroll.y);
        let base_flags = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK;

        let mut inputs = Vec::new();

        // Move cursor to scroll location first
        inputs.push(make_mouse_input(dx, dy, base_flags, 0));

        // Vertical scroll
        if scroll.delta_y.abs() > 0.001 {
            let wheel_data = (scroll.delta_y * WHEEL_DELTA).round() as i32;
            inputs.push(make_mouse_input(
                dx,
                dy,
                MOUSEEVENTF_WHEEL,
                wheel_data as u32,
            ));
        }

        // Horizontal scroll
        if scroll.delta_x.abs() > 0.001 {
            let hwheel_data = (scroll.delta_x * WHEEL_DELTA).round() as i32;
            inputs.push(make_mouse_input(
                dx,
                dy,
                MOUSEEVENTF_HWHEEL,
                hwheel_data as u32,
            ));
        }

        send_inputs(&inputs)
    }

    /// Inject virtual keyboard event (down or up) with extended key and scancode support.
    pub fn inject_key(&self, key: &KeyEvent) -> Result<(), InputError> {
        if key.vk_code == 0 || key.vk_code > 0xFF {
            return Err(InputError::InvalidKeyCode { code: key.vk_code });
        }

        let mut flags = KEYBD_EVENT_FLAGS(0);

        if key.action == KeyAction::Up {
            flags |= KEYEVENTF_KEYUP;
        }

        if is_extended_key(key.vk_code) {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }

        let scancode = unsafe { MapVirtualKeyW(key.vk_code, MAPVK_VK_TO_VSC) as u16 };

        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(key.vk_code as u16),
                    wScan: scancode,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        send_inputs(&[input])
    }

    /// Inject Unicode text string into the active application.
    pub fn inject_unicode(&self, text: &str) -> Result<(), InputError> {
        let mut inputs = Vec::new();

        for utf16_char in text.encode_utf16() {
            // Key Down with KEYEVENTF_UNICODE
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: utf16_char,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });

            // Key Up with KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: utf16_char,
                        dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
        }

        if !inputs.is_empty() {
            send_inputs(&inputs)?;
        }

        Ok(())
    }
}

impl Default for InputInjector {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate normalized coordinates [0.0, 1.0].
pub fn validate_coordinates(x: f32, y: f32) -> Result<(), InputError> {
    if !x.is_finite() || !y.is_finite() {
        return Err(InputError::InvalidCoordinate {
            x,
            y,
            reason: "Coordinates must be finite numbers".to_string(),
        });
    }
    if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
        return Err(InputError::InvalidCoordinate {
            x,
            y,
            reason: "Normalized coordinates must be within [0.0, 1.0]".to_string(),
        });
    }
    Ok(())
}

/// Convert normalized [0.0, 1.0] coordinates to Windows normalized [0, 65535] absolute coordinate space.
pub fn normalize_to_win32_coords(x: f32, y: f32) -> (i32, i32) {
    let clamped_x = x.clamp(0.0, 1.0);
    let clamped_y = y.clamp(0.0, 1.0);
    let dx = (clamped_x * 65535.0).round() as i32;
    let dy = (clamped_y * 65535.0).round() as i32;
    (dx, dy)
}

fn make_mouse_input(dx: i32, dy: i32, flags: MOUSE_EVENT_FLAGS, mouse_data: u32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: mouse_data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn ensure_interactive_desktop() {
    unsafe {
        // Try OpenInputDesktop first
        if let Ok(h_input) = OpenInputDesktop(
            DESKTOP_CONTROL_FLAGS(0),
            false,
            DESKTOP_ACCESS_FLAGS(0x01FF),
        ) {
            if !h_input.is_invalid() {
                let _ = SetThreadDesktop(h_input);
                return;
            }
        }
        // Fallback to "Default" desktop
        if let Ok(h_desk) = OpenDesktopA(s!("Default"), DESKTOP_CONTROL_FLAGS(0), false, 0x01FF) {
            if !h_desk.is_invalid() {
                let _ = SetThreadDesktop(h_desk);
            }
        }
    }
}

fn send_inputs(inputs: &[INPUT]) -> Result<(), InputError> {
    if inputs.is_empty() {
        return Ok(());
    }

    ensure_interactive_desktop();

    let count = inputs.len() as u32;
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };

    if sent != count {
        let err = std::io::Error::last_os_error();
        warn!(sent = sent, expected = count, error = %err, "SendInput injected fewer events than expected");
        return Err(InputError::InjectionFailed {
            reason: format!("SendInput injected {} of {} inputs: {}", sent, count, err),
        });
    }

    trace!(count = count, "SendInput successfully injected events");
    Ok(())
}

/// Extended keys (arrows, navigation, Windows key, etc.) require KEYEVENTF_EXTENDEDKEY.
fn is_extended_key(vk: u32) -> bool {
    matches!(
        vk,
        0x21..=0x28 // VK_PRIOR, VK_NEXT, VK_END, VK_HOME, VK_LEFT, VK_UP, VK_RIGHT, VK_DOWN
        | 0x2D // VK_INSERT
        | 0x2E // VK_DELETE
        | 0x5B // VK_LWIN
        | 0x5C // VK_RWIN
        | 0x5D // VK_APPS
        | 0x90 // VK_NUMLOCK
        | 0x6F // VK_DIVIDE
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_normalization_clamps_and_scales_properly() {
        assert_eq!(normalize_to_win32_coords(0.0, 0.0), (0, 0));
        assert_eq!(normalize_to_win32_coords(1.0, 1.0), (65535, 65535));
        assert_eq!(normalize_to_win32_coords(0.5, 0.5), (32768, 32768));
    }

    #[test]
    fn coordinate_validation_rejects_out_of_bounds_and_nans() {
        assert!(validate_coordinates(0.5, 0.5).is_ok());
        assert!(validate_coordinates(-0.1, 0.5).is_err());
        assert!(validate_coordinates(1.1, 0.5).is_err());
        assert!(validate_coordinates(f32::NAN, 0.5).is_err());
        assert!(validate_coordinates(0.5, f32::INFINITY).is_err());
    }

    #[test]
    fn rate_limiter_allows_up_to_limit_and_blocks_excess() {
        let injector = InputInjector::with_rate_limit(5);

        for _ in 0..5 {
            assert!(injector.check_rate_limit().is_ok());
        }

        // 6th event in the same second should exceed rate limit
        let res = injector.check_rate_limit();
        assert!(res.is_err());
        match res.unwrap_err() {
            InputError::RateLimitExceeded { count, limit } => {
                assert_eq!(count, 6);
                assert_eq!(limit, 5);
            }
            other => panic!("Unexpected error: {:?}", other),
        }
    }

    #[test]
    fn injector_inactive_rejects_all_events() {
        let injector = InputInjector::new();
        injector.set_active(false);

        let event = InputEvent::Mouse(MouseEvent {
            x: 0.5,
            y: 0.5,
            button: MouseButton::Left,
            action: MouseAction::Down,
        });

        let res = injector.inject(&event);
        assert!(res.is_err());
        match res.unwrap_err() {
            InputError::UnauthorizedSession { .. } => (),
            other => panic!("Unexpected error: {:?}", other),
        }
    }

    #[test]
    fn display_metrics_query_succeeds() {
        let (w, h) = InputInjector::primary_display_resolution();
        assert!(w >= 0);
        assert!(h >= 0);

        let (vw, vh) = InputInjector::virtual_desktop_resolution();
        assert!(vw >= 0);
        assert!(vh >= 0);
    }
}
