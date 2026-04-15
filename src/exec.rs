use crate::engine::{RegistryEngine, TaskEngine};
use crate::error::{Error, Result};
use crate::model::*;
use crate::prompt;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::SystemTime;

/// Result returned by `exec --detach`.
#[derive(serde::Serialize)]
pub struct ExecDetachResult {
    pub pid: u32,
    pub session_id: String,
    pub log_path: String,
}

/// Run the exec command.
pub fn run(task_id: &str, node_id: &str, adapter: &str, detach: bool) -> Result<()> {
    let registry = RegistryEngine::default_root()?;
    let task_dir = registry.task_dir(task_id);
    let task_engine = TaskEngine::open(&task_dir)?;

    // Validate node is Ready
    let node = task_engine.get_node(node_id)?;
    if node.status != NodeStatus::Ready {
        return Err(Error::Db(format!(
            "node {} is {:?}, expected Ready",
            node_id, node.status
        )));
    }

    // Generate session_id and log_path
    let session_id = uuid::Uuid::new_v4().to_string();
    let logs_dir = task_dir.join("logs");
    fs::create_dir_all(&logs_dir).map_err(|e| Error::io(&logs_dir, e))?;
    let log_path = logs_dir.join(format!("{node_id}.jsonl"));

    // Record process and mark Running
    let self_pid = std::process::id();
    task_engine.process_create(
        node_id,
        self_pid,
        &session_id,
        adapter,
        &log_path.to_string_lossy(),
    )?;
    task_engine.update_nodes(vec![NodeUpdate {
        id: node_id.to_string(),
        status: Some(NodeStatus::Running),
        metadata: None,
    }])?;

    if detach {
        // Fork self without --detach
        let exe = std::env::current_exe().map_err(|e| Error::io("current_exe", e))?;
        let child = Command::new(exe)
            .args([
                "exec",
                "--task",
                task_id,
                "--node",
                node_id,
                "--adapter",
                adapter,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::io("fork exec", e))?;

        let child_pid = child.id();

        // Update PID to the actual child
        // (we recorded self_pid above; now replace with the forked child)
        if let Some(proc) = task_engine.process_get_by_node(node_id)? {
            task_engine.process_mark_exited(proc.id, 0)?; // close our record
            task_engine.process_create(
                node_id,
                child_pid,
                &session_id,
                adapter,
                &log_path.to_string_lossy(),
            )?;
        }

        let result = ExecDetachResult {
            pid: child_pid,
            session_id,
            log_path: log_path.to_string_lossy().into_owned(),
        };
        crate::output::output_json(&result);
        return Ok(());
    }

    // ── Foreground mode ──
    run_foreground(
        &registry,
        &task_engine,
        task_id,
        node_id,
        adapter,
        &session_id,
        &log_path,
    )
}

/// Resume an existing session (called by send).
pub fn run_resume(
    task_id: &str,
    node_id: &str,
    adapter: &str,
    session_id: &str,
    message: &str,
) -> Result<()> {
    let registry = RegistryEngine::default_root()?;
    let task_dir = registry.task_dir(task_id);
    let task_engine = TaskEngine::open(&task_dir)?;

    let logs_dir = task_dir.join("logs");
    let log_path = logs_dir.join(format!("{node_id}.jsonl"));

    // Record this exec instance
    let pid = std::process::id();
    task_engine.process_create(
        node_id,
        pid,
        session_id,
        adapter,
        &log_path.to_string_lossy(),
    )?;

    run_agent_turn(
        &task_engine,
        node_id,
        adapter,
        session_id,
        Some(message),
        &log_path,
    )?;
    drain_message_queue(
        &registry,
        &task_engine,
        task_id,
        node_id,
        adapter,
        session_id,
        &log_path,
    )
}

fn run_foreground(
    registry: &RegistryEngine,
    _task_engine: &TaskEngine,
    task_id: &str,
    node_id: &str,
    adapter: &str,
    session_id: &str,
    log_path: &PathBuf,
) -> Result<()> {
    let task_dir = registry.task_dir(task_id);
    let te = TaskEngine::open(&task_dir)?;
    run_agent_turn(&te, node_id, adapter, session_id, None, log_path)?;
    drain_message_queue(
        registry, &te, task_id, node_id, adapter, session_id, log_path,
    )
}

/// Run a single agent turn. If `resume_message` is None, starts a new session with initial prompt.
fn run_agent_turn(
    _task_engine: &TaskEngine,
    node_id: &str,
    adapter: &str,
    session_id: &str,
    resume_message: Option<&str>,
    log_path: &PathBuf,
) -> Result<()> {
    let registry = RegistryEngine::default_root()?;
    let task_id = find_task_id_for_node(&registry, node_id)?;

    // Open log file (append)
    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|e| Error::io(log_path, e))?;

    let mode = if resume_message.is_some() {
        "resume"
    } else {
        "run"
    };
    write_log_event(
        &mut log_file,
        &serde_json::json!({
            "type": "kerrigan",
            "event": "exec_start",
            "mode": mode,
            "node": node_id,
            "session": session_id,
            "ts": now_iso(),
        }),
    );

    // Build command
    let skills_dir = registry.skills_dir();
    let task_entry = registry.task_get(&task_id)?;
    let workdir = &task_entry.workdir;

    let mut cmd = match adapter {
        "claude" => {
            let mut c = Command::new("claude");
            c.args(["--print", "--verbose", "--dangerously-skip-permissions"]);
            c.args(["--output-format", "stream-json"]);
            if let Some(msg) = resume_message {
                c.args(["--resume", session_id]);
                c.args(["-p", msg]);
            } else {
                c.args(["--session-id", session_id]);
                c.args(["--add-dir", &skills_dir.to_string_lossy()]);
                let prompt_text = prompt::build_prompt(
                    &registry,
                    &TaskEngine::open(&registry.task_dir(&task_id))?,
                    &task_id,
                    node_id,
                )?;
                c.args(["-p", &prompt_text]);
            }
            c
        }
        "kimi" => {
            let mut c = Command::new("kimi");
            c.args(["--print", "--yolo"]);
            c.args(["--output-format", "stream-json"]);
            if let Some(msg) = resume_message {
                c.args(["--continue", "--session", session_id]);
                c.args(["--prompt", msg]);
            } else {
                c.args(["--session", session_id]);
                c.args(["--skills-dir", &skills_dir.to_string_lossy()]);
                let prompt_text = prompt::build_prompt(
                    &registry,
                    &TaskEngine::open(&registry.task_dir(&task_id))?,
                    &task_id,
                    node_id,
                )?;
                c.args(["--prompt", &prompt_text]);
            }
            c
        }
        _ => return Err(Error::Db(format!("unknown adapter: {adapter}"))),
    };

    cmd.current_dir(workdir);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::null());

    // Spawn agent
    let mut child = cmd.spawn().map_err(|e| Error::io(adapter, e))?;

    // Stream stdout to log
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(|r| r.ok()) {
            writeln!(log_file, "{line}").ok();
        }
    }

    // Wait for exit
    let status = child.wait().map_err(|e| Error::io("wait", e))?;
    let exit_code = status.code().unwrap_or(-1);

    write_log_event(
        &mut log_file,
        &serde_json::json!({
            "type": "kerrigan",
            "event": "agent_exit",
            "exit_code": exit_code,
            "ts": now_iso(),
        }),
    );

    // Re-open task engine to see agent's updates
    let task_dir = registry.task_dir(&task_id);
    let task_engine = TaskEngine::open(&task_dir)?;
    let node = task_engine.get_node(node_id)?;

    match (node.status, exit_code) {
        // Agent updated to terminal — done
        (NodeStatus::Succeeded, _) | (NodeStatus::Failed, _) => {
            write_log_event(
                &mut log_file,
                &serde_json::json!({
                    "type": "kerrigan",
                    "event": "exec_end",
                    "node_status": format!("{:?}", node.status).to_lowercase(),
                    "ts": now_iso(),
                }),
            );
            // Update process record
            if let Some(proc) = task_engine.process_get_by_node(node_id)? {
                task_engine.process_mark_exited(proc.id, exit_code)?;
            }
        }
        // Normal exit, still Running — turn complete, check queue
        (NodeStatus::Running, 0) => {
            write_log_event(
                &mut log_file,
                &serde_json::json!({
                    "type": "kerrigan",
                    "event": "turn_complete",
                    "ts": now_iso(),
                }),
            );
        }
        // Crashed
        (NodeStatus::Running, code) => {
            let reason = format!("agent process exited with code {code} without updating status");
            task_engine.update_nodes(vec![NodeUpdate {
                id: node_id.to_string(),
                status: Some(NodeStatus::Failed),
                metadata: Some(NodeMetadata {
                    failure_reason: Some(reason.clone()),
                    ..Default::default()
                }),
            }])?;
            write_log_event(
                &mut log_file,
                &serde_json::json!({
                    "type": "kerrigan",
                    "event": "exec_end",
                    "node_status": "failed",
                    "reason": reason,
                    "ts": now_iso(),
                }),
            );
            if let Some(proc) = task_engine.process_get_by_node(node_id)? {
                task_engine.process_mark_exited(proc.id, code)?;
            }
        }
        _ => {}
    }

    Ok(())
}

