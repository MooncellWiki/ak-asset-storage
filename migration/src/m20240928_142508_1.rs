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
                    .col(char_len(Files::Hash, 64))
                    .col(integer(Files::Size))
                    .index(Index::create().col(Files::Hash).unique())
                    .take(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Bundles::Table)
                    .if_not_exists()
                    .col(pk_auto(Bundles::Id))
                    .col(string_len(Bundles::Path, 256))
                    .col(integer(Bundles::Version))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Bundles::Table, Bundles::Version)
                            .to(Versions::Table, Versions::Id),
                    )
                    .col(integer(Bundles::File))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Bundles::Table, Bundles::File)
                            .to(Files::Table, Files::Id),
                    )
                    .index(
                        Index::create()
                            .col(Bundles::Path)
                            .col(Bundles::Version)
                            .col(Bundles::File)
                            .unique(),
                    )
                    .take(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Bundles::Table).to_owned())
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
    Hash,
    Size,
}

#[derive(DeriveIden)]
enum Bundles {
    Table,
    Id,
    Path,
    Version,
    File,
}
