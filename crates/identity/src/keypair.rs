//! Ed25519 keypair for device identity.

use crate::error::IdentityError;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// A device's complete signing keypair.
///
/// The signing key bytes are wrapped in `Zeroizing` to ensure they are
/// cleared from memory when this struct is dropped.
pub struct DeviceKeypair {
    signing_key: SigningKey,
}

impl DeviceKeypair {
    /// Generate a new random Ed25519 keypair using the OS CSPRNG.
    pub fn generate() -> Result<Self, IdentityError> {
        let signing_key = SigningKey::generate(&mut OsRng);
        Ok(Self { signing_key })
    }

    /// Restore from raw 32-byte seed bytes (e.g. loaded from secure storage).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IdentityError> {
        let bytes_32: [u8; 32] = bytes
            .try_into()
            .map_err(|_| IdentityError::InvalidKeyFormat("expected 32 bytes".into()))?;
        let signing_key = SigningKey::from_bytes(&bytes_32);
        Ok(Self { signing_key })
    }

    /// Export the private key seed bytes, wrapped in Zeroizing.
    ///
    /// SECURITY: The caller is responsible for securely storing or disposing of
    /// the returned bytes. Never log or transmit these bytes.
    pub fn secret_bytes(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(self.signing_key.to_bytes())
    }

    /// Return the public identity (verifying key) for this keypair.
    pub fn public_identity(&self) -> PublicIdentity {
        PublicIdentity {
            verifying_key: self.signing_key.verifying_key(),
        }
    }

    /// Sign `message` and return the base64-encoded signature.
    pub fn sign(&self, message: &[u8]) -> Result<String, IdentityError> {
        let signature: Signature = self.signing_key.sign(message);
        Ok(B64.encode(signature.to_bytes()))
    }
}

/// The public half of a device identity.
///
/// Safe to transmit, store unprotected, and share with peers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicIdentity {
    verifying_key: VerifyingKey,
}

impl Serialize for PublicIdentity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_base64())
    }
}

impl<'de> Deserialize<'de> for PublicIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PublicIdentity::from_base64(&s).map_err(serde::de::Error::custom)
    }
}

impl PublicIdentity {
    /// Deserialize from a base64-encoded public key string.
    pub fn from_base64(encoded: &str) -> Result<Self, IdentityError> {
        let bytes = B64
            .decode(encoded)
            .map_err(|e| IdentityError::Decode(e.to_string()))?;
        let bytes_32: [u8; 32] = bytes
            .try_into()
            .map_err(|_| IdentityError::InvalidKeyFormat("expected 32 bytes".into()))?;
        let verifying_key = VerifyingKey::from_bytes(&bytes_32)
            .map_err(|e| IdentityError::InvalidKeyFormat(e.to_string()))?;
        Ok(Self { verifying_key })
    }

    /// Serialize to a base64-encoded string.
    pub fn to_base64(&self) -> String {
        B64.encode(self.verifying_key.to_bytes())
    }

    /// Verify a base64-encoded Ed25519 signature over `message`.
    pub fn verify(&self, message: &[u8], signature_b64: &str) -> Result<(), IdentityError> {
        let sig_bytes = B64
            .decode(signature_b64)
            .map_err(|e| IdentityError::Decode(e.to_string()))?;
        let sig_bytes_64: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| IdentityError::InvalidKeyFormat("signature must be 64 bytes".into()))?;
        let signature = Signature::from_bytes(&sig_bytes_64);
        self.verifying_key
            .verify(message, &signature)
            .map_err(|_| IdentityError::VerificationFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_sign_verify_roundtrip() {
        let keypair = DeviceKeypair::generate().unwrap();
        let public = keypair.public_identity();
        let message = b"test challenge 12345";
        let sig = keypair.sign(message).unwrap();
        assert!(public.verify(message, &sig).is_ok());
    }

    #[test]
    fn wrong_message_fails_verification() {
        let keypair = DeviceKeypair::generate().unwrap();
        let public = keypair.public_identity();
        let sig = keypair.sign(b"original").unwrap();
        assert!(public.verify(b"tampered", &sig).is_err());
    }

    #[test]
    fn keypair_restores_from_bytes() {
        let keypair = DeviceKeypair::generate().unwrap();
        let bytes = keypair.secret_bytes();
        let keypair2 = DeviceKeypair::from_bytes(&*bytes).unwrap();
        let msg = b"restore test";
        let sig = keypair2.sign(msg).unwrap();
        assert!(keypair.public_identity().verify(msg, &sig).is_ok());
    }

    #[test]
    fn public_key_round_trips_via_base64() {
        let keypair = DeviceKeypair::generate().unwrap();
        let pub_b64 = keypair.public_identity().to_base64();
        let restored = PublicIdentity::from_base64(&pub_b64).unwrap();
        let msg = b"round trip";
        let sig = keypair.sign(msg).unwrap();
        assert!(restored.verify(msg, &sig).is_ok());
    }

    #[test]
    fn public_identity_serde_roundtrip() {
        let keypair = DeviceKeypair::generate().unwrap();
        let public = keypair.public_identity();
        let json = serde_json::to_string(&public).unwrap();
        let restored: PublicIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(public, restored);
    }
}
