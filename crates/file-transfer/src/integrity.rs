//! SHA-256 integrity computation for file transfers.

use ring::digest::{Context, SHA256};
use std::io::Read;

/// Compute the SHA-256 digest of a byte slice, returning a hex string.
pub fn sha256_hex(data: &[u8]) -> String {
    let digest = ring::digest::digest(&SHA256, data);
    hex_encode(digest.as_ref())
}

/// Compute the SHA-256 digest of a reader, returning a hex string.
pub fn sha256_hex_reader<R: Read>(mut reader: R) -> std::io::Result<String> {
    let mut ctx = Context::new(&SHA256);
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        ctx.update(&buf[..n]);
    }
    Ok(hex_encode(ctx.finish().as_ref()))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_value() {
        // SHA-256 of empty string
        let result = sha256_hex(b"");
        assert_eq!(
            result,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_of_hello() {
        let result = sha256_hex(b"hello");
        assert_eq!(
            result,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
