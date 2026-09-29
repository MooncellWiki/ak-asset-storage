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
    pub docker: Option<DockerClient>,
    pub search_gate: SearchGate,
}

/// Caps how many asset searches execute at once (issue #177).
///
/// The gate is shared by the REST and MCP entry points and applies to both
/// search backends. Requests beyond the limit fail fast instead of queueing,
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

    /// Fails with `AppError::ExternalService` (503 at the REST layer) when
    /// all permits are taken.
    pub fn try_acquire(&self) -> AppResult<OwnedSemaphorePermit> {
        self.permits.clone().try_acquire_owned().map_err(|_| {
            AppError::ExternalService(anyhow::anyhow!("search capacity is busy; retry shortly"))
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
                plocate: build_plocate_index(&settings),
            },
            settings,
            docker,
            search_gate,
        })
    }
}

/// If the index cannot be constructed (binaries missing, asset root absent),
/// keep serving: searches degrade to the legacy tree walk, which is slow but
/// functional. The error is loud enough for Sentry to pick up.
fn build_plocate_index(settings: &AppSettings) -> Option<PlocateIndex> {
    if !settings.torappu.plocate.enabled {
        info!("torappu.plocate disabled; asset search falls back to walking the tree");
        return None;
    }
    match PlocateIndex::new(
        std::path::Path::new(&settings.torappu.asset_base_path),
        &settings.torappu.plocate,
    ) {
        Ok(index) => Some(index),
        Err(err) => {
            tracing::error!(error = %err, "plocate index construction failed; asset search falls back to walking the tree");
            None
        }
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
        assert!(matches!(err, AppError::ExternalService(_)), "{err:?}");
        drop(first);
        assert!(gate.try_acquire().is_ok(), "dropped permit is reusable");
        drop(second);
    }
}
