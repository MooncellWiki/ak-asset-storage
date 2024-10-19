use super::_entities::files::ActiveModel;
pub use super::_entities::files::{self, Column, Entity, Model};
use crate::error::Result;
use sea_orm::{entity::prelude::*, Condition};
use serde::Deserialize;

impl ActiveModelBehavior for ActiveModel {
    // extend activemodel below (keep comment for generators)
}
#[derive(Deserialize, Debug)]
pub struct Filter {
    path: Option<String>,
    hash: Option<String>,
    file: Option<i32>,
    version: Option<i32>,
}
impl Model {
    pub async fn list_by_version_id(db: &DatabaseConnection, id: i32) -> Result<Vec<Self>> {
        Ok(Entity::find()
            .filter(Column::Version.eq(id))
            .all(db)
            .await?)
    }
    pub async fn filter(db: &DatabaseConnection, filter: Filter) -> Result<Vec<Self>> {
        let mut condition = Condition::all();
        if let Some(path) = filter.path {
            condition = condition.add(Column::Path.contains(path));
        }
        if let Some(hash) = filter.hash {
            condition = condition.add(Column::Hash.contains(hash));
        }
        if let Some(file) = filter.file {
            condition = condition.add(Column::Id.eq(file));
        }
        if let Some(version) = filter.version {
            condition = condition.add(Column::Version.eq(version));
        }
        Ok(Entity::find().filter(condition).all(db).await?)
    }
    pub async fn find_by_version_path(
        db: &DatabaseConnection,
        version: i32,
        path: &str,
    ) -> Result<Option<Self>> {
        Ok(Entity::find()
            .filter(Column::Version.eq(version))
            .filter(Column::Path.eq(path))
            .one(db)
            .await?)
    }
}
