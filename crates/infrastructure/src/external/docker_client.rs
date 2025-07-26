use crate::InfraError;
use ak_asset_storage_application::{AppResult, DockerConfig};
use async_trait::async_trait;
use bollard::{
    models::{ContainerCreateBody, HostConfig},
    query_parameters::{
        CreateContainerOptions, CreateImageOptions, InspectContainerOptions,
        RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
    },
    Docker,
};
use futures::stream::StreamExt;
use tracing::{error, info, warn};

// DockerService trait is defined in ak_asset_storage_application

#[derive(Clone)]
pub struct BollardDockerService {
    docker: Docker,
    config: DockerConfig,
}

impl BollardDockerService {
    pub fn new(config: DockerConfig) -> AppResult<Self> {
        let docker = Docker::connect_with_unix(&config.docker_host, 120, &bollard::API_DEFAULT_VERSION)
            .map_err(|e| InfraError::Docker(format!("Failed to connect to Docker: {}", e)))?;

        Ok(Self { docker, config })
    }

    fn parse_env_vars(
        &self,
        env_vars: &[String],
        client_version: &str,
        res_version: &str,
    ) -> Vec<String> {
        let mut env = Vec::new();

        for var in env_vars {
            let processed = var
                .replace("${CLIENT_VERSION}", client_version)
                .replace("${RES_VERSION}", res_version);
            env.push(processed);
        }

        env
    }

    fn parse_volume_mapping(&self, mapping: &str) -> Vec<String> {
        mapping.split(',').map(|s| s.to_string()).collect()
    }
}

#[async_trait]
impl ak_asset_storage_application::DockerService for BollardDockerService {
    async fn launch_container(&self, client_version: &str, res_version: &str) -> AppResult<String> {
        let image_url = &self.config.image_url;
        let container_name = &self.config.container_name;

        // 检查容器是否已存在
        if self.container_exists(&container_name).await? {
            warn!(
                "Container {} already exists, stopping and removing it",
                container_name
            );
            self.stop_container(&container_name).await.ok();
            self.remove_container(&container_name).await.ok();
        }

        info!("Pulling Docker image: {}", image_url);

        // 拉取镜像
        let options = CreateImageOptions {
            from_image: Some(image_url.clone()),
            ..Default::default()
        };

        let mut stream = self.docker.create_image(Some(options), None, None);
        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        info!("Pull progress: {}", status);
                    }
                }
                Err(e) => {
                    error!("Failed to pull image: {}", e);
                    return Err(InfraError::Docker(format!("Failed to pull image: {}", e)).into());
                }
            }
        }

        info!("Creating container with name: {}", container_name);

        // 创建容器配置
        let env_vars = if let Some(env_vars) = &self.config.env_vars {
            self.parse_env_vars(env_vars, client_version, res_version)
        } else {
            Vec::new()
        };

        // 设置卷映射
        let mut host_config = HostConfig::default();

        if let Some(volume_mapping) = &self.config.volume_mapping {
            host_config.binds = Some(self.parse_volume_mapping(volume_mapping));
        }

        let container_config = ContainerCreateBody {
            image: Some(image_url.clone()),
            env: if env_vars.is_empty() {
                None
            } else {
                Some(env_vars)
            },
            host_config: Some(host_config),
            ..Default::default()
        };

        // 创建容器
        let options = CreateContainerOptions {
            name: Some(container_name.clone()),
            platform: String::new(),
        };

        let _container = self
            .docker
            .create_container(Some(options), container_config)
            .await
            .map_err(|e| InfraError::Docker(format!("Failed to create container: {}", e)))?;

        info!("Starting container: {}", container_name);

        // 启动容器
        self.docker
            .start_container(&container_name, None::<StartContainerOptions>)
            .await
            .map_err(|e| InfraError::Docker(format!("Failed to start container: {}", e)))?;

        info!("Container started successfully: {}", container_name);

        Ok(container_name.to_string())
    }

    async fn stop_container(&self, container_name: &str) -> AppResult<()> {
        self.docker
            .stop_container(container_name, None::<StopContainerOptions>)
            .await
            .map_err(|e| InfraError::Docker(format!("Failed to stop container: {}", e)))?;

        info!("Container stopped: {}", container_name);
        Ok(())
    }

    async fn remove_container(&self, container_name: &str) -> AppResult<()> {
        self.docker
            .remove_container(container_name, None::<RemoveContainerOptions>)
            .await
            .map_err(|e| InfraError::Docker(format!("Failed to remove container: {}", e)))?;

        info!("Container removed: {}", container_name);
        Ok(())
    }

    async fn container_exists(&self, container_name: &str) -> AppResult<bool> {
        match self
            .docker
            .inspect_container(container_name, None::<InspectContainerOptions>)
            .await
        {
            Ok(_) => Ok(true),
            Err(bollard::errors::Error::DockerResponseServerError { status_code, .. }) => {
                if status_code == 404 {
                    Ok(false)
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false),
        }
    }
}
