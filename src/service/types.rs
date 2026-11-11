use crate::AppResult;
use anyhow::Context;
use md5::{Digest, Md5};
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
    #[serde(default)]
    pub meta: ABMeta,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
}

/// An entry's `meta` bit flags (`HotUpdateInfo.ABMetaFlag`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ABMeta {
    /// No `meta`, or `null`.
    #[default]
    None,
    Flags(i64),
    /// Not an integer. Such an entry is downloaded without trusting its MD5,
    /// instead of making the whole list unreadable.
    Unrecognized,
}

impl<'de> Deserialize<'de> for ABMeta {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match serde_json::Value::deserialize(deserializer)? {
            serde_json::Value::Null => Self::None,
            value => value.as_i64().map_or(Self::Unrecognized, Self::Flags),
        })
    }
}

impl ABInfo {
    /// `HotUpdateInfo.ABMetaFlag.IGNORE_MD5`: `md5` is only a short signature.
    const IGNORE_MD5: i64 = 1;

    const fn ignores_md5(&self) -> bool {
        matches!(self.meta, ABMeta::Flags(flags) if flags & Self::IGNORE_MD5 != 0)
    }

    /// The full MD5 of the extracted bundle, if this entry carries one.
    fn trusted_md5(&self) -> Option<&str> {
        let short_signature = match self.meta {
            ABMeta::None => false,
            ABMeta::Flags(_) => self.ignores_md5(),
            // Unknown flags might include IGNORE_MD5.
            ABMeta::Unrecognized => true,
        };
        (!short_signature
            && self.md5.len() == 32
            && self.md5.as_bytes().iter().all(u8::is_ascii_hexdigit))
        .then_some(self.md5.as_str())
    }

    pub(crate) fn matches_archived_content(&self, baseline: &Self) -> bool {
        self.name == baseline.name
            && self.ab_size == baseline.ab_size
            && self
                .trusted_md5()
                .zip(baseline.trusted_md5())
                .is_some_and(|(md5, archived)| md5.eq_ignore_ascii_case(archived))
    }

    /// Checks downloaded content the way the game client checks its local
    /// copy (`PersistentResRecover._CheckIfLocalABValid` in 2.7.91): the full
    /// MD5 when there is one, otherwise the size for `IGNORE_MD5` bundles.
    ///
    /// Later versions reuse an archived file whenever their MD5 is unchanged
    /// without downloading it again, so the archive must only record a file
    /// under an MD5 it really has.
    pub(crate) fn verify_content(&self, content: &[u8]) -> anyhow::Result<()> {
        if let Some(expected) = self.trusted_md5() {
            let actual = hex::encode(Md5::digest(content));
            anyhow::ensure!(
                actual.eq_ignore_ascii_case(expected),
                "{}: downloaded MD5 {actual} does not match the manifest's {expected}",
                self.name
            );
        } else if self.ignores_md5() {
            anyhow::ensure!(
                u64::try_from(content.len()).is_ok_and(|len| len == self.ab_size),
                "{}: downloaded {} bytes, the manifest's abSize is {}",
                self.name,
                content.len(),
                self.ab_size
            );
        }
        // Neither a short MD5 without IGNORE_MD5 nor an unrecognised `meta`
        // can be checked.
        Ok(())
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
    use super::{ABInfo, ABMeta, HotUpdateList};

    #[test]
    fn archive_match_requires_trusted_md5_and_same_size() {
        let archived = ABInfo {
            ab_size: 128,
            hash: "game-hash".to_string(),
            md5: "0123456789abcdef0123456789abcdef".to_string(),
            name: "sample.ab".to_string(),
            meta: ABMeta::None,
            total_size: 64,
        };
        let mut target = archived.clone();
        target.md5.make_ascii_uppercase();
        assert!(target.matches_archived_content(&archived));

        target.ab_size += 1;
        assert!(!target.matches_archived_content(&archived));

        target.ab_size = archived.ab_size;
        target.meta = ABMeta::Flags(1);
        assert!(!target.matches_archived_content(&archived));
        assert!(!archived.matches_archived_content(&target));

        target.meta = ABMeta::None;
        target.md5 = "x".repeat(32);
        assert!(!target.matches_archived_content(&target));
    }

    #[test]
    fn verifies_downloads_like_the_game_client() {
        let content = b"bundle";
        let mut info = ABInfo {
            ab_size: 6,
            hash: "game-hash".to_string(),
            // MD5 of "bundle"; the case does not matter.
            md5: "94377C156735B39DFA4AC607234CB87C".to_string(),
            name: "sample.ab".to_string(),
            meta: ABMeta::None,
            total_size: 64,
        };
        info.verify_content(content).unwrap();
        // The full MD5 decides; the client does not compare sizes here.
        info.ab_size += 1;
        info.verify_content(content).unwrap();
        let error = info.verify_content(b"bundlf").unwrap_err().to_string();
        assert!(error.contains("sample.ab"), "{error}");

        // IGNORE_MD5 bundles carry a short signature and are checked by size.
        info.meta = ABMeta::Flags(1);
        info.md5 = "155c".to_string();
        info.ab_size = 6;
        info.verify_content(content).unwrap();
        info.verify_content(b"bundle!").unwrap_err();
        // The flag also overrides a value that looks like a full MD5.
        info.md5 = "0".repeat(32);
        info.verify_content(content).unwrap();

        // A short MD5 without the flag cannot be checked at all.
        info.meta = ABMeta::None;
        info.md5 = "155c".to_string();
        info.verify_content(b"anything").unwrap();
    }

    #[test]
    fn unrecognized_meta_only_affects_its_own_entry() {
        let md5 = "94377c156735b39dfa4ac607234cb87c";
        let metas = ["absent", "null", "0", "1", "4294967297", r#""1""#, "1.5"];
        let list = metas
            .iter()
            .enumerate()
            .map(|(index, meta)| {
                let meta = if *meta == "absent" {
                    String::new()
                } else {
                    format!(r#","meta":{meta}"#)
                };
                format!(
                    r#"{{"name":"{index}.ab","hash":"h","md5":"{md5}","abSize":6,"totalSize":6{meta}}}"#
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let list = HotUpdateList::new(&format!(r#"{{"abInfos":[{list}]}}"#)).unwrap();
        let metas: Vec<_> = list.ab_infos().iter().map(|info| info.meta).collect();
        assert_eq!(
            metas,
            [
                ABMeta::None,
                ABMeta::None,
                ABMeta::Flags(0),
                ABMeta::Flags(1),
                ABMeta::Flags(4_294_967_297),
                ABMeta::Unrecognized,
                ABMeta::Unrecognized,
            ]
        );

        let [absent, .., unrecognized] = list.ab_infos() else {
            unreachable!()
        };
        let mut renamed = absent.clone();
        renamed.name.clone_from(&unrecognized.name);
        // An unknown flag might be IGNORE_MD5: the MD5 is neither reused nor
        // used to reject a download.
        assert!(!unrecognized.matches_archived_content(&renamed));
        assert!(!renamed.matches_archived_content(unrecognized));
        unrecognized.verify_content(b"other bytes").unwrap();
        absent.verify_content(b"other bytes").unwrap_err();
    }
}
