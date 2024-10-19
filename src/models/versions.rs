use super::_entities::{
    sea_orm_active_enums::StatusEnum,
    versions::{ActiveModel, Column, Entity, Model},
};
use crate::error::Result;
use sea_orm::{entity::prelude::*, QueryOrder};

impl ActiveModelBehavior for ActiveModel {
    // extend activemodel below (keep comment for generators)
}
impl Model {
    pub async fn latest(db: &DatabaseConnection, check_status: bool) -> Result<Option<Self>> {
        let mut select = Entity::find().order_by_desc(Column::Id);
        if check_status {
            select = select.filter(Column::Status.eq(StatusEnum::Ready));
        }
        Ok(select.one(db).await?)
    }

    pub async fn has_working_version(db: &DatabaseConnection) -> Result<bool> {
        Ok(Entity::find()
            .filter(Column::Status.eq(StatusEnum::Working))
            .one(db)
            .await?
            .is_some())
    }
}
