//! # OpenRemote Input (Windows)
//!
//! Mouse and keyboard input injection using the Windows SendInput API.
//!
//! M0: Stub. Real input injection is implemented in M2.
//!
//! SECURITY: Input injection is only permitted during active authorized
//! sessions. The session agent validates authorization before invoking
//! any input operations.

#[cfg(not(windows))]
compile_error!("openremote-input-windows only supports Windows targets");

pub mod error;
pub mod events;

pub use error::InputError;
pub use events::{InputEvent, KeyEvent, MouseEvent};
