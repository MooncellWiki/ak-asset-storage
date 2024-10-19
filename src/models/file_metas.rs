use super::{
    _entities::file_metas::{ActiveModel, Column, Entity, Model},
    files::files,
};
use crate::error::Result;
use sea_orm::{entity::prelude::*, ActiveValue::NotSet, QueryOrder, Set};

impl ActiveModelBehavior for ActiveModel {
    // extend activemodel below (keep comment for generators)
}
impl Model {
    pub async fn find_by_md5(
        db: &DatabaseConnection,
        md5: &str,
    ) -> Result<Option<(Model, Option<files::Model>)>> {
        Ok(Entity::find()
            .filter(Column::Key.eq("md5"))
            .filter(Column::Value.eq(md5))
            .order_by_desc(Column::Id)
            .find_also_related(files::Entity)
            .one(db)
            .await?)
    }
    pub async fn set_md5(db: &DatabaseConnection, file_id: i32, md5: &str) -> Result<()> {
        Entity::insert(ActiveModel {
            key: Set(String::from("md5")),
            value: Set(md5.to_string()),
            file_id: Set(file_id),
            id: NotSet,
        })
        .exec(db)
        .await?;
        Ok(())
    }
}
