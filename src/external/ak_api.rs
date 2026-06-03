use crate::{AppError, AppResult, config::AkApiConfig, service::types::RemoteVersion};
use reqwest::Client;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{info, instrument};

#[derive(Clone, Debug)]
pub struct AkApi {
    client: Client,
    conf_url: String,
    asset_url: String,
}

impl AkApi {
    pub fn new(config: &AkApiConfig) -> AppResult<Self> {
        let client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|err| AppError::ExternalService(err.into()))?;

        Ok(Self {
            client,
            conf_url: config.conf_url.clone(),
            asset_url: config.asset_url.clone(),
        })
    }

    #[instrument(name = "ak_api.get_version", skip(self))]
    pub async fn get_version(&self) -> AppResult<RemoteVersion> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| AppError::ExternalService(err.into()))?
            .as_secs();

        let url = format!("{}/{}?sign={timestamp}", self.conf_url, "version");
        info!("req version {url}");

        let version = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?
            .json()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;

        Ok(version)
    }

    #[instrument(name = "ak_api.get_hot_update_list", skip(self))]
    pub async fn get_hot_update_list(&self, res_version: &str) -> AppResult<String> {
        let url = format!("{}/{res_version}/hot_update_list.json", self.asset_url);
        info!("req hot update list {url}");

        self.client
            .get(url)
            .send()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?
            .text()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))
    }

    #[instrument(name = "ak_api.download_file", skip(self))]
    pub async fn download_file(&self, res_version: &str, path: &str) -> AppResult<Vec<u8>> {
        let url = format!("{}/{res_version}/{path}", self.asset_url);
        info!("downloading file from {res_version}/{path}");

        let bytes = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?
            .bytes()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;

        Ok(bytes.to_vec())
    }
}
