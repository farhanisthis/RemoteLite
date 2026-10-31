//! # OpenRemote Input (Windows)
//!
//! Production-grade mouse and keyboard input injection using the Windows SendInput API.
//!
//! ## Security Design
//! - Input injection is permitted only during active, authorized sessions.
//! - Normalized coordinates are rigorously validated and clamped to `[0.0, 1.0]`.
//! - Input frequency is bounded by a rate limiter to eliminate flooding.
//! - Runs strictly in the interactive session agent desktop context, never in session 0.
//! - No sensitive keystroke contents are recorded in trace logs.

#[cfg(not(windows))]
compile_error!("openremote-input-windows only supports Windows targets");

pub mod clipboard;
pub mod error;
pub mod events;
pub mod injector;

pub use clipboard::WindowsClipboard;
pub use error::InputError;
pub use events::{
    InputEvent, KeyAction, KeyEvent, KeyModifiers, MouseAction, MouseButton, MouseEvent,
    ScrollEvent, UnicodeTextEvent,
};
pub use injector::{normalize_to_win32_coords, validate_coordinates, InputInjector};

/// Convenience function to inject an input event using a standard injector.
pub fn inject_input(event: &InputEvent) -> Result<(), InputError> {
    static INJECTOR: std::sync::OnceLock<InputInjector> = std::sync::OnceLock::new();
    let injector = INJECTOR.get_or_init(InputInjector::new);
    injector.inject(event)
}
