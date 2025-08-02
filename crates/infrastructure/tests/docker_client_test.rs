use ak_asset_storage_application::{DockerConfig, DockerService};
use ak_asset_storage_infrastructure::external::docker_client::BollardDockerClient;

#[tokio::test]
#[ignore = "need docker"]
async fn test_docker_client_launch_container() {
    // Skip test if Docker is not available
    if std::env::var("SKIP_DOCKER_TESTS").is_ok() {
        println!("Skipping Docker tests");
        return;
    }

    let docker_config = DockerConfig {
        docker_host: "/var/run/docker.sock".to_string(),
        image_url: "registry.cn-shanghai.aliyuncs.com/prts/torappu:latest".to_string(),
        container_name: "test-docker-client".to_string(),
        volume_mapping: Some(vec![
            "/tmp/test-volume:/app/data:rw".to_string(),
            "/tmp/test-config:/app/config:ro".to_string(),
        ]),
        env_vars: Some(vec![
            "TEST_ENV=test_value".to_string(),
            "ANOTHER_ENV=another_value".to_string(),
            "CONTAINER_TYPE=test".to_string(),
        ]),
        username: String::new(),
        password: String::new(),
    };

    let client = BollardDockerClient::new(docker_config.clone()).unwrap();

    // Test launching container with parameters
    let result = client
        .launch_container(
            "client-v1.0.0",
            "res-v2.0.0",
            "prev-client-v0.9.0",
            "prev-res-v1.9.0",
        )
        .await;
    println!("{result:?}");
}
