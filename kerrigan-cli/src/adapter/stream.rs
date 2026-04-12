use crate::adapter::traits::AgentOutput;
use crate::error::CliError;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tracing::debug;

/// Execute a command with streaming stdout/stderr capture.
///
/// - Streams stdout line by line (for real-time visibility)
/// - Optionally writes all output to a log file
/// - Returns the complete output as AgentOutput
pub async fn exec_streaming(
    mut cmd: Command,
    adapter_name: &str,
    log_path: Option<&Path>,
) -> Result<AgentOutput, CliError> {
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| CliError::AgentExec {
        adapter: adapter_name.into(),
        source: e,
    })?;

    let stdout = child.stdout.take().expect("stdout should be piped");
    let stderr = child.stderr.take().expect("stderr should be piped");

    // Open log file if requested
    let mut log_file = if let Some(path) = log_path {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        Some(
            tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(path)
                .await
                .map_err(|e| CliError::AgentExec {
                    adapter: adapter_name.into(),
                    source: e,
                })?,
        )
    } else {
        None
    };

    let mut text = String::new();

    // Stream stdout
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    // Read stdout and stderr concurrently
    loop {
        tokio::select! {
            line = stdout_reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        debug!(target: "kerrigan::agent", "{}", line);
                        text.push_str(&line);
                        text.push('\n');
                        if let Some(ref mut f) = log_file {
                            let _ = f.write_all(line.as_bytes()).await;
                            let _ = f.write_all(b"\n").await;
                        }
                    }
                    Ok(None) => break, // EOF
                    Err(_) => break,
                }
            }
            line = stderr_reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        debug!(target: "kerrigan::agent::stderr", "{}", line);
                        if let Some(ref mut f) = log_file {
                            let _ = f.write_all(b"[stderr] ").await;
                            let _ = f.write_all(line.as_bytes()).await;
                            let _ = f.write_all(b"\n").await;
                        }
                    }
                    Ok(None) => {} // stderr EOF, continue reading stdout
                    Err(_) => {}
                }
            }
        }
    }

    // Drain any remaining stderr after stdout EOF
    while let Ok(Some(line)) = stderr_reader.next_line().await {
        debug!(target: "kerrigan::agent::stderr", "{}", line);
        if let Some(ref mut f) = log_file {
            let _ = f.write_all(b"[stderr] ").await;
            let _ = f.write_all(line.as_bytes()).await;
            let _ = f.write_all(b"\n").await;
        }
    }

    let status = child.wait().await.map_err(|e| CliError::AgentExec {
        adapter: adapter_name.into(),
        source: e,
    })?;

    Ok(AgentOutput {
        text,
        exit_code: status.code().unwrap_or(-1),
    })
}
