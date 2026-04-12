pub mod db;
pub mod engine;
pub mod error;
pub mod model;

// Re-export serde_json for CLI binary
pub use serde_json;

pub use engine::StorageEngine;
pub use error::{Result, StorageError};
pub use model::*;
