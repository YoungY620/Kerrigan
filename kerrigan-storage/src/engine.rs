use crate::db;
use crate::error::{Result, StorageError};
use crate::model::*;
use rusqlite::{Connection, params};
use std::fs;
use std::path::PathBuf;

pub struct StorageEngine {
    root: PathBuf,
    conn: Connection,
}

impl StorageEngine {
    /// Create a new engine pointing at the given root directory.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        // Ensure root directory exists so SQLite can create the db file
        fs::create_dir_all(&root).map_err(|e| StorageError::io(&root, e))?;
        let db_path = root.join("kerrigan.db");
        let conn = db::open(&db_path)?;
        Ok(Self { root, conn })
    }

    /// Default root: `~/.kerrigan/`.
    pub fn default_root() -> Result<Self> {
        let home = std::env::var("HOME").map_err(|_| StorageError::NotInitialized)?;
        Self::new(PathBuf::from(home).join(".kerrigan"))
    }

    // ── Paths ──

    pub fn task_dir(&self, task_id: &str) -> PathBuf {
        self.root.join("tasks").join(task_id)
    }

    /// Path to the skills directory: `~/.kerrigan/skills/`.
    pub fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    /// Path to the shared directory: `~/.kerrigan/shared/`.
    pub fn shared_dir(&self) -> PathBuf {
        self.root.join("shared")
    }

    // ── Init ──

    // Files compiled into the binary
    const SKILL_MD: &str = include_str!("../skills/kerrigan-storage/SKILL.md");
    const GUIDE_MD: &str = include_str!("../shared/guide.md");
    const GUIDE_SOFTWARE_DEVELOPMENT_MD: &str =
        include_str!("../shared/guide-software-development.md");
    const GUIDE_RESEARCH_MD: &str = include_str!("../shared/guide-research.md");
    const GUIDE_DECISION_AB_TESTING_MD: &str =
        include_str!("../shared/guide-decision-ab-testing.md");
    const GUIDE_GIT_MANAGEMENT_MD: &str = include_str!("../shared/guide-git-management.md");

    /// Initialize `~/.kerrigan/` directory structure. Idempotent.
    /// Creates database, deploys skills and guide files.
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(self.root.join("tasks")).map_err(|e| StorageError::io(&self.root, e))?;

        // Deploy bundled skill
        let skill_dir = self.skills_dir().join("kerrigan-storage");
        fs::create_dir_all(&skill_dir).map_err(|e| StorageError::io(&skill_dir, e))?;
        let skill_path = skill_dir.join("SKILL.md");
        fs::write(&skill_path, Self::SKILL_MD).map_err(|e| StorageError::io(&skill_path, e))?;

        // Deploy bundled guide files
        let shared = self.shared_dir();
        fs::create_dir_all(&shared).map_err(|e| StorageError::io(&shared, e))?;
        for (name, content) in [
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
            let path = shared.join(name);
            fs::write(&path, content).map_err(|e| StorageError::io(&path, e))?;
        }

        Ok(())
    }

    // ── Task management ──

    /// Create a new task, returning its id.
    pub fn task_create(&self, summary: &str, workdir: &str) -> Result<TaskId> {
        let task_id = uuid::Uuid::new_v4().to_string();
        let created_at = now_iso();

        self.conn
            .execute(
                "INSERT INTO tasks (id, summary, workdir, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![task_id, summary, workdir, created_at],
            )
            .map_err(|e| StorageError::Db(format!("task_create: {e}")))?;

        // Create task directory for board.md (still file-based for human editing)
        let dir = self.task_dir(&task_id);
        fs::create_dir_all(&dir).map_err(|e| StorageError::io(&dir, e))?;

        Ok(task_id)
    }

    /// List all registered tasks.
    pub fn task_list(&self) -> Result<Vec<TaskEntry>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, summary, workdir, created_at FROM tasks ORDER BY created_at")
            .map_err(|e| StorageError::Db(format!("task_list: {e}")))?;

        let tasks = stmt
            .query_map([], |row| {
                Ok(TaskEntry {
                    id: row.get(0)?,
                    summary: row.get(1)?,
                    workdir: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|e| StorageError::Db(format!("task_list: {e}")))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| StorageError::Db(format!("task_list: {e}")))?;

        Ok(tasks)
    }

    // ── DAG operations ──

    /// Add a node to a task's DAG.
    pub fn add_node(
        &self,
        task_id: &str,
        new_node: NewNode,
        depends_on: Vec<NodeId>,
    ) -> Result<NodeId> {
        self.ensure_task_exists(task_id)?;
        let node_id = uuid::Uuid::new_v4().to_string();

        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| StorageError::Db(format!("add_node tx: {e}")))?;

        // Validate dependencies exist
        for dep_id in &depends_on {
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM nodes WHERE id = ?1 AND task_id = ?2)",
                    params![dep_id, task_id],
                    |r| r.get(0),
                )
                .map_err(|e| StorageError::Db(format!("add_node validate: {e}")))?;
            if !exists {
                return Err(StorageError::NodeNotFound(dep_id.clone()));
            }
        }

        // Determine initial status
        let status = if depends_on.is_empty() {
            "ready"
        } else {
            let all_succeeded: bool = {
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
                    .map_err(|e| StorageError::Db(format!("add_node status: {e}")))?;
                let dep_refs: Vec<&dyn rusqlite::types::ToSql> = depends_on
                    .iter()
                    .map(|s| s as &dyn rusqlite::types::ToSql)
                    .collect();
                stmt.query_row(dep_refs.as_slice(), |r| r.get(0))
                    .map_err(|e| StorageError::Db(format!("add_node status: {e}")))?
            };
            if all_succeeded { "ready" } else { "pending" }
        };

        // Insert node
        tx.execute(
            "INSERT INTO nodes (id, task_id, summary, description, status) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![node_id, task_id, new_node.summary, new_node.description, status],
        )
        .map_err(|e| StorageError::Db(format!("add_node insert: {e}")))?;

        // Insert edges
        for dep_id in &depends_on {
            tx.execute(
                "INSERT INTO node_edges (predecessor_id, successor_id, task_id) VALUES (?1, ?2, ?3)",
                params![dep_id, node_id, task_id],
            )
            .map_err(|e| StorageError::Db(format!("add_node edge: {e}")))?;
        }

        tx.commit()
            .map_err(|e| StorageError::Db(format!("add_node commit: {e}")))?;

        Ok(node_id)
    }

    /// List all nodes in a task's DAG (lightweight summaries).
    pub fn list_all(&self, task_id: &str) -> Result<Vec<NodeSummary>> {
        self.ensure_task_exists(task_id)?;
        self.query_node_summaries(task_id, None)
    }

    /// List all Ready nodes in a task's DAG.
    pub fn list_ready(&self, task_id: &str) -> Result<Vec<NodeSummary>> {
        self.ensure_task_exists(task_id)?;
        self.query_node_summaries(task_id, Some("ready"))
    }

    /// Batch update nodes. Handles cascading Ready status.
    pub fn update_nodes(&self, task_id: &str, updates: Vec<NodeUpdate>) -> Result<()> {
        self.ensure_task_exists(task_id)?;

        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| StorageError::Db(format!("update_nodes tx: {e}")))?;

        let mut newly_succeeded = Vec::new();

        for update in &updates {
            // Get current status
            let current_status: String = tx
                .query_row(
                    "SELECT status FROM nodes WHERE id = ?1 AND task_id = ?2",
                    params![update.id, task_id],
                    |r| r.get(0),
                )
                .map_err(|_| StorageError::NodeNotFound(update.id.clone()))?;

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
                .map_err(|e| StorageError::Db(format!("update_nodes status: {e}")))?;
            }

            if let Some(ref meta) = update.metadata {
                tx.execute(
                    "UPDATE nodes SET result_summary = ?1, failure_reason = ?2 WHERE id = ?3",
                    params![meta.result_summary, meta.failure_reason, update.id],
                )
                .map_err(|e| StorageError::Db(format!("update_nodes meta: {e}")))?;

                // Update session_ids
                tx.execute(
                    "DELETE FROM node_sessions WHERE node_id = ?1",
                    params![update.id],
                )
                .map_err(|e| StorageError::Db(format!("update_nodes sessions: {e}")))?;
                for sid in &meta.session_ids {
                    tx.execute(
                        "INSERT OR IGNORE INTO node_sessions (node_id, session_id) VALUES (?1, ?2)",
                        params![update.id, sid],
                    )
                    .map_err(|e| StorageError::Db(format!("update_nodes session: {e}")))?;
                }

                // Update spawned_tasks
                tx.execute(
                    "DELETE FROM spawned_tasks WHERE node_id = ?1",
                    params![update.id],
                )
                .map_err(|e| StorageError::Db(format!("update_nodes spawned: {e}")))?;
                for st in &meta.spawned_tasks {
                    tx.execute(
                        "INSERT OR IGNORE INTO spawned_tasks (node_id, spawned_node_id) VALUES (?1, ?2)",
                        params![update.id, st],
                    )
                    .map_err(|e| StorageError::Db(format!("update_nodes spawned: {e}")))?;
                }
            }
        }

        // Cascade ready
        if !newly_succeeded.is_empty() {
            cascade_ready_sql(&tx, task_id, &newly_succeeded)?;
        }

        tx.commit()
            .map_err(|e| StorageError::Db(format!("update_nodes commit: {e}")))?;
        Ok(())
    }

    /// Get full details of a single node.
    pub fn get_node(&self, task_id: &str, node_id: &str) -> Result<Node> {
        self.ensure_task_exists(task_id)?;

        let (summary, description, status_str, result_summary, failure_reason): (
            String,
            String,
            String,
            Option<String>,
            Option<String>,
        ) = self
            .conn
            .query_row(
                "SELECT summary, description, status, result_summary, failure_reason FROM nodes WHERE id = ?1 AND task_id = ?2",
                params![node_id, task_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .map_err(|_| StorageError::NodeNotFound(node_id.to_string()))?;

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

    // ── Board messages ──

    /// Post a message to a task's board.
    pub fn board_post(&self, task_id: &str, content: &str) -> Result<()> {
        self.ensure_task_exists(task_id)?;
        self.conn
            .execute(
                "INSERT INTO board_messages (task_id, content, created_at) VALUES (?1, ?2, ?3)",
                params![task_id, content, now_iso()],
            )
            .map_err(|e| StorageError::Db(format!("board_post: {e}")))?;
        Ok(())
    }

    /// Get all unread messages for a task.
    pub fn board_unread(&self, task_id: &str) -> Result<Vec<BoardMessage>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, content, created_at FROM board_messages WHERE task_id = ?1 AND read = 0 ORDER BY id",
            )
            .map_err(|e| StorageError::Db(format!("board_unread: {e}")))?;

        let msgs = stmt
            .query_map(params![task_id], |row| {
                Ok(BoardMessage {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })
            .map_err(|e| StorageError::Db(format!("board_unread: {e}")))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| StorageError::Db(format!("board_unread: {e}")))?;

        Ok(msgs)
    }

    /// Mark all messages for a task as read.
    pub fn board_mark_read(&self, task_id: &str) -> Result<()> {
        self.conn
            .execute(
                "UPDATE board_messages SET read = 1 WHERE task_id = ?1 AND read = 0",
                params![task_id],
            )
            .map_err(|e| StorageError::Db(format!("board_mark_read: {e}")))?;
        Ok(())
    }

    // ── Internal helpers ──

    fn ensure_task_exists(&self, task_id: &str) -> Result<()> {
        let exists: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?1)",
                params![task_id],
                |r| r.get(0),
            )
            .map_err(|e| StorageError::Db(format!("ensure_task: {e}")))?;
        if !exists {
            return Err(StorageError::TaskNotFound(task_id.to_string()));
        }
        Ok(())
    }

    fn query_node_summaries(
        &self,
        task_id: &str,
        status_filter: Option<&str>,
    ) -> Result<Vec<NodeSummary>> {
        let (sql, filter_params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            if let Some(status) = status_filter {
                (
                    "SELECT id, summary, status FROM nodes WHERE task_id = ?1 AND status = ?2"
                        .to_string(),
                    vec![
                        Box::new(task_id.to_string()) as Box<dyn rusqlite::types::ToSql>,
                        Box::new(status.to_string()),
                    ],
                )
            } else {
                (
                    "SELECT id, summary, status FROM nodes WHERE task_id = ?1".to_string(),
                    vec![Box::new(task_id.to_string()) as Box<dyn rusqlite::types::ToSql>],
                )
            };

        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| StorageError::Db(format!("query_summaries: {e}")))?;

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            filter_params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                let id: String = row.get(0)?;
                let summary: String = row.get(1)?;
                let status_s: String = row.get(2)?;
                Ok((id, summary, status_s))
            })
            .map_err(|e| StorageError::Db(format!("query_summaries: {e}")))?;

        let mut summaries = Vec::new();
        for row in rows {
            let (id, summary, status_s) =
                row.map_err(|e| StorageError::Db(format!("query_summaries row: {e}")))?;
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

    fn query_predecessors(&self, node_id: &str) -> Result<Vec<NodeId>> {
        let mut stmt = self
            .conn
            .prepare("SELECT predecessor_id FROM node_edges WHERE successor_id = ?1")
            .map_err(|e| StorageError::Db(format!("predecessors: {e}")))?;
        let ids = stmt
            .query_map(params![node_id], |r| r.get(0))
            .map_err(|e| StorageError::Db(format!("predecessors: {e}")))?
            .collect::<std::result::Result<Vec<String>, _>>()
            .map_err(|e| StorageError::Db(format!("predecessors: {e}")))?;
        Ok(ids)
    }

    fn query_successors(&self, node_id: &str) -> Result<Vec<NodeId>> {
        let mut stmt = self
            .conn
            .prepare("SELECT successor_id FROM node_edges WHERE predecessor_id = ?1")
            .map_err(|e| StorageError::Db(format!("successors: {e}")))?;
        let ids = stmt
            .query_map(params![node_id], |r| r.get(0))
            .map_err(|e| StorageError::Db(format!("successors: {e}")))?
            .collect::<std::result::Result<Vec<String>, _>>()
            .map_err(|e| StorageError::Db(format!("successors: {e}")))?;
        Ok(ids)
    }

    fn query_session_ids(&self, node_id: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT session_id FROM node_sessions WHERE node_id = ?1")
            .map_err(|e| StorageError::Db(format!("session_ids: {e}")))?;
        let ids = stmt
            .query_map(params![node_id], |r| r.get(0))
            .map_err(|e| StorageError::Db(format!("session_ids: {e}")))?
            .collect::<std::result::Result<Vec<String>, _>>()
            .map_err(|e| StorageError::Db(format!("session_ids: {e}")))?;
        Ok(ids)
    }

    fn query_spawned_tasks(&self, node_id: &str) -> Result<Vec<NodeId>> {
        let mut stmt = self
            .conn
            .prepare("SELECT spawned_node_id FROM spawned_tasks WHERE node_id = ?1")
            .map_err(|e| StorageError::Db(format!("spawned_tasks: {e}")))?;
        let ids = stmt
            .query_map(params![node_id], |r| r.get(0))
            .map_err(|e| StorageError::Db(format!("spawned_tasks: {e}")))?
            .collect::<std::result::Result<Vec<String>, _>>()
            .map_err(|e| StorageError::Db(format!("spawned_tasks: {e}")))?;
        Ok(ids)
    }
}

