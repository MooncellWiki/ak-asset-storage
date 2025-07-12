/// File entity represents a physical file stored in the system
#[derive(Debug, Clone)]
pub struct File {
    pub id: Option<i32>,
    pub hash: String,
    pub size: i32,
}

impl File {
    #[must_use]
    /// Check if this is a large file (> 5MB)
    pub const fn is_large(&self) -> bool {
        self.size > 5 * 1024 * 1024
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_large_file() {
        let large_file = File {
            id: None,
            hash: "a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456".into(),
            size: 6 * 1024 * 1024,
        };

        assert!(large_file.is_large());
    }
}
