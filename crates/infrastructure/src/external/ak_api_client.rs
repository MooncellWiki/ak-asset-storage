use crate::error::InfraError;
use application::ports::external_services::AkApiClient;
use application::AkApiConfig;
use application::{error::AppResult, RemoteVersion};
use async_trait::async_trait;
use reqwest::Client;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{info, instrument};

trait IntoAppResult<T, E> {
    fn into_app_result(self) -> AppResult<T>;
}
impl<T, E1: Into<AkApiClientError>> IntoAppResult<T, E1> for Result<T, E1> {
    fn into_app_result(self) -> AppResult<T> {
        self.map_err(|e| InfraError::AkApiClient(e.into()).into())
    }
}

#[derive(Clone, Debug)]
pub struct HttpAkApiClient {
    client: Client,
    conf_url: String,
    asset_url: String,
}

impl HttpAkApiClient {
    #[must_use]
    pub fn new(config: &AkApiConfig) -> Self {
        let client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            conf_url: config.conf_url.to_string(),
            asset_url: config.asset_url.to_string(),
        }
    }
}

#[async_trait]
impl AkApiClient for HttpAkApiClient {
    #[instrument(name = "ak_api.get_version", skip(self))]
    async fn get_version(&self) -> AppResult<RemoteVersion> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .into_app_result()?
            .as_secs();

        let url = format!("{}/{}?sign={}", self.conf_url, "version", timestamp);
        info!("req version {}", url);

        let ver: RemoteVersion = self
            .client
            .get(url)
            .send()
            .await
            .into_app_result()?
            .json()
            .await
            .into_app_result()?;

        info!(
            "remote version {} {}",
            &ver.client_version, &ver.res_version
        );
        Ok(ver)
    }

    #[instrument(name = "ak_api.get_hot_update_list", skip(self))]
    async fn get_hot_update_list(&self, res_version: &str) -> AppResult<String> {
        let url = format!("{}/{}/hot_update_list.json", self.asset_url, res_version);
        info!("req hot update list {}", url);

        let resp = self
            .client
            .get(url)
            .send()
            .await
            .into_app_result()?
            .text()
            .await
            .into_app_result()?;

        Ok(resp)
    }

    #[instrument(name = "ak_api.download_file", skip(self))]
    async fn download_file(&self, res_version: &str, path: &str) -> AppResult<Vec<u8>> {
        let url = format!("{}/{}/{}", self.asset_url, res_version, path);
        info!("downloading file from {}/{}", res_version, path);

        let resp = self
            .client
            .get(url)
            .send()
            .await
            .into_app_result()?
            .bytes()
            .await
            .into_app_result()?;

        Ok(resp.to_vec())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum AkApiClientError {
    #[error("Request error")]
    Request(#[from] reqwest::Error),

    #[error("Failed to parse response")]
    Parse(#[from] serde_json::Error),

    #[error("Failed to get system time")]
    Time(#[from] std::time::SystemTimeError),
}
