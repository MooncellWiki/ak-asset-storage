#![allow(clippy::unwrap_used)]

#[path = "e2e/import_manifest.rs"]
mod import_manifest;
#[path = "e2e/item_demand.rs"]
mod item_demand;
#[path = "e2e/manifest_watcher.rs"]
mod manifest_watcher;
#[path = "e2e/seed_server.rs"]
mod seed_server;
#[path = "e2e/support.rs"]
mod support;
#[path = "e2e/worker_poll.rs"]
mod worker_poll;
