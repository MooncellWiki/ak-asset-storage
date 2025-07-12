use crate::{AppError, AppResult};
use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait ScheduledTask: Send + Sync {
    /// Execute the task
    async fn run(&self) -> AppResult<()>;

    /// Get the interval between task executions
    fn interval(&self) -> Duration;

    /// Handle errors (default implementation logs the error)
    fn on_error(&self, error: &AppError) {
        tracing::error!("Task execution failed: {:?}", error);
    }
    fn stop(&self) {}

    /// Check if the task should continue running (default: always true)
    fn should_continue(&self) -> bool {
        true
    }
}
