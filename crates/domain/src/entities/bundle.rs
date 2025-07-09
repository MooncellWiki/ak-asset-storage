use crate::entities::{FileId, VersionId};
use crate::value_objects::FilePath;

/// Bundle entity represents a file within a specific version
#[derive(Debug, Clone)]
pub struct Bundle {
    pub id: Option<BundleId>,
    pub path: FilePath,
    pub version_id: VersionId,
    pub file_id: FileId,
}

/// Bundle identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundleId(pub i32);

impl Bundle {
    #[must_use]
    /// Create a new bundle
    pub const fn new(path: FilePath, version_id: VersionId, file_id: FileId) -> Self {
        Self {
            id: None,
            path,
            version_id,
            file_id,
        }
    }

    #[must_use]
    /// Create a bundle with existing ID (for loading from persistence)
    pub const fn with_id(
        id: BundleId,
        path: FilePath,
        version_id: VersionId,
        file_id: FileId,
    ) -> Self {
        Self {
            id: Some(id),
            path,
            version_id,
            file_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_bundle() -> Bundle {
        Bundle::new(
            FilePath::new("assets/texture.png").unwrap(),
            VersionId(1),
            FileId(1),
        )
    }

    #[test]
    fn test_bundle_creation() {
        let bundle = create_test_bundle();
        assert!(bundle.id.is_none());
        assert_eq!(bundle.version_id, VersionId(1));
        assert_eq!(bundle.file_id, FileId(1));
        assert_eq!(bundle.path.as_str(), "assets/texture.png");
    }

    #[test]
    fn test_bundle_with_id() {
        let bundle = Bundle::with_id(
            BundleId(1),
            FilePath::new("assets/texture.png").unwrap(),
            VersionId(1),
            FileId(1),
        );

        assert_eq!(bundle.id, Some(BundleId(1)));
    }

    #[test]
    fn test_direct_field_access() {
        let mut bundle = create_test_bundle();

        // Direct field access instead of setters/getters
        bundle.id = Some(BundleId(42));
        assert_eq!(bundle.id, Some(BundleId(42)));

        // Direct path modification (since FilePath is validated at creation)
        let new_path = FilePath::new("assets/new_texture.png").unwrap();
        bundle.path = new_path;
        assert_eq!(bundle.path.as_str(), "assets/new_texture.png");
    }
}
