use crate::engine::{RegistryEngine, TaskEngine};
use crate::error::Result;
use crate::model::*;
use std::collections::HashMap;
use std::io::Write;
use std::time::{Duration, SystemTime};

pub fn run(task_id: &str, interval_secs: u64) -> Result<()> {
    let registry = RegistryEngine::default_root()?;
    let task_dir = registry.task_dir(task_id);

    let mut prev_snapshot: HashMap<String, NodeStatus> = HashMap::new();
    let mut dead_pids: HashMap<i64, u64> = HashMap::new(); // process.id -> timestamp of first death detection
    let timeout_secs = 60u64;

    // Initial snapshot
    {
        let te = TaskEngine::open(&task_dir)?;
        for node in te.list_all()? {
            prev_snapshot.insert(node.id.clone(), node.status);
        }
    }

    loop {
        std::thread::sleep(Duration::from_secs(interval_secs));

        // Open fresh connection each poll to see WAL updates from other processes
        let te = TaskEngine::open(&task_dir)?;
        let all_nodes = te.list_all()?;
        let running_procs = te.process_list_running()?;

        // 1. Detect node status changes
        for node in &all_nodes {
            let prev = prev_snapshot.get(&node.id).copied();
            if prev != Some(node.status) {
                emit_event(&serde_json::json!({
                    "ts": now_iso(),
                    "node": node.id,
                    "event": format!("{:?}", node.status).to_lowercase(),
                    "summary": node.summary,
                }));
            }
        }

        // Update snapshot
        prev_snapshot.clear();
        for node in &all_nodes {
            prev_snapshot.insert(node.id.clone(), node.status);
        }

        // 2. Check PID alive for running processes
        for proc in &running_procs {
            let alive = unsafe { libc::kill(proc.pid as i32, 0) == 0 };
            if !alive {
                let now_ts = now_epoch();

                if let Some(&first_seen) = dead_pids.get(&proc.id) {
                    // Already detected — check timeout
                    if now_ts - first_seen >= timeout_secs {
                        // Timeout: mark Failed
                        let node = te.get_node(&proc.node_id)?;
                        if node.status == NodeStatus::Running {
                            te.update_nodes(vec![NodeUpdate {
                                id: proc.node_id.clone(),
                                status: Some(NodeStatus::Failed),
                                metadata: Some(NodeMetadata {
                                    failure_reason: Some(format!(
                                        "exec wrapper (pid {}) died without updating status (timeout {}s)",
                                        proc.pid, timeout_secs
                                    )),
                                    ..Default::default()
                                }),
                            }])?;
                            te.process_mark_exited(proc.id, -1)?;
                            emit_event(&serde_json::json!({
                                "ts": now_iso(),
                                "node": proc.node_id,
                                "event": "timeout_failed",
                                "pid": proc.pid,
                            }));
                        }
                        dead_pids.remove(&proc.id);
                    }
                } else {
                    // First detection
                    dead_pids.insert(proc.id, now_ts);
                    emit_event(&serde_json::json!({
                        "ts": now_iso(),
                        "node": proc.node_id,
                        "event": "process_died",
                        "pid": proc.pid,
                    }));
                }
            } else {
                // PID alive — clear any stale death tracking
                dead_pids.remove(&proc.id);
            }
        }

        // 3. Check termination: all nodes terminal + no running processes
        let all_terminal = all_nodes
            .iter()
            .all(|n| n.status == NodeStatus::Succeeded || n.status == NodeStatus::Failed);
        let no_running = running_procs.iter().all(|p| {
            let alive = unsafe { libc::kill(p.pid as i32, 0) == 0 };
            !alive
        });

        if all_terminal && no_running {
            let succeeded = all_nodes
                .iter()
                .filter(|n| n.status == NodeStatus::Succeeded)
                .count();
            let failed = all_nodes
                .iter()
                .filter(|n| n.status == NodeStatus::Failed)
                .count();
            emit_event(&serde_json::json!({
                "ts": now_iso(),
                "event": "task_complete",
                "succeeded": succeeded,
                "failed": failed,
            }));
            break;
        }
    }

    Ok(())
}

fn emit_event(event: &serde_json::Value) {
    let line = serde_json::to_string(event).unwrap_or_default();
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{line}").ok();
    handle.flush().ok();
}

fn now_iso() -> String {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
