//! # OpenRemote Identity
//!
//! Per-device asymmetric identity management.
//!
//! Each device (Windows host or Android client) has a persistent Ed25519
//! keypair that serves as its cryptographic identity. Private keys are
//! stored in platform-appropriate secure storage; this crate provides the
//! abstract interface and the cryptographic operations.
//!
//! SECURITY:
//! - Private keys are never transmitted.
//! - Private key bytes are zeroed on drop (via zeroize).
//! - This crate does not implement any custom cryptographic algorithms.

pub mod error;
pub mod keypair;
pub mod store;

pub use error::IdentityError;
pub use keypair::{DeviceKeypair, PublicIdentity};
#[cfg(windows)]
pub use store::DpapiIdentityStore;
pub use store::{FileIdentityStore, IdentityStore};
