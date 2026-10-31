//! Path safety checks to prevent traversal and unsafe filenames.

use crate::error::TransferError;
use std::path::{Component, Path, PathBuf};

/// Validate and sanitize an incoming filename.
///
/// Returns an error if the filename:
/// - Is empty.
/// - Contains path separators.
/// - Contains null bytes.
/// - Is a reserved Windows name (e.g. CON, NUL, AUX).
/// - Contains `.` or `..` components.
pub fn sanitize_filename(name: &str) -> Result<String, TransferError> {
    if name.is_empty() {
        return Err(TransferError::InvalidFilename { name: name.into() });
    }
    if name.contains('\0') || name.contains('/') || name.contains('\\') {
        return Err(TransferError::InvalidFilename { name: name.into() });
    }
    // Reject Windows reserved names
    let stem = name.split('.').next().unwrap_or("").to_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if reserved.contains(&stem.as_str()) {
        return Err(TransferError::InvalidFilename { name: name.into() });
    }
    Ok(name.to_string())
}

/// Resolve `filename` under `base_dir`, rejecting any path traversal.
///
/// Returns the final absolute path only if it remains within `base_dir`.
pub fn resolve_safe_path(base_dir: &Path, filename: &str) -> Result<PathBuf, TransferError> {
    let sanitized = sanitize_filename(filename)?;
    let candidate = base_dir.join(&sanitized);

    // Walk components to reject any .. or .
    for component in candidate.components() {
        match component {
            Component::ParentDir | Component::CurDir => {
                return Err(TransferError::PathTraversal {
                    path: filename.into(),
                });
            }
            _ => {}
        }
    }

    let base_canonical = base_dir
        .canonicalize()
        .unwrap_or_else(|_| base_dir.to_path_buf());
    let base_str = base_canonical.to_string_lossy();
    let clean_base = base_str.strip_prefix(r"\\?\").unwrap_or(&base_str);

    let candidate_str = candidate.to_string_lossy();
    let clean_candidate = candidate_str
        .strip_prefix(r"\\?\")
        .unwrap_or(&candidate_str);

    let clean_base_path = Path::new(clean_base);
    let clean_cand_path = Path::new(clean_candidate);

    if !clean_cand_path.starts_with(clean_base_path) && !candidate.starts_with(base_dir) {
        return Err(TransferError::PathTraversal {
            path: candidate.to_string_lossy().into(),
        });
    }

    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_filename_passes() {
        assert!(sanitize_filename("document.pdf").is_ok());
        assert!(sanitize_filename("photo-2024.jpg").is_ok());
    }

    #[test]
    fn traversal_in_filename_rejected() {
        assert!(sanitize_filename("../secret.txt").is_err());
        assert!(sanitize_filename("dir/file.txt").is_err());
        assert!(sanitize_filename("dir\\file.txt").is_err());
    }

    #[test]
    fn null_byte_rejected() {
        assert!(sanitize_filename("file\0.txt").is_err());
    }

    #[test]
    fn reserved_windows_names_rejected() {
        assert!(sanitize_filename("CON").is_err());
        assert!(sanitize_filename("NUL.txt").is_err());
        assert!(sanitize_filename("COM1").is_err());
    }

    #[test]
    fn empty_filename_rejected() {
        assert!(sanitize_filename("").is_err());
    }
}
