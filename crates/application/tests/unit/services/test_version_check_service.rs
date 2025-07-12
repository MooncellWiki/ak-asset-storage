// VersionCheckService tests
use crate::common::*;
use ak_asset_storage_application::{RemoteVersion, VersionCheckService, VersionRepository};

#[tokio::test]
async fn test_perform_check_new_version() {
    // Arrange
    let version_repo = MockVersionRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();

    let remote_version = RemoteVersion {
        client_version: "1.1.0".to_string(),
        res_version: "1.1.0".to_string(),
    };

    api_client.set_remote_version(remote_version);
    api_client.set_hot_update_list(SAMPLE_HOT_UPDATE_LIST.to_string());

    let service = VersionCheckService::new(version_repo.clone(), api_client, notification.clone());

    // Act
    let result = service.perform_check().await;

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap()); // Should return true for new version

    // Verify version was created
    let versions = version_repo.versions.lock().unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].client, "1.1.0");
    assert_eq!(versions[0].res, "1.1.0");
    assert!(!versions[0].is_ready);
    drop(versions);
    // Verify notification was sent
    let notifications = notification.get_update_notifications();
    assert_eq!(notifications.len(), 1);
    assert_eq!(
        notifications[0],
        (
            String::new(),
            String::new(),
            "1.1.0".to_string(),
            "1.1.0".to_string()
        )
    );
}

#[tokio::test]
async fn test_perform_check_no_update() {
    // Arrange
    let version_repo = MockVersionRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();

    // Add existing version
    let existing_version = TestData::create_version(Some(1), "1.0.0", "1.0.0", true);
    version_repo.create_version(existing_version).await.unwrap();

    let remote_version = RemoteVersion {
        client_version: "1.0.0".to_string(),
        res_version: "1.0.0".to_string(),
    };

    api_client.set_remote_version(remote_version);

    let service = VersionCheckService::new(version_repo.clone(), api_client, notification.clone());

    // Act
    let result = service.perform_check().await;

    // Assert
    assert!(result.is_ok());
    assert!(!result.unwrap()); // Should return false for no update

    // Verify no new version was created
    assert_eq!(version_repo.versions.lock().unwrap().len(), 1);

    // Verify no notification was sent
    let notifications = notification.get_update_notifications();
    assert_eq!(notifications.len(), 0);
}

#[tokio::test]
async fn test_check_and_save_first_version() {
    // Arrange
    let version_repo = MockVersionRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();

    api_client.set_hot_update_list(SAMPLE_HOT_UPDATE_LIST.to_string());

    let service = VersionCheckService::new(version_repo.clone(), api_client, notification.clone());

    let remote_version = RemoteVersion {
        client_version: "1.0.0".to_string(),
        res_version: "1.0.0".to_string(),
    };

    // Act
    let result = service.check_and_save(remote_version).await;

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap());

    // Verify version was created
    let versions = version_repo.versions.lock().unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].client, "1.0.0");
    assert_eq!(versions[0].res, "1.0.0");
    drop(versions);

    // Verify notification was sent (first version scenario)
    let notifications = notification.get_update_notifications();
    assert_eq!(notifications.len(), 1);
    assert_eq!(
        notifications[0],
        (
            String::new(),
            String::new(),
            "1.0.0".to_string(),
            "1.0.0".to_string()
        )
    );
}

#[tokio::test]
async fn test_check_and_save_with_previous_version() {
    // Arrange
    let version_repo = MockVersionRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();

    // Add previous version
    let previous_version = TestData::create_version(Some(1), "1.0.0", "1.0.0", true);
    version_repo.create_version(previous_version).await.unwrap();

    api_client.set_hot_update_list(SAMPLE_HOT_UPDATE_LIST.to_string());

    let service = VersionCheckService::new(version_repo.clone(), api_client, notification.clone());

    let remote_version = RemoteVersion {
        client_version: "1.1.0".to_string(),
        res_version: "1.1.0".to_string(),
    };

    // Act
    let result = service.check_and_save(remote_version).await;

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap());

    // Verify notification was sent with correct previous version
    let notifications = notification.get_update_notifications();
    assert_eq!(notifications.len(), 1);
    assert_eq!(
        notifications[0],
        (
            "1.0.0".to_string(),
            "1.0.0".to_string(),
            "1.1.0".to_string(),
            "1.1.0".to_string()
        )
    );
}

#[tokio::test]
async fn test_api_failure_handling() {
    // Arrange
    let version_repo = MockVersionRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();

    api_client.set_should_fail(true);

    let service = VersionCheckService::new(version_repo, api_client, notification);

    // Act
    let result = service.perform_check().await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn test_hot_update_list_parsing() {
    // Arrange
    let version_repo = MockVersionRepository::new();
    let api_client = MockAkApiClient::new();
    let notification = MockNotificationService::new();

    let remote_version = RemoteVersion {
        client_version: "1.0.0".to_string(),
        res_version: "1.0.0".to_string(),
    };

    api_client.set_remote_version(remote_version);
    api_client.set_hot_update_list(LARGE_HOT_UPDATE_LIST.to_string());

    let service = VersionCheckService::new(version_repo.clone(), api_client, notification);

    // Act
    let result = service.perform_check().await;

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap());

    // Verify version was created with correct hot update list
    let versions = version_repo.versions.lock().unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].hot_update_list.ab_infos().len(), 5);
    drop(versions);
}
