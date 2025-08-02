use crate::InfraError;
use ak_asset_storage_application::{AppResult, DockerConfig};
use anyhow::anyhow;
use async_trait::async_trait;
use bollard::{
    auth::DockerCredentials,
    errors::Error,
    models::{ContainerCreateBody, HostConfig},
    query_parameters::{
        CreateContainerOptionsBuilder, CreateImageOptionsBuilder, InspectContainerOptions,
        RemoveContainerOptions, StartContainerOptions,
    },
    Docker,
};
use futures::stream::StreamExt;
use tracing::{info, warn};

// DockerService trait is defined in ak_asset_storage_application

#[derive(Clone)]
pub struct BollardDockerClient {
    docker: Docker,
    config: DockerConfig,
}

impl BollardDockerClient {
    pub fn new(config: DockerConfig) -> AppResult<Self> {
        let docker =
            Docker::connect_with_unix(&config.docker_host, 120, bollard::API_DEFAULT_VERSION)
                .into_app_result()?;

        Ok(Self { docker, config })
    }
}

#[async_trait]
impl ak_asset_storage_application::DockerService for BollardDockerClient {
    async fn launch_container(
        &self,
        client_version: &str,
        res_version: &str,
        prev_client_version: &str,
        prev_res_version: &str,
    ) -> AppResult<String> {
        let image_url = &self.config.image_url;
        let container_name = &self.config.container_name;

        // 检查容器状态
        match self
            .docker
            .inspect_container(container_name, None::<InspectContainerOptions>)
            .await
        {
            Ok(container_info) => {
                // 检查容器是否正在运行
                if let Some(ref state) = container_info.state {
                    if state.running.unwrap_or(false) {
                        return Err(InfraError::Docker(
                            anyhow!("Container {container_name} is already running").into(),
                        )
                        .into());
                    }
                }

                // 容器存在但不是运行状态，删除它
                warn!(
                    "Container {} exists but not running, removing it {:?}",
                    container_name, &container_info.state
                );
                self.docker
                    .remove_container(container_name, None::<RemoveContainerOptions>)
                    .await
                    .into_app_result()?;
                info!("Container removed: {container_name}");
            }
            Err(err @ Error::DockerResponseServerError { status_code, .. }) => {
                if status_code == 404 {
                    // 容器不存在，这是正常情况
                    info!("Container {container_name} does not exist, will create new one");
                } else {
                    return Err(InfraError::Docker(err.into()).into());
                }
            }
            Err(e) => {
                return Err(InfraError::Docker(e.into()).into());
            }
        }

        info!("Pulling Docker image: {image_url}");
        // 拉取镜像
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
        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        info!("Pull progress: {status}");
                    }
                }
                Err(e) => {
                    return Err(InfraError::Docker(e.into()).into());
                }
            }
        }

        info!("Creating container with name: {}", container_name);
        // 创建容器配置
        // 设置卷映射
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
            ..Default::default()
        };

        // 创建容器
        let options = CreateContainerOptionsBuilder::default()
            .name(container_name)
            .build();

        let _container = self
            .docker
            .create_container(Some(options), container_config)
            .await
            .into_app_result()?;

        info!("Starting container: {container_name}");
        // 启动容器
        self.docker
            .start_container(container_name, None::<StartContainerOptions>)
            .await
            .into_app_result()?;

        info!("Container started successfully: {container_name}");
        Ok(container_name.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DockerClientError {
    #[error("Custom error:\n{0}")]
    Custom(#[from] anyhow::Error),

    #[error("Docker error:\n{0}")]
    Docker(#[from] Error),
}

trait IntoAppResult<T, E> {
    fn into_app_result(self) -> AppResult<T>;
}

impl<T, E1: Into<DockerClientError>> IntoAppResult<T, E1> for Result<T, E1> {
    fn into_app_result(self) -> AppResult<T> {
        self.map_err(|e| InfraError::Docker(e.into()).into())
    }
}
