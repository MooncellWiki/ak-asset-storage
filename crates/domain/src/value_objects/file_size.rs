use crate::error::{DomainError, DomainResult};

/// Represents a file size in bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileSize(i32);

impl FileSize {
    /// Create a new file size
    pub fn new(size: i32) -> DomainResult<Self> {
        if size < 0 {
            return Err(DomainError::InvalidValue {
                message: "File size cannot be negative".to_string(),
            });
        }

        Ok(Self(size))
    }

    /// Get the size in bytes
    #[must_use]
    pub const fn bytes(&self) -> i32 {
        self.0
    }

    /// Check if the file is large (> 5MB)
    #[must_use]
    pub const fn is_large(&self) -> bool {
        self.0 > 5 * 1024 * 1024
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_file_size() {
        let size = FileSize::new(1024).unwrap();
        assert_eq!(size.bytes(), 1024);
    }

    #[test]
    fn test_negative_file_size() {
        assert!(FileSize::new(-1).is_err());
    }

    #[test]
    fn test_is_large() {
        let small = FileSize::new(1024).unwrap();
        let large = FileSize::new(6 * 1024 * 1024).unwrap();

        assert!(!small.is_large());
        assert!(large.is_large());
    }
}
