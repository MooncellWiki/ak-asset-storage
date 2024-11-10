use crate::{
    config,
    error::{any_anyhow, Result},
};
use lettre::{transport::smtp::authentication::Credentials, Message, SmtpTransport, Transport};
pub mod notify;

#[derive(Debug, Clone)]
pub struct Mailer {
    pub smtp: SmtpTransport,
    pub fe_url: String,
}
impl Mailer {
    pub fn new(config: &config::Mailer, fe_url: &str) -> Result<Self> {
        let creds = Credentials::new(
            config.smtp.auth.user.clone(),
            config.smtp.auth.password.clone(),
        );
        let smtp = SmtpTransport::relay(&config.smtp.host)?
            .port(config.smtp.port)
            .credentials(creds)
            .build();
        Ok(Self {
            smtp,
            fe_url: fe_url.to_string(),
        })
    }
    pub fn send(&self, email: &Message) -> Result<()> {
        self.smtp.send(email).map_err(any_anyhow)?;
        Ok(())
    }
}
