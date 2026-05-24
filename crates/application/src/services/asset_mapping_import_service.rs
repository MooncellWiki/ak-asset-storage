use crate::{AppResult, AssetMapping, AssetMappingRepository, VersionRepository};
use anyhow::{Context, anyhow};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tracing::info;

const MANIFEST_NAME: &str = "resource_manifest_idx.json";

#[derive(Debug, Clone)]
pub struct AssetMappingImportService<R> {
    repo: R,
    gamedata_root: PathBuf,
}

impl<R> AssetMappingImportService<R>
where
    R: VersionRepository + AssetMappingRepository,
{
    pub fn new(repo: R, gamedata_root: PathBuf) -> Self {
        Self {
            repo,
            gamedata_root,
        }
    }

    pub async fn import_by_res_version(&self, res_version: &str) -> AppResult<()> {
        self.import_from_version_dir(&self.gamedata_root.join(res_version))
            .await
    }

    pub async fn import_from_version_dir(&self, version_dir: &Path) -> AppResult<()> {
        let res_version = version_dir
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .context("Failed to extract res version from version dir")?;
        let manifest_path = version_dir.join(MANIFEST_NAME);

        let mappings = parse_manifest(&manifest_path, res_version)?;

        let version = self
            .repo
            .get_version_by_res(res_version)
            .await?
            .ok_or_else(|| anyhow!("Version not found for res {res_version}"))?;
        let version_id = version
            .id
            .ok_or_else(|| anyhow!("Version ID missing for res {res_version}"))?;

        let mappings = mappings
            .into_iter()
            .map(|mapping| AssetMapping {
                version_id,
                ..mapping
            })
            .collect::<Vec<_>>();

        if !self.repo.import_asset_mappings(version_id, &mappings).await? {
            return Err(anyhow!("Asset mapping import already running for {res_version}").into());
        }

        info!("asset mapping import finished for {res_version}");
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestFile {
    bundles: Vec<ManifestBundle>,
    asset_to_bundle_list: Vec<ManifestAsset>,
}

#[derive(Debug, Deserialize)]
struct ManifestBundle {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestAsset {
    asset_name: String,
    bundle_index: usize,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    path: Option<String>,
}

fn parse_manifest(path: &Path, res_version: &str) -> AppResult<Vec<AssetMapping>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read manifest: {}", path.display()))?;
    let manifest: ManifestFile = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse manifest: {}", path.display()))?;

    let bundles = manifest.bundles;

    manifest
        .asset_to_bundle_list
        .into_iter()
        .map(|asset| {
            let bundle = bundles.get(asset.bundle_index).ok_or_else(|| {
                anyhow!(
                    "Invalid bundleIndex {} for {res_version}",
                    asset.bundle_index
                )
            })?;

            Ok(AssetMapping {
                id: None,
                version_id: 0,
                asset_name: asset.asset_name,
                bundle_path: bundle.name.clone(),
                asset_path: asset.path,
                short_name: asset.name,
            })
        })
        .collect()
}
