use crate::{AppError, AppResult, config::DockerConfig};
use anyhow::anyhow;
use bollard::{
    Docker,
    auth::DockerCredentials,
    container::NetworkingConfig,
    errors::Error,
    models::EndpointSettings,
    models::{ContainerCreateBody, HostConfig},
    query_parameters::{
        CreateContainerOptionsBuilder, CreateImageOptionsBuilder, InspectContainerOptions,
        RemoveContainerOptions, StartContainerOptions,
    },
};
use futures::stream::StreamExt;
use std::collections::HashMap;
use tokio::time::{Duration, sleep};
use tracing::{info, warn};

const MAX_IMAGE_PULL_RETRIES: usize = 3;
const IMAGE_PULL_RETRY_DELAYS_SECS: [u64; MAX_IMAGE_PULL_RETRIES] = [1, 4, 8];

#[derive(Debug, Clone)]
pub struct DockerClient {
    docker: Docker,
    config: DockerConfig,
}

impl DockerClient {
    pub fn new(config: DockerConfig) -> AppResult<Self> {
        let docker =
            Docker::connect_with_unix(&config.docker_host, 120, bollard::API_DEFAULT_VERSION)
                .map_err(|err| AppError::ExternalService(err.into()))?;
        Ok(Self { docker, config })
    }

    #[allow(clippy::too_many_lines)]
    pub async fn launch_container(
        &self,
        client_version: &str,
        res_version: &str,
        prev_client_version: &str,
        prev_res_version: &str,
    ) -> AppResult<String> {
        let image_url = &self.config.image_url;
        let container_name = &self.config.container_name;

        match self
            .docker
            .inspect_container(container_name, None::<InspectContainerOptions>)
            .await
        {
            Ok(container_info) => {
                if let Some(state) = &container_info.state
                    && state.running.unwrap_or(false)
                {
                    return Err(AppError::ExternalService(anyhow!(
                        "Container {container_name} is already running"
                    )));
                }

                warn!(
                    "Container {} exists but not running, removing it {:?}",
                    container_name, &container_info.state
                );
                self.docker
                    .remove_container(container_name, None::<RemoveContainerOptions>)
                    .await
                    .map_err(|err| AppError::ExternalService(err.into()))?;
                info!("Container removed: {container_name}");
            }
            Err(Error::DockerResponseServerError {
                status_code: 404, ..
            }) => {
                info!("Container {container_name} does not exist, will create new one");
            }
            Err(err) => return Err(AppError::ExternalService(err.into())),
        }

        info!("Pulling Docker image: {image_url}");
        let mut last_error = None;
        for attempt in 0..=MAX_IMAGE_PULL_RETRIES {
            if attempt > 0 {
                let delay_secs = IMAGE_PULL_RETRY_DELAYS_SECS[attempt - 1];
                warn!(
                    "Retrying image pull (attempt {}/{}) after {}s delay",
                    attempt + 1,
                    MAX_IMAGE_PULL_RETRIES,
                    delay_secs
                );
                sleep(Duration::from_secs(delay_secs)).await;
            }

            let options = CreateImageOptionsBuilder::default()
                .from_image(image_url)
                .build();
            let mut stream = self.docker.create_image(
                Some(options),
                None,
                Some(DockerCredentials {
                    username: Some(self.config.username.clone()),
                    password: Some(self.config.password.clone()),
                    ..Default::default()
                }),
            );

            let mut pull_failed = false;
            while let Some(result) = stream.next().await {
                match result {
                    Ok(progress) => {
                        if let Some(status) = progress.status {
                            info!("Pull progress: {status}");
                        }
                    }
                    Err(err) => {
                        warn!("Image pull failed on attempt {}: {:?}", attempt + 1, err);
                        last_error = Some(err);
                        pull_failed = true;
                        break;
                    }
                }
            }

            if !pull_failed {
                info!("Image pulled successfully");
                break;
            }

            if attempt == MAX_IMAGE_PULL_RETRIES {
                return Err(AppError::ExternalService(
                    last_error
                        .expect("last_error should be set when all retries are exhausted")
                        .into(),
                ));
            }
        }

        let mut host_config = HostConfig::default();
        if let Some(volume_mapping) = &self.config.volume_mapping {
            host_config.binds = Some(volume_mapping.clone());
        }

        let container_config = ContainerCreateBody {
            image: Some(image_url.clone()),
            env: self.config.env_vars.clone(),
            host_config: Some(host_config),
            cmd: Some(vec![
                client_version.to_string(),
                res_version.to_string(),
                "-c".to_string(),
                prev_client_version.to_string(),
                "-r".to_string(),
                prev_res_version.to_string(),
            ]),
            networking_config: Some(
                NetworkingConfig {
                    endpoints_config: HashMap::from([(
                        self.config.network.clone(),
                        EndpointSettings::default(),
                    )]),
                }
                .into(),
            ),
            ..Default::default()
        };

        let options = CreateContainerOptionsBuilder::default()
            .name(container_name)
            .build();

        self.docker
            .create_container(Some(options), container_config)
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;

        self.docker
            .start_container(container_name, None::<StartContainerOptions>)
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;

        info!("Container started successfully: {container_name}");
        Ok(container_name.clone())
    }
}
