use lettre::{
    message::{header, MultiPart, SinglePart},
    Message,
};
use tracing::error;

use crate::error::{any_anyhow, Error, Result};

use super::Mailer;

impl Mailer {
    fn diff_url(&self) -> String {
        format!("{0}/diif?diff=", self.fe_url)
    }
    pub fn notify_update(&self, old_client: &str, old_res: &str, new_client: &str, new_res: &str) {
        if let Err(e) = self.inner_notify_update(old_client, old_res, new_client, new_res) {
            error!("notify update failed: {e:?} {old_client} {old_res} {new_client} {new_res}");
        }
    }
    fn inner_notify_update(
        &self,
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
                                "UPDATE: {old_client} {old_res} -> {new_client} {new_res} \n {0}{old_res}...{new_res}", self.diff_url()
                            )),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_HTML)
                            .body(format!(
                                "UPDATE: {old_client} {old_res} -> {new_client} {new_res} \n <a href='{0}{old_res}...{new_res}'>details</a>", self.diff_url()
                            )),
                    ),
            )
            .map_err(any_anyhow)?;
        self.send(&email)
    }

    pub fn notify_error(&self, err: &Error) {
        if let Err(e) = self.inner_notify_error(err) {
            error!("notify error failed: {e:?} {err:?}");
        }
    }
    fn inner_notify_error(&self, err: &Error) -> Result<()> {
        let email = Message::builder()
            .from("ak-asset-storage bot <dev.xwbx@qq.com>".parse()?)
            .to("xwbx <1677759063@qq.com>".parse()?)
            .subject(format!("ak res update error: {err:?}"))
            .body(err.to_string())
            .map_err(any_anyhow)?;
        self.send(&email)
    }

    pub fn notify_download_finished(&self, new_client: &str, new_res: &str) {
        if let Err(e) = self.inner_notify_download_finished(new_client, new_res) {
            error!("notify download finished failed: {e:?} {new_client} {new_res}");
        }
    }
    fn inner_notify_download_finished(&self, new_client: &str, new_res: &str) -> Result<()> {
        let email = Message::builder()
            .from("ak-asset-storage bot <dev.xwbx@qq.com>".parse()?)
            .to("xwbx <1677759063@qq.com>".parse()?)
            .subject(format!("ak res update finished: {new_client} {new_res}"))
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_PLAIN) // plain fallback
                            .body(format!("ak res update finished: {new_client} {new_res} <a href='{0}{new_res}'>details</a>", self.diff_url())),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_HTML)
                            .body(format!("ak res update finished: {new_client} {new_res} <a href='{0}{new_res}'>details</a>", self.diff_url())),
                    ),
            )
            .map_err(any_anyhow)?;
        self.send(&email)
    }
}
