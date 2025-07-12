use ak_asset_storage_application::{NotificationService, SmtpConfig};
use ak_asset_storage_infrastructure::SmtpNotificationClient;
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum NotificationClient {
    Smtp(SmtpNotificationClient),
    Empty,
}
impl NotificationClient {
    pub fn new(config: &Option<SmtpConfig>) -> Result<Self> {
        Ok(if let Some(cfg) = config {
            Self::Smtp(SmtpNotificationClient::new(cfg)?)
        } else {
            Self::Empty
        })
    }
}

#[async_trait]
impl NotificationService for NotificationClient {
    async fn notify_update(
        &self,
        old_client: &str,
        old_res: &str,
        new_client: &str,
        new_res: &str,
    ) {
        match self {
            Self::Smtp(client) => {
                client
                    .notify_update(old_client, old_res, new_client, new_res)
                    .await;
            }
            Self::Empty => {}
        }
    }
    async fn notify_download_finished(&self, client_version: &str, res_version: &str) {
        match self {
            Self::Smtp(client) => {
                client
                    .notify_download_finished(client_version, res_version)
                    .await;
            }
            Self::Empty => {}
        }
    }
}
