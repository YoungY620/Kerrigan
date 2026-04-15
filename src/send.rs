use crate::engine::{RegistryEngine, TaskEngine};
use crate::error::{Error, Result};
use crate::model::*;
use std::process::{Command, Stdio};

#[derive(serde::Serialize)]
pub struct SendResult {
    pub queued: bool,
    pub exec_running: bool,
    pub action: String,
}

pub fn run(node_id: &str, message: &str, interrupt: bool) -> Result<()> {
    let registry = RegistryEngine::default_root()?;

    // Find task for this node
    let task_id = find_task_for_node(&registry, node_id)?;
    let task_dir = registry.task_dir(&task_id);
    let task_engine = TaskEngine::open(&task_dir)?;

    // Verify node is Running
    let node = task_engine.get_node(node_id)?;
    if node.status != NodeStatus::Running {
        return Err(Error::Db(format!(
            "node {} is {:?}, expected Running",
            node_id, node.status
        )));
    }

    // Enqueue message
    task_engine.message_enqueue(node_id, message)?;

    // Check if exec is running
    let proc = task_engine.process_get_by_node(node_id)?;
    let exec_running = proc
        .as_ref()
        .map(|p| p.status == ProcessStatus::Running && is_pid_alive(p.pid))
        .unwrap_or(false);

    let action;

    if exec_running {
        if interrupt {
            // Send SIGTERM to exec wrapper
            if let Some(ref p) = proc {
                unsafe {
                    libc::kill(p.pid as i32, libc::SIGTERM);
                }
            }
            action = "interrupted".to_string();
        } else {
            action = "queued".to_string();
        }
    } else {
        // No running exec — fork a new one to consume the queue
        let proc_entry = proc
            .as_ref()
            .ok_or_else(|| Error::ProcessNotFound(node_id.to_string()))?;

        let exe = std::env::current_exe().map_err(|e| Error::io("current_exe", e))?;
        Command::new(exe)
            .args([
                "exec",
                "--task",
                &task_id,
                "--node",
                node_id,
                "--adapter",
                &proc_entry.adapter,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::io("fork exec", e))?;

        action = "spawned_exec".to_string();
    }

    let result = SendResult {
        queued: true,
        exec_running,
        action,
    };
    crate::output::output_json(&result);
    Ok(())
}

fn find_task_for_node(registry: &RegistryEngine, node_id: &str) -> Result<String> {
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

fn is_pid_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}
