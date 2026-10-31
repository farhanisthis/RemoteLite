//! Platform-agnostic persistent identity storage interface.
//!
//! Provides Windows DPAPI secure storage for private keys at rest,
//! and a fallback file store for development/testing environments.

use crate::error::IdentityError;
use std::path::PathBuf;

/// Persistent storage for the device's private key seed.
pub trait IdentityStore: Send + Sync {
    /// Load the raw 32-byte private key seed. Returns `None` if no
    /// identity has been stored yet.
    fn load_secret(&self) -> Result<Option<Vec<u8>>, IdentityError>;

    /// Persist the raw 32-byte private key seed.
    fn save_secret(&self, bytes: &[u8]) -> Result<(), IdentityError>;

    /// Delete the stored identity (e.g. on factory reset).
    fn delete_secret(&self) -> Result<(), IdentityError>;
}

/// Windows DPAPI-encrypted identity store.
/// Encrypts private key bytes with Windows DPAPI (`CryptProtectData`) at rest.
#[cfg(windows)]
pub struct DpapiIdentityStore {
    path: PathBuf,
}

#[cfg(windows)]
impl DpapiIdentityStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[cfg(windows)]
impl IdentityStore for DpapiIdentityStore {
    fn load_secret(&self) -> Result<Option<Vec<u8>>, IdentityError> {
        use windows::Win32::Foundation::{LocalFree, HLOCAL};
        use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

        if !self.path.exists() {
            return Ok(None);
        }

        let encrypted =
            std::fs::read(&self.path).map_err(|e| IdentityError::Storage(e.to_string()))?;

        if encrypted.is_empty() {
            return Ok(None);
        }

        unsafe {
            let data_in = CRYPT_INTEGER_BLOB {
                cbData: encrypted.len() as u32,
                pbData: encrypted.as_ptr() as *mut u8,
            };
            let mut data_out = CRYPT_INTEGER_BLOB::default();

            CryptUnprotectData(&data_in, None, None, None, None, 0, &mut data_out)
                .map_err(|e| IdentityError::Storage(format!("CryptUnprotectData failed: {e}")))?;

            let secret =
                std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec();
            let _ = LocalFree(HLOCAL(data_out.pbData as _));
            Ok(Some(secret))
        }
    }

    fn save_secret(&self, bytes: &[u8]) -> Result<(), IdentityError> {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{LocalFree, HLOCAL};
        use windows::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};

        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        unsafe {
            let data_in = CRYPT_INTEGER_BLOB {
                cbData: bytes.len() as u32,
                pbData: bytes.as_ptr() as *mut u8,
            };
            let mut data_out = CRYPT_INTEGER_BLOB::default();

            CryptProtectData(&data_in, PCWSTR::null(), None, None, None, 0, &mut data_out)
                .map_err(|e| IdentityError::Storage(format!("CryptProtectData failed: {e}")))?;

            let encrypted =
                std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec();
            let _ = LocalFree(HLOCAL(data_out.pbData as _));

            std::fs::write(&self.path, encrypted)
                .map_err(|e| IdentityError::Storage(e.to_string()))?;
            Ok(())
        }
    }

    fn delete_secret(&self) -> Result<(), IdentityError> {
        if self.path.exists() {
            std::fs::remove_file(&self.path).map_err(|e| IdentityError::Storage(e.to_string()))?;
        }
        Ok(())
    }
}

/// File-based identity store for development and testing.
pub struct FileIdentityStore {
    path: PathBuf,
}

impl FileIdentityStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl IdentityStore for FileIdentityStore {
    fn load_secret(&self) -> Result<Option<Vec<u8>>, IdentityError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&self.path).map_err(|e| IdentityError::Storage(e.to_string()))?;
        Ok(Some(bytes))
    }

    fn save_secret(&self, bytes: &[u8]) -> Result<(), IdentityError> {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&self.path, bytes).map_err(|e| IdentityError::Storage(e.to_string()))
    }

    fn delete_secret(&self) -> Result<(), IdentityError> {
        if self.path.exists() {
            std::fs::remove_file(&self.path).map_err(|e| IdentityError::Storage(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_store_round_trip() {
        let temp_dir = std::env::temp_dir().join(format!("identity_test_{}", uuid::Uuid::new_v4()));
        let key_path = temp_dir.join("test_key.seed");
        let store = FileIdentityStore::new(&key_path);

        assert_eq!(store.load_secret().unwrap(), None);

        let secret = [42u8; 32];
        store.save_secret(&secret).unwrap();

        let loaded = store.load_secret().unwrap().expect("must be found");
        assert_eq!(loaded, secret);

        store.delete_secret().unwrap();
        assert_eq!(store.load_secret().unwrap(), None);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[cfg(windows)]
    #[test]
    fn dpapi_store_round_trip() {
        let temp_dir = std::env::temp_dir().join(format!("dpapi_test_{}", uuid::Uuid::new_v4()));
        let key_path = temp_dir.join("dpapi_key.dat");
        let store = DpapiIdentityStore::new(&key_path);

        assert_eq!(store.load_secret().unwrap(), None);

        let secret = [99u8; 32];
        store.save_secret(&secret).unwrap();

        // Raw file on disk is encrypted, not equal to plaintext secret
        let raw_disk = std::fs::read(&key_path).unwrap();
        assert_ne!(raw_disk, secret);

        // Unprotect recovers original bytes
        let loaded = store.load_secret().unwrap().expect("must decrypt");
        assert_eq!(loaded, secret);

        store.delete_secret().unwrap();
        assert_eq!(store.load_secret().unwrap(), None);

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
