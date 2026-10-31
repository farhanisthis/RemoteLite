//! # OpenRemote Protocol
//!
//! Shared protocol definitions, message types, and serialization for
//! all OpenRemote components. This crate is platform-agnostic and must
//! compile on Windows, Linux, and Android targets.

pub mod error;
pub mod input;
pub mod messages;
pub mod pairing;
pub mod session;
pub mod types;

pub use error::ProtocolError;
pub use input::{
    InputEvent, KeyAction, KeyEvent, KeyModifiers, MouseAction, MouseButton, MouseEvent,
    ScrollEvent, UnicodeTextEvent,
};
pub use types::{DeviceId, SessionId, TransferId};
