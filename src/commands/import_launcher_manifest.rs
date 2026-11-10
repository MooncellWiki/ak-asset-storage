use crate::{
    config::AppSettings,
    database::Database,
    external::launcher::{LauncherClient, LauncherPublication},
    service::launcher_archive::import_publication,
};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Deserialize)]
struct ManifestMetadata {
    version: String,
    client_version: String,
    file_path: String,
    game_files_md5: String,
    game_files_path: Option<PathBuf>,
}

pub async fn execute(settings: &AppSettings, metadata_paths: &[PathBuf]) -> Result<()> {
    let database = Database::connect(&settings.database).await?;
    let config = settings.launcher.clone().unwrap_or_default();
    let client = LauncherClient::new(&config)?;
    for path in metadata_paths {
        import_metadata(&database, &client, path)
            .await
            .with_context(|| format!("import launcher metadata {}", path.display()))?;
    }
    Ok(())
}

async fn import_metadata(database: &Database, client: &LauncherClient, path: &Path) -> Result<()> {
    let bytes = tokio::fs::read(path).await?;
    let metadata: ManifestMetadata = serde_json::from_slice(&bytes)?;
    let publication = LauncherPublication {
        package_version: metadata.version,
        game_version: metadata.client_version,
        file_path: metadata.file_path,
        manifest_md5: metadata.game_files_md5.to_ascii_lowercase(),
    };
    publication.validate()?;
    let game_files_path = metadata
        .game_files_path
        .map(|file| path.with_file_name(file));
    if !import_publication(database, client, &publication, game_files_path.as_deref()).await? {
        info!(
            package_version = publication.package_version,
            manifest_md5 = publication.manifest_md5,
            "PC launcher release already imported"
        );
    }
    Ok(())
}
