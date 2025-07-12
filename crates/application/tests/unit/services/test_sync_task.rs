// SyncTask tests
use crate::common::*;
use ak_asset_storage_application::{
    AssetDownloadService, RemoteVersion, SyncTask, VersionCheckService,
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_sync_task_creation() {
    // Arrange
    let repository = MockRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();
    let storage = MockStorageService::new();

    let version_check_service =
        VersionCheckService::new(repository.clone(), api_client.clone(), notification.clone());

    let download_service =
        AssetDownloadService::new(repository, api_client, notification, storage, 5);

    // Act
    let _sync_task = SyncTask::new(
        version_check_service,
        download_service,
        Duration::from_secs(60),
    );

    // Assert - Just verify it can be created successfully
    // Test passes if no panic occurs during construction
}

#[tokio::test]
async fn test_perform_poll_with_new_version() {
    // Arrange
    let repository = MockRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();
    let storage = MockStorageService::new();

    // Setup API responses
    let remote_version = RemoteVersion {
        client_version: "1.0.0".to_string(),
        res_version: "1.0.0".to_string(),
    };

    api_client.set_remote_version(remote_version);
    api_client.set_hot_update_list(SAMPLE_HOT_UPDATE_LIST.to_string());
    api_client.set_file_data(
        "arts_furniture_group_hub.dat".to_string(),
        TestData::sample_file_data1(),
    );
    api_client.set_file_data(
        "arts_[pack]common.dat".to_string(),
        TestData::sample_file_data2(),
    );

    let version_check_service =
        VersionCheckService::new(repository.clone(), api_client.clone(), notification.clone());

    let download_service =
        AssetDownloadService::new(repository.clone(), api_client, notification, storage, 5);

    let sync_task = SyncTask::new(
        version_check_service,
        download_service,
        Duration::from_secs(60),
    );

    // Act
    let result = sync_task.perform_poll().await;
    // 得等一下让里面的download task跑完
    sleep(Duration::from_secs(1)).await;

    // Assert
    assert!(result.is_ok());
    // Verify version was processed

    let versions = repository.version.versions.lock().unwrap();
    assert_eq!(versions.len(), 1);
    assert!(versions[0].is_ready); // Should be ready after download
    drop(versions);
}
