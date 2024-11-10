pub use super::_entities::versions::{ActiveModel, Column, Entity, Model};
use crate::error::Result;
use sea_orm::{entity::prelude::*, Order, QueryOrder};

impl ActiveModelBehavior for ActiveModel {
    // extend activemodel below (keep comment for generators)
}
impl Model {
    pub async fn latest(db: &DatabaseConnection, check_is_ready: bool) -> Result<Option<Self>> {
        let mut select = Entity::find().order_by_desc(Column::Id);
        if check_is_ready {
            select = select.filter(Column::IsReady.eq(true));
        }
        Ok(select.one(db).await?)
    }

    pub async fn first_unready(db: &DatabaseConnection) -> Result<Option<Self>> {
        Ok(Entity::find()
            .filter(Column::IsReady.eq(false))
            .order_by(Column::Id, Order::Asc)
            .one(db)
            .await?)
    }
    pub async fn find_by_client_res(
        db: &DatabaseConnection,
        client: &str,
        res: &str,
    ) -> Result<Option<Self>> {
        Ok(Entity::find()
            .filter(Column::Client.eq(client))
            .filter(Column::Res.eq(res))
            .one(db)
            .await?)
    }
}
