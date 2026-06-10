use crate::service::item_demand_import::ItemDemandImportService;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    task::JoinHandle,
};
use tracing::{debug, error, info, warn};

const SCAN_INTERVAL: Duration = Duration::from_secs(10);
const IMPORT_TICK_INTERVAL: Duration = Duration::from_secs(10);
const IMPORT_DEBOUNCE: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileFingerprint {
    modified: SystemTime,
    len: u64,
}

#[derive(Clone, Copy, Debug)]
struct PendingImport {
    due_at: Instant,
    fingerprint: FileFingerprint,
}

pub struct ItemDemandWatcher {
    event_tx: Option<UnboundedSender<FileFingerprint>>,
    scan_handle: Option<JoinHandle<()>>,
    import_handle: Option<JoinHandle<()>>,
    file_path: PathBuf,
}

impl Drop for ItemDemandWatcher {
    fn drop(&mut self) {
        if let Some(handle) = self.scan_handle.take() {
            handle.abort();
        }
        let _ = self.event_tx.take();
        if let Some(handle) = self.import_handle.take() {
            handle.abort();
        }
        info!("item demand watcher stopped: {}", self.file_path.display());
    }
}

impl ItemDemandWatcher {
    pub fn new(service: ItemDemandImportService, file_path: &Path) -> anyhow::Result<Self> {
        let file_path = std::fs::canonicalize(file_path).or_else(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                Ok(file_path.to_path_buf())
            } else {
                Err(err)
            }
        })?;
        let (event_tx, event_rx) = unbounded_channel();
        let scan_handle = Some(spawn_scan_loop(event_tx.clone(), file_path.clone()));
        let import_handle = Some(spawn_import_loop(event_rx, service));

        info!("polling item demand file: {}", file_path.display());

        Ok(Self {
            event_tx: Some(event_tx),
            scan_handle,
            import_handle,
            file_path,
        })
    }
}

fn spawn_scan_loop(
    event_tx: UnboundedSender<FileFingerprint>,
    file_path: PathBuf,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut known: Option<FileFingerprint> = None;
        let mut ticker = tokio::time::interval(SCAN_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;

            let file_path_clone = file_path.clone();
            let fingerprint =
                match tokio::task::spawn_blocking(move || file_fingerprint(&file_path_clone)).await
                {
                    Ok(Ok(fp)) => fp,
                    Ok(Err(err)) => {
                        error!(
                            "failed to inspect item demand file {}: {err:?}",
                            file_path.display()
                        );
                        continue;
                    }
                    Err(err) => {
                        error!(
                            "item demand scan task failed for {}: {err}",
                            file_path.display()
                        );
                        continue;
                    }
                };

            let Some(fingerprint) = fingerprint else {
                known = None;
                continue;
            };

            let changed = known != Some(fingerprint);
            if changed {
                let is_new = known.is_none();
                known = Some(fingerprint);
                if is_new {
                    info!("discovered item demand file: {}", file_path.display());
                } else {
                    info!("item demand file changed: {}", file_path.display());
                }
                if event_tx.send(fingerprint).is_err() {
                    warn!(
                        "item demand import loop closed while scanning {}",
                        file_path.display()
                    );
                    return;
                }
            }
        }
    })
}

fn spawn_import_loop(
    mut event_rx: UnboundedReceiver<FileFingerprint>,
    service: ItemDemandImportService,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut pending: Option<PendingImport> = None;
        let mut ticker = tokio::time::interval(IMPORT_TICK_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                recv = event_rx.recv() => {
                    let Some(fingerprint) = recv else {
                        break;
                    };
                    pending = Some(PendingImport {
                        due_at: Instant::now() + IMPORT_DEBOUNCE,
                        fingerprint,
                    });
                    debug!("scheduled item demand import");
                }
                _ = ticker.tick() => {
                    let Some(pending_import) = pending.take() else {
                        continue;
                    };
                    if pending_import.due_at > Instant::now() {
                        pending = Some(pending_import);
                        continue;
                    }
                    debug!(
                        "importing item demand with fingerprint {:?}",
                        pending_import.fingerprint
                    );
                    match service.import().await {
                        Ok(()) => debug!("imported item demand"),
                        Err(err) => error!("failed to import item demand: {err:?}"),
                    }
                }
            }
        }
    })
}

fn file_fingerprint(path: &Path) -> anyhow::Result<Option<FileFingerprint>> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };

    if !metadata.is_file() {
        return Ok(None);
    }

    Ok(Some(FileFingerprint {
        modified: metadata.modified()?,
        len: metadata.len(),
    }))
}
