use crate::{
    AppResult,
    database::{
        Database,
        row::{BundleRow, FileRow, VersionRow},
    },
    external::{ak_api::AkApi, notification::NotificationClient, s3::S3Storage},
    service::types::{ABInfo, HotUpdateList},
};
use anyhow::Context;
use futures::{StreamExt, TryStreamExt, stream};
use sha256::digest;
use std::io::{Cursor, Read};
use tracing::{debug, error, info, instrument};
use zip::ZipArchive;

#[derive(Clone)]
pub struct AssetDownloadService {
    pub database: Database,
    pub ak_api: AkApi,
    pub notification: NotificationClient,
    pub storage: S3Storage,
    pub concurrent: usize,
}

impl AssetDownloadService {
    #[instrument(name = "service.asset_download", skip(self))]
    pub async fn perform_download(&self) -> AppResult<bool> {
        match self.sync_oldest_version().await {
            Ok(has_more) => Ok(has_more),
            Err(err) => {
                error!("download failed: {err:?}");
                Err(err)
            }
        }
    }

    pub async fn manual_download(&self, version_id: Option<i32>) -> AppResult<()> {
        if let Some(id) = version_id {
            self.sync_specific_version(id).await
        } else {
            self.sync_oldest_version().await?;
            Ok(())
        }
    }

    async fn sync_oldest_version(&self) -> AppResult<bool> {
        let version = self.database.get_oldest_unready_version().await?;
        if let Some(version) = version {
            info!("start sync {}-{}", version.res, version.client);
            self.sync_version(&version).await?;
            return Ok(true);
        }
        info!("no pending version to sync");
        Ok(false)
    }

    async fn sync_specific_version(&self, version_id: i32) -> AppResult<()> {
        let version = self
            .database
            .get_version_by_id(version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Version not found: {version_id}"))?;

        info!(
            "start sync specific version {}-{}",
            version.res, version.client
        );
        self.sync_version(&version).await
    }

    async fn sync_version(&self, version: &VersionRow) -> AppResult<()> {
        let version_id = version
            .id
            .ok_or_else(|| anyhow::anyhow!("Version ID is missing"))?;
        let hot_update_list = HotUpdateList::new(&version.hot_update_list)?;

        stream::iter(hot_update_list.ab_infos())
            .map(Ok)
            .try_for_each_concurrent(self.concurrent, |info| {
                self.skip_or_download(info.clone(), version_id, version.res.as_str())
            })
            .await?;

        self.database.mark_version_ready(version_id).await?;
        info!("sync version {} finished", version.res);

        self.notification
            .notify_download_finished(version.client.as_str(), version.res.as_str())
            .await;
        Ok(())
    }

    async fn skip_or_download(
        &self,
        info: ABInfo,
        version_id: i32,
        res_version: &str,
    ) -> AppResult<()> {
        if self
            .database
            .get_bundle_by_version_and_path(version_id, &info.name)
            .await?
            .is_some()
        {
            info!("{} is already downloaded, skip", info.name);
            return Ok(());
        }

        let file_id = self.sync_file(res_version, &info.url()).await?;
        let bundle_path = info.name.clone();
        let bundle = BundleRow {
            id: None,
            path: bundle_path.clone(),
            version_id,
            file_id,
        };

        self.database.create_bundle(bundle).await?;
        info!("{} sync finished", bundle_path);
        Ok(())
    }

    fn calc_sha256(bytes: &[u8]) -> AppResult<String> {
        let mut zip =
            ZipArchive::new(Cursor::new(bytes)).context("Failed to create zip archive")?;
        let mut buffer = Vec::new();
        let mut entries = zip
            .file_names()
            .enumerate()
            .map(|(index, name)| name.map(|name| (name.into_owned(), index)))
            .collect::<zip::result::ZipResult<Vec<_>>>()
            .context("Failed to read zip file names")?;
        entries.sort_by(|(left, _), (right, _)| left.cmp(right));

        // Keep sorting decoded names, but read by index because zip 9 looks up
        // names using their raw bytes, which may not be UTF-8.
        for (_, index) in entries {
            let mut file = zip.by_index(index).context("Failed to read zip file")?;
            file.read_to_end(&mut buffer)
                .context("Failed to push zip file content to buffer")?;
        }

        Ok(digest(&buffer))
    }

    async fn sync_file(&self, res_version: &str, path: &str) -> AppResult<i32> {
        let bytes = self.ak_api.download_file(res_version, path).await?;
        let sha = Self::calc_sha256(&bytes)?;

        if let Some(file) = self.database.get_file_by_hash(&sha).await? {
            debug!("file {} already exists, skip", path);
            return file
                .id
                .ok_or_else(|| anyhow::anyhow!("File ID is missing"))
                .map_err(Into::into);
        }

        let storage_path = format!("/{}/{}/{}", &sha[..2], &sha[2..4], &sha[4..]);
        self.storage.upload(&storage_path, &bytes).await?;

        let file = FileRow {
            id: None,
            hash: sha,
            size: i32::try_from(bytes.len()).context("Failed to convert file size to i32")?,
        };

        let file_id = self.database.create_file(file).await?;
        debug!("sync file {} finished", path);
        Ok(file_id)
    }
}

#[cfg(test)]
mod tests {
    use super::AssetDownloadService;
    use std::io::{Cursor, Write};
    use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

