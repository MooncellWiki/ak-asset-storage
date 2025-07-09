use crate::{
    error::{DomainError, DomainResult},
    value_objects::{ClientVersion, HotUpdateList, ResVersion},
};

/// Version entity represents a game client version with resource updates
#[derive(Debug, Clone)]
pub struct Version {
    pub id: Option<VersionId>,
    pub res: ResVersion,
    pub client: ClientVersion,
    pub is_ready: bool,
    pub hot_update_list: HotUpdateList,
}

/// Version identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionId(pub i32);

impl Version {
    #[must_use]
    /// Create a new version
    pub const fn new(
        res: ResVersion,
        client: ClientVersion,
        hot_update_list: HotUpdateList,
    ) -> Self {
        Self {
            id: None,
            res,
            client,
            is_ready: false,
            hot_update_list,
        }
    }

    #[must_use]
    /// Create a version with existing ID (for loading from persistence)
    pub const fn with_id(
        id: VersionId,
        res: ResVersion,
        client: ClientVersion,
        is_ready: bool,
        hot_update_list: HotUpdateList,
    ) -> Self {
        Self {
            id: Some(id),
            res,
            client,
            is_ready,
            hot_update_list,
        }
    }

    /// Mark version as ready for distribution
    pub fn mark_ready(&mut self) -> DomainResult<()> {
        if self.is_ready {
            return Err(DomainError::InvalidState {
                message: "Version is already ready".to_string(),
            });
        }
        self.is_ready = true;
        Ok(())
    }
}

impl std::fmt::Display for VersionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for VersionId {
    fn from(id: i32) -> Self {
        Self(id)
    }
}

impl From<VersionId> for i32 {
    fn from(id: VersionId) -> Self {
        id.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_version() -> Version {
        Version::new(
            ResVersion::new("24-09-23-11-27-19-c6564b").unwrap(),
            ClientVersion::new("2.3.61").unwrap(),
            HotUpdateList::new(r#"{"abInfos": []}"#).unwrap(),
        )
    }

    #[test]
    fn test_version_creation() {
        let version = create_test_version();
        assert!(!version.is_ready);
        assert!(version.id.is_none());
    }

    #[test]
    fn test_mark_ready() {
        let mut version = create_test_version();
        assert!(version.mark_ready().is_ok());
        assert!(version.is_ready);

        // Should fail if already ready
        assert!(version.mark_ready().is_err());
    }

    #[test]
    fn test_version_with_id() {
        let version = Version::with_id(
            VersionId(1),
            ResVersion::new("24-09-23-11-27-19-c6564b").unwrap(),
            ClientVersion::new("2.3.61").unwrap(),
            true,
            HotUpdateList::new(r#"{"abInfos": []}"#).unwrap(),
        );

        assert_eq!(version.id, Some(VersionId(1)));
        assert!(version.is_ready);
    }
}
