use crate::error::{DomainError, DomainResult};
use std::fmt;

/// Represents a resource version string (e.g., "24-09-23-11-27-19-c6564b")
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResVersion(String);

impl ResVersion {
    /// Create a new resource version
    pub fn new(version: &str) -> DomainResult<Self> {
        if version.trim().is_empty() {
            return Err(DomainError::InvalidVersionFormat {
                message: "Resource version cannot be empty".to_string(),
            });
        }

        if version.len() > 32 {
            return Err(DomainError::InvalidVersionFormat {
                message: "Resource version too long (max 32 characters)".to_string(),
            });
        }

        // Basic validation - alphanumeric and hyphens only
        if !version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(DomainError::InvalidVersionFormat {
                message: "Resource version contains invalid characters".to_string(),
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

impl fmt::Display for ResVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ResVersion {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_res_version() {
        let version = ResVersion::new("24-09-23-11-27-19-c6564b").unwrap();
        assert_eq!(version.as_str(), "24-09-23-11-27-19-c6564b");
    }

    #[test]
    fn test_empty_res_version() {
        assert!(ResVersion::new("").is_err());
        assert!(ResVersion::new("   ").is_err());
    }

    #[test]
    fn test_long_res_version() {
        let long_version = "a".repeat(33);
        assert!(ResVersion::new(&long_version).is_err());
    }

    #[test]
    fn test_invalid_characters() {
        assert!(ResVersion::new("24/09/23-11:27:19").is_err());
        assert!(ResVersion::new("version@1.0").is_err());
    }
}
