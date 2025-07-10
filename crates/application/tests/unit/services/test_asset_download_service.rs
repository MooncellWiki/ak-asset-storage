// AssetDownloadService tests
use crate::common::*;
use application::AssetDownloadService;

#[tokio::test]
async fn test_perform_download_success() {
    // Arrange
    let repository = MockRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();
    let storage = MockStorageService::new();

    // Add an unready version
    let version = TestData::create_version(Some(1), "1.0.0", "client-1.0.0", false);
    repository.version.versions.lock().unwrap().push(version);

    // Setup API responses
    api_client.set_hot_update_list(SAMPLE_HOT_UPDATE_LIST.to_string());
    api_client.set_file_data("test_file1.dat".to_string(), TestData::sample_file_data1());
    api_client.set_file_data("test_file2.dat".to_string(), TestData::sample_file_data2());

    let service = AssetDownloadService::new(
        repository.clone(),
        api_client,
        notification.clone(),
        storage.clone(),
    );

    // Act
    let result = service.perform_download().await;

    // Assert
    assert!(result.is_ok());
    assert!(!result.unwrap()); // Should return false (work was done)

    // Verify version was marked as ready
    assert!(repository.version.versions.lock().unwrap()[0].is_ready);

    // Verify files were created
    assert_eq!(repository.file.files.lock().unwrap().len(), 2);

    // Verify bundles were created
    assert_eq!(repository.bundle.bundles.lock().unwrap().len(), 2);

    // Verify notification was sent
    let notifications = notification.get_download_notifications();
    assert_eq!(notifications.len(), 1);
    assert_eq!(
        notifications[0],
        ("client-1.0.0".to_string(), "1.0.0".to_string())
    );
}

#[tokio::test]
async fn test_perform_download_no_pending_versions() {
    // Arrange
    let repository = MockRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();
    let storage = MockStorageService::new();

    let service = AssetDownloadService::new(repository, api_client, notification, storage);

    // Act
    let result = service.perform_download().await;

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap()); // Should return true (no work to do)
}

#[tokio::test]
async fn test_skip_existing_bundle() {
    // Arrange
    let repository = MockRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();
    let storage = MockStorageService::new();

    // Add version and existing bundle
    let version = TestData::create_version(Some(1), "1.0.0", "client-1.0.0", false);
    repository.version.versions.lock().unwrap().push(version);

    let bundle1 = TestData::create_bundle(Some(1), "test/file1.ab", 1, 1);
    let bundle2 = TestData::create_bundle(Some(2), "test/file2.ab", 1, 1);
    repository.bundle.bundles.lock().unwrap().push(bundle1);
    repository.bundle.bundles.lock().unwrap().push(bundle2);

    let service = AssetDownloadService::new(repository.clone(), api_client, notification, storage);

    // Act
    let result = service.perform_download().await;

    // Assert should throw error if it try to download existing bundle
    assert!(result.is_ok());
    assert!(!result.unwrap());

    // Verify no new bundles were created (should skip existing)
    assert_eq!(repository.bundle.bundles.lock().unwrap().len(), 2); // Still only the original bundle
}

#[tokio::test]
async fn test_file_deduplication() {
    // Arrange
    let repository = MockRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();
    let storage = MockStorageService::new();

    // Add existing file with specific hash
    let existing_file = TestData::create_file(
        Some(1),
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
        1024,
    );
    repository.file.files.lock().unwrap().push(existing_file);

    // Add version
    let version = TestData::create_version(Some(1), "1.0.0", "client-1.0.0", false);
    repository.version.versions.lock().unwrap().push(version);

    // Setup API to return file data that produces the same hash
    api_client.set_file_data("test_file1.dat".to_string(), b"hello world".to_vec());
    api_client.set_file_data("test_file2.dat".to_string(), b"hello world".to_vec());

    let service = AssetDownloadService::new(
        repository.clone(),
        api_client,
        notification,
        storage.clone(),
    );

    // Act
    let result = service.perform_download().await;

    // Assert
    assert!(result.is_ok());

    // Verify files were deduplicated (should reuse existing file)
    assert_eq!(repository.file.files.lock().unwrap().len(), 1); // Should still be only 1 file (deduplicated)

    // Verify no duplicate storage uploads
    let stored_files = storage.get_stored_files();
    assert_eq!(stored_files.len(), 0); // No new files stored (reused existing)
}
