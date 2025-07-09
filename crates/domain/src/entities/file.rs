use crate::value_objects::{FileHash, FileSize};

/// File entity represents a physical file stored in the system
#[derive(Debug, Clone)]
pub struct File {
    pub id: Option<FileId>,
    pub hash: FileHash,
    pub size: FileSize,
}

/// File identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileId(pub i32);

impl File {
    #[must_use]
    /// Create a new file
    pub const fn new(hash: FileHash, size: FileSize) -> Self {
        Self {
            id: None,
            hash,
            size,
        }
    }
    #[must_use]
    /// Create a file with existing ID (for loading from persistence)
    pub const fn with_id(id: FileId, hash: FileHash, size: FileSize) -> Self {
        Self {
            id: Some(id),
            hash,
            size,
        }
    }
    #[must_use]
    /// Check if this is a large file (> 5MB)
    pub const fn is_large(&self) -> bool {
        self.size.is_large()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_large_file() {
        let large_file = File::new(
            FileHash::new("a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456")
                .unwrap(),
            FileSize::new(6 * 1024 * 1024).unwrap(),
        );

        assert!(large_file.is_large());
    }
}