/// After a turn, drain the message queue: consume messages and resume.
fn drain_message_queue(
    registry: &RegistryEngine,
    _task_engine: &TaskEngine,
    task_id: &str,
    node_id: &str,
    adapter: &str,
    session_id: &str,
    log_path: &PathBuf,
) -> Result<()> {
    loop {
        // Re-check node status
        let task_dir = registry.task_dir(task_id);
        let te = TaskEngine::open(&task_dir)?;
        let node = te.get_node(node_id)?;

        if node.status == NodeStatus::Succeeded || node.status == NodeStatus::Failed {
            break; // Terminal — done
        }

        // Check queue
        if let Some(msg) = te.message_consume(node_id)? {
            run_agent_turn(
                &te,
                node_id,
                adapter,
                session_id,
                Some(&msg.content),
                log_path,
            )?;
        } else {
            break; // No messages — exit, wait for send to wake us
        }
    }
    Ok(())
}

/// Find task_id for a given node by scanning registry.
fn find_task_id_for_node(registry: &RegistryEngine, node_id: &str) -> Result<String> {
    let tasks = registry.task_list()?;
    for task in &tasks {
        let task_dir = registry.task_dir(&task.id);
        if let Ok(te) = TaskEngine::open(&task_dir)
            && te.get_node(node_id).is_ok()
        {
            return Ok(task.id.clone());
        }
    }
    Err(Error::NodeNotFound(node_id.to_string()))
}

fn write_log_event(file: &mut File, event: &serde_json::Value) {
    writeln!(file, "{}", serde_json::to_string(event).unwrap_or_default()).ok();
}

fn now_iso() -> String {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}
