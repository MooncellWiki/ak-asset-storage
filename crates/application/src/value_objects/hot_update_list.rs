use crate::AppResult;
use anyhow::Context;
use serde::Deserialize;

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
/// Represents a hot update list (JSON string)
#[derive(Debug, Clone, Deserialize)]
pub struct HotUpdateList {
    #[serde(rename = "abInfos")]
    ab_infos: Vec<ABInfo>,
    #[serde(skip)]
    raw: String,
}

impl HotUpdateList {
    /// Create a new hot update list
    pub fn new(json_string: &str) -> AppResult<Self> {
        // Validate that it's valid JSON
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
    use super::*;

    #[test]
    fn test_update_info_url() {
        let info = ABInfo {
            ab_size: 398_586,
            hash: "0c1b5433cc652adf9ec1c3f522d2e4cd".to_string(),
            md5: "3574616c7b8c8392424992df9392bb84".to_string(),
            name: "ui/skin/2018#sale.ab".to_string(),
            total_size: 334_148,
        };

        assert_eq!(info.url(), "ui_skin_2018__sale.dat");
    }

    #[test]
    fn test_hot_update_list_new_valid_json() {
        let json = r#"{
            "fullPack": {
                "name": "__FULLPACK__",
                "hash": "",
                "md5": "",
                "totalSize": 2012245432,
                "abSize": 0
            },
            "versionId": "20-07-08-12-58-19-ee6a0d",
            "abInfos": [{
                "name": "ui/skin/2018#sale.ab",
                "hash": "0c1b5433cc652adf9ec1c3f522d2e4cd",
                "md5": "3574616c7b8c8392424992df9392bb84",
                "totalSize": 334148,
                "abSize": 398586
            }]
        }"#;

        let result = HotUpdateList::new(json);
        assert!(result.is_ok());

        let list = result.unwrap();
        assert_eq!(list.ab_infos.len(), 1);
        assert_eq!(list.ab_infos[0].name, "ui/skin/2018#sale.ab");
        assert_eq!(list.ab_infos[0].ab_size, 398_586);
        assert_eq!(list.ab_infos[0].hash, "0c1b5433cc652adf9ec1c3f522d2e4cd");
        assert_eq!(list.ab_infos[0].md5, "3574616c7b8c8392424992df9392bb84");
        assert_eq!(list.ab_infos[0].total_size, 334_148);
        assert_eq!(list.raw, json);
    }

    #[test]
    fn test_hot_update_list_new_invalid_json() {
        let invalid_json = r"{ invalid json }";

        let result = HotUpdateList::new(invalid_json);
        assert!(result.is_err());
    }
}
