use crate::common::*;
use ak_asset_storage_application::{AssetMappingImportService, AssetMappingStatus, NodeType};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn make_temp_dir() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ak-asset-storage-test-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[tokio::test]
async fn test_import_manifest_success() {
    let root = make_temp_dir();
    let version_dir = root.join("1.0.0");
    fs::create_dir_all(&version_dir).unwrap();
    fs::write(
        version_dir.join("resource_manifest_idx.json"),
        r#"{"bundles":[{"name":"bundle/test.ab"}],"assetToBundleList":[{"assetName":"arts/test","bundleIndex":0,"name":"test","path":"dyn/arts/test.prefab"}],"rawCount":0}"#,
    )
    .unwrap();

    let repository = MockRepository::new();
    repository
        .version
        .versions
        .lock()
        .unwrap()
        .push(TestData::create_version(
            Some(1),
            "1.0.0",
            "client-1.0.0",
            false,
        ));

    let service = AssetMappingImportService::new(repository.clone(), root);
    service.import_from_version_dir(&version_dir).await.unwrap();

    let mappings = repository.asset_mappings.lock().unwrap();
    assert_eq!(mappings.len(), 2);

    let file_mapping = mappings
        .iter()
        .find(|m| m.node_type == NodeType::File)
        .unwrap();
    assert_eq!(file_mapping.version_id, 1);
    assert_eq!(file_mapping.bundle_path, "bundle/test.ab");

    let dir_mapping = mappings
        .iter()
        .find(|m| m.node_type.is_directory())
        .unwrap();
    assert_eq!(dir_mapping.asset_name, "arts");
    assert_eq!(dir_mapping.dir_name, "");
    drop(mappings);

    assert_eq!(
        repository.version.versions.lock().unwrap()[0].asset_mapping_status,
        AssetMappingStatus::Ready
    );
}

#[tokio::test]
async fn test_import_manifest_rejects_when_locked() {
    let root = make_temp_dir();
    let version_dir = root.join("1.0.0");
    fs::create_dir_all(&version_dir).unwrap();
    fs::write(
        version_dir.join("resource_manifest_idx.json"),
        r#"{"bundles":[{"name":"bundle/test.ab"}],"assetToBundleList":[{"assetName":"arts/test","bundleIndex":0,"name":"test","path":"dyn/arts/test.prefab"}],"rawCount":0}"#,
    )
    .unwrap();

    let repository = MockRepository::new();
    repository
        .version
        .versions
        .lock()
        .unwrap()
        .push(TestData::create_version(
            Some(1),
            "1.0.0",
            "client-1.0.0",
            false,
        ));
    repository.locked_versions.lock().unwrap().insert(1);

    let service = AssetMappingImportService::new(repository.clone(), root);
    let result = service.import_from_version_dir(&version_dir).await;

    assert!(result.is_err());
    assert!(repository.asset_mappings.lock().unwrap().is_empty());
}
