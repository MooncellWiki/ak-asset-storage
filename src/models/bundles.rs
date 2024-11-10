pub use super::_entities::bundles::{self, ActiveModel, Column, Entity, Model};
use super::_entities::{files, versions};

use crate::{error::Result, views::FileDetail};
use sea_orm::{entity::prelude::*, Condition, JoinType, QuerySelect, SelectColumns};
use serde::Deserialize;
use utoipa::IntoParams;

impl ActiveModelBehavior for ActiveModel {
    // extend activemodel below (keep comment for generators)
}

#[derive(IntoParams, Deserialize, Debug)]
pub struct Filter {
    path: Option<String>,
    hash: Option<String>,
    file: Option<i32>,
    version: Option<i32>,
}

impl Model {
    pub async fn list_by_version_id(db: &DatabaseConnection, id: i32) -> Result<Vec<FileDetail>> {
        Ok(Entity::find()
            .filter(Column::Version.eq(id))
            .join_rev(JoinType::InnerJoin, files::Relation::Bundles.def())
            .join_rev(JoinType::InnerJoin, versions::Relation::Bundles.def())
            .select_column(files::Column::Hash)
            .select_column(files::Column::Size)
            .select_column(versions::Column::Client)
            .select_column(versions::Column::Res)
            .select_column(versions::Column::IsReady)
            .into_model::<FileDetail>()
            .all(db)
            .await?)
    }
    pub async fn filter(db: &DatabaseConnection, filter: Filter) -> Result<Vec<FileDetail>> {
        let mut condition = Condition::all();
        if let Some(path) = filter.path {
            condition = condition.add(Column::Path.contains(path));
        }
        if let Some(hash) = filter.hash {
            condition = condition.add(files::Column::Hash.contains(hash));
        }
        if let Some(file) = filter.file {
            condition = condition.add(Column::Id.eq(file));
        }
        if let Some(version) = filter.version {
            condition = condition.add(Column::Version.eq(version));
        }
        let resp = Entity::find()
            .filter(condition)
            .join_rev(JoinType::InnerJoin, files::Relation::Bundles.def())
            .join_rev(JoinType::InnerJoin, versions::Relation::Bundles.def())
            .select_column(files::Column::Hash)
            .select_column(files::Column::Size)
            .select_column(versions::Column::Client)
            .select_column(versions::Column::Res)
            .select_column(versions::Column::IsReady)
            .into_model::<FileDetail>()
            .all(db)
            .await?;
        Ok(resp)
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
    pub async fn detail_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<FileDetail>> {
        Ok(Entity::find_by_id(id)
            .join_rev(JoinType::InnerJoin, files::Relation::Bundles.def())
            .join_rev(JoinType::InnerJoin, versions::Relation::Bundles.def())
            .select_column(files::Column::Hash)
            .select_column(files::Column::Size)
            .select_column(versions::Column::Client)
            .select_column(versions::Column::Res)
            .select_column(versions::Column::IsReady)
            .into_model::<FileDetail>()
            .one(db)
            .await?)
    }
}

#[cfg(test)]
mod test {
    use crate::models::_entities::{files, versions};
    use sea_orm::{
        ColumnTrait, Condition, DbBackend, EntityTrait, JoinType, QueryFilter, QuerySelect,
        QueryTrait, RelationTrait, SelectColumns,
    };

    use super::{Column, Entity};

    #[test]
    pub fn filter_join() {
        let mut condition = Condition::all();
        condition = condition.add(Column::Path.contains("char"));
        let result = Entity::find()
            .filter(condition)
            .join_rev(JoinType::InnerJoin, files::Relation::Bundles.def())
            .join_rev(JoinType::InnerJoin, versions::Relation::Bundles.def())
            .select_column(files::Column::Hash)
            .select_column(files::Column::Size)
            .select_column(versions::Column::Client)
            .select_column(versions::Column::Res)
            .select_column(versions::Column::IsReady)
            .build(DbBackend::Postgres);
        assert_eq!(
            result.to_string(),
            r#"SELECT "bundles"."id", "bundles"."path", "bundles"."version", "bundles"."file", "files"."hash", "files"."size", "versions"."client", "versions"."res", "versions"."is_ready" FROM "bundles" INNER JOIN "files" ON "files"."id" = "bundles"."file" INNER JOIN "versions" ON "versions"."id" = "bundles"."version" WHERE "bundles"."path" LIKE '%char%'"#
        );
    }
}
