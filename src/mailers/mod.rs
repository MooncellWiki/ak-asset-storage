use crate::{
    config,
    error::{any_anyhow, Result},
};
use lettre::{transport::smtp::authentication::Credentials, Message, SmtpTransport, Transport};
pub mod notify;

#[derive(Debug, Clone)]
pub struct Mailer {
    pub smtp: SmtpTransport,
}
impl Mailer {
    pub fn new(config: &config::Mailer) -> Result<Self> {
        let creds = Credentials::new(
            config.smtp.auth.user.clone(),
            config.smtp.auth.password.clone(),
        );
        let smtp = SmtpTransport::relay(&config.smtp.host)?
            .port(config.smtp.port)
            .credentials(creds)
            .build();
        Ok(Self { smtp })
    }
    pub fn send(&self, email: &Message) -> Result<()> {
        self.smtp.send(email).map_err(any_anyhow)?;
        Ok(())
    }
}
