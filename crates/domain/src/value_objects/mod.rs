// Value objects module
// Contains immutable value objects that represent business concepts

pub mod client_version;
pub mod file_hash;
pub mod file_path;
pub mod file_size;
pub mod hot_update_list;
pub mod res_version;

// Re-exports for convenience
pub use client_version::ClientVersion;
pub use file_hash::FileHash;
pub use file_path::FilePath;
pub use file_size::FileSize;
pub use hot_update_list::HotUpdateList;
pub use res_version::ResVersion;
