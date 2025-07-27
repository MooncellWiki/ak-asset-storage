use ak_asset_storage_application::{GithubConfig, GithubService};
use ak_asset_storage_infrastructure::external::github_client::GithubClient;
use std::env;

#[tokio::test]
#[ignore]
async fn test_github_client_dispatch_workflow() {
    // Skip test if GitHub token is not available
    let token = env::var("GITHUB_TOKEN").unwrap();

    let github_config = GithubConfig {
        owner: "MooncellWiki".to_string(),
        repo: "Ptilopsis_Bot".to_string(),
        workflow_id: "main1.yml".to_string(),
        r#ref: "master".to_string(),
        token,
    };

    let client = GithubClient::new(github_config).unwrap();

    // Test dispatching workflow
    let result = client.dispatch_workflow().await;
    println!("Workflow dispatch result: {result:?}");

    // For testing purposes, we expect this might fail due to permissions
    // but we want to verify the client can make the API call
    assert!(result.is_ok());
}
