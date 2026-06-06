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
    #[serde(rename = "totalSize")]
    pub total_size: u64,
}

impl ABInfo {
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
