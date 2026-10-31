//! # OpenRemote Windows Host Service Library
//!
//! Provides the core state management, identity persistence, authorization
//! evaluation, and signaling client management for the Windows host service.

pub mod error;
pub mod pairing;
pub mod service;

pub use error::HostServiceError;
pub use pairing::PairingManager;
pub use service::{HostConfig, HostService};
