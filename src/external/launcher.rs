use crate::{AppError, AppResult, config::LauncherConfig, database::launcher::LauncherEntry};
use aes::cipher::{BlockModeDecrypt, KeyIvInit, block_padding::Pkcs7};
use anyhow::{Context, ensure};
use md5::{Digest, Md5};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{collections::HashSet, time::Duration};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LauncherPublication {
    pub package_version: String,
    pub game_version: String,
    pub file_path: String,
    pub manifest_md5: String,
}

#[derive(Debug, Clone)]
pub struct LauncherClient {
    client: Client,
    config: LauncherConfig,
}

impl LauncherClient {
    pub fn new(config: &LauncherConfig) -> AppResult<Self> {
        Ok(Self {
            client: Client::builder()
                .connect_timeout(Duration::from_secs(30))
                .timeout(Duration::from_secs(config.download_timeout_seconds))
                .build()
                .map_err(|err| AppError::ExternalService(err.into()))?,
            config: config.clone(),
        })
    }

    pub async fn latest(&self) -> AppResult<LauncherPublication> {
        let response = self
            .client
            .post(&self.config.latest_url)
            .json(&self.config.request)
            .send()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?
            .error_for_status()
            .map_err(|err| AppError::ExternalService(err.into()))?
            .json::<serde_json::Value>()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;
        let publication = parse_latest(response)?;
        publication.validate()?;
        Ok(publication)
    }

    pub async fn manifest(
        &self,
        publication: &LauncherPublication,
    ) -> anyhow::Result<(Vec<u8>, Vec<LauncherEntry>)> {
        let response = self
            .client
            .get(file_url(&publication.file_path, "game_files")?)
            .send()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?
            .error_for_status()
            .map_err(|err| AppError::ExternalService(err.into()))?;
        // A corrupt endpoint must not consume unbounded memory before parsing.
        let mut response = response;
        let mut ciphertext = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?
        {
            ensure!(
                ciphertext.len() + chunk.len() <= 64 * 1024 * 1024,
                "launcher manifest exceeds 64 MiB"
            );
            ciphertext.extend_from_slice(&chunk);
        }
        let entries = decrypt_manifest(
            &ciphertext,
            &publication.manifest_md5,
            &self.config.aes_key,
            &self.config.aes_iv,
        )?;
        Ok((ciphertext, entries))
    }

    /// Download to an anonymous temporary file; only verified bytes reach S3.
    /// The file is removed even when this future is cancelled during shutdown.
    pub async fn download(
        &self,
        base: &str,
        entry: &LauncherEntry,
    ) -> anyhow::Result<(tokio::fs::File, String)> {
        let mut response = self
            .client
            .get(file_url(base, &entry.path)?)
            .send()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?
            .error_for_status()
            .map_err(|err| AppError::ExternalService(err.into()))?;
        let mut file = tokio::fs::File::from_std(tempfile::tempfile()?);
        let mut md5 = Md5::new();
        let mut sha256 = Sha256::new();
        let mut size = 0_i64;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?
        {
            size = size
                .checked_add(i64::try_from(chunk.len()).context("download chunk too large")?)
                .context("download size overflow")?;
            ensure!(
                size <= entry.size,
                "launcher file exceeds manifest size: {}",
                entry.path
            );
            md5.update(&chunk);
            sha256.update(&chunk);
            file.write_all(&chunk).await?;
        }
        ensure!(size == entry.size, "launcher size mismatch: {}", entry.path);
        ensure!(
            hex::encode(md5.finalize()) == entry.md5,
            "launcher MD5 mismatch: {}",
            entry.path
        );
        file.flush().await?;
        file.rewind().await?;
        Ok((file, hex::encode(sha256.finalize())))
    }
}

