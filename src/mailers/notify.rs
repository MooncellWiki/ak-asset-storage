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
            .from("ak-asset-storage bot <dev.xwbx@qq.com>".parse()?)
            .to("xwbx <1677759063@qq.com>".parse()?)
            .subject(format!("ak res update: {old_res} -> {new_res} "))
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_PLAIN) // plain fallback
                            .body(format!(
                                "UPDATE: {old_client} {old_res} -> {new_client} {new_res} \n {fe_url}/{old_res}...{new_res}"
                            )),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_HTML)
                            .body(format!(
                                "UPDATE: {old_client} {old_res} -> {new_client} {new_res} \n <a href='{fe_url}/#{old_res}...{new_res}'>details</a>"
                            )),
                    ),
            )
            .map_err(any_anyhow)?;
        self.send(&email)
    }
    pub fn notify_error(&self, err: &Error) -> Result<()> {
        let email = Message::builder()
            .from("ak-asset-storage bot <dev.xwbx@qq.com>".parse()?)
            .to("xwbx <1677759063@qq.com>".parse()?)
            .subject(format!("ak res update error: {err:?}"))
            .body(err.to_string())
            .map_err(any_anyhow)?;
        self.send(&email)
    }
}
