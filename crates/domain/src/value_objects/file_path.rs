use crate::error::{DomainError, DomainResult};

/// Represents a file path in the asset bundle
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePath(String);

impl FilePath {
    /// Create a new file path
    pub fn new(path: &str) -> DomainResult<Self> {
        if path.trim().is_empty() {
            return Err(DomainError::InvalidFilePath {
                message: "File path cannot be empty".to_string(),
            });
        }

        if path.len() > 256 {
            return Err(DomainError::InvalidFilePath {
                message: "File path too long (max 256 characters)".to_string(),
            });
        }

        // Basic path validation - no null bytes, control characters
        if path.contains('\0') || path.chars().any(char::is_control) {
            return Err(DomainError::InvalidFilePath {
                message: "File path contains invalid characters".to_string(),
            });
        }

        Ok(Self(path.to_string()))
    }

    /// Get the path string
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the file extension if any
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        std::path::Path::new(&self.0)
            .extension()
            .and_then(|ext| ext.to_str())
    }

    /// Get the file name if any
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        std::path::Path::new(&self.0)
            .file_name()
            .and_then(|name| name.to_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_file_path() {
        let path = FilePath::new("assets/textures/character.png").unwrap();
        assert_eq!(path.as_str(), "assets/textures/character.png");
        assert_eq!(path.extension(), Some("png"));
        assert_eq!(path.file_name(), Some("character.png"));
    }

    #[test]
    fn test_empty_file_path() {
        assert!(FilePath::new("").is_err());
        assert!(FilePath::new("   ").is_err());
    }

    #[test]
    fn test_long_file_path() {
        let long_path = "a".repeat(257);
        assert!(FilePath::new(&long_path).is_err());
    }

    #[test]
    fn test_invalid_characters() {
        assert!(FilePath::new("path\0with\0null").is_err());
        assert!(FilePath::new("path\nwith\nnewline").is_err());
    }
}
