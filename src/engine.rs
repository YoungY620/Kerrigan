use crate::db;
use crate::error::{Error, Result};
use crate::model::*;
use rusqlite::{Connection, params};
use std::fs;
use std::path::PathBuf;

// ════════════════════════════════════════════════════════════════
//  RegistryEngine — operates on ~/.kerrigan/registry.db
// ════════════════════════════════════════════════════════════════

pub struct RegistryEngine {
    root: PathBuf,
    conn: Connection,
}

impl RegistryEngine {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|e| Error::io(&root, e))?;
        let conn = db::open_registry(&root.join("registry.db"))?;
        Ok(Self { root, conn })
    }

    pub fn default_root() -> Result<Self> {
        let home = std::env::var("HOME").map_err(|_| Error::NotInitialized)?;
        Self::new(PathBuf::from(home).join(".kerrigan"))
    }

    // ── Paths ──

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    pub fn task_dir(&self, task_id: &str) -> PathBuf {
        self.root.join("tasks").join(task_id)
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    // ── Init ──

    const SKILL_MD: &str = include_str!("../skills/kerrigan/SKILL.md");
    const GUIDE_MD: &str = include_str!("../skills/kerrigan/guide.md");
    const GUIDE_SOFTWARE_DEVELOPMENT_MD: &str =
        include_str!("../skills/kerrigan/guide-software-development.md");
    const GUIDE_RESEARCH_MD: &str = include_str!("../skills/kerrigan/guide-research.md");
    const GUIDE_DECISION_AB_TESTING_MD: &str =
        include_str!("../skills/kerrigan/guide-decision-ab-testing.md");
    const GUIDE_GIT_MANAGEMENT_MD: &str =
        include_str!("../skills/kerrigan/guide-git-management.md");

    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(self.root.join("tasks")).map_err(|e| Error::io(&self.root, e))?;

        let skill_dir = self.skills_dir().join("kerrigan");
        fs::create_dir_all(&skill_dir).map_err(|e| Error::io(&skill_dir, e))?;
        for (name, content) in [
            ("SKILL.md", Self::SKILL_MD),
            ("guide.md", Self::GUIDE_MD),
            (
                "guide-software-development.md",
                Self::GUIDE_SOFTWARE_DEVELOPMENT_MD,
            ),
            ("guide-research.md", Self::GUIDE_RESEARCH_MD),
            (
                "guide-decision-ab-testing.md",
                Self::GUIDE_DECISION_AB_TESTING_MD,
            ),
            ("guide-git-management.md", Self::GUIDE_GIT_MANAGEMENT_MD),
        ] {
            let path = skill_dir.join(name);
            fs::write(&path, content).map_err(|e| Error::io(&path, e))?;
        }

        Ok(())
    }

    // ── Task management ──

    pub fn task_create(&self, summary: &str, workdir: &str) -> Result<TaskId> {
        let task_id = uuid::Uuid::new_v4().to_string();
        let created_at = now_iso();

        self.conn
            .execute(
                "INSERT INTO tasks (id, summary, workdir, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![task_id, summary, workdir, created_at],
            )
            .map_err(|e| Error::Db(format!("task_create: {e}")))?;

        let dir = self.task_dir(&task_id);
        fs::create_dir_all(dir.join("logs")).map_err(|e| Error::io(&dir, e))?;

        // Initialize the task DB
        let _task_engine = TaskEngine::open(&dir)?;

        Ok(task_id)
    }

    pub fn task_list(&self) -> Result<Vec<TaskEntry>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, summary, workdir, created_at FROM tasks ORDER BY created_at")
            .map_err(|e| Error::Db(format!("task_list: {e}")))?;

        let tasks = stmt
            .query_map([], |row| {
                Ok(TaskEntry {
                    id: row.get(0)?,
                    summary: row.get(1)?,
                    workdir: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|e| Error::Db(format!("task_list: {e}")))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::Db(format!("task_list: {e}")))?;

        Ok(tasks)
    }

    pub fn task_get(&self, task_id: &str) -> Result<TaskEntry> {
        self.conn
            .query_row(
                "SELECT id, summary, workdir, created_at FROM tasks WHERE id = ?1",
                params![task_id],
                |row| {
                    Ok(TaskEntry {
                        id: row.get(0)?,
                        summary: row.get(1)?,
                        workdir: row.get(2)?,
                        created_at: row.get(3)?,
                    })
                },
            )
            .map_err(|_| Error::TaskNotFound(task_id.to_string()))
    }
}

// ════════════════════════════════════════════════════════════════
//  TaskEngine — operates on ~/.kerrigan/tasks/<id>/task.db
// ════════════════════════════════════════════════════════════════

