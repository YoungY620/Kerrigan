use std::path::Path;

/// Output from an agent invocation.
#[derive(Debug, Clone)]
pub struct AgentOutput {
    /// The agent's text output
    pub text: String,
    /// Exit code from the agent process
    pub exit_code: i32,
}

/// Trait for agent CLI adapters.
///
/// Each adapter wraps a specific agent CLI (claude, kimi, etc.) and provides
/// non-interactive prompt execution with subagent and skills support.
#[async_trait::async_trait]
pub trait AgentAdapter: Send + Sync {
    /// Execute a prompt in a new or existing session.
    ///
    /// - `prompt`: The prompt text to send to the agent
    /// - `session_id`: Session identifier for the agent CLI
    /// - `skills_dir`: Optional path to a skills directory to inject
    /// - `workdir`: Working directory for the agent process
    /// - `log_path`: Optional path to write agent output log
    async fn run(
        &self,
        prompt: &str,
        session_id: &str,
        skills_dir: Option<&Path>,
        workdir: &Path,
        log_path: Option<&Path>,
    ) -> Result<AgentOutput, crate::error::CliError>;

    /// Resume an existing session with a new prompt.
    ///
    /// - `session_id`: The session to resume
    /// - `prompt`: The prompt to send on resume
    /// - `workdir`: Working directory for the agent process
    /// - `log_path`: Optional path to write agent output log
    async fn resume(
        &self,
        session_id: &str,
        prompt: &str,
        workdir: &Path,
        log_path: Option<&Path>,
    ) -> Result<AgentOutput, crate::error::CliError>;

    /// Return the adapter name (e.g. "claude", "kimi").
    fn name(&self) -> &str;
}