/// The launcher batch response reports package and game versions separately.
pub fn parse_latest(response: serde_json::Value) -> anyhow::Result<LauncherPublication> {
    #[derive(Deserialize)]
    struct Batch {
        proxy_rsps: Vec<Proxy>,
    }
    #[derive(Deserialize)]
    struct Proxy {
        kind: String,
        get_latest_game_rsp: Option<Latest>,
    }
    #[derive(Deserialize)]
    struct Latest {
        version: String,
        client_version: String,
        pkg: Package,
    }
    #[derive(Deserialize)]
    struct Package {
        file_path: String,
        game_files_md5: String,
    }
    let batch: Batch =
        serde_json::from_value(response).context("invalid launcher batch response")?;
    let latest = batch
        .proxy_rsps
        .into_iter()
        .find(|response| response.kind == "get_latest_game")
        .and_then(|response| response.get_latest_game_rsp)
        .context("launcher response has no latest game package")?;
    let publication = LauncherPublication {
        package_version: latest.version,
        game_version: latest.client_version,
        file_path: latest.pkg.file_path,
        manifest_md5: latest.pkg.game_files_md5.to_ascii_lowercase(),
    };
    publication.validate()?;
    Ok(publication)
}

impl LauncherPublication {
    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            !self.package_version.trim().is_empty() && !self.game_version.trim().is_empty(),
            "empty launcher version"
        );
        ensure!(
            valid_hex(&self.manifest_md5, 32),
            "invalid launcher manifest MD5"
        );
        file_url(&self.file_path, "game_files")?;
        Ok(())
    }
}

pub fn decrypt_manifest(
    ciphertext: &[u8],
    expected_md5: &str,
    key: &str,
    iv: &str,
) -> anyhow::Result<Vec<LauncherEntry>> {
    ensure!(
        hex::encode(Md5::digest(ciphertext)) == expected_md5.to_ascii_lowercase(),
        "launcher manifest MD5 mismatch"
    );
    let key = hex::decode(key).context("launcher AES key must be hex")?;
    let iv = hex::decode(iv).context("launcher AES IV must be hex")?;
    let plaintext = cbc::Decryptor::<aes::Aes256>::new_from_slices(&key, &iv)
        .map_err(|_| anyhow::anyhow!("invalid launcher AES key/IV length"))?
        .decrypt_padded_vec::<Pkcs7>(ciphertext)
        .map_err(|_| anyhow::anyhow!("invalid launcher manifest padding"))?;
    parse_manifest(std::str::from_utf8(&plaintext).context("launcher manifest is not UTF-8")?)
}

pub fn parse_manifest(plaintext: &str) -> anyhow::Result<Vec<LauncherEntry>> {
    let mut entries = Vec::new();
    let mut paths = HashSet::new();
    for (index, line) in plaintext.trim_start_matches('\u{feff}').lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut entry: LauncherEntry = serde_json::from_str(line)
            .with_context(|| format!("invalid launcher manifest line {}", index + 1))?;
        entry.md5.make_ascii_lowercase();
        validate_path(&entry.path)?;
        ensure!(
            valid_hex(&entry.md5, 32) && entry.size >= 0,
            "invalid launcher file metadata: {}",
            entry.path
        );
        ensure!(
            paths.insert(entry.path.clone()),
            "duplicate launcher path: {}",
            entry.path
        );
        entries.push(entry);
    }
    ensure!(!entries.is_empty(), "launcher manifest contains no files");
    Ok(entries)
}

pub fn validate_path(path: &str) -> anyhow::Result<()> {
    ensure!(
        !path.is_empty()
            && !path.contains(['\\', '\0', '\r', '\n'])
            && path
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != ".."),
        "invalid launcher path"
    );
    Ok(())
}

pub fn file_url(base: &str, path: &str) -> anyhow::Result<Url> {
    validate_path(path)?;
    let mut url = Url::parse(base).context("invalid launcher CDN base URL")?;
    ensure!(
        matches!(url.scheme(), "http" | "https") && url.host_str().is_some(),
        "invalid launcher URL scheme"
    );
    url.set_query(None);
    url.set_fragment(None);
    url.path_segments_mut()
        .map_err(|()| anyhow::anyhow!("invalid launcher CDN base"))?
        .pop_if_empty()
        .extend(path.split('/'));
    Ok(url)
}

fn valid_hex(value: &str, len: usize) -> bool {
    value.len() == len && value.bytes().all(|b| b.is_ascii_hexdigit())
}
