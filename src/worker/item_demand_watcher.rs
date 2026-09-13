use crate::{
    service::item_demand_import::ItemDemandImportService,
    worker::file_watcher::{PathResolution, PollingFileWatcher},
};
use std::path::Path;

pub struct ItemDemandWatcher {
    _watcher: PollingFileWatcher,
}

impl ItemDemandWatcher {
    #[must_use]
    pub fn new(service: ItemDemandImportService, file_path: &Path) -> Self {
        let watcher = PollingFileWatcher::new(
            file_path,
            PathResolution::Direct,
            "item demand file",
            "item demand",
            move || {
                let service = service.clone();
                async move { service.import().await }
            },
        );
        Self { _watcher: watcher }
    }
}
