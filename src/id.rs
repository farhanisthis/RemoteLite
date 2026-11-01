use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A persistent, unique 9-digit device identifier.
/// Formatted for display as "123 456 789".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(String);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DeviceIdError {
    #[error("Device ID must contain exactly 9 digits")]
    InvalidLength,
    #[error("Device ID must contain only digits")]
    InvalidCharacters,
    #[error("Device ID cannot start with 0")]
    LeadingZero,
}

impl DeviceId {
    /// Generate a new cryptographically random 9-digit device ID (100,000,000 to 999,999,999).
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let num: u32 = rng.gen_range(100_000_000..=999_999_999);
        Self(num.to_string())
    }

    /// Create from a raw 9-digit string.
    pub fn new(raw: &str) -> Result<Self, DeviceIdError> {
        let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 9 {
            return Err(DeviceIdError::InvalidLength);
        }
        if digits.starts_with('0') {
            return Err(DeviceIdError::LeadingZero);
        }
        Ok(Self(digits))
    }

    /// Return raw 9-digit string (e.g. "123456789").
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Formatted with spaces for prominent display (e.g. "123 456 789").
    pub fn formatted(&self) -> String {
        format!(
            "{} {} {}",
            &self.0[0..3],
            &self.0[3..6],
            &self.0[6..9]
        )
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.formatted())
    }
}

impl FromStr for DeviceId {
    type Err = DeviceIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_id_generate() {
        let id = DeviceId::generate();
        assert_eq!(id.as_str().len(), 9);
        assert!(!id.as_str().starts_with('0'));
        assert_eq!(id.formatted().len(), 11); // 9 digits + 2 spaces
    }

    #[test]
    fn test_device_id_parse_and_format() {
        let id = DeviceId::new("123456789").unwrap();
        assert_eq!(id.formatted(), "123 456 789");

        let id_with_spaces = DeviceId::new("123 456 789").unwrap();
        assert_eq!(id_with_spaces.as_str(), "123456789");

        let id_with_dashes = DeviceId::new("123-456-789").unwrap();
        assert_eq!(id_with_dashes.as_str(), "123456789");
    }

    #[test]
    fn test_device_id_validation_errors() {
        assert_eq!(DeviceId::new("123"), Err(DeviceIdError::InvalidLength));
        assert_eq!(DeviceId::new("012345678"), Err(DeviceIdError::LeadingZero));
        assert_eq!(DeviceId::new("1234567890"), Err(DeviceIdError::InvalidLength));
    }
}
