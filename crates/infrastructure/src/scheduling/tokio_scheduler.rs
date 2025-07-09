use application::error::AppResult;
use application::ports::scheduler::{ScheduleId, ScheduledTaskFn, Scheduler};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{info, instrument};

pub struct TokioScheduler {
    next_id: AtomicU64,
    tasks: Arc<RwLock<HashMap<ScheduleId, JoinHandle<()>>>>,
}

impl TokioScheduler {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn next_schedule_id(&self) -> ScheduleId {
        ScheduleId(self.next_id.fetch_add(1, Ordering::SeqCst))
    }
}

#[async_trait]
impl Scheduler for TokioScheduler {
    #[instrument(name = "scheduler.schedule_periodic", skip(self, task))]
    async fn schedule_periodic(
        &self,
        interval_duration: Duration,
        task: ScheduledTaskFn,
    ) -> AppResult<ScheduleId> {
        let schedule_id = self.next_schedule_id();
        let tasks = self.tasks.clone();

        info!(
            "Starting periodic task with interval of {:?}",
            interval_duration
        );

        let handle = tokio::spawn(async move {
            let mut timer = interval(interval_duration);

            loop {
                timer.tick().await;
                let future = task();
                future.await;
            }
        });

        tasks.write().await.insert(schedule_id, handle);
        Ok(schedule_id)
    }

    #[instrument(name = "scheduler.schedule_once", skip(self, task))]
    async fn schedule_once(&self, delay: Duration, task: ScheduledTaskFn) -> AppResult<ScheduleId> {
        let schedule_id = self.next_schedule_id();
        let tasks = self.tasks.clone();

        info!("Scheduling one-time task with delay of {:?}", delay);

        let tasks_for_cleanup = tasks.clone();
        let cleanup_id = schedule_id;

        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let future = task();
            future.await;

            // Auto-remove the task when it completes
            tasks_for_cleanup.write().await.remove(&cleanup_id);
        });

        tasks.write().await.insert(schedule_id, handle);
        Ok(schedule_id)
    }

    #[instrument(name = "scheduler.cancel", skip(self))]
    async fn cancel(&self, schedule_id: ScheduleId) -> AppResult<()> {
        let mut tasks = self.tasks.write().await;

        if let Some(handle) = tasks.remove(&schedule_id) {
            handle.abort();
            info!("Cancelled scheduled task: {:?}", schedule_id);
        }

        Ok(())
    }

    #[instrument(name = "scheduler.shutdown", skip(self))]
    async fn shutdown(&self) -> AppResult<()> {
        let mut tasks = self.tasks.write().await;

        info!("Shutting down scheduler, cancelling {} tasks", tasks.len());

        for (_, handle) in tasks.drain() {
            handle.abort();
        }

        Ok(())
    }
}
