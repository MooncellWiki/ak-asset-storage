use crate::{
    AppResult, config::AppSettings, database::Database,
    service::asset_mapping_import::AssetMappingImportService,
};

pub async fn execute(settings: &AppSettings, res_version: &str) -> AppResult<()> {
    let database = Database::connect(&settings.database).await?;
    let service = AssetMappingImportService {
        database,
        gamedata_root: std::path::PathBuf::from(&settings.torappu.asset_base_path).join("gamedata"),
    };

    service.import_by_res_version(res_version, false).await
}
