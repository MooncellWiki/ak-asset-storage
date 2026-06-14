use crate::{AppResult, database::Database};
use std::{collections::HashMap, fs, path::PathBuf};
use tracing::info;

#[derive(Debug, Clone)]
pub struct ItemDemandImportService {
    pub database: Database,
    pub file_path: PathBuf,
}

impl ItemDemandImportService {
    pub async fn import(&self) -> AppResult<()> {
        let content = fs::read_to_string(&self.file_path).map_err(|err| {
            crate::AppError::Application(anyhow::anyhow!("Failed to read itemDemand.json: {err}"))
        })?;

        let demands: HashMap<String, serde_json::Value> =
            serde_json::from_str(&content).map_err(|err| {
                crate::AppError::Application(anyhow::anyhow!(
                    "Failed to parse itemDemand.json: {err}"
                ))
            })?;

        let demands: Vec<(String, String)> = demands
            .into_iter()
            .map(|(name, usage)| {
                serde_json::to_string(&usage)
                    .map(|serialized| (name.clone(), serialized))
                    .map_err(|err| {
                        crate::AppError::Application(anyhow::anyhow!(
                            "Failed to serialize demand for {name}: {err}"
                        ))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        self.database.replace_all_demands(demands).await?;
        info!("item demand import finished");
        Ok(())
    }
}
