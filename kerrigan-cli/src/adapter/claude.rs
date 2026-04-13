use super::stream::exec_streaming;
use super::traits::{AgentAdapter, AgentOutput};
use crate::error::CliError;
use std::path::Path;
use tokio::process::Command;

/// Claude Code adapter.
///
/// Invokes `claude` CLI in non-interactive YOLO mode with `--print`.
#[derive(Default)]
pub struct ClaudeAdapter;

impl ClaudeAdapter {
    pub fn new() -> Self {
        Self
    }

    fn build_run_command(
        &self,
        prompt: &str,
        session_id: &str,
        skills_dir: Option<&Path>,
        workdir: &Path,
    ) -> Command {
        let mut cmd = Command::new("claude");
        cmd.current_dir(workdir);
        cmd.args(["--print", "--verbose", "--dangerously-skip-permissions"]);
        cmd.args(["--output-format", "stream-json"]);
        cmd.args(["--session-id", session_id]);

        if let Some(dir) = skills_dir {
            cmd.args(["--add-dir", &dir.to_string_lossy()]);
        }

        cmd.args(["-p", prompt]);
        cmd
    }

    fn build_resume_command(&self, session_id: &str, prompt: &str, workdir: &Path) -> Command {
        let mut cmd = Command::new("claude");
        cmd.current_dir(workdir);
        cmd.args(["--print", "--verbose", "--dangerously-skip-permissions"]);
        cmd.args(["--output-format", "stream-json"]);
        cmd.args(["--resume", session_id]);
        cmd.args(["-p", prompt]);
        cmd
    }
}

#[async_trait::async_trait]
impl AgentAdapter for ClaudeAdapter {
    async fn run(
        &self,
        prompt: &str,
        session_id: &str,
        skills_dir: Option<&Path>,
        workdir: &Path,
        log_path: Option<&Path>,
    ) -> Result<AgentOutput, CliError> {
        let cmd = self.build_run_command(prompt, session_id, skills_dir, workdir);
        exec_streaming(cmd, "claude", log_path).await
    }

    async fn resume(
        &self,
        session_id: &str,
        prompt: &str,
        workdir: &Path,
        log_path: Option<&Path>,
    ) -> Result<AgentOutput, CliError> {
        let cmd = self.build_resume_command(session_id, prompt, workdir);
        exec_streaming(cmd, "claude", log_path).await
    }

    fn name(&self) -> &str {
        "claude"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_of(cmd: &Command) -> Vec<String> {
        cmd.as_std()
            .get_args()
            .map(|a| a.to_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn run_command_construction() {
        let adapter = ClaudeAdapter::new();
        let cmd = adapter.build_run_command(
            "hello world",
            "550e8400-e29b-41d4-a716-446655440000",
            Some(Path::new("/tmp/skills")),
            Path::new("/tmp/work"),
        );

        assert_eq!(cmd.as_std().get_program(), "claude");
        let args = args_of(&cmd);
        assert!(args.contains(&"--print".into()));
        assert!(args.contains(&"--dangerously-skip-permissions".into()));
        assert!(args.contains(&"--session-id".into()));
        assert!(args.contains(&"550e8400-e29b-41d4-a716-446655440000".into()));
        assert!(args.contains(&"--add-dir".into()));
        assert!(args.contains(&"/tmp/skills".into()));
        assert!(args.contains(&"-p".into()));
        assert!(args.contains(&"hello world".into()));
    }

    #[test]
    fn run_command_without_skills() {
        let adapter = ClaudeAdapter::new();
        let cmd = adapter.build_run_command("test", "s1", None, Path::new("/tmp"));
        assert!(!args_of(&cmd).contains(&"--add-dir".into()));
    }

    #[test]
    fn resume_command_construction() {
        let adapter = ClaudeAdapter::new();
        let cmd = adapter.build_resume_command(
            "550e8400-e29b-41d4-a716-446655440000",
            "continue please",
            Path::new("/tmp"),
        );
        let args = args_of(&cmd);

        assert!(args.contains(&"--resume".into()));
        assert!(args.contains(&"550e8400-e29b-41d4-a716-446655440000".into()));
        assert!(args.contains(&"-p".into()));
        assert!(args.contains(&"continue please".into()));
        assert!(args.contains(&"--dangerously-skip-permissions".into()));
        assert!(!args.contains(&"--session-id".into()));
    }

    // Regression: --print --output-format=stream-json requires --verbose
    #[test]
    fn run_has_verbose_for_stream_json() {
        let adapter = ClaudeAdapter::new();
        let args = args_of(&adapter.build_run_command("p", "s1", None, Path::new("/tmp")));
        assert!(args.contains(&"stream-json".into()));
        assert!(
            args.contains(&"--verbose".into()),
            "run() must include --verbose when using stream-json"
        );
    }

    #[test]
    fn resume_has_verbose_for_stream_json() {
        let adapter = ClaudeAdapter::new();
        let args = args_of(&adapter.build_resume_command("s1", "p", Path::new("/tmp")));
        assert!(args.contains(&"stream-json".into()));
        assert!(
            args.contains(&"--verbose".into()),
            "resume() must include --verbose when using stream-json"
        );
    }

    #[test]
    fn run_and_resume_share_core_flags() {
        let adapter = ClaudeAdapter::new();
        let run_args = args_of(&adapter.build_run_command("p", "s1", None, Path::new("/tmp")));
        let resume_args = args_of(&adapter.build_resume_command("s1", "p", Path::new("/tmp")));

        for flag in ["--print", "--verbose", "--dangerously-skip-permissions", "--output-format", "stream-json"] {
            assert!(run_args.contains(&flag.into()), "run() missing: {flag}");
            assert!(resume_args.contains(&flag.into()), "resume() missing: {flag}");
        }
    }

    #[test]
    fn output_format_value_is_stream_json() {
        let adapter = ClaudeAdapter::new();
        for (label, args) in [
            ("run", args_of(&adapter.build_run_command("p", "s1", None, Path::new("/tmp")))),
            ("resume", args_of(&adapter.build_resume_command("s1", "p", Path::new("/tmp")))),
        ] {
            let idx = args.iter().position(|a| a == "--output-format")
                .unwrap_or_else(|| panic!("{label}: missing --output-format"));
            assert_eq!(args[idx + 1], "stream-json", "{label}: wrong format value");
        }
    }

    #[test]
    fn adapter_name_is_claude() {
        assert_eq!(ClaudeAdapter::new().name(), "claude");
    }
}
