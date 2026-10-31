//! # OpenRemote Authorization
//!
//! Authorization checks, trusted device registry interface, and
//! permission evaluation for OpenRemote.
//!
//! SECURITY INVARIANTS:
//! - Unattended access is disabled by default.
//! - Authorization is always device-specific.
//! - Revocation takes effect immediately.
//! - Every authorization decision is logged.

pub mod error;
pub mod evaluator;
pub mod file_registry;
pub mod registry;

pub use error::AuthorizationError;
pub use evaluator::{AuthDecision, AuthorizationEvaluator};
pub use file_registry::FileRegistry;
pub use registry::TrustedDeviceRegistry;
