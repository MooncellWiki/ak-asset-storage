use crate::{AppError, AppResult, config::GithubConfig};
use std::sync::Arc;

#[derive(Clone)]
pub struct GithubClient {
    client: Arc<octocrab::Octocrab>,
    config: GithubConfig,
}

impl GithubClient {
    pub fn new(config: GithubConfig) -> AppResult<Self> {
        let client = octocrab::OctocrabBuilder::new()
            .user_access_token(config.token.clone())
            .build()
            .map_err(|err| AppError::ExternalService(err.into()))?;

        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }

    pub async fn dispatch_workflow(&self) -> AppResult<()> {
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
            .map_err(|err| AppError::ExternalService(err.into()))
    }
}
