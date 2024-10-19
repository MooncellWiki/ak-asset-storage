use lettre::{
    message::{header, MultiPart, SinglePart},
    Message,
};

use crate::error::{any_anyhow, Error, Result};

use super::Mailer;

impl Mailer {
    pub fn notify_update(
        &self,
        fe_url: &str,
        old_client: &str,
        old_res: &str,
        new_client: &str,
        new_res: &str,
    ) -> Result<()> {
        let email = Message::builder()
            .from("ak-asset-storage bot <dev.xwbx@qq.com>".parse().unwrap())
            .to("xwbx <1677759063@qq.com>".parse().unwrap())
            .subject(format!("ak res update: {} -> {} ", old_res, new_res))
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_PLAIN) // plain fallback
                            .body(format!(
                                "UPDATE: {} {} -> {} {} \n {}/{}...{}",
                                old_client, old_res, new_client, new_res, fe_url, old_res, new_res
                            )),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_HTML)
                            .body(format!(
                                "UPDATE: {} {} -> {} {} \n <a href='{}/#{}...{}'>details</a>",
                                old_client, old_res, new_client, new_res, fe_url, old_res, new_res
                            )),
                    ),
            )
            .map_err(any_anyhow)?;
        self.send(email)
    }
    pub fn notify_error(&self, err: &Error) -> Result<()> {
        let email = Message::builder()
            .from("ak-asset-storage bot <dev.xwbx@qq.com>".parse().unwrap())
            .to("xwbx <1677759063@qq.com>".parse().unwrap())
            .subject(format!("ak res update error: {:?}", err))
            .body(err.to_string())
            .map_err(any_anyhow)?;
        self.send(email)
    }
}
