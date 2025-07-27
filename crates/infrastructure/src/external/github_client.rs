use ak_asset_storage_application::{AppResult, GithubConfig, GithubService};
use octocrab::Error;
use std::sync::Arc;

use crate::InfraError;

#[derive(Clone)]
pub struct GithubClient {
    client: Arc<octocrab::Octocrab>,
    config: GithubConfig,
}

impl GithubClient {
    pub fn new(config: GithubConfig) -> AppResult<Self> {
        let client = octocrab::OctocrabBuilder::new()
            .user_access_token(config.token.to_string())
            .build()
            .into_app_result()?;
        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }
}
#[async_trait::async_trait]
impl GithubService for GithubClient {
    async fn dispatch_workflow(&self) -> AppResult<()> {
        self.client
            .actions()
            .create_workflow_dispatch(
                &self.config.owner,
                &self.config.repo,
                &self.config.workflow_id,
                &self.config.r#ref,
            )
            .inputs(serde_json::json!({}))
            .send()
            .await
            .into_app_result()
    }
}
#[derive(Debug, thiserror::Error)]
pub enum GithubClientError {
    #[error("Github error:\n{0}")]
    Github(#[from] Error),
}

trait IntoAppResult<T, E> {
    fn into_app_result(self) -> AppResult<T>;
}

impl<T, E1: Into<GithubClientError>> IntoAppResult<T, E1> for Result<T, E1> {
    fn into_app_result(self) -> AppResult<T> {
        self.map_err(|e| InfraError::Github(e.into()).into())
    }
}
