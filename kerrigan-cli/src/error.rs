#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("unknown adapter: {0}. Supported: claude, kimi")]
    UnknownAdapter(String),

    #[error("agent execution failed ({adapter}): {source}")]
    AgentExec {
        adapter: String,
        source: std::io::Error,
    },

    #[error("no progress after {stall_rounds} rounds (round {round}): {reason}")]
    Stall {
        round: usize,
        stall_rounds: usize,
        reason: String,
    },

    #[error("storage error: {0}")]
    Storage(#[from] kerrigan_storage::StorageError),
}
