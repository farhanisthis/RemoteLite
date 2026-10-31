//! # OpenRemote Session
//!
//! Session lifecycle and state management.

pub mod error;
pub mod manager;

pub use error::SessionError;
pub use manager::SessionManager;
