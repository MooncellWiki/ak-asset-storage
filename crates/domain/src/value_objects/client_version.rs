use crate::error::{DomainError, DomainResult};
use std::fmt;

/// Represents a client version string (e.g., "2.3.61")
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientVersion(String);

impl ClientVersion {
    /// Create a new client version
    pub fn new(version: &str) -> DomainResult<Self> {
        if version.trim().is_empty() {
            return Err(DomainError::InvalidValue {
                message: "Client version cannot be empty".to_string(),
            });
        }

        if version.len() > 32 {
            return Err(DomainError::InvalidValue {
                message: "Client version too long (max 32 characters)".to_string(),
            });
        }

        // Basic semantic version validation (x.y.z format)
        if !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return Err(DomainError::InvalidValue {
                message: "Client version contains invalid characters".to_string(),
            });
        }

        Ok(Self(version.to_string()))
    }

    /// Get the version string
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ClientVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ClientVersion {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&ClientVersion> for String {
    fn from(version: &ClientVersion) -> Self {
        version.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_client_version() {
        let version = ClientVersion::new("2.3.61").unwrap();
        assert_eq!(version.as_str(), "2.3.61");
    }

    #[test]
    fn test_empty_client_version() {
        assert!(ClientVersion::new("").is_err());
        assert!(ClientVersion::new("   ").is_err());
    }

    #[test]
    fn test_long_client_version() {
        let long_version = "1".repeat(33);
        assert!(ClientVersion::new(&long_version).is_err());
    }

    #[test]
    fn test_invalid_characters() {
        assert!(ClientVersion::new("2.3.61-alpha").is_err());
        assert!(ClientVersion::new("v2.3.61").is_err());
    }
}
