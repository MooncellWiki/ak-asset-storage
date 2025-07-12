// Test data generators and fixtures
use ak_asset_storage_application::{Bundle, File, HotUpdateList, Version};

/// Test data generators
pub struct TestData;

impl TestData {
    pub fn create_version(id: Option<i32>, res: &str, client: &str, is_ready: bool) -> Version {
        Version {
            id,
            res: res.to_string(),
            client: client.to_string(),
            is_ready,
            hot_update_list: Self::default_hot_update_list(),
        }
    }

    pub fn create_file(id: Option<i32>, hash: &str, size: i32) -> File {
        File {
            id,
            hash: hash.to_string(),
            size,
        }
    }

    pub fn create_bundle(id: Option<i32>, path: &str, version_id: i32, file_id: i32) -> Bundle {
        Bundle {
            id,
            path: path.to_string(),
            version_id,
            file_id,
        }
    }

    pub fn default_hot_update_list() -> HotUpdateList {
        let json = r#"{
            "abInfos": [
                {
                    "abSize": 1024,
                    "hash": "abc123",
                    "md5": "def456",
                    "name": "test/file1.ab",
                    "totalSize": 1024
                },
                {
                    "abSize": 2048,
                    "hash": "xyz789",
                    "md5": "uvw012",
                    "name": "test/file2.ab",
                    "totalSize": 2048
                }
            ]
        }"#;
        HotUpdateList::new(json).expect("Valid test hot update list")
    }
    pub fn sample_file_data1() -> Vec<u8> {
        b"sample file1 content for testing".to_vec()
    }
    pub fn sample_file_data2() -> Vec<u8> {
        b"sample file2 content for testing".to_vec()
    }
}

pub const SAMPLE_HOT_UPDATE_LIST: &str = r#"{
    "abInfos": [
        {
            "abSize": 1024,
            "hash": "abc123",
            "md5": "def456",
            "name": "test/file1.ab",
            "totalSize": 1024
        },
        {
            "abSize": 2048,
            "hash": "xyz789",
            "md5": "uvw012",
            "name": "test/file2.ab",
            "totalSize": 2048
        }
    ]
}"#;

pub const LARGE_HOT_UPDATE_LIST: &str = r#"{
    "abInfos": [
        {
            "abSize": 1024,
            "hash": "file1_hash",
            "md5": "file1_md5",
            "name": "test/file1.ab",
            "totalSize": 1024
        },
        {
            "abSize": 2048,
            "hash": "file2_hash",
            "md5": "file2_md5",
            "name": "test/file2.ab",
            "totalSize": 2048
        },
        {
            "abSize": 4096,
            "hash": "file3_hash",
            "md5": "file3_md5",
            "name": "test/file3.ab",
            "totalSize": 4096
        },
        {
            "abSize": 8192,
            "hash": "file4_hash",
            "md5": "file4_md5",
            "name": "test/file4.ab",
            "totalSize": 8192
        },
        {
            "abSize": 16384,
            "hash": "file5_hash",
            "md5": "file5_md5",
            "name": "test/file5.ab",
            "totalSize": 16384
        }
    ]
}"#;
