//! # OpenRemote File Transfer
//!
//! Bidirectional file transfer with integrity verification.
//!
//! SECURITY:
//! - Filenames are sanitized before use.
//! - Path traversal is rejected.
//! - Directory access requires explicit user permission.
//! - File integrity is verified with SHA-256 after transfer.

pub mod error;
pub mod integrity;
pub mod path_safety;

pub use error::TransferError;

/// Chunk size for streaming transfers: 256 KiB.
pub const CHUNK_SIZE: usize = 256 * 1024;

/// Maximum single-file transfer size in the MVP: 4 GiB.
pub const MAX_FILE_SIZE: u64 = 4 * 1024 * 1024 * 1024;
