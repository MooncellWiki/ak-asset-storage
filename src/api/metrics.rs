use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use prometheus::{Encoder, HistogramOpts, HistogramVec, Registry, TextEncoder};
use std::time::Duration;
use tracing::error;

const DURATION_BUCKETS: [f64; 14] = [
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0,
];

#[derive(Clone)]
pub(super) struct McpMetrics {
    registry: Registry,
    call_duration: HistogramVec,
}

impl McpMetrics {
    pub(super) fn new() -> Self {
        let registry = Registry::new();
        let call_duration = HistogramVec::new(
            HistogramOpts::new(
                "ak_asset_storage_mcp_tool_call_duration_seconds",
                "Processing time of completed MCP tool calls in seconds.",
            )
            .buckets(DURATION_BUCKETS.to_vec()),
            &["tool", "result"],
        )
        .expect("valid MCP tool call histogram");
        registry
            .register(Box::new(call_duration.clone()))
            .expect("register MCP tool call histogram in a new registry");
        Self {
            registry,
            call_duration,
        }
    }

    pub(super) fn initialize_tool(&self, tool: &str) {
        // Expose a zero baseline so rate queries can include the first call.
        self.call_duration.with_label_values(&[tool, "success"]);
        self.call_duration.with_label_values(&[tool, "error"]);
    }

    pub(super) fn observe(&self, tool: &str, success: bool, duration: Duration) {
        let result = if success { "success" } else { "error" };
        self.call_duration
            .with_label_values(&[tool, result])
            .observe(duration.as_secs_f64());
    }

    pub(super) fn scrape(&self) -> Response {
        let encoder = TextEncoder::new();
        let mut body = Vec::new();
        if let Err(error) = encoder.encode(&self.registry.gather(), &mut body) {
            error!(%error, "Failed to encode Prometheus metrics");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        (
            [
                (header::CONTENT_TYPE, encoder.format_type()),
                (header::CACHE_CONTROL, "no-store"),
            ],
            body,
        )
            .into_response()
    }
}
