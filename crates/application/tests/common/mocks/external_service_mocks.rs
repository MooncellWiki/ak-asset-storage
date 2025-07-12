// External Service Mock implementations
use ak_asset_storage_application::{
    AkApiClient, AppError, AppResult, NotificationService, RemoteVersion, StorageService,
};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

// Mock AK API Client
#[derive(Clone, Debug)]
pub struct MockAkApiClient {
    pub remote_version: Arc<Mutex<Option<RemoteVersion>>>,
    pub hot_update_list: Arc<Mutex<Option<String>>>,
    pub file_data: Arc<Mutex<std::collections::HashMap<String, Vec<u8>>>>,
    pub should_fail: Arc<Mutex<bool>>,
}

impl MockAkApiClient {
    pub fn new() -> Self {
        Self {
            remote_version: Arc::new(Mutex::new(None)),
            hot_update_list: Arc::new(Mutex::new(None)),
            file_data: Arc::new(Mutex::new(std::collections::HashMap::new())),
            should_fail: Arc::new(Mutex::new(false)),
        }
    }

    pub fn set_remote_version(&self, version: RemoteVersion) {
        *self.remote_version.lock().unwrap() = Some(version);
    }

    pub fn set_hot_update_list(&self, list: String) {
        *self.hot_update_list.lock().unwrap() = Some(list);
    }

    pub fn set_file_data(&self, path: String, data: Vec<u8>) {
        self.file_data.lock().unwrap().insert(path, data);
    }

    pub fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.lock().unwrap() = should_fail;
    }
}

#[async_trait]
impl AkApiClient for MockAkApiClient {
    async fn get_version(&self) -> AppResult<RemoteVersion> {
        if *self.should_fail.lock().unwrap() {
            return Err(AppError::ExternalService(anyhow::anyhow!(
                "Mock API failure"
            )));
        }

        self.remote_version.lock().unwrap().as_ref().map_or_else(
            || {
                Ok(RemoteVersion {
                    client_version: "1.0.0".to_string(),
                    res_version: "1.0.0".to_string(),
                })
            },
            |version| {
                Ok(RemoteVersion {
                    client_version: version.client_version.clone(),
                    res_version: version.res_version.clone(),
                })
            },
        )
    }

    async fn get_hot_update_list(&self, _res_version: &str) -> AppResult<String> {
        if *self.should_fail.lock().unwrap() {
            return Err(AppError::ExternalService(anyhow::anyhow!(
                "Mock API failure"
            )));
        }
        let result = self.hot_update_list.lock().unwrap();
        result.as_ref().map_or_else(
            || {
                Err(AppError::ExternalService(anyhow::anyhow!(
                    "Hot update list not set"
                )))
            },
            |result| Ok(result.clone()),
        )
    }

    async fn download_file(&self, _res_version: &str, path: &str) -> AppResult<Vec<u8>> {
        if *self.should_fail.lock().unwrap() {
            return Err(AppError::ExternalService(anyhow::anyhow!(
                "Mock download failure"
            )));
        }

        self.file_data.lock().unwrap().get(path).map_or_else(
            || {
                Err(AppError::ExternalService(anyhow::anyhow!(
                    "File not found: {}",
                    path
                )))
            },
            |data| Ok(data.clone()),
        )
    }
}

// Mock Storage Service
#[derive(Clone, Debug)]
pub struct MockStorageService {
    pub stored_files: Arc<Mutex<std::collections::HashMap<String, Vec<u8>>>>,
    pub should_fail: Arc<Mutex<bool>>,
}

impl MockStorageService {
    pub fn new() -> Self {
        Self {
            stored_files: Arc::new(Mutex::new(std::collections::HashMap::new())),
            should_fail: Arc::new(Mutex::new(false)),
        }
    }

    pub fn get_stored_files(&self) -> std::collections::HashMap<String, Vec<u8>> {
        self.stored_files.lock().unwrap().clone()
    }
}

#[async_trait]
impl StorageService for MockStorageService {
    async fn upload(&self, path: &str, data: &[u8]) -> AppResult<()> {
        if *self.should_fail.lock().unwrap() {
            return Err(anyhow::anyhow!("Mock storage failure").into());
        }

        self.stored_files
            .lock()
            .unwrap()
            .insert(path.to_string(), data.to_vec());
        Ok(())
    }
}

type NotificationArgs = (String, String, String, String);
// Mock Notification Service
#[derive(Clone, Debug)]
pub struct MockNotificationService {
    pub update_notifications: Arc<Mutex<Vec<NotificationArgs>>>,
    pub download_notifications: Arc<Mutex<Vec<(String, String)>>>,
}

impl MockNotificationService {
    pub fn new() -> Self {
        Self {
            update_notifications: Arc::new(Mutex::new(Vec::new())),
            download_notifications: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_update_notifications(&self) -> Vec<(String, String, String, String)> {
        self.update_notifications.lock().unwrap().clone()
    }

    pub fn get_download_notifications(&self) -> Vec<(String, String)> {
        self.download_notifications.lock().unwrap().clone()
    }
}

#[async_trait]
impl NotificationService for MockNotificationService {
    async fn notify_update(
        &self,
        old_client: &str,
        old_res: &str,
        new_client: &str,
        new_res: &str,
    ) {
        self.update_notifications.lock().unwrap().push((
            old_client.to_string(),
            old_res.to_string(),
            new_client.to_string(),
            new_res.to_string(),
        ));
    }

    async fn notify_download_finished(&self, client_version: &str, res_version: &str) {
        self.download_notifications
            .lock()
            .unwrap()
            .push((client_version.to_string(), res_version.to_string()));
    }
}
