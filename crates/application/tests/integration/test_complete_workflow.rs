// Integration tests for complete workflow
use crate::common::*;
use application::{AssetDownloadService, RemoteVersion, VersionCheckService};

#[tokio::test]
async fn test_complete_sync_workflow() {
    // Arrange
    let repository = MockRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();
    let storage = MockStorageService::new();

    // Setup remote version
    let remote_version = RemoteVersion {
        client_version: "1.1.0".to_string(),
        res_version: "1.1.0".to_string(),
    };

    api_client.set_remote_version(remote_version);
    api_client.set_hot_update_list(SAMPLE_HOT_UPDATE_LIST.to_string());

    // Setup file data for download
    api_client.set_file_data("test_file1.dat".to_string(), TestData::sample_file_data1());
    api_client.set_file_data("test_file2.dat".to_string(), TestData::sample_file_data2());

    let version_service =
        VersionCheckService::new(repository.clone(), api_client.clone(), notification.clone());

    let download_service = AssetDownloadService::new(
        repository.clone(),
        api_client,
        notification.clone(),
        storage.clone(),
    );

    // Act - Step 1: Check for new version
    let check_result = version_service.perform_check().await;
    assert!(check_result.is_ok());
    assert!(check_result.unwrap()); // Should find new version

    // Act - Step 2: Download assets
    let download_result = download_service.perform_download().await;
    assert!(download_result.is_ok());
    assert!(!download_result.unwrap()); // Should return false indicating work was done

    // Assert - Verify complete workflow
    let versions = repository.version.versions.lock().unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].client, "1.1.0");
    assert_eq!(versions[0].res, "1.1.0");
    assert!(versions[0].is_ready); // Should be marked as ready after download
    drop(versions);
    assert_eq!(repository.file.files.lock().unwrap().len(), 2); // Should have created 2 files

    assert_eq!(repository.bundle.bundles.lock().unwrap().len(), 2); // Should have created 2 bundles

    // Verify notifications were sent
    let update_notifications = notification.get_update_notifications();
    assert_eq!(update_notifications.len(), 1);

    let download_notifications = notification.get_download_notifications();
    assert_eq!(download_notifications.len(), 1);

    // Verify files were stored
    let stored_files = storage.get_stored_files();
    assert_eq!(stored_files.len(), 2);
}

#[tokio::test]
async fn test_multiple_versions_sync() {
    // Arrange
    let repository = MockRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();
    let storage = MockStorageService::new();

    let download_service = AssetDownloadService::new(
        repository.clone(),
        api_client.clone(),
        notification.clone(),
        storage.clone(),
    );

    // Add multiple unready versions
    let version1 = TestData::create_version(Some(1), "1.0.0", "client-1.0.0", false);
    let version2 = TestData::create_version(Some(2), "1.1.0", "client-1.1.0", false);
    let version3 = TestData::create_version(Some(3), "1.2.0", "client-1.2.0", false);

    repository.version.versions.lock().unwrap().push(version1);
    repository.version.versions.lock().unwrap().push(version2);
    repository.version.versions.lock().unwrap().push(version3);

    // Setup API responses
    api_client.set_file_data("test_file1.dat".to_string(), TestData::sample_file_data1());
    api_client.set_file_data("test_file2.dat".to_string(), TestData::sample_file_data2());

    // Act - Download should process oldest version first
    let result1 = download_service.perform_download().await;
    assert!(result1.is_ok());
    assert!(!result1.unwrap()); // Should return false (more work to do)

    // Act - Download next version
    let result2 = download_service.perform_download().await;
    assert!(result2.is_ok());
    assert!(!result2.unwrap()); // Should return false (still more work to do)

    // Act - Download final version
    let result3 = download_service.perform_download().await;
    assert!(result3.is_ok());
    assert!(!result3.unwrap()); // Should return false (last version processed)

    // Now all versions should be ready
    let result4 = download_service.perform_download().await;
    assert!(result4.is_ok());
    assert!(result4.unwrap()); // Should return true (no more work)

    // Verify all versions are marked as ready
    assert!(repository
        .version
        .versions
        .lock()
        .unwrap()
        .iter()
        .all(|v| v.is_ready));
}

#[tokio::test]
async fn test_error_recovery() {
    // Arrange
    let repository = MockRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();
    let storage = MockStorageService::new();

    let download_service = AssetDownloadService::new(
        repository.clone(),
        api_client.clone(),
        notification.clone(),
        storage.clone(),
    );

    // Add a version
    let version = TestData::create_version(Some(1), "1.0.0", "client-1.0.0", false);
    repository.version.versions.lock().unwrap().push(version);

    // Setup API to fail initially
    api_client.set_should_fail(true);

    // Act - First attempt should fail
    let result1 = download_service.perform_download().await;
    assert!(result1.is_err());

    // Fix the API
    api_client.set_should_fail(false);
    api_client.set_file_data("test_file1.dat".to_string(), TestData::sample_file_data1());
    api_client.set_file_data("test_file2.dat".to_string(), TestData::sample_file_data2());

    // Act - Second attempt should succeed
    let result2 = download_service.perform_download().await;
    assert!(result2.is_ok());
    assert!(!result2.unwrap());

    // Verify version was processed successfully
    assert!(repository.version.versions.lock().unwrap()[0].is_ready);
}
