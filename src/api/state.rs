use crate::{
    AppError, AppResult,
    config::AppSettings,
    database::Database,
    external::{docker::DockerClient, plocate::PlocateIndex, torappu::TorappuClient},
};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct AppState {
    pub database: Database,
    pub settings: Arc<AppSettings>,
    pub torappu: TorappuClient,
    pub plocate: PlocateIndex,
    pub docker: Option<DockerClient>,
    pub search_gate: SearchGate,
}

/// Caps how many asset searches execute at once (issue #177).
///
/// The gate is shared by the REST and MCP entry points.
/// Requests beyond the limit fail fast instead of queueing,
/// so a burst of expensive searches cannot exhaust worker threads or pile up
/// memory. Callers move the acquired permit into the blocking task so it is
/// held until the search actually finishes.
#[derive(Debug, Clone)]
pub struct SearchGate {
    permits: Arc<Semaphore>,
}

impl SearchGate {
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(limit)),
        }
    }

    /// Fails with `AppError::Unavailable` (503 at the REST layer, a
    /// retryable error over MCP) when all permits are taken.
    pub fn try_acquire(&self) -> AppResult<OwnedSemaphorePermit> {
        self.permits.clone().try_acquire_owned().map_err(|_| {
            AppError::Unavailable("search capacity is busy; retry shortly".to_string())
        })
    }
}

impl AppState {
    pub async fn from_settings(settings: Arc<AppSettings>) -> AppResult<Self> {
        let database = Database::connect(&settings.database).await?;
        let docker = settings.torappu.docker.as_ref().map_or_else(
            || {
                info!("Docker configuration not found, skipping Docker service");
                Ok(None)
            },
            |docker_config| {
                info!("Docker configuration found, creating Docker client");
                DockerClient::new(docker_config.clone())
                    .map(Some)
                    .map_err(|err| {
                        warn!("Failed to create Docker client: {err}");
                        err
                    })
            },
        )?;

        let search_gate = SearchGate::new(settings.torappu.search_concurrency);
        Ok(Self {
            database,
            torappu: TorappuClient {
                asset_base_path: PathBuf::from(&settings.torappu.asset_base_path),
            },
            plocate: PlocateIndex::new(
                std::path::Path::new(&settings.torappu.asset_base_path),
                &settings.torappu.plocate,
            )?,
            settings,
            docker,
            search_gate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_rejects_beyond_the_limit_and_releases_on_drop() {
        let gate = SearchGate::new(2);
        let first = gate.try_acquire().expect("first permit");
        let second = gate.try_acquire().expect("second permit");
        let err = gate.try_acquire().unwrap_err();
        assert!(matches!(err, AppError::Unavailable(_)), "{err:?}");
        drop(first);
        assert!(gate.try_acquire().is_ok(), "dropped permit is reusable");
        drop(second);
    }
}
