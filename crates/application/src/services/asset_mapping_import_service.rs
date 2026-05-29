use crate::{AppResult, AssetMapping, AssetMappingRepository, NodeType, VersionRepository};
use anyhow::{Context, anyhow};
use serde::Deserialize;
use std::{
    collections::HashSet,
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
    pub const fn new(repo: R, gamedata_root: PathBuf) -> Self {
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

        if !self
            .repo
            .import_asset_mappings(version_id, &mappings)
            .await?
        {
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

    let mut seen_dirs: HashSet<String> = HashSet::new();
    let mut asset_names: HashSet<String> = HashSet::new();

    for asset in &manifest.asset_to_bundle_list {
        asset_names.insert(asset.asset_name.clone());
        let mut dir = asset.asset_name.as_str();
        while let Some(pos) = dir.rfind('/') {
            dir = &dir[..pos];
            seen_dirs.insert(dir.to_string());
        }
    }

    let mut mappings: Vec<AssetMapping> = Vec::new();

    for asset in manifest.asset_to_bundle_list {
        let bundle = bundles.get(asset.bundle_index).ok_or_else(|| {
            anyhow!(
                "Invalid bundleIndex {} for {res_version}",
                asset.bundle_index
            )
        })?;

        let dir_name = asset
            .asset_name
            .rfind('/')
            .map_or(String::new(), |pos| asset.asset_name[..pos].to_string());

        let node_type = if seen_dirs.contains(&asset.asset_name) {
            NodeType::Both
        } else {
            NodeType::File
        };

        mappings.push(AssetMapping {
            id: None,
            version_id: 0,
            asset_name: asset.asset_name,
            bundle_path: bundle.name.clone(),
            asset_path: asset.path,
            short_name: asset.name,
            dir_name,
            node_type,
        });
    }

    for dir_path in seen_dirs {
        if asset_names.contains(&dir_path) {
            continue;
        }
        let dir_name = dir_path
            .rfind('/')
            .map_or(String::new(), |pos| dir_path[..pos].to_string());
        mappings.push(AssetMapping {
            id: None,
            version_id: 0,
            asset_name: dir_path,
            bundle_path: String::new(),
            asset_path: None,
            short_name: None,
            dir_name,
            node_type: NodeType::Directory,
        });
    }

    Ok(mappings)
}
