//! Platform-agnostic persistent identity storage interface.
//!
//! Platform-specific implementations (Windows DPAPI, Android Keystore)
//! live in the respective application crates. This module defines the
//! abstract trait and a file-based fallback for testing only.

use crate::error::IdentityError;

/// Persistent storage for the device's private key seed.
///
/// Implementations must encrypt or protect the stored bytes using
/// platform-appropriate mechanisms.
pub trait IdentityStore: Send + Sync {
    /// Load the raw 32-byte private key seed. Returns `None` if no
    /// identity has been stored yet.
    fn load_secret(&self) -> Result<Option<Vec<u8>>, IdentityError>;

    /// Persist the raw 32-byte private key seed.
    fn save_secret(&self, bytes: &[u8]) -> Result<(), IdentityError>;

    /// Delete the stored identity (e.g. on factory reset).
    fn delete_secret(&self) -> Result<(), IdentityError>;
}

/// File-based identity store for development and testing.
///
/// SECURITY: This store writes the key to disk in plaintext and must
/// never be used in production. Use platform secure storage instead.
#[cfg(debug_assertions)]
pub struct FileIdentityStore {
    path: std::path::PathBuf,
}

#[cfg(debug_assertions)]
impl FileIdentityStore {
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[cfg(debug_assertions)]
impl IdentityStore for FileIdentityStore {
    fn load_secret(&self) -> Result<Option<Vec<u8>>, IdentityError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&self.path).map_err(|e| IdentityError::Storage(e.to_string()))?;
        Ok(Some(bytes))
    }

    fn save_secret(&self, bytes: &[u8]) -> Result<(), IdentityError> {
        std::fs::write(&self.path, bytes).map_err(|e| IdentityError::Storage(e.to_string()))
    }

    fn delete_secret(&self) -> Result<(), IdentityError> {
        if self.path.exists() {
            std::fs::remove_file(&self.path).map_err(|e| IdentityError::Storage(e.to_string()))?;
        }
        Ok(())
    }
}
