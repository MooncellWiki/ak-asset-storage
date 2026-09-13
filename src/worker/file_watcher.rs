use crate::AppResult;
use std::{
    future::Future,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};
use tokio::{sync::mpsc::unbounded_channel, task::JoinHandle};
use tracing::{debug, error, info, warn};

const SCAN_INTERVAL: Duration = Duration::from_secs(10);
const IMPORT_TICK_INTERVAL: Duration = Duration::from_secs(10);
const IMPORT_DEBOUNCE: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
pub enum PathResolution {
    Direct,
    Canonical,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileFingerprint {
    path: PathBuf,
    modified: SystemTime,
    len: u64,
}

pub struct PollingFileWatcher {
    scan_handle: Option<JoinHandle<()>>,
    import_handle: Option<JoinHandle<()>>,
    source_name: &'static str,
    path: PathBuf,
}

impl Drop for PollingFileWatcher {
    fn drop(&mut self) {
        if let Some(handle) = self.scan_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.import_handle.take() {
            handle.abort();
        }
        info!(
            "{} watcher stopped: {}",
            self.source_name,
            self.path.display()
        );
    }
}

impl PollingFileWatcher {
    pub fn new<Import, ImportFuture>(
        path: &Path,
        resolution: PathResolution,
        source_name: &'static str,
        import_name: &'static str,
        import: Import,
    ) -> Self
    where
        Import: FnMut() -> ImportFuture + Send + 'static,
        ImportFuture: Future<Output = AppResult<()>> + Send + 'static,
    {
        let path = path.to_path_buf();
        let (event_tx, event_rx) = unbounded_channel();
        let scan_handle = Some(spawn_scan_loop(
            event_tx,
            path.clone(),
            resolution,
            source_name,
        ));
        let import_handle = Some(spawn_import_loop(event_rx, import_name, import));

        info!("polling {source_name}: {}", path.display());

        Self {
            scan_handle,
            import_handle,
            source_name,
            path,
        }
    }
}

fn spawn_scan_loop(
    event_tx: tokio::sync::mpsc::UnboundedSender<()>,
    path: PathBuf,
    resolution: PathResolution,
    source_name: &'static str,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut known: Option<FileFingerprint> = None;
        let mut ticker = tokio::time::interval(SCAN_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;

            let inspected_path = path.clone();
            let fingerprint = match tokio::task::spawn_blocking(move || {
                file_fingerprint(&inspected_path, resolution)
            })
            .await
            {
                Ok(Ok(fingerprint)) => fingerprint,
                Ok(Err(err)) => {
                    error!(
                        "failed to inspect {source_name} {}: {err:?}",
                        path.display()
                    );
                    continue;
                }
                Err(err) => {
                    error!(
                        "{source_name} scan task failed for {}: {err}",
                        path.display()
                    );
                    continue;
                }
            };

            let Some(fingerprint) = fingerprint else {
                known = None;
                continue;
            };

            if known == Some(fingerprint.clone()) {
                continue;
            }

            let is_new = known.is_none();
            known = Some(fingerprint.clone());
            let change = if is_new { "discovered" } else { "changed" };
            info!("{change} {source_name}: {}", fingerprint.path.display());
            if event_tx.send(()).is_err() {
                warn!(
                    "import loop closed while scanning {source_name} {}",
                    path.display()
                );
                return;
            }
        }
    })
}

fn spawn_import_loop<Import, ImportFuture>(
    mut event_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
    import_name: &'static str,
    mut import: Import,
) -> JoinHandle<()>
where
    Import: FnMut() -> ImportFuture + Send + 'static,
    ImportFuture: Future<Output = AppResult<()>> + Send + 'static,
{
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
                    debug!("scheduled {import_name} import");
                }
                _ = ticker.tick() => {
                    let Some(due_at) = pending.take() else {
                        continue;
                    };
                    if due_at > Instant::now() {
                        pending = Some(due_at);
                        continue;
                    }
                    debug!("importing {import_name}");
                    match import().await {
                        Ok(()) => debug!("imported {import_name}"),
                        Err(err) => error!("failed to import {import_name}: {err:?}"),
                    }
                }
            }
        }
    })
}

fn file_fingerprint(
    path: &Path,
    resolution: PathResolution,
) -> anyhow::Result<Option<FileFingerprint>> {
    let resolved = match resolution {
        PathResolution::Direct => path.to_path_buf(),
        PathResolution::Canonical => match std::fs::canonicalize(path) {
            Ok(resolved) => resolved,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.into()),
        },
    };

    let metadata = match std::fs::metadata(&resolved) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    if !metadata.is_file() {
        return Ok(None);
    }

    Ok(Some(FileFingerprint {
        path: resolved,
        modified: metadata.modified()?,
        len: metadata.len(),
    }))
}
