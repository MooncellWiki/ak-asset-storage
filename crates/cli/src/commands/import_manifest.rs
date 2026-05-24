use ak_asset_storage_application::{AssetMappingImportService, ConfigProvider};
use ak_asset_storage_infrastructure::PostgresRepository;
use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;

pub async fn execute(config: &impl ConfigProvider, res_version: &str) -> Result<()> {
    let pool = PgPoolOptions::new()
        .connect(&config.database_config().uri)
        .await?;
    let repository = PostgresRepository { pool };
    let service = AssetMappingImportService::new(
        repository,
        std::path::PathBuf::from(&config.torappu_config().asset_base_path).join("gamedata"),
    );

    service
        .import_by_res_version(res_version)
        .await
        .with_context(|| format!("Failed to import manifest for {res_version}"))
}
