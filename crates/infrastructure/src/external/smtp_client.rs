use crate::error::{InfraError, InfraResult};
use ak_asset_storage_application::{AppResult, NotificationService, SmtpConfig};
use async_trait::async_trait;
use lettre::{
    message::{header, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use tracing::{info, instrument};

#[derive(Debug, Clone)]
pub struct SmtpNotificationClient {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from_email: String,
    to_email: String,
    frontend_url: String,
}

impl SmtpNotificationClient {
    pub fn new(config: &SmtpConfig) -> InfraResult<Self> {
        let creds = Credentials::new(config.auth.user.clone(), config.auth.password.clone());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
            .map_err(|e| InfraError::Email(e.into()))?
            .port(config.port)
            .credentials(creds)
            .build();

        Ok(Self {
            mailer,
            from_email: config.from_email.clone(),
            to_email: config.to_email.clone(),
            frontend_url: config.frontend_url.clone(),
        })
    }

    fn diff_url(&self) -> String {
        format!("{}/diff?diff=", self.frontend_url)
    }

    async fn send_email(&self, message: Message) -> InfraResult<()> {
        self.mailer
            .send(message)
            .await
            .map_err(|e| InfraError::Email(e.into()))?;
        Ok(())
    }
}
impl SmtpNotificationClient {
    #[instrument(name = "smtp.notify_update", skip(self))]
    async fn inner_notify_update(
        &self,
        old_client: &str,
        old_res: &str,
        new_client: &str,
        new_res: &str,
    ) -> AppResult<()> {
        let subject = format!("AK Asset Update: {old_res} -> {new_res}");

        let plain_body = format!(
            "UPDATE: {} {} -> {} {}\n{}{}...{}",
            old_client,
            old_res,
            new_client,
            new_res,
            self.diff_url(),
            old_res,
            new_res
        );

        let html_body = format!(
            "UPDATE: {} {} -> {} {}<br><a href='{}{}...{}'>View Details</a>",
            old_client,
            old_res,
            new_client,
            new_res,
            self.diff_url(),
            old_res,
            new_res
        );

        let email = Message::builder()
            .from(
                format!("AK Asset Storage Bot <{}>", self.from_email)
                    .parse()
                    .into_app_result()?,
            )
            .to(self.to_email.parse().into_app_result()?)
            .subject(subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_PLAIN)
                            .body(plain_body),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_HTML)
                            .body(html_body),
                    ),
            )
            .into_app_result()?;

        self.send_email(email).await?;
        info!("Version update notification sent");
        Ok(())
    }
    #[instrument(name = "smtp.notify_download_finished", skip(self))]
    async fn inner_notify_download_finished(
        &self,
        client_version: &str,
        res_version: &str,
    ) -> AppResult<()> {
        let subject = format!("AK Asset Download Completed: {client_version} {res_version}");

        let body = format!("Download completed for version {client_version} {res_version}");

        let email = Message::builder()
            .from(
                format!("AK Asset Storage Bot <{}>", self.from_email)
                    .parse()
                    .into_app_result()?,
            )
            .to(self.to_email.parse().into_app_result()?)
            .subject(subject)
            .body(body)
            .into_app_result()?;

        self.send_email(email).await?;
        info!("Download completion notification sent");
        Ok(())
    }
}
#[async_trait]
impl NotificationService for SmtpNotificationClient {
    #[instrument(name = "smtp.notify_update", skip(self))]
    async fn notify_update(
        &self,
        old_client: &str,
        old_res: &str,
        new_client: &str,
        new_res: &str,
    ) {
        self.inner_notify_update(old_client, old_res, new_client, new_res)
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Failed to send update notification: {:?}", e);
            });
    }

    #[instrument(name = "smtp.notify_download_finished", skip(self))]
    async fn notify_download_finished(&self, client_version: &str, res_version: &str) {
        self.inner_notify_download_finished(client_version, res_version)
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Failed to send download completion notification: {:?}", e);
            });
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EmailError {
    #[error("Transport error:\n{0}")]
    Transport(#[from] lettre::transport::smtp::Error),

    #[error("Content error:\n{0}")]
    Content(#[from] lettre::error::Error),

    #[error("Address error:\n{0}")]
    Address(#[from] lettre::address::AddressError),
}

trait IntoAppResult<T, E> {
    fn into_app_result(self) -> AppResult<T>;
}

impl<T, E1: Into<EmailError>> IntoAppResult<T, E1> for Result<T, E1> {
    fn into_app_result(self) -> AppResult<T> {
        self.map_err(|e| InfraError::Email(e.into()).into())
    }
}
