// Domain entities module
// Contains core business entities with business logic

pub mod bundle;
pub mod file;
pub mod version;

// Re-exports for convenience
pub use bundle::*;
pub use file::*;
pub use version::*;
