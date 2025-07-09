// Domain layer - Core business logic and entities
// This layer contains pure business logic with no external dependencies

pub mod entities;
pub mod error;
pub mod value_objects;

// Re-exports
pub use entities::*;
pub use error::{DomainError, DomainResult};
pub use value_objects::*;
