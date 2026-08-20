use crate::service::story_usage::StoryUsageImportService;
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

/// The logical marker path contains the `latest` symlink, which torappu
/// re-points at every finished version, so the fingerprint must include the
/// resolved marker path besides mtime + size.
#[derive(Clone, Debug, Eq, PartialEq)]
struct MarkerFingerprint {
    resolved: PathBuf,
    modified: SystemTime,
    len: u64,
}

pub struct GameDataReadyWatcher {
    event_tx: Option<UnboundedSender<MarkerFingerprint>>,
    scan_handle: Option<JoinHandle<()>>,
    import_handle: Option<JoinHandle<()>>,
    marker_path: PathBuf,
}

impl Drop for GameDataReadyWatcher {
    fn drop(&mut self) {
        if let Some(handle) = self.scan_handle.take() {
            handle.abort();
        }
        let _ = self.event_tx.take();
        if let Some(handle) = self.import_handle.take() {
            handle.abort();
        }
        info!(
            "gamedata ready watcher stopped: {}",
            self.marker_path.display()
        );
    }
}

impl GameDataReadyWatcher {
    pub fn new(service: StoryUsageImportService, marker_path: &Path) -> anyhow::Result<Self> {
        // Intentionally NOT canonicalized: `latest` may be re-pointed at a new
        // version directory at any time; every scan re-resolves the symlink.
        let marker_path = marker_path.to_path_buf();
        let (event_tx, event_rx) = unbounded_channel();
        let scan_handle = Some(spawn_scan_loop(event_tx.clone(), marker_path.clone()));
        let import_handle = Some(spawn_import_loop(event_rx, service));

        info!("polling gamedata ready marker: {}", marker_path.display());

        Ok(Self {
            event_tx: Some(event_tx),
            scan_handle,
            import_handle,
            marker_path,
        })
    }
}

fn spawn_scan_loop(
    event_tx: UnboundedSender<MarkerFingerprint>,
    marker_path: PathBuf,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut known: Option<MarkerFingerprint> = None;
        let mut ticker = tokio::time::interval(SCAN_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;

            let path = marker_path.clone();
            let fingerprint =
                match tokio::task::spawn_blocking(move || marker_fingerprint(&path)).await {
                    Ok(Ok(fp)) => fp,
                    Ok(Err(err)) => {
                        error!(
                            "failed to inspect gamedata ready marker {}: {err:?}",
                            marker_path.display()
                        );
                        continue;
                    }
                    Err(err) => {
                        error!(
                            "gamedata ready marker scan task failed for {}: {err}",
                            marker_path.display()
                        );
                        continue;
                    }
                };

            let Some(fingerprint) = fingerprint else {
                known = None;
                continue;
            };

            if known != Some(fingerprint.clone()) {
                let is_new = known.is_none();
                known = Some(fingerprint.clone());
                if is_new {
                    info!(
                        "discovered gamedata ready marker: {}",
                        fingerprint.resolved.display()
                    );
                } else {
                    info!(
                        "gamedata ready marker changed: {}",
                        fingerprint.resolved.display()
                    );
                }
                if event_tx.send(fingerprint).is_err() {
                    warn!(
                        "gamedata ready import loop closed while scanning {}",
                        marker_path.display()
                    );
                    return;
                }
            }
        }
    })
}

fn spawn_import_loop(
    mut event_rx: UnboundedReceiver<MarkerFingerprint>,
    service: StoryUsageImportService,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut pending: Option<Instant> = None;
        let mut ticker = tokio::time::interval(IMPORT_TICK_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                recv = event_rx.recv() => {
                    if recv.is_none() {
                        break;
                    }
                    pending = Some(Instant::now() + IMPORT_DEBOUNCE);
                    debug!("scheduled story usage import");
                }
                _ = ticker.tick() => {
                    let Some(due_at) = pending.take() else {
                        continue;
                    };
                    if due_at > Instant::now() {
                        pending = Some(due_at);
                        continue;
                    }
                    debug!("importing story usages from gamedata ready marker");
                    // Failures are logged only: no pending state and no retry,
                    // the manual import command re-runs the same service.
                    match service.import().await {
                        Ok(()) => debug!("imported story usages"),
                        Err(err) => error!("failed to import story usages: {err:?}"),
                    }
                }
            }
        }
    })
}

fn marker_fingerprint(path: &Path) -> anyhow::Result<Option<MarkerFingerprint>> {
    let resolved = match std::fs::canonicalize(path) {
        Ok(resolved) => resolved,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };

    let metadata = std::fs::metadata(&resolved)?;
    if !metadata.is_file() {
        return Ok(None);
    }

    Ok(Some(MarkerFingerprint {
        resolved,
        modified: metadata.modified()?,
        len: metadata.len(),
    }))
}
