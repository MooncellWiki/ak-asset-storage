use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Versions::Table)
                    .if_not_exists()
                    .col(pk_auto(Versions::Id))
                    .col(string_len(Versions::Res, 32))
                    .col(string_len(Versions::Client, 32))
                    .col(boolean(Versions::IsReady))
                    .col(text(Versions::HotUpdateList))
                    .take(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Files::Table)
                    .col(pk_auto(Files::Id))
                    .col(string_len(Files::Path, 256))
                    .col(char_len(Files::Hash, 64))
                    .col(integer(Files::Version))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Files::Table, Files::Version)
                            .to(Versions::Table, Versions::Id),
                    )
                    .index(
                        Index::create()
                            .col(Files::Version)
                            .col(Files::Path)
                            .unique(),
                    )
                    .take(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(FileMetas::Table)
                    .if_not_exists()
                    .col(pk_auto(FileMetas::Id))
                    .col(string_len(FileMetas::Key, 255))
                    .col(text(FileMetas::Value))
                    .col(integer(FileMetas::FileId))
                    .foreign_key(
                        ForeignKey::create()
                            .from(FileMetas::Table, FileMetas::FileId)
                            .to(Files::Table, Files::Id),
                    )
                    .index(
                        Index::create()
                            .col(FileMetas::FileId)
                            .col(FileMetas::Value)
                            .col(FileMetas::Key)
                            .unique(),
                    )
                    .take(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FileMetas::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Files::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Versions::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Versions {
    Table,
    Id,
    Res,
    Client,
    IsReady,
    HotUpdateList,
}

#[derive(DeriveIden)]
enum Files {
    Table,
    Id,
    Path,
    Version,
    Hash,
}

#[derive(DeriveIden)]
enum FileMetas {
    Table,
    Id,
    Key,
    Value,
    FileId,
}
