use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DockerConfig {
    pub enabled: bool,
    pub image_url: Option<String>,
    pub container_name: Option<String>,
    pub env_vars: Option<Vec<String>>,
    pub port_mapping: Option<String>,
    pub volume_mapping: Option<String>,
    pub restart_policy: Option<String>,
    pub docker_host: Option<String>,
}