pub struct TaskEngine {
    conn: Connection,
}

impl TaskEngine {
    pub fn open(task_dir: &std::path::Path) -> Result<Self> {
        fs::create_dir_all(task_dir).map_err(|e| Error::io(task_dir, e))?;
        let conn = db::open_task(&task_dir.join("task.db"))?;
        Ok(Self { conn })
    }

    // ── DAG operations ──

    pub fn add_node(&self, new_node: NewNode, depends_on: Vec<NodeId>) -> Result<NodeId> {
        let node_id = uuid::Uuid::new_v4().to_string();

        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| Error::Db(format!("add_node tx: {e}")))?;

        for dep_id in &depends_on {
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM nodes WHERE id = ?1)",
                    params![dep_id],
                    |r| r.get(0),
                )
                .map_err(|e| Error::Db(format!("add_node validate: {e}")))?;
            if !exists {
                return Err(Error::NodeNotFound(dep_id.clone()));
            }
        }

        let status = if depends_on.is_empty() {
            "ready"
        } else {
            let placeholders: Vec<String> = depends_on
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect();
            let sql = format!(
                "SELECT NOT EXISTS(SELECT 1 FROM nodes WHERE id IN ({}) AND status != 'succeeded')",
                placeholders.join(",")
            );
            let mut stmt = tx
                .prepare(&sql)
                .map_err(|e| Error::Db(format!("add_node status: {e}")))?;
            let dep_refs: Vec<&dyn rusqlite::types::ToSql> = depends_on
                .iter()
                .map(|s| s as &dyn rusqlite::types::ToSql)
                .collect();
            let all_succeeded: bool = stmt
                .query_row(dep_refs.as_slice(), |r| r.get(0))
                .map_err(|e| Error::Db(format!("add_node status: {e}")))?;
            if all_succeeded { "ready" } else { "pending" }
        };

        tx.execute(
            "INSERT INTO nodes (id, summary, description, status) VALUES (?1, ?2, ?3, ?4)",
            params![node_id, new_node.summary, new_node.description, status],
        )
        .map_err(|e| Error::Db(format!("add_node insert: {e}")))?;

        for dep_id in &depends_on {
            tx.execute(
                "INSERT INTO node_edges (predecessor_id, successor_id) VALUES (?1, ?2)",
                params![dep_id, node_id],
            )
            .map_err(|e| Error::Db(format!("add_node edge: {e}")))?;
        }

        tx.commit()
            .map_err(|e| Error::Db(format!("add_node commit: {e}")))?;

        Ok(node_id)
    }

    pub fn list_all(&self) -> Result<Vec<NodeSummary>> {
        self.query_node_summaries(None)
    }

    pub fn list_ready(&self) -> Result<Vec<NodeSummary>> {
        self.query_node_summaries(Some("ready"))
    }

    pub fn get_node(&self, node_id: &str) -> Result<Node> {
        let (summary, description, status_str, result_summary, failure_reason): (
            String,
            String,
            String,
            Option<String>,
            Option<String>,
        ) = self
            .conn
            .query_row(
                "SELECT summary, description, status, result_summary, failure_reason FROM nodes WHERE id = ?1",
                params![node_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .map_err(|_| Error::NodeNotFound(node_id.to_string()))?;

        let predecessors = self.query_predecessors(node_id)?;
        let successors = self.query_successors(node_id)?;
        let session_ids = self.query_session_ids(node_id)?;
        let spawned_tasks = self.query_spawned_tasks(node_id)?;

        Ok(Node {
            id: node_id.to_string(),
            summary,
            description,
            predecessors,
            successors,
            status: parse_status(&status_str)?,
            metadata: NodeMetadata {
                result_summary,
                failure_reason,
                spawned_tasks,
                session_ids,
            },
        })
    }

    pub fn update_nodes(&self, updates: Vec<NodeUpdate>) -> Result<()> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| Error::Db(format!("update_nodes tx: {e}")))?;

        let mut newly_succeeded = Vec::new();

        for update in &updates {
            let current_status: String = tx
                .query_row(
                    "SELECT status FROM nodes WHERE id = ?1",
                    params![update.id],
                    |r| r.get(0),
                )
                .map_err(|_| Error::NodeNotFound(update.id.clone()))?;

            if let Some(new_status) = update.status {
                let from = parse_status(&current_status)?;
                validate_transition(&update.id, from, new_status)?;

                if new_status == NodeStatus::Succeeded {
                    newly_succeeded.push(update.id.clone());
                }

                tx.execute(
                    "UPDATE nodes SET status = ?1 WHERE id = ?2",
                    params![status_str(new_status), update.id],
                )
                .map_err(|e| Error::Db(format!("update_nodes status: {e}")))?;
            }

            if let Some(ref meta) = update.metadata {
                tx.execute(
                    "UPDATE nodes SET result_summary = ?1, failure_reason = ?2 WHERE id = ?3",
                    params![meta.result_summary, meta.failure_reason, update.id],
                )
                .map_err(|e| Error::Db(format!("update_nodes meta: {e}")))?;

                tx.execute(
                    "DELETE FROM node_sessions WHERE node_id = ?1",
                    params![update.id],
                )
                .map_err(|e| Error::Db(format!("update_nodes sessions: {e}")))?;
                for sid in &meta.session_ids {
                    tx.execute(
                        "INSERT OR IGNORE INTO node_sessions (node_id, session_id) VALUES (?1, ?2)",
                        params![update.id, sid],
                    )
                    .map_err(|e| Error::Db(format!("update_nodes session: {e}")))?;
                }

                tx.execute(
                    "DELETE FROM spawned_tasks WHERE node_id = ?1",
                    params![update.id],
                )
                .map_err(|e| Error::Db(format!("update_nodes spawned: {e}")))?;
                for st in &meta.spawned_tasks {
                    tx.execute(
                        "INSERT OR IGNORE INTO spawned_tasks (node_id, spawned_node_id) VALUES (?1, ?2)",
                        params![update.id, st],
                    )
                    .map_err(|e| Error::Db(format!("update_nodes spawned: {e}")))?;
                }
            }
        }

        if !newly_succeeded.is_empty() {
            cascade_ready_sql(&tx)?;
        }

        tx.commit()
            .map_err(|e| Error::Db(format!("update_nodes commit: {e}")))?;
        Ok(())
    }

    // ── Process tracking ──

    pub fn process_create(
        &self,
        node_id: &str,
        pid: u32,
        session_id: &str,
        adapter: &str,
        log_path: &str,
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO processes (node_id, pid, session_id, adapter, log_path, started_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![node_id, pid, session_id, adapter, log_path, now_iso()],
            )
            .map_err(|e| Error::Db(format!("process_create: {e}")))?;
        Ok(())
    }

    pub fn process_get_by_node(&self, node_id: &str) -> Result<Option<ProcessEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, node_id, pid, session_id, adapter, log_path, started_at, ended_at, exit_code, status FROM processes WHERE node_id = ?1 ORDER BY id DESC LIMIT 1",
            )
            .map_err(|e| Error::Db(format!("process_get_by_node: {e}")))?;

        let mut rows = stmt
            .query_map(params![node_id], |row| {
                Ok(ProcessEntry {
                    id: row.get(0)?,
                    node_id: row.get(1)?,
                    pid: row.get::<_, u32>(2)?,
                    session_id: row.get(3)?,
                    adapter: row.get(4)?,
                    log_path: row.get(5)?,
                    started_at: row.get(6)?,
                    ended_at: row.get(7)?,
                    exit_code: row.get(8)?,
                    status: parse_process_status(&row.get::<_, String>(9)?),
                })
            })
            .map_err(|e| Error::Db(format!("process_get_by_node: {e}")))?;

        match rows.next() {
            Some(Ok(entry)) => Ok(Some(entry)),
            Some(Err(e)) => Err(Error::Db(format!("process_get_by_node: {e}"))),
            None => Ok(None),
        }
    }

    pub fn process_list_running(&self) -> Result<Vec<ProcessEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, node_id, pid, session_id, adapter, log_path, started_at, ended_at, exit_code, status FROM processes WHERE status = 'running'",
            )
            .map_err(|e| Error::Db(format!("process_list_running: {e}")))?;

        let entries = stmt
            .query_map([], |row| {
                Ok(ProcessEntry {
                    id: row.get(0)?,
                    node_id: row.get(1)?,
                    pid: row.get::<_, u32>(2)?,
                    session_id: row.get(3)?,
                    adapter: row.get(4)?,
                    log_path: row.get(5)?,
                    started_at: row.get(6)?,
                    ended_at: row.get(7)?,
                    exit_code: row.get(8)?,
                    status: parse_process_status(&row.get::<_, String>(9)?),
                })
            })
            .map_err(|e| Error::Db(format!("process_list_running: {e}")))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::Db(format!("process_list_running: {e}")))?;

        Ok(entries)
    }

    pub fn process_mark_exited(&self, id: i64, exit_code: i32) -> Result<()> {
        self.conn
            .execute(
                "UPDATE processes SET status = 'exited', exit_code = ?1, ended_at = ?2 WHERE id = ?3",
                params![exit_code, now_iso(), id],
            )
            .map_err(|e| Error::Db(format!("process_mark_exited: {e}")))?;
        Ok(())
    }

    pub fn process_mark_killed(&self, id: i64) -> Result<()> {
        self.conn
            .execute(
                "UPDATE processes SET status = 'killed', ended_at = ?1 WHERE id = ?2",
                params![now_iso(), id],
            )
            .map_err(|e| Error::Db(format!("process_mark_killed: {e}")))?;
        Ok(())
    }

    // ── Message queue ──

    pub fn message_enqueue(&self, node_id: &str, content: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO node_messages (node_id, content, created_at) VALUES (?1, ?2, ?3)",
                params![node_id, content, now_iso()],
            )
            .map_err(|e| Error::Db(format!("message_enqueue: {e}")))?;
        Ok(())
    }

    pub fn message_consume(&self, node_id: &str) -> Result<Option<NodeMessage>> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| Error::Db(format!("message_consume tx: {e}")))?;

        let msg = tx
            .query_row(
                "SELECT id, node_id, content, created_at FROM node_messages WHERE node_id = ?1 AND consumed = 0 ORDER BY id LIMIT 1",
                params![node_id],
                |row| {
                    Ok(NodeMessage {
                        id: row.get(0)?,
                        node_id: row.get(1)?,
                        content: row.get(2)?,
                        created_at: row.get(3)?,
                    })
                },
            )
            .ok();

        if let Some(ref m) = msg {
            tx.execute(
                "UPDATE node_messages SET consumed = 1 WHERE id = ?1",
                params![m.id],
            )
            .map_err(|e| Error::Db(format!("message_consume update: {e}")))?;
        }

        tx.commit()
            .map_err(|e| Error::Db(format!("message_consume commit: {e}")))?;
        Ok(msg)
    }

    // ── Internal helpers ──

    fn query_node_summaries(&self, status_filter: Option<&str>) -> Result<Vec<NodeSummary>> {
        let (sql, filter_params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            if let Some(status) = status_filter {
                (
                    "SELECT id, summary, status FROM nodes WHERE status = ?1".to_string(),
                    vec![Box::new(status.to_string()) as Box<dyn rusqlite::types::ToSql>],
                )
            } else {
                ("SELECT id, summary, status FROM nodes".to_string(), vec![])
            };

        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| Error::Db(format!("query_summaries: {e}")))?;

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            filter_params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                let id: String = row.get(0)?;
                let summary: String = row.get(1)?;
                let status_s: String = row.get(2)?;
                Ok((id, summary, status_s))
            })
            .map_err(|e| Error::Db(format!("query_summaries: {e}")))?;

        let mut summaries = Vec::new();
        for row in rows {
            let (id, summary, status_s) =
                row.map_err(|e| Error::Db(format!("query_summaries row: {e}")))?;
            let successors = self.query_successors(&id)?;
            summaries.push(NodeSummary {
                id,
                summary,
                successors,
                status: parse_status(&status_s)?,
            });
        }
        Ok(summaries)
    }

    pub fn viz_data(&self) -> Result<(Vec<crate::model::VizNodeData>, Vec<crate::model::VizEdgeData>)> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, summary, status, result_summary, failure_reason FROM nodes ORDER BY rowid",
            )
            .map_err(|e| Error::Db(format!("viz_data nodes: {e}")))?;

        let nodes = stmt
            .query_map([], |row| {
                let status_s: String = row.get(2)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    status_s,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(|e| Error::Db(format!("viz_data nodes query: {e}")))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::Db(format!("viz_data nodes row: {e}")))?
            .into_iter()
            .map(|(id, summary, status_s, result_summary, failure_reason)| {
                Ok(crate::model::VizNodeData {
                    id,
                    summary,
                    status: parse_status(&status_s)?,
                    result_summary,
                    failure_reason,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let mut stmt = self
            .conn
            .prepare("SELECT predecessor_id, successor_id FROM node_edges")
            .map_err(|e| Error::Db(format!("viz_data edges: {e}")))?;

        let edges = stmt
            .query_map([], |row| {
                Ok(crate::model::VizEdgeData {
                    predecessor_id: row.get(0)?,
                    successor_id: row.get(1)?,
                })
            })
            .map_err(|e| Error::Db(format!("viz_data edges query: {e}")))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::Db(format!("viz_data edges row: {e}")))?;

        Ok((nodes, edges))
    }

    fn query_predecessors(&self, node_id: &str) -> Result<Vec<NodeId>> {
        let mut stmt = self
            .conn
            .prepare("SELECT predecessor_id FROM node_edges WHERE successor_id = ?1")
            .map_err(|e| Error::Db(format!("predecessors: {e}")))?;
        let ids = stmt
            .query_map(params![node_id], |r| r.get(0))
            .map_err(|e| Error::Db(format!("predecessors: {e}")))?
            .collect::<std::result::Result<Vec<String>, _>>()
            .map_err(|e| Error::Db(format!("predecessors: {e}")))?;
        Ok(ids)
    }

    fn query_successors(&self, node_id: &str) -> Result<Vec<NodeId>> {
        let mut stmt = self
            .conn
            .prepare("SELECT successor_id FROM node_edges WHERE predecessor_id = ?1")
            .map_err(|e| Error::Db(format!("successors: {e}")))?;
        let ids = stmt
            .query_map(params![node_id], |r| r.get(0))
            .map_err(|e| Error::Db(format!("successors: {e}")))?
            .collect::<std::result::Result<Vec<String>, _>>()
            .map_err(|e| Error::Db(format!("successors: {e}")))?;
        Ok(ids)
    }

    fn query_session_ids(&self, node_id: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT session_id FROM node_sessions WHERE node_id = ?1")
            .map_err(|e| Error::Db(format!("session_ids: {e}")))?;
        let ids = stmt
            .query_map(params![node_id], |r| r.get(0))
            .map_err(|e| Error::Db(format!("session_ids: {e}")))?
            .collect::<std::result::Result<Vec<String>, _>>()
            .map_err(|e| Error::Db(format!("session_ids: {e}")))?;
        Ok(ids)
    }

    fn query_spawned_tasks(&self, node_id: &str) -> Result<Vec<NodeId>> {
        let mut stmt = self
            .conn
            .prepare("SELECT spawned_node_id FROM spawned_tasks WHERE node_id = ?1")
            .map_err(|e| Error::Db(format!("spawned_tasks: {e}")))?;
        let ids = stmt
            .query_map(params![node_id], |r| r.get(0))
            .map_err(|e| Error::Db(format!("spawned_tasks: {e}")))?
            .collect::<std::result::Result<Vec<String>, _>>()
            .map_err(|e| Error::Db(format!("spawned_tasks: {e}")))?;
        Ok(ids)
    }
}

// ── Cascade ready ──

fn cascade_ready_sql(tx: &rusqlite::Transaction<'_>) -> Result<()> {
    loop {
        let changed = tx
            .execute(
                "UPDATE nodes SET status = 'ready'
                 WHERE status = 'pending'
                   AND NOT EXISTS (
                       SELECT 1 FROM node_edges e
                       JOIN nodes pred ON pred.id = e.predecessor_id
                       WHERE e.successor_id = nodes.id
                         AND pred.status != 'succeeded'
                   )",
                [],
            )
            .map_err(|e| Error::Db(format!("cascade_ready: {e}")))?;

        if changed == 0 {
            break;
        }
    }
    Ok(())
}

// ── Status helpers ──

fn status_str(s: NodeStatus) -> &'static str {
    match s {
        NodeStatus::Pending => "pending",
        NodeStatus::Ready => "ready",
        NodeStatus::Running => "running",
        NodeStatus::Succeeded => "succeeded",
        NodeStatus::Failed => "failed",
    }
}

fn parse_status(s: &str) -> Result<NodeStatus> {
    match s {
        "pending" => Ok(NodeStatus::Pending),
        "ready" => Ok(NodeStatus::Ready),
        "running" => Ok(NodeStatus::Running),
        "succeeded" => Ok(NodeStatus::Succeeded),
        "failed" => Ok(NodeStatus::Failed),
        _ => Err(Error::Db(format!("unknown status: {s}"))),
    }
}

fn parse_process_status(s: &str) -> ProcessStatus {
    match s {
        "running" => ProcessStatus::Running,
        "exited" => ProcessStatus::Exited,
        "killed" => ProcessStatus::Killed,
        _ => ProcessStatus::Exited,
    }
}

fn validate_transition(node_id: &str, from: NodeStatus, to: NodeStatus) -> Result<()> {
    let valid = matches!(
        (from, to),
        (NodeStatus::Ready, NodeStatus::Running)
            | (NodeStatus::Running, NodeStatus::Succeeded)
            | (NodeStatus::Running, NodeStatus::Failed)
            | (NodeStatus::Ready, NodeStatus::Succeeded)
            | (NodeStatus::Ready, NodeStatus::Failed)
    );

    if valid {
        Ok(())
    } else {
        Err(Error::InvalidStateTransition {
            node_id: node_id.to_string(),
            from,
            to,
        })
    }
}

fn now_iso() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}