// ── Cascade ready via SQL ──

/// After nodes become Succeeded, cascade Ready status to their successors.
/// Loops until no more nodes can be promoted.
fn cascade_ready_sql(
    tx: &rusqlite::Transaction<'_>,
    task_id: &str,
    newly_succeeded: &[NodeId],
) -> Result<()> {
    // TODO: When multiple external Kerrigan instances access the same db concurrently,
    // this cascade logic may need additional coordination. Current WAL mode handles
    // single-writer serialization, but complex multi-swarm scenarios may require
    // per-swarm isolation or connection pooling.

    loop {
        let changed = tx
            .execute(
                "UPDATE nodes SET status = 'ready'
                 WHERE status = 'pending'
                   AND task_id = ?1
                   AND NOT EXISTS (
                       SELECT 1 FROM node_edges e
                       JOIN nodes pred ON pred.id = e.predecessor_id
                       WHERE e.successor_id = nodes.id
                         AND pred.status != 'succeeded'
                   )",
                params![task_id],
            )
            .map_err(|e| StorageError::Db(format!("cascade_ready: {e}")))?;

        if changed == 0 {
            break;
        }
    }

    // Suppress unused variable warning — newly_succeeded is used to trigger the cascade,
    // but the SQL approach promotes all eligible nodes in the task, not just successors
    // of the specific nodes.
    let _ = newly_succeeded;

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
        _ => Err(StorageError::Db(format!("unknown status: {s}"))),
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
        Err(StorageError::InvalidStateTransition {
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
