use super::_entities::files::ActiveModel;
pub use super::_entities::files::{self, Column, Entity, Model};
use crate::error::Result;
use sea_orm::entity::prelude::*;

impl ActiveModelBehavior for ActiveModel {
    // extend activemodel below (keep comment for generators)
}

impl Model {
    pub async fn find_by_hash(db: &DatabaseConnection, hash: &str) -> Result<Option<Self>> {
        Ok(Entity::find()
            .filter(files::Column::Hash.eq(hash))
            .one(db)
            .await?)
    }
}
