use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("not initialized: run `kerrigan-storage init` first")]
    NotInitialized,

    #[error("task not found: {0}")]
    TaskNotFound(String),

    #[error("node not found: {0}")]
    NodeNotFound(String),

    #[error("duplicate node id: {0}")]
    DuplicateNodeId(String),

    #[error("adding node would create a cycle")]
    CycleDetected,

    #[error("invalid state transition: {from:?} -> {to:?} for node {node_id}")]
    InvalidStateTransition {
        node_id: String,
        from: crate::model::NodeStatus,
        to: crate::model::NodeStatus,
    },

    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("json error at {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
}

pub type Result<T> = std::result::Result<T, StorageError>;

impl StorageError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub fn json(path: impl Into<PathBuf>, source: serde_json::Error) -> Self {
        Self::Json {
            path: path.into(),
            source,
        }
    }
}
