use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "not initialized: run 'kerrigan init' to create ~/.kerrigan/ directory structure before using any other commands"
    )]
    NotInitialized,

    #[error(
        "task not found: '{0}'. Use 'kerrigan task list' to see available tasks. Create one with 'kerrigan task create --summary \"...\" --workdir <path>'"
    )]
    TaskNotFound(String),

    #[error(
        "node not found: '{0}'. Use 'kerrigan dag list --task <task-id>' to see all nodes, or 'kerrigan dag ready --task <task-id>' for executable nodes"
    )]
    NodeNotFound(String),

    #[error(
        "duplicate node id: '{0}'. Node IDs are auto-generated UUIDs — this indicates a collision or race condition. Retry the operation"
    )]
    DuplicateNodeId(String),

    #[error(
        "adding node would create a cycle in the DAG. Check the --depends-on list: a node cannot depend (directly or transitively) on itself. Use 'kerrigan dag list --task <task-id>' to inspect the current dependency graph"
    )]
    CycleDetected,

    #[error(
        "invalid state transition for node '{node_id}': {from:?} -> {to:?}. Valid flow: pending->ready (automatic when predecessors succeed), ready->running (set by exec), running->succeeded/failed (set by dag update). Check current status with 'kerrigan dag get --task <task-id> --node {node_id}'"
    )]
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

    #[error(
        "json parse error at {path}: {source}. For 'dag update', the expected input format is: '[{{\"id\":\"<node-id>\",\"status\":\"succeeded\",\"metadata\":{{\"result_summary\":\"...\"}}}}]'"
    )]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("database error: {0}")]
    Db(String),

    #[error("contract violation for node '{node_summary}':\n{details}\n\nFix the --contract JSON and retry dag add.")]
    InvalidContract { node_summary: String, details: String },

    #[error(
        "output schema validation failed for node '{node_id}':\n{validation_errors}\n\n\
        Option A — fix result_summary to match the declared output schema, then retry:\n  \
          kerrigan dag update --task <task> --input '[{{\"id\":\"{node_id}\",\"status\":\"succeeded\",\"metadata\":{{\"result_summary\":\"<fixed output>\"}}}}]'\n\n\
        Option B — the contract was wrong at creation time (execution was actually correct):\n  \
          kerrigan dag override --task <task> --node {node_id} --result-summary \"<actual output>\""
    )]
    OutputSchemaFailed { node_id: String, validation_errors: String },

    #[error(
        "no process record found for node '{0}'. The node may not have been executed yet. Start execution with 'kerrigan exec --task <task-id> --node {0}'"
    )]
    ProcessNotFound(String),

    #[error(
        "node already has an active process running (pid {pid}). Stop it with 'kerrigan stop --node <node-id>' before starting a new execution, or send a message with 'kerrigan send --node <node-id> --message \"...\"'"
    )]
    NodeBusy { pid: u32 },
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
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
