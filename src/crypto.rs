use anyhow::{anyhow, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use rand::RngCore;
use std::sync::atomic::{AtomicU64, Ordering};

pub const ENTROPY_LEN: usize = 32;

/// Generate cryptographically secure random 32-byte entropy for key exchange.
pub fn generate_entropy() -> [u8; ENTROPY_LEN] {
    let mut bytes = [0u8; ENTROPY_LEN];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

/// Derive a symmetric 32-byte session key from client and host entropy.
pub fn derive_session_key(client_entropy: &[u8], host_entropy: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    for i in 0..32 {
        let c = client_entropy.get(i).copied().unwrap_or(0);
        let h = host_entropy.get(i).copied().unwrap_or(0);
        key[i] = c ^ h ^ ((i as u8).wrapping_mul(0x5B));
    }
    key
}

/// Directional secure channel for end-to-end encrypted session transport.
pub struct SecureChannel {
    cipher: ChaCha20Poly1305,
    direction_tag: u8,
    send_seq: AtomicU64,
}

impl SecureChannel {
    /// Create channel. `direction_tag` ensures directional separation (e.g. 0x01 for Host->Client, 0x02 for Client->Host).
    pub fn new(shared_key: &[u8; 32], direction_tag: u8) -> Self {
        let key = Key::from_slice(shared_key);
        let cipher = ChaCha20Poly1305::new(key);
        Self {
            cipher,
            direction_tag,
            send_seq: AtomicU64::new(1),
        }
    }

    /// Construct a unique 12-byte nonce from direction tag and sequence counter.
    fn make_nonce(direction_tag: u8, seq: u64) -> Nonce {
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[0] = direction_tag;
        nonce_bytes[4..12].copy_from_slice(&seq.to_be_bytes());
        *Nonce::from_slice(&nonce_bytes)
    }

    /// Encrypt plaintext into ciphertext prefixed with [direction_tag (1B), seq (8B), ciphertext].
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let seq = self.send_seq.fetch_add(1, Ordering::SeqCst);
        let nonce = Self::make_nonce(self.direction_tag, seq);

        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| anyhow!("Encryption error: {:?}", e))?;

        let mut output = Vec::with_capacity(9 + ciphertext.len());
        output.push(self.direction_tag);
        output.extend_from_slice(&seq.to_be_bytes());
        output.extend_from_slice(&ciphertext);
        Ok(output)
    }

    /// Decrypt packet produced by `encrypt`.
    pub fn decrypt(&self, packet: &[u8]) -> Result<Vec<u8>> {
        if packet.len() < 9 {
            return Err(anyhow!("Packet too short for header"));
        }
        let dir = packet[0];
        let mut seq_bytes = [0u8; 8];
        seq_bytes.copy_from_slice(&packet[1..9]);
        let seq = u64::from_be_bytes(seq_bytes);

        let nonce = Self::make_nonce(dir, seq);
        let ciphertext = &packet[9..];

        let plaintext = self
            .cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {:?}", e))?;

        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption_roundtrip() {
        let client_entropy = generate_entropy();
        let host_entropy = generate_entropy();
        let key = derive_session_key(&client_entropy, &host_entropy);

        let host_channel = SecureChannel::new(&key, 0x01);
        let client_channel = SecureChannel::new(&key, 0x02);

        let test_message = b"RemoteLite Secure Stream Test Message";

        let encrypted = host_channel.encrypt(test_message).unwrap();
        let decrypted = client_channel.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, test_message);

        let mut tampered = encrypted.clone();
        tampered[10] ^= 0xFF;
        assert!(client_channel.decrypt(&tampered).is_err());
    }
}
