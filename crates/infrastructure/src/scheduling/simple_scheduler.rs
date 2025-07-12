use ak_asset_storage_application::{AppError, AppResult, ScheduledTask};
use anyhow::anyhow;
use std::sync::Arc;
use tokio::task::JoinHandle;

pub struct SimpleScheduler<T: ScheduledTask> {
    task: Arc<T>,
    handle: Option<JoinHandle<()>>,
}

impl<T: ScheduledTask + 'static> SimpleScheduler<T> {
    pub fn new(task: T) -> Self {
        Self {
            task: Arc::new(task),
            handle: None,
        }
    }

    pub fn start(&mut self) -> AppResult<()> {
        if self.handle.is_some() {
            return Err(AppError::Application(anyhow!("Scheduler already started")));
        }

        let task = self.task.clone();
        let interval_duration = task.interval();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval_duration);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            while task.should_continue() {
                interval.tick().await;

                match task.run().await {
                    Ok(()) => {}
                    Err(e) => task.on_error(&e),
                }
            }
        });

        self.handle = Some(handle);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.task.stop();
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }

    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.handle.is_some()
    }

    #[must_use]
    pub fn task(&self) -> &T {
        &self.task
    }
}
