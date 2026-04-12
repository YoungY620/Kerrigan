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
    ) -> Result<AgentOutput, CliError> {
        let output = self
            .build_run_command(prompt, session_id, skills_dir, workdir)
            .output()
            .await
            .map_err(|e| CliError::AgentExec {
                adapter: "kimi".into(),
                source: e,
            })?;

        Ok(AgentOutput {
            text: String::from_utf8_lossy(&output.stdout).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    async fn resume(
        &self,
        session_id: &str,
        prompt: &str,
        workdir: &Path,
    ) -> Result<AgentOutput, CliError> {
        let output = self
            .build_resume_command(session_id, prompt, workdir)
            .output()
            .await
            .map_err(|e| CliError::AgentExec {
                adapter: "kimi".into(),
                source: e,
            })?;

        Ok(AgentOutput {
            text: String::from_utf8_lossy(&output.stdout).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    fn name(&self) -> &str {
        "kimi"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn run_command_construction() {
        let adapter = KimiAdapter::new();
        let cmd = adapter.build_run_command(
            "hello",
            "sess_001",
            Some(Path::new("/tmp/skills")),
            Path::new("/tmp/work"),
        );
        let cmd = cmd.as_std();

        assert_eq!(cmd.get_program(), "kimi");
        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();
        assert!(args.contains(&"--print"));
        assert!(args.contains(&"--yolo"));
        assert!(args.contains(&"--session"));
        assert!(args.contains(&"sess_001"));
        assert!(args.contains(&"--skills-dir"));
        assert!(args.contains(&"/tmp/skills"));
        assert!(args.contains(&"--prompt"));
        assert!(args.contains(&"hello"));
    }

    #[test]
    fn run_command_without_skills() {
        let adapter = KimiAdapter::new();
        let cmd = adapter.build_run_command("test", "s1", None, Path::new("/tmp"));
        let cmd = cmd.as_std();

        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();
        assert!(!args.contains(&"--skills-dir"));
    }

    #[test]
    fn resume_command_construction() {
        let adapter = KimiAdapter::new();
        let cmd = adapter.build_resume_command("sess_001", "continue", Path::new("/tmp"));
        let cmd = cmd.as_std();

        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();
        assert!(args.contains(&"--continue"));
        assert!(args.contains(&"--session"));
        assert!(args.contains(&"sess_001"));
        assert!(args.contains(&"--prompt"));
        assert!(args.contains(&"continue"));
        assert!(args.contains(&"--yolo"));
    }
}
