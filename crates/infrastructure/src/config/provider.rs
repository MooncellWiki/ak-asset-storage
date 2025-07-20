use crate::config::AppSettings;
use ak_asset_storage_application::{
    AkApiConfig as AppAkApiConfig, ConfigProvider, DatabaseConfig, LoggerConfig as AppLoggerConfig,
    S3Config as AppS3Config, SentryConfig as AppSentryConfig, ServerConfig as AppServerConfig,
    SmtpConfig as AppSmtpConfig, TorappuConfig,
};
use async_trait::async_trait;

/// Infrastructure implementation of `ConfigProvider`
#[derive(Debug, Clone)]
pub struct InfraConfigProvider {
    pub settings: AppSettings,
}

#[async_trait]
impl ConfigProvider for InfraConfigProvider {
    fn database_config(&self) -> &DatabaseConfig {
        &self.settings.database
    }

    fn server_config(&self) -> &AppServerConfig {
        &self.settings.server
    }

    fn ak_api_config(&self) -> &AppAkApiConfig {
        &self.settings.ak
    }

    fn s3_config(&self) -> &AppS3Config {
        &self.settings.s3
    }

    fn smtp_config(&self) -> &Option<AppSmtpConfig> {
        &self.settings.mailer
    }

    fn logger_config(&self) -> &AppLoggerConfig {
        &self.settings.logger
    }

    fn sentry_config(&self) -> &AppSentryConfig {
        &self.settings.sentry
    }

    fn torappu_config(&self) -> &TorappuConfig {
        &self.settings.torappu
    }
}
