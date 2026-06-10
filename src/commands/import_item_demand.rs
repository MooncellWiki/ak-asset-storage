use crate::{
    AppResult, config::AppSettings, database::Database,
    service::item_demand_import::ItemDemandImportService,
};
use std::path::PathBuf;

pub async fn execute(settings: &AppSettings) -> AppResult<()> {
    let database = Database::connect(&settings.database).await?;
    let file_path = PathBuf::from(&settings.torappu.asset_base_path)
        .join("raw")
        .join("itemDemand.json");

    let service = ItemDemandImportService {
        database,
        file_path,
    };

    service.import().await
}
