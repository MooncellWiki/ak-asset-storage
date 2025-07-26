use crate::InfraError;
use ak_asset_storage_application::{AppResult, DockerConfig};
use async_trait::async_trait;
use bollard::{
    container::{
        Config as ContainerConfig, CreateContainerOptions, InspectContainerOptions,
        RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
    },
    image::CreateImageOptions,
    models::HostConfig,
    models::PortBinding,
    service::RestartPolicy,
    service::RestartPolicyNameEnum,
    Docker,
};
use futures::stream::StreamExt;
use std::collections::HashMap;
use tracing::{error, info, warn};

// DockerService trait is defined in ak_asset_storage_application

#[derive(Clone)]
pub struct BollardDockerService {
    docker: Docker,
    config: DockerConfig,
}

impl BollardDockerService {
    pub fn new(config: DockerConfig) -> AppResult<Self> {
        let docker = if let Some(_host) = &config.docker_host {
            Docker::connect_with_http_defaults()
                .map_err(|e| InfraError::Docker(format!("Failed to connect to Docker: {}", e)))?
        } else {
            Docker::connect_with_local_defaults()
                .map_err(|e| InfraError::Docker(format!("Failed to connect to Docker: {}", e)))?
        };

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
        if !self.config.enabled {
            return Err(InfraError::Docker("Docker service is disabled".to_string()).into());
        }

        let image_url = self
            .config
            .image_url
            .as_ref()
            .ok_or_else(|| InfraError::Docker("Docker image URL not configured".to_string()))?;

        let container_name = self
            .config
            .container_name
            .as_ref()
            .unwrap_or(&format!("ak-asset-{}-{}", client_version, res_version))
            .to_string();

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
            from_image: image_url.clone(),
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

        // 设置端口映射和卷映射
        let mut host_config = HostConfig::default();

        // 处理端口映射
        if let Some(port_mapping) = &self.config.port_mapping {
            let mut port_bindings = HashMap::new();
            for binding in port_mapping.split(',') {
                let parts: Vec<&str> = binding.split(':').collect();
                if parts.len() == 2 {
                    let host_port = parts[0];
                    let container_port = parts[1];
                    let binding = PortBinding {
                        host_ip: None,
                        host_port: Some(host_port.to_string()),
                    };
                    port_bindings.insert(format!("{}/tcp", container_port), Some(vec![binding]));
                }
            }
            host_config.port_bindings = Some(port_bindings);
        }

        if let Some(volume_mapping) = &self.config.volume_mapping {
            host_config.binds = Some(self.parse_volume_mapping(volume_mapping));
        }

        // 处理重启策略
        if let Some(restart_policy) = &self.config.restart_policy {
            let name = match restart_policy.as_str() {
                "always" => RestartPolicyNameEnum::ALWAYS,
                "unless-stopped" => RestartPolicyNameEnum::UNLESS_STOPPED,
                "on-failure" => RestartPolicyNameEnum::ON_FAILURE,
                _ => RestartPolicyNameEnum::NO,
            };
            host_config.restart_policy = Some(RestartPolicy {
                name: Some(name),
                maximum_retry_count: None,
            });
        }

        let container_config = ContainerConfig {
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
            name: container_name.clone(),
            platform: None,
        };

        let container = self
            .docker
            .create_container(Some(options), container_config)
            .await
            .map_err(|e| InfraError::Docker(format!("Failed to create container: {}", e)))?;

        info!("Starting container: {}", container_name);

        // 启动容器
        self.docker
            .start_container(&container_name, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| InfraError::Docker(format!("Failed to start container: {}", e)))?;

        info!("Container started successfully: {}", container_name);

        Ok(container_name)
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
