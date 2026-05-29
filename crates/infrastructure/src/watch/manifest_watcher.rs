use ak_asset_storage_application::AssetMappingImportService;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    task::JoinHandle,
};
use tracing::{debug, error, info, warn};

const MANIFEST_NAME: &str = "resource_manifest_idx.json";
const SCAN_INTERVAL: Duration = Duration::from_secs(10);
const IMPORT_TICK_INTERVAL: Duration = Duration::from_secs(10);
const IMPORT_DEBOUNCE: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ManifestFingerprint {
    modified: SystemTime,
    len: u64,
}

#[derive(Clone, Debug)]
struct ManifestSignal {
    res_version: String,
    fingerprint: ManifestFingerprint,
}

#[derive(Clone, Copy, Debug)]
struct PendingImport {
    due_at: Instant,
    fingerprint: ManifestFingerprint,
}

pub struct ManifestWatcher<R>
where
    R: ak_asset_storage_application::VersionRepository
        + ak_asset_storage_application::AssetMappingRepository,
{
    event_tx: Option<UnboundedSender<ManifestSignal>>,
    scan_handle: Option<JoinHandle<()>>,
    import_handle: Option<JoinHandle<()>>,
    gamedata_root: PathBuf,
    _marker: std::marker::PhantomData<R>,
}

impl<R> Drop for ManifestWatcher<R>
where
    R: ak_asset_storage_application::VersionRepository
        + ak_asset_storage_application::AssetMappingRepository,
{
    fn drop(&mut self) {
        if let Some(handle) = self.scan_handle.take() {
            handle.abort();
        }
        let _ = self.event_tx.take();
        if let Some(handle) = self.import_handle.take() {
            handle.abort();
        }
        info!("manifest watcher stopped: {}", self.gamedata_root.display());
    }
}

impl<R> ManifestWatcher<R>
where
    R: ak_asset_storage_application::VersionRepository
        + ak_asset_storage_application::AssetMappingRepository
        + Clone
        + Send
        + Sync
        + 'static,
{
    pub fn new(
        service: AssetMappingImportService<R>,
        gamedata_root: &Path,
    ) -> anyhow::Result<Self> {
        let gamedata_root = fs::canonicalize(gamedata_root)?;
        let (event_tx, event_rx) = unbounded_channel();
        let scan_handle = Some(spawn_scan_loop(event_tx.clone(), gamedata_root.clone()));
        let import_handle = Some(spawn_import_loop(event_rx, service));

        info!("polling gamedata root: {}", gamedata_root.display());

        Ok(Self {
            event_tx: Some(event_tx),
            scan_handle,
            import_handle,
            gamedata_root,
            _marker: std::marker::PhantomData,
        })
    }
}

fn spawn_scan_loop(
    event_tx: UnboundedSender<ManifestSignal>,
    gamedata_root: PathBuf,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut known = HashMap::<String, ManifestFingerprint>::new();
        let mut ticker = tokio::time::interval(SCAN_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;

            let signals = match scan_manifests(&gamedata_root, &mut known) {
                Ok(signals) => signals,
                Err(err) => {
                    error!(
                        "failed to scan gamedata root {}: {err:?}",
                        gamedata_root.display()
                    );
                    continue;
                }
            };

            for signal in signals {
                if event_tx.send(signal).is_err() {
                    warn!(
                        "manifest import loop closed while scanning {}",
                        gamedata_root.display()
                    );
                    return;
                }
            }
        }
    })
}

fn spawn_import_loop<R>(
    mut event_rx: UnboundedReceiver<ManifestSignal>,
    service: AssetMappingImportService<R>,
) -> JoinHandle<()>
where
    R: ak_asset_storage_application::VersionRepository
        + ak_asset_storage_application::AssetMappingRepository
        + Clone
        + Send
        + Sync
        + 'static,
{
    tokio::spawn(async move {
        let mut pending = HashMap::<String, PendingImport>::new();
        let mut ticker = tokio::time::interval(IMPORT_TICK_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                recv = event_rx.recv() => {
                    let Some(signal) = recv else {
                        break;
                    };
                    let res_version = signal.res_version.clone();
                    pending.insert(
                        signal.res_version,
                        PendingImport {
                            due_at: Instant::now() + IMPORT_DEBOUNCE,
                            fingerprint: signal.fingerprint,
                        },
                    );
                    debug!("scheduled manifest import for {res_version}");
                }
                _ = ticker.tick() => {
                    let now = Instant::now();
                    let ready = pending.iter()
                        .filter(|(_, pending_import)| pending_import.due_at <= now)
                        .map(|(res, _)| res.clone())
                        .collect::<Vec<_>>();

                    for res_version in ready {
                        let Some(pending_import) = pending.remove(&res_version) else {
                            continue;
                        };
                        debug!(
                            "importing manifest for {res_version} with fingerprint {:?}",
                            pending_import.fingerprint
                        );
                        match service.import_by_res_version(&res_version).await {
                            Ok(()) => debug!("imported asset mapping for {res_version}"),
                            Err(err) => error!("failed to import asset mapping for {res_version}: {err:?}"),
                        }
                    }
                }
            }
        }
    })
}

fn scan_manifests(
    root: &Path,
    known: &mut HashMap<String, ManifestFingerprint>,
) -> anyhow::Result<Vec<ManifestSignal>> {
    let mut current_versions = HashSet::new();
    let mut signals = Vec::new();

    for dir in list_version_dirs(root)? {
        let Some(res_version) = extract_res_version(root, &dir) else {
            continue;
        };
        current_versions.insert(res_version.clone());

        let manifest_path = dir.join(MANIFEST_NAME);
        match manifest_fingerprint(&manifest_path) {
            Ok(Some(fingerprint)) => {
                let changed = known.get(&res_version) != Some(&fingerprint);
                if changed {
                    let is_new = known.insert(res_version.clone(), fingerprint).is_none();
                    if is_new {
                        info!(
                            "discovered manifest for {res_version}: {}",
                            manifest_path.display()
                        );
                    } else {
                        info!(
                            "manifest changed for {res_version}: {}",
                            manifest_path.display()
                        );
                    }
                    signals.push(ManifestSignal {
                        res_version,
                        fingerprint,
                    });
                }
            }
            Ok(None) => {
                known.remove(&res_version);
            }
            Err(err) => {
                warn!(
                    "failed to inspect manifest {}: {err:?}",
                    manifest_path.display()
                );
            }
        }
    }

    known.retain(|res_version, _| current_versions.contains(res_version));
    Ok(signals)
}

fn list_version_dirs(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_dir() && !ft.is_symlink() {
            dirs.push(entry.path());
        }
    }
    Ok(dirs)
}

fn extract_res_version(root: &Path, path: &Path) -> Option<String> {
    if path.parent() == Some(root) {
        return path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string);
    }
    None
}

fn manifest_fingerprint(path: &Path) -> anyhow::Result<Option<ManifestFingerprint>> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };

    if !metadata.is_file() {
        return Ok(None);
    }

    Ok(Some(ManifestFingerprint {
        modified: metadata.modified()?,
        len: metadata.len(),
    }))
}
