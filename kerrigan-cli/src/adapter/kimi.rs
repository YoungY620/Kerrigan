use super::stream::exec_streaming;
use super::traits::{AgentAdapter, AgentOutput};
use crate::error::CliError;
use std::path::Path;
use tokio::process::Command;

/// Kimi CLI adapter.
///
/// Invokes `kimi` CLI in non-interactive mode with `--print --yolo`.
#[derive(Default)]
pub struct KimiAdapter;

impl KimiAdapter {
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
        let mut cmd = Command::new("kimi");
        cmd.current_dir(workdir);
        cmd.args(["--print", "--yolo"]);
        cmd.args(["--output-format", "stream-json"]);
        cmd.args(["--session", session_id]);

        if let Some(dir) = skills_dir {
            cmd.args(["--skills-dir", &dir.to_string_lossy()]);
        }

        cmd.args(["--prompt", prompt]);
        cmd
    }

    fn build_resume_command(&self, session_id: &str, prompt: &str, workdir: &Path) -> Command {
        let mut cmd = Command::new("kimi");
        cmd.current_dir(workdir);
        cmd.args(["--print", "--yolo"]);
        cmd.args(["--output-format", "stream-json"]);
        cmd.args(["--session", session_id]);
        cmd.arg("--continue");
        cmd.args(["--prompt", prompt]);
        cmd
    }
}

#[async_trait::async_trait]
impl AgentAdapter for KimiAdapter {
    async fn run(
        &self,
        prompt: &str,
        session_id: &str,
        skills_dir: Option<&Path>,
        workdir: &Path,
        log_path: Option<&Path>,
    ) -> Result<AgentOutput, CliError> {
        let cmd = self.build_run_command(prompt, session_id, skills_dir, workdir);
        exec_streaming(cmd, "kimi", log_path).await
    }

    async fn resume(
        &self,
        session_id: &str,
        prompt: &str,
        workdir: &Path,
        log_path: Option<&Path>,
    ) -> Result<AgentOutput, CliError> {
        let cmd = self.build_resume_command(session_id, prompt, workdir);
        exec_streaming(cmd, "kimi", log_path).await
    }

    fn name(&self) -> &str {
        "kimi"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn args_of(cmd: &Command) -> Vec<String> {
        cmd.as_std()
            .get_args()
            .map(|a| a.to_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn run_command_construction() {
        let adapter = KimiAdapter::new();
        let cmd = adapter.build_run_command(
            "hello",
            "sess_001",
            Some(Path::new("/tmp/skills")),
            Path::new("/tmp/work"),
        );

        assert_eq!(cmd.as_std().get_program(), "kimi");
        let args = args_of(&cmd);
        assert!(args.contains(&"--print".into()));
        assert!(args.contains(&"--yolo".into()));
        assert!(args.contains(&"--session".into()));
        assert!(args.contains(&"sess_001".into()));
        assert!(args.contains(&"--skills-dir".into()));
        assert!(args.contains(&"/tmp/skills".into()));
        assert!(args.contains(&"--prompt".into()));
        assert!(args.contains(&"hello".into()));
    }

    #[test]
    fn run_command_without_skills() {
        let adapter = KimiAdapter::new();
        let cmd = adapter.build_run_command("test", "s1", None, Path::new("/tmp"));
        assert!(!args_of(&cmd).contains(&"--skills-dir".into()));
    }

    #[test]
    fn resume_command_construction() {
        let adapter = KimiAdapter::new();
        let args = args_of(&adapter.build_resume_command("sess_001", "continue", Path::new("/tmp")));
        assert!(args.contains(&"--continue".into()));
        assert!(args.contains(&"--session".into()));
        assert!(args.contains(&"sess_001".into()));
        assert!(args.contains(&"--prompt".into()));
        assert!(args.contains(&"continue".into()));
        assert!(args.contains(&"--yolo".into()));
    }

    #[test]
    fn run_has_stream_json_output() {
        let adapter = KimiAdapter::new();
        let args = args_of(&adapter.build_run_command("p", "s1", None, Path::new("/tmp")));
        assert!(args.contains(&"--output-format".into()));
        assert!(args.contains(&"stream-json".into()));
    }

    #[test]
    fn resume_has_stream_json_output() {
        let adapter = KimiAdapter::new();
        let args = args_of(&adapter.build_resume_command("s1", "p", Path::new("/tmp")));
        assert!(args.contains(&"--output-format".into()));
        assert!(args.contains(&"stream-json".into()));
    }

    #[test]
    fn run_and_resume_share_core_flags() {
        let adapter = KimiAdapter::new();
        let run_args = args_of(&adapter.build_run_command("p", "s1", None, Path::new("/tmp")));
        let resume_args = args_of(&adapter.build_resume_command("s1", "p", Path::new("/tmp")));

        for flag in ["--print", "--yolo", "--output-format", "stream-json", "--session"] {
            assert!(run_args.contains(&flag.into()), "run() missing: {flag}");
            assert!(resume_args.contains(&flag.into()), "resume() missing: {flag}");
        }
    }

    #[test]
    fn output_format_value_is_stream_json() {
        let adapter = KimiAdapter::new();
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
    fn adapter_name_is_kimi() {
        assert_eq!(KimiAdapter::new().name(), "kimi");
    }
}