    fn archive(entries: &[(&str, &[u8])], compression: CompressionMethod) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default().compression_method(compression);
        for (name, contents) in entries {
            writer.start_file(*name, options).expect("start ZIP entry");
            writer.write_all(contents).expect("write ZIP entry");
        }
        writer.finish().expect("finish ZIP archive").into_inner()
    }

    #[test]
    fn bundle_hash_ignores_entry_order_and_compression() {
        let expected = "da83f63e1a473003712c18f5afc5a79044221943d1083c7c5a7ac7236d85e8d2";
        let stored = archive(
            &[("b.ab", b"second"), ("a.ab", b"first")],
            CompressionMethod::Stored,
        );
        let deflated = archive(
            &[("a.ab", b"first"), ("b.ab", b"second")],
            CompressionMethod::Deflated,
        );

        for bytes in [stored, deflated] {
            assert_eq!(
                AssetDownloadService::calc_sha256(&bytes).expect("hash bundle"),
                expected
            );
        }
    }

    #[test]
    fn bundle_hash_reads_legacy_encoded_names() {
        let mut bytes = archive(&[("caf_.ab", b"payload")], CompressionMethod::Stored);
        let offsets = bytes
            .windows(b"caf_.ab".len())
            .enumerate()
            .filter_map(|(index, name)| (name == b"caf_.ab").then_some(index))
            .collect::<Vec<_>>();
        assert_eq!(offsets.len(), 2);
        for offset in offsets {
            // Replace the local and central names with CP437-encoded "café.ab".
            bytes[offset + 3] = 0x82;
        }

        assert_eq!(
            AssetDownloadService::calc_sha256(&bytes).expect("hash legacy filename"),
            "239f59ed55e737c77147cf55ad0c1b030b6d7ee748a7426952f9b852d5a935e5"
        );
    }

    #[test]
    fn bundle_hash_preserves_game_fixture_hashes() {
        let fixtures: &[(&[u8], &str)] = &[
            (
                include_bytes!(
                    "../../e2e/fixtures/upstream/26-05-27-13-32-37_d44f28/arts_avg_shader_profile.dat"
                ),
                "e7997e26db3e1957dfbd96b84d11a89d5c484422850fb83e591727a9ebb9e7c4",
            ),
            (
                include_bytes!(
                    "../../e2e/fixtures/upstream/26-05-27-13-32-37_d44f28/arts_charportraits_portraits_hub.dat"
                ),
                "db3c7be7c006029db5bf2c710bf98b8b1e767d33573e95f8b664d91847abe3fe",
            ),
            (
                include_bytes!(
                    "../../e2e/fixtures/upstream/26-05-27-13-32-37_d44f28/scenes_activities_a001_level_a001_01_level_a001_01_lightingdata.dat"
                ),
                "e044b54cc71050bec6ad3bd118cff5c198ca5316b724e88ebd60ecc408e5338b",
            ),
        ];

        for (bytes, expected) in fixtures {
            assert_eq!(
                AssetDownloadService::calc_sha256(bytes).expect("hash game fixture"),
                *expected
            );
        }
    }
}
