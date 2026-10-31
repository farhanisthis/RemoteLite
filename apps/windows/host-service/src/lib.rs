//! # OpenRemote Windows Host Service Library
//!
//! Provides the core state management, identity persistence, authorization
//! evaluation, and signaling client management for the Windows host service.

pub mod error;
pub mod pairing;
pub mod reboot;
pub mod service;

pub use error::HostServiceError;
pub use pairing::PairingManager;
pub use reboot::trigger_system_reboot;
pub use service::{HostConfig, HostService};
