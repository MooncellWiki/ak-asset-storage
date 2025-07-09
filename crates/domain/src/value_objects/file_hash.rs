use crate::error::{DomainError, DomainResult};

/// Represents a SHA-256 file hash
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHash(String);

impl FileHash {
    /// Create a new file hash
    pub fn new(hash: &str) -> DomainResult<Self> {
        if hash.len() != 64 {
            return Err(DomainError::InvalidFileHash {
                message: "File hash must be exactly 64 characters".to_string(),
            });
        }

        if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(DomainError::InvalidFileHash {
                message: "File hash must contain only hexadecimal characters".to_string(),
            });
        }

        Ok(Self(hash.to_string()))
    }

    /// Get the hash string
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate the file hash
    pub fn validate(&self) -> DomainResult<()> {
        if self.0.len() != 64 {
            return Err(DomainError::InvalidFileHash {
                message: "File hash must be exactly 64 characters".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_file_hash() {
        let hash =
            FileHash::new("a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456")
                .unwrap();
        assert_eq!(
            hash.as_str(),
            "a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456"
        );
    }

    #[test]
    fn test_invalid_length() {
        assert!(FileHash::new("short").is_err());
        assert!(FileHash::new(&"a".repeat(63)).is_err());
        assert!(FileHash::new(&"a".repeat(65)).is_err());
    }

    #[test]
    fn test_invalid_characters() {
        let invalid_hash = "g1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456";
        assert!(FileHash::new(invalid_hash).is_err());
    }
}
