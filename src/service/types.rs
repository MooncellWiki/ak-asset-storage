use crate::AppResult;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RemoteVersion {
    pub client_version: String,
    pub res_version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ABInfo {
    #[serde(rename = "abSize")]
    pub ab_size: u64,
    pub hash: String,
    pub md5: String,
    pub name: String,
    pub meta: Option<i32>,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
}

impl ABInfo {
    pub(crate) fn matches_archived_content(&self, baseline: &Self) -> bool {
        const IGNORE_MD5: i32 = 1;

        self.name == baseline.name
            && self.ab_size == baseline.ab_size
            && (self.meta.unwrap_or_default() | baseline.meta.unwrap_or_default()) & IGNORE_MD5 == 0
            && self.md5.len() == 32
            && self.md5.as_bytes().iter().all(u8::is_ascii_hexdigit)
            && self.md5.eq_ignore_ascii_case(&baseline.md5)
    }

    #[must_use]
    pub fn url(&self) -> String {
        let path = self.name.replace('/', "_").replace('#', "__");
        if let Some((left, _)) = path.rsplit_once('.') {
            format!("{left}.dat")
        } else {
            path
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct HotUpdateList {
    #[serde(rename = "abInfos")]
    ab_infos: Vec<ABInfo>,
    #[serde(skip)]
    raw: String,
}

impl HotUpdateList {
    pub fn new(json_string: &str) -> AppResult<Self> {
        let parsed: Self =
            serde_json::from_str(json_string).context("Invalid JSON in hot update list")?;

        Ok(Self {
            raw: json_string.to_string(),
            ab_infos: parsed.ab_infos,
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    #[must_use]
    pub fn ab_infos(&self) -> &[ABInfo] {
        &self.ab_infos
    }
}

#[cfg(test)]
mod tests {
    use super::ABInfo;

    #[test]
    fn archive_match_requires_trusted_md5_and_same_size() {
        let archived = ABInfo {
            ab_size: 128,
            hash: "game-hash".to_string(),
            md5: "0123456789abcdef0123456789abcdef".to_string(),
            name: "sample.ab".to_string(),
            meta: None,
            total_size: 64,
        };
        let mut target = archived.clone();
        target.md5.make_ascii_uppercase();
        assert!(target.matches_archived_content(&archived));

        target.ab_size += 1;
        assert!(!target.matches_archived_content(&archived));

        target.ab_size = archived.ab_size;
        target.meta = Some(1);
        assert!(!target.matches_archived_content(&archived));
        assert!(!archived.matches_archived_content(&target));

        target.meta = None;
        target.md5 = "x".repeat(32);
        assert!(!target.matches_archived_content(&target));
    }
}
