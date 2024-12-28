use std::sync::Arc;

use crate::config;
use sentry::{ClientInitGuard, ClientOptions, TransactionContext};
use tracing::debug;

pub fn init(config: &config::Sentry) -> ClientInitGuard {
    let traces_sample_rate = config.traces_sample_rate;
    let traces_sampler = move |ctx: &TransactionContext| -> f32 {
        if let Some(sampled) = ctx.sampled() {
            debug!("sampled");
            return if sampled { 1.0 } else { 0.0 };
        }

        let op = ctx.operation();
        debug!("op {}", op);
        if op.starts_with("worker") {
            // Record all traces for background tasks
            return 1.;
        }

        traces_sample_rate
    };

    sentry::init((
        config.dsn.to_string(),
        ClientOptions {
            traces_sample_rate: config.traces_sample_rate,
            release: sentry::release_name!(),
            max_breadcrumbs: 10,
            traces_sampler: Some(Arc::new(traces_sampler)),
            debug: true,
            ..Default::default()
        },
    ))
}
