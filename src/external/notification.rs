use crate::{AppError, AppResult, config::SmtpConfig};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    address::AddressError,
    message::{MultiPart, SinglePart, header},
    transport::smtp::authentication::Credentials,
};
use tracing::{error, info, instrument};

#[derive(Debug, Clone)]
pub enum NotificationClient {
    Smtp(SmtpNotificationClient),
    Empty,
}

impl NotificationClient {
    pub fn new(config: &Option<SmtpConfig>) -> AppResult<Self> {
        Ok(if let Some(cfg) = config {
            Self::Smtp(SmtpNotificationClient::new(cfg)?)
        } else {
            Self::Empty
        })
    }

    pub async fn notify_update(
        &self,
        old_client: &str,
        old_res: &str,
        new_client: &str,
        new_res: &str,
    ) {
        if let Self::Smtp(client) = self {
            client
                .notify_update(old_client, old_res, new_client, new_res)
                .await;
        }
    }

    pub async fn notify_download_finished(&self, client_version: &str, res_version: &str) {
        if let Self::Smtp(client) = self {
            client
                .notify_download_finished(client_version, res_version)
                .await;
        }
    }
}

#[derive(Debug, Clone)]
pub struct SmtpNotificationClient {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from_email: String,
    to_email: String,
    frontend_url: String,
}

impl SmtpNotificationClient {
    pub fn new(config: &SmtpConfig) -> AppResult<Self> {
        let creds = Credentials::new(config.auth.user.clone(), config.auth.password.clone());
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
            .map_err(|err| AppError::ExternalService(err.into()))?
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

    async fn send_email(&self, message: Message) -> AppResult<()> {
        self.mailer
            .send(message)
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;
        Ok(())
    }

    #[instrument(name = "smtp.notify_update", skip(self))]
    pub async fn notify_update(
        &self,
        old_client: &str,
        old_res: &str,
        new_client: &str,
        new_res: &str,
    ) {
        if let Err(err) = self
            .inner_notify_update(old_client, old_res, new_client, new_res)
            .await
        {
            error!("Failed to send update notification: {err:?}");
        }
    }

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
                    .map_err(|err: AddressError| AppError::ExternalService(err.into()))?,
            )
            .to(self
                .to_email
                .parse()
                .map_err(|err: AddressError| AppError::ExternalService(err.into()))?)
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
            .map_err(|err| AppError::ExternalService(err.into()))?;

        self.send_email(email).await?;
        info!("Version update notification sent");
        Ok(())
    }

    #[instrument(name = "smtp.notify_download_finished", skip(self))]
    pub async fn notify_download_finished(&self, client_version: &str, res_version: &str) {
        if let Err(err) = self
            .inner_notify_download_finished(client_version, res_version)
            .await
        {
            error!("Failed to send download completion notification: {err:?}");
        }
    }

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
                    .map_err(|err: AddressError| AppError::ExternalService(err.into()))?,
            )
            .to(self
                .to_email
                .parse()
                .map_err(|err: AddressError| AppError::ExternalService(err.into()))?)
            .subject(subject)
            .body(body)
            .map_err(|err| AppError::ExternalService(err.into()))?;

        self.send_email(email).await?;
        info!("Download completion notification sent");
        Ok(())
    }
}
