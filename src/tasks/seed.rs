use crate::{
    error::Result,
    models::_entities::{sea_orm_active_enums::StatusEnum, versions},
    workers::{
        check_and_download::{CheckAndDownload, RemoteVersion, UpdateList},
        WorkerOptions,
    },
};
use sea_orm::{ActiveModelTrait, ActiveValue::NotSet, IntoActiveModel, Set};
use std::{fs, path::PathBuf};

pub async fn seed(path: PathBuf, opt: WorkerOptions) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let versions = content
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            let parts = line.split(',').collect::<Vec<&str>>();
            (parts[1].to_string(), parts[2].to_string())
        })
        .collect::<Vec<(String, String)>>();
    let worker = CheckAndDownload::new(opt.clone())?;
    let client = reqwest::Client::new();
    for (res_version, client_version) in versions {
        let update_list = UpdateList::get(&opt.ak.base_url, &res_version, &client).await?;
        let version = versions::ActiveModel {
            client: Set(client_version.clone()),
            res: Set(res_version.clone()),
            hot_update_list: Set(update_list.raw),
            status: Set(StatusEnum::Working),
            id: NotSet,
        }
        .insert(&opt.conn)
        .await?;
        let remove_verion = RemoteVersion {
            client_version,
            res_version,
        };
        for ab in update_list.ab_infos {
            worker.sync(ab, &remove_verion, version.id).await?;
        }
        let mut version = version.into_active_model();
        version.status = Set(StatusEnum::Ready);
        version.save(&opt.conn).await?;
    }
    Ok(())
}
