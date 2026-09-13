use crate::{
    AppResult, config::AppSettings, database::Database,
    service::story_usage::StoryUsageImportService,
};
use std::path::PathBuf;

pub async fn execute(settings: &AppSettings) -> AppResult<()> {
    let database = Database::connect(&settings.database).await?;
    let gamedata_root = PathBuf::from(&settings.torappu.asset_base_path).join("gamedata");

    let service = StoryUsageImportService {
        database,
        gamedata_root,
    };

    service.import().await
}
