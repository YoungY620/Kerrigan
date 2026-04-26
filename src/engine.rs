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

    pub fn add_node(
        &self,
        new_node: NewNode,
        depends_on: Vec<NodeId>,
        successors: Vec<NodeId>,
    ) -> Result<NodeId> {
        let node_id = uuid::Uuid::new_v4().to_string();

        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| Error::Db(format!("add_node tx: {e}")))?;

        // Validate predecessors exist
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

        // Validate successors exist
        for succ_id in &successors {
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM nodes WHERE id = ?1)",
                    params![succ_id],
                    |r| r.get(0),
                )
                .map_err(|e| Error::Db(format!("add_node validate successor: {e}")))?;
            if !exists {
                return Err(Error::NodeNotFound(succ_id.clone()));
            }
        }

        // Cycle detection: if any successor can reach any predecessor via existing edges,
        // inserting this node as a bridge (pred→N→succ) would form a cycle.
        if any_path_exists(&tx, &successors, &depends_on)? {
            return Err(Error::CycleDetected);
        }

        // Determine initial status
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

        let contract_json = serde_json::to_string(&new_node.contract)
            .map_err(|e| Error::Db(format!("add_node contract serialize: {e}")))?;

        tx.execute(
            "INSERT INTO nodes (id, summary, description, status, contract_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![node_id, new_node.summary, new_node.description, status, contract_json],
        )
        .map_err(|e| Error::Db(format!("add_node insert: {e}")))?;

        // Predecessor edges: dep → new node
        for dep_id in &depends_on {
            tx.execute(
                "INSERT INTO node_edges (predecessor_id, successor_id) VALUES (?1, ?2)",
                params![dep_id, node_id],
            )
            .map_err(|e| Error::Db(format!("add_node edge: {e}")))?;
        }

        // Successor edges: new node → succ
        for succ_id in &successors {
            tx.execute(
                "INSERT INTO node_edges (predecessor_id, successor_id) VALUES (?1, ?2)",
                params![node_id, succ_id],
            )
            .map_err(|e| Error::Db(format!("add_node successor edge: {e}")))?;
        }

        // If new node is pending, any ready successors must be demoted to pending
        if status == "pending" && !successors.is_empty() {
            cascade_pending_sql(&tx)?;
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
        let (summary, description, status_str, result_summary, failure_reason, contract_json): (
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = self
            .conn
            .query_row(
                "SELECT summary, description, status, result_summary, failure_reason, contract_json FROM nodes WHERE id = ?1",
                params![node_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .map_err(|_| Error::NodeNotFound(node_id.to_string()))?;

        let predecessors = self.query_predecessors(node_id)?;
        let successors = self.query_successors(node_id)?;
        let session_ids = self.query_session_ids(node_id)?;
        let spawned_tasks = self.query_spawned_tasks(node_id)?;
        let contract = contract_json
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());

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
            contract,
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

fn cascade_pending_sql(tx: &rusqlite::Transaction<'_>) -> Result<()> {
    loop {
        let changed = tx
            .execute(
                "UPDATE nodes SET status = 'pending'
                 WHERE status = 'ready'
                   AND EXISTS (
                       SELECT 1 FROM node_edges e
                       JOIN nodes pred ON pred.id = e.predecessor_id
                       WHERE e.successor_id = nodes.id
                         AND pred.status != 'succeeded'
                   )",
                [],
            )
            .map_err(|e| Error::Db(format!("cascade_pending: {e}")))?;
        if changed == 0 {
            break;
        }
    }
    Ok(())
}

/// Multi-source BFS: check if any node in `from_set` can reach any node in `to_set`
/// via existing forward edges. Used for cycle detection before inserting new edges.
fn any_path_exists(
    tx: &rusqlite::Transaction<'_>,
    from_set: &[NodeId],
    to_set: &[NodeId],
) -> Result<bool> {
    if from_set.is_empty() || to_set.is_empty() {
        return Ok(false);
    }

    let to_set: std::collections::HashSet<&str> =
        to_set.iter().map(|s| s.as_str()).collect();

    // Load entire edge set once
    let mut adj: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut stmt = tx
        .prepare("SELECT predecessor_id, successor_id FROM node_edges")
        .map_err(|e| Error::Db(format!("cycle check edges: {e}")))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| Error::Db(format!("cycle check edges: {e}")))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| Error::Db(format!("cycle check edges: {e}")))?;
    for (pred, succ) in rows {
        adj.entry(pred).or_default().push(succ);
    }

    // BFS from all sources simultaneously
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut queue: std::collections::VecDeque<String> =
        from_set.iter().cloned().collect();

    while let Some(current) = queue.pop_front() {
        if to_set.contains(current.as_str()) {
            return Ok(true);
        }
        if !visited.insert(current.clone()) {
            continue;
        }
        if let Some(nexts) = adj.get(&current) {
            for next in nexts {
                queue.push_back(next.clone());
            }
        }
    }
    Ok(false)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn eng() -> (TaskEngine, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let e = TaskEngine::open(dir.path()).unwrap();
        (e, dir)
    }

    fn add(e: &TaskEngine, summary: &str, deps: &[&str], succs: &[&str]) -> NodeId {
        e.add_node(
            NewNode { summary: summary.into(), description: String::new(), contract: NodeContract::default() },
            deps.iter().map(|s| s.to_string()).collect(),
            succs.iter().map(|s| s.to_string()).collect(),
        )
        .unwrap()
    }

    fn try_add(
        e: &TaskEngine,
        summary: &str,
        deps: &[&str],
        succs: &[&str],
    ) -> Result<NodeId> {
        e.add_node(
            NewNode { summary: summary.into(), description: String::new(), contract: NodeContract::default() },
            deps.iter().map(|s| s.to_string()).collect(),
            succs.iter().map(|s| s.to_string()).collect(),
        )
    }

    fn status(e: &TaskEngine, id: &str) -> NodeStatus {
        e.get_node(id).unwrap().status
    }

    fn succeed(e: &TaskEngine, id: &str) {
        e.update_nodes(vec![NodeUpdate {
            id: id.to_string(),
            status: Some(NodeStatus::Succeeded),
            metadata: Some(NodeMetadata {
                result_summary: Some("ok".into()),
                ..Default::default()
            }),
        }])
        .unwrap();
    }

    fn run(e: &TaskEngine, id: &str) {
        e.update_nodes(vec![NodeUpdate {
            id: id.to_string(),
            status: Some(NodeStatus::Running),
            metadata: None,
        }])
        .unwrap();
    }

    // ── Basic node creation ───────────────────────────────────────────────────

    #[test]
    fn root_node_is_ready() {
        let (e, _d) = eng();
        let n = add(&e, "root", &[], &[]);
        assert_eq!(status(&e, &n), NodeStatus::Ready);
    }

    #[test]
    fn node_is_pending_when_dep_not_succeeded() {
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[&a], &[]);
        assert_eq!(status(&e, &a), NodeStatus::Ready);
        assert_eq!(status(&e, &b), NodeStatus::Pending);
    }

    #[test]
    fn node_is_ready_when_all_deps_succeeded() {
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        succeed(&e, &a);
        let b = add(&e, "B", &[&a], &[]);
        assert_eq!(status(&e, &b), NodeStatus::Ready);
    }

    #[test]
    fn node_pending_when_one_dep_not_succeeded() {
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[], &[]);
        succeed(&e, &a);
        // c depends on both; b is ready but not succeeded
        let c = add(&e, "C", &[&a, &b], &[]);
        assert_eq!(status(&e, &c), NodeStatus::Pending);
    }

    // ── cascade_ready ─────────────────────────────────────────────────────────

    #[test]
    fn cascade_ready_single_hop() {
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[&a], &[]);
        assert_eq!(status(&e, &b), NodeStatus::Pending);
        succeed(&e, &a);
        assert_eq!(status(&e, &b), NodeStatus::Ready);
    }

    #[test]
    fn cascade_ready_chain() {
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[&a], &[]);
        let c = add(&e, "C", &[&b], &[]);
        let d = add(&e, "D", &[&c], &[]);
        succeed(&e, &a);
        assert_eq!(status(&e, &b), NodeStatus::Ready);
        assert_eq!(status(&e, &c), NodeStatus::Pending);
        succeed(&e, &b);
        assert_eq!(status(&e, &c), NodeStatus::Ready);
        assert_eq!(status(&e, &d), NodeStatus::Pending);
        succeed(&e, &c);
        assert_eq!(status(&e, &d), NodeStatus::Ready);
    }

    #[test]
    fn cascade_ready_fan_out() {
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[&a], &[]);
        let c = add(&e, "C", &[&a], &[]);
        let d = add(&e, "D", &[&a], &[]);
        succeed(&e, &a);
        assert_eq!(status(&e, &b), NodeStatus::Ready);
        assert_eq!(status(&e, &c), NodeStatus::Ready);
        assert_eq!(status(&e, &d), NodeStatus::Ready);
    }

    #[test]
    fn cascade_ready_fan_in_partial() {
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[], &[]);
        let c = add(&e, "C", &[&a, &b], &[]);
        succeed(&e, &a);
        assert_eq!(status(&e, &c), NodeStatus::Pending); // b still not succeeded
        succeed(&e, &b);
        assert_eq!(status(&e, &c), NodeStatus::Ready);
    }

    // ── list_ready ────────────────────────────────────────────────────────────

    #[test]
    fn list_ready_returns_only_ready_nodes() {
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[&a], &[]);
        let c = add(&e, "C", &[], &[]);
        let ready_ids: Vec<_> = e.list_ready().unwrap().into_iter().map(|n| n.id).collect();
        assert!(ready_ids.contains(&a));
        assert!(ready_ids.contains(&c));
        assert!(!ready_ids.contains(&b));
    }

    // ── --successor: edge creation and visibility ─────────────────────────────

    #[test]
    fn successor_edges_visible_in_get_node() {
        let (e, _d) = eng();
        let s = add(&e, "S", &[], &[]);
        succeed(&e, &s);
        let n = add(&e, "N", &[], &[&s]);
        let n_node = e.get_node(&n).unwrap();
        let s_node = e.get_node(&s).unwrap();
        assert!(n_node.successors.contains(&s));
        assert!(s_node.predecessors.contains(&n));
    }

    #[test]
    fn deps_and_succs_both_visible() {
        let (e, _d) = eng();
        let p = add(&e, "P", &[], &[]);
        succeed(&e, &p);
        let s = add(&e, "S", &[], &[]);
        let n = add(&e, "N", &[&p], &[&s]);
        let n_node = e.get_node(&n).unwrap();
        assert!(n_node.predecessors.contains(&p));
        assert!(n_node.successors.contains(&s));
    }

    // ── --successor: cascade_pending ─────────────────────────────────────────

    #[test]
    fn ready_successor_demoted_when_new_node_is_pending() {
        let (e, _d) = eng();
        let dep = add(&e, "dep", &[], &[]); // ready, not succeeded
        let s = add(&e, "S", &[], &[]);     // independent, ready
        assert_eq!(status(&e, &s), NodeStatus::Ready);
        let n = add(&e, "N", &[&dep], &[&s]); // N is pending (dep not succeeded)
        assert_eq!(status(&e, &n), NodeStatus::Pending);
        assert_eq!(status(&e, &s), NodeStatus::Pending); // demoted
    }

    #[test]
    fn ready_successor_stays_ready_when_new_node_is_ready() {
        let (e, _d) = eng();
        let dep = add(&e, "dep", &[], &[]);
        succeed(&e, &dep); // dep succeeded → N will be ready
        let s = add(&e, "S", &[], &[]);
        let n = add(&e, "N", &[&dep], &[&s]);
        assert_eq!(status(&e, &n), NodeStatus::Ready);
        assert_eq!(status(&e, &s), NodeStatus::Ready); // not demoted
    }

    #[test]
    fn multiple_ready_successors_all_demoted() {
        let (e, _d) = eng();
        let dep = add(&e, "dep", &[], &[]);
        let s1 = add(&e, "S1", &[], &[]);
        let s2 = add(&e, "S2", &[], &[]);
        let s3 = add(&e, "S3", &[], &[]);
        add(&e, "N", &[&dep], &[&s1, &s2, &s3]);
        assert_eq!(status(&e, &s1), NodeStatus::Pending);
        assert_eq!(status(&e, &s2), NodeStatus::Pending);
        assert_eq!(status(&e, &s3), NodeStatus::Pending);
    }

    #[test]
    fn cascade_pending_chain() {
        // N(pending) → S1(ready) → S2 was ready because S1 was succeeded at S2 creation.
        // After inserting N→S1: S1 becomes pending (direct effect).
        // S2 has S1 (now pending) as predecessor → S2 should also become pending.
        let (e, _d) = eng();
        let s1 = add(&e, "S1", &[], &[]);
        succeed(&e, &s1); // S1 succeeded
        let s2 = add(&e, "S2", &[&s1], &[]); // S2 is ready (S1 succeeded)
        assert_eq!(status(&e, &s2), NodeStatus::Ready);
        // Now insert N(pending) with S1 as successor.
        // But S1 is succeeded — cascade_pending only touches ready nodes → S1 not demoted.
        // So S2 (which has only S1 as dep, and S1 is succeeded) stays ready.
        let dep = add(&e, "dep", &[], &[]);
        add(&e, "N", &[&dep], &[&s1]);
        assert_eq!(status(&e, &s1), NodeStatus::Succeeded); // not demoted
        assert_eq!(status(&e, &s2), NodeStatus::Ready);     // not affected

        // Real chain: S1 and S2 both ready (not succeeded), then insert pending predecessor of S1.
        let (e2, _d2) = eng();
        let r1 = add(&e2, "R1", &[], &[]); // ready
        let r2 = add(&e2, "R2", &[], &[]); // ready (independent)
        // Make r2 depend on r1 via a new intermediate node (using --successor)
        // Actually we can't add deps after creation. Let's just verify direct demotion:
        let dep2 = add(&e2, "dep2", &[], &[]);
        add(&e2, "M", &[&dep2], &[&r1, &r2]); // both demoted
        assert_eq!(status(&e2, &r1), NodeStatus::Pending);
        assert_eq!(status(&e2, &r2), NodeStatus::Pending);
    }

    #[test]
    fn cascade_pending_skips_running_node() {
        let (e, _d) = eng();
        let s = add(&e, "S", &[], &[]);
        run(&e, &s); // S is now running
        let dep = add(&e, "dep", &[], &[]);
        add(&e, "N", &[&dep], &[&s]);
        // cascade_pending only touches 'ready' nodes
        assert_eq!(status(&e, &s), NodeStatus::Running);
    }

    #[test]
    fn cascade_pending_skips_succeeded_node() {
        let (e, _d) = eng();
        let s = add(&e, "S", &[], &[]);
        succeed(&e, &s);
        let dep = add(&e, "dep", &[], &[]);
        add(&e, "N", &[&dep], &[&s]);
        assert_eq!(status(&e, &s), NodeStatus::Succeeded);
    }

    #[test]
    fn cascade_pending_skips_failed_node() {
        let (e, _d) = eng();
        let s = add(&e, "S", &[], &[]);
        e.update_nodes(vec![NodeUpdate {
            id: s.clone(),
            status: Some(NodeStatus::Failed),
            metadata: None,
        }])
        .unwrap();
        let dep = add(&e, "dep", &[], &[]);
        add(&e, "N", &[&dep], &[&s]);
        assert_eq!(status(&e, &s), NodeStatus::Failed);
    }

    #[test]
    fn unrelated_ready_node_not_affected() {
        let (e, _d) = eng();
        let dep = add(&e, "dep", &[], &[]);
        let s = add(&e, "S", &[], &[]);   // successor of N
        let x = add(&e, "X", &[], &[]);   // unrelated, should stay ready
        add(&e, "N", &[&dep], &[&s]);
        assert_eq!(status(&e, &s), NodeStatus::Pending);
        assert_eq!(status(&e, &x), NodeStatus::Ready); // untouched
    }

    #[test]
    fn no_successor_no_cascade_pending() {
        let (e, _d) = eng();
        let dep = add(&e, "dep", &[], &[]);
        let other = add(&e, "other", &[], &[]); // ready, no relation to N
        add(&e, "N", &[&dep], &[]);
        assert_eq!(status(&e, &other), NodeStatus::Ready);
    }

    // ── Cycle detection ───────────────────────────────────────────────────────

    #[test]
    fn cycle_direct_same_node_as_dep_and_succ() {
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let err = try_add(&e, "N", &[&a], &[&a]).unwrap_err();
        assert!(matches!(err, Error::CycleDetected));
    }

    #[test]
    fn cycle_indirect_path_exists() {
        // A → B → C exists; inserting N with dep=C and succ=A creates C→N→A…→C
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[&a], &[]);
        let c = add(&e, "C", &[&b], &[]);
        let err = try_add(&e, "N", &[&c], &[&a]).unwrap_err();
        assert!(matches!(err, Error::CycleDetected));
    }

    #[test]
    fn cycle_longer_path() {
        // A → B → C → D; inserting N with dep=D and succ=A
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[&a], &[]);
        let c = add(&e, "C", &[&b], &[]);
        let d = add(&e, "D", &[&c], &[]);
        assert!(matches!(
            try_add(&e, "N", &[&d], &[&a]),
            Err(Error::CycleDetected)
        ));
    }

    #[test]
    fn no_cycle_parallel_branches() {
        // A → B and A → C; adding N with dep=B and succ=C is fine (C cannot reach B)
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[&a], &[]);
        let c = add(&e, "C", &[&a], &[]);
        assert!(try_add(&e, "N", &[&b], &[&c]).is_ok());
    }

    #[test]
    fn no_cycle_successor_has_no_outgoing_edges() {
        // P → S exists; inserting N with dep=P and succ=S is fine (S has no edges)
        let (e, _d) = eng();
        let p = add(&e, "P", &[], &[]);
        let s = add(&e, "S", &[&p], &[]);
        // S has no outgoing edges → no path from S back to P
        assert!(try_add(&e, "N", &[&p], &[&s]).is_ok());
    }

    #[test]
    fn no_cycle_with_successor_no_predecessors() {
        // N has no deps but has a successor — no cycle possible
        let (e, _d) = eng();
        let s = add(&e, "S", &[], &[]);
        assert!(try_add(&e, "N", &[], &[&s]).is_ok());
    }

    #[test]
    fn no_cycle_complex_diamond() {
        // A → B → D and A → C → D (diamond); adding N with dep=D and succ=... not A/B/C
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[&a], &[]);
        let c = add(&e, "C", &[&a], &[]);
        let d = add(&e, "D", &[&b, &c], &[]);
        let x = add(&e, "X", &[], &[]); // independent
        // N with dep=D, succ=X: X has no path to A/B/C/D → ok
        assert!(try_add(&e, "N", &[&d], &[&x]).is_ok());
        // N2 with dep=D, succ=A: path A→B→D exists, so D→N2→A→B→D is a cycle → error
        assert!(matches!(
            try_add(&e, "N2", &[&d], &[&a]),
            Err(Error::CycleDetected)
        ));
    }

    #[test]
    fn cycle_via_one_of_multiple_successors() {
        // A → B; adding N with dep=A and succs=[B, C] where B can reach A → cycle
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[&a], &[]);
        let c = add(&e, "C", &[], &[]);
        // B can reach A? No, B→A doesn't exist. Let's set up a real cycle:
        // A → B → X; N with dep=X, succ=A → cycle X→N→A→B→X (via existing A→B)
        let x = add(&e, "X", &[&b], &[]);
        // succ A: path from A → B → X (A can reach X), so path from A back to X? No.
        // We need path FROM succ TO dep.
        // dep=X, succ=A: path from A to X? A→B→X yes! → cycle detected
        let err = try_add(&e, "N", &[&x], &[&a, &c]).unwrap_err();
        assert!(matches!(err, Error::CycleDetected));
    }

    // ── Validation errors ─────────────────────────────────────────────────────

    #[test]
    fn error_predecessor_not_found() {
        let (e, _d) = eng();
        let err = try_add(&e, "N", &["bad-id"], &[]).unwrap_err();
        assert!(matches!(err, Error::NodeNotFound(_)));
    }

    #[test]
    fn error_successor_not_found() {
        let (e, _d) = eng();
        let err = try_add(&e, "N", &[], &["bad-id"]).unwrap_err();
        assert!(matches!(err, Error::NodeNotFound(_)));
    }

    // ── update_nodes state machine ────────────────────────────────────────────

    #[test]
    fn invalid_transition_rejected() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        // pending → succeeded is not valid (must go through running)
        // actually ready → succeeded IS valid per validate_transition
        // Let's try succeeded → running
        succeed(&e, &n);
        let err = e
            .update_nodes(vec![NodeUpdate {
                id: n.clone(),
                status: Some(NodeStatus::Running),
                metadata: None,
            }])
            .unwrap_err();
        assert!(matches!(err, Error::InvalidStateTransition { .. }));
    }

    #[test]
    fn ready_to_failed_is_valid() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        e.update_nodes(vec![NodeUpdate {
            id: n.clone(),
            status: Some(NodeStatus::Failed),
            metadata: Some(NodeMetadata {
                failure_reason: Some("deliberate".into()),
                ..Default::default()
            }),
        }])
        .unwrap();
        assert_eq!(status(&e, &n), NodeStatus::Failed);
    }

    #[test]
    fn update_metadata_stored_and_retrieved() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        succeed(&e, &n);
        let node = e.get_node(&n).unwrap();
        assert_eq!(node.metadata.result_summary.as_deref(), Some("ok"));
    }

    // ── Error cases ───────────────────────────────────────────────────────────

    #[test]
    fn get_node_not_found() {
        let (e, _d) = eng();
        assert!(matches!(e.get_node("no-such-id"), Err(Error::NodeNotFound(_))));
    }

    #[test]
    fn update_nodes_not_found() {
        let (e, _d) = eng();
        let err = e
            .update_nodes(vec![NodeUpdate {
                id: "no-such-id".into(),
                status: Some(NodeStatus::Succeeded),
                metadata: None,
            }])
            .unwrap_err();
        assert!(matches!(err, Error::NodeNotFound(_)));
    }

    // ── Batch updates ─────────────────────────────────────────────────────────

    #[test]
    fn update_nodes_batch_multiple_nodes() {
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[], &[]);
        e.update_nodes(vec![
            NodeUpdate {
                id: a.clone(),
                status: Some(NodeStatus::Succeeded),
                metadata: Some(NodeMetadata {
                    result_summary: Some("a_result".into()),
                    ..Default::default()
                }),
            },
            NodeUpdate {
                id: b.clone(),
                status: Some(NodeStatus::Failed),
                metadata: Some(NodeMetadata {
                    failure_reason: Some("b_fail".into()),
                    ..Default::default()
                }),
            },
        ])
        .unwrap();
        assert_eq!(status(&e, &a), NodeStatus::Succeeded);
        assert_eq!(status(&e, &b), NodeStatus::Failed);
        assert_eq!(
            e.get_node(&a).unwrap().metadata.result_summary.as_deref(),
            Some("a_result")
        );
        assert_eq!(
            e.get_node(&b).unwrap().metadata.failure_reason.as_deref(),
            Some("b_fail")
        );
    }

    #[test]
    fn update_nodes_session_ids_roundtrip() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        e.update_nodes(vec![NodeUpdate {
            id: n.clone(),
            status: Some(NodeStatus::Succeeded),
            metadata: Some(NodeMetadata {
                result_summary: Some("ok".into()),
                session_ids: vec!["sess-aaa".into(), "sess-bbb".into()],
                ..Default::default()
            }),
        }])
        .unwrap();
        let node = e.get_node(&n).unwrap();
        assert!(node.metadata.session_ids.contains(&"sess-aaa".to_string()));
        assert!(node.metadata.session_ids.contains(&"sess-bbb".to_string()));
    }

    #[test]
    fn update_nodes_session_ids_replaced_on_reupdate() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        run(&e, &n);
        e.update_nodes(vec![NodeUpdate {
            id: n.clone(),
            status: None,
            metadata: Some(NodeMetadata {
                session_ids: vec!["old".into()],
                ..Default::default()
            }),
        }])
        .unwrap();
        e.update_nodes(vec![NodeUpdate {
            id: n.clone(),
            status: None,
            metadata: Some(NodeMetadata {
                session_ids: vec!["new".into()],
                ..Default::default()
            }),
        }])
        .unwrap();
        let node = e.get_node(&n).unwrap();
        assert!(!node.metadata.session_ids.contains(&"old".to_string()));
        assert!(node.metadata.session_ids.contains(&"new".to_string()));
    }

    #[test]
    fn update_nodes_spawned_tasks_roundtrip() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        let c1 = add(&e, "child1", &[], &[]);
        let c2 = add(&e, "child2", &[], &[]);
        e.update_nodes(vec![NodeUpdate {
            id: n.clone(),
            status: None,
            metadata: Some(NodeMetadata {
                spawned_tasks: vec![c1.clone(), c2.clone()],
                ..Default::default()
            }),
        }])
        .unwrap();
        let node = e.get_node(&n).unwrap();
        assert!(node.metadata.spawned_tasks.contains(&c1));
        assert!(node.metadata.spawned_tasks.contains(&c2));
    }

    // ── list_all ──────────────────────────────────────────────────────────────

    #[test]
    fn list_all_includes_all_statuses() {
        let (e, _d) = eng();
        let ready = add(&e, "ready", &[], &[]);
        let dep = add(&e, "dep", &[], &[]);
        let pending = add(&e, "pending", &[&dep], &[]);
        let failed = add(&e, "failed", &[], &[]);
        e.update_nodes(vec![NodeUpdate {
            id: failed.clone(),
            status: Some(NodeStatus::Failed),
            metadata: None,
        }])
        .unwrap();

        let all = e.list_all().unwrap();
        let map: std::collections::HashMap<_, _> =
            all.iter().map(|n| (n.id.clone(), n.status)).collect();
        assert_eq!(map[&ready], NodeStatus::Ready);
        assert_eq!(map[&pending], NodeStatus::Pending);
        assert_eq!(map[&failed], NodeStatus::Failed);
    }

    #[test]
    fn list_all_empty() {
        let (e, _d) = eng();
        assert!(e.list_all().unwrap().is_empty());
    }

    // ── Process tracking ──────────────────────────────────────────────────────

    #[test]
    fn process_create_and_get_by_node() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        e.process_create(&n, 12345, "sess-1", "kimi", "/tmp/log.jsonl")
            .unwrap();
        let proc = e.process_get_by_node(&n).unwrap().unwrap();
        assert_eq!(proc.node_id, n);
        assert_eq!(proc.pid, 12345);
        assert_eq!(proc.session_id, "sess-1");
        assert_eq!(proc.adapter, "kimi");
        assert_eq!(proc.status, ProcessStatus::Running);
        assert!(proc.exit_code.is_none());
        assert!(proc.ended_at.is_none());
    }

    #[test]
    fn process_get_by_node_returns_latest() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        e.process_create(&n, 1, "sess-1", "kimi", "/log").unwrap();
        e.process_create(&n, 2, "sess-2", "kimi", "/log").unwrap();
        let proc = e.process_get_by_node(&n).unwrap().unwrap();
        assert_eq!(proc.pid, 2);
        assert_eq!(proc.session_id, "sess-2");
    }

    #[test]
    fn process_get_by_node_none_when_no_process() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        assert!(e.process_get_by_node(&n).unwrap().is_none());
    }

    #[test]
    fn process_mark_exited_zero() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        e.process_create(&n, 100, "s", "kimi", "/log").unwrap();
        let id = e.process_get_by_node(&n).unwrap().unwrap().id;
        e.process_mark_exited(id, 0).unwrap();
        let updated = e.process_get_by_node(&n).unwrap().unwrap();
        assert_eq!(updated.status, ProcessStatus::Exited);
        assert_eq!(updated.exit_code, Some(0));
        assert!(updated.ended_at.is_some());
    }

    #[test]
    fn process_mark_exited_nonzero() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        e.process_create(&n, 100, "s", "kimi", "/log").unwrap();
        let id = e.process_get_by_node(&n).unwrap().unwrap().id;
        e.process_mark_exited(id, 1).unwrap();
        assert_eq!(
            e.process_get_by_node(&n).unwrap().unwrap().exit_code,
            Some(1)
        );
    }

    #[test]
    fn process_mark_killed() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        e.process_create(&n, 100, "s", "kimi", "/log").unwrap();
        let id = e.process_get_by_node(&n).unwrap().unwrap().id;
        e.process_mark_killed(id).unwrap();
        let updated = e.process_get_by_node(&n).unwrap().unwrap();
        assert_eq!(updated.status, ProcessStatus::Killed);
        assert!(updated.ended_at.is_some());
    }

    #[test]
    fn process_list_running_filters_non_running() {
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[], &[]);
        let c = add(&e, "C", &[], &[]);
        e.process_create(&a, 1, "s1", "kimi", "/log").unwrap();
        e.process_create(&b, 2, "s2", "kimi", "/log").unwrap();
        e.process_create(&c, 3, "s3", "kimi", "/log").unwrap();

        let id_b = e.process_get_by_node(&b).unwrap().unwrap().id;
        e.process_mark_exited(id_b, 0).unwrap();
        let id_c = e.process_get_by_node(&c).unwrap().unwrap().id;
        e.process_mark_killed(id_c).unwrap();

        let running = e.process_list_running().unwrap();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].pid, 1);
    }

    #[test]
    fn process_list_running_empty_initially() {
        let (e, _d) = eng();
        assert!(e.process_list_running().unwrap().is_empty());
    }

    // ── Message queue ─────────────────────────────────────────────────────────

    #[test]
    fn message_enqueue_and_consume_fifo() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        e.message_enqueue(&n, "first").unwrap();
        e.message_enqueue(&n, "second").unwrap();
        e.message_enqueue(&n, "third").unwrap();
        assert_eq!(e.message_consume(&n).unwrap().unwrap().content, "first");
        assert_eq!(e.message_consume(&n).unwrap().unwrap().content, "second");
        assert_eq!(e.message_consume(&n).unwrap().unwrap().content, "third");
        assert!(e.message_consume(&n).unwrap().is_none());
    }

    #[test]
    fn message_consume_empty_returns_none() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        assert!(e.message_consume(&n).unwrap().is_none());
    }

    #[test]
    fn message_consume_idempotent_after_consumed() {
        let (e, _d) = eng();
        let n = add(&e, "N", &[], &[]);
        e.message_enqueue(&n, "msg").unwrap();
        e.message_consume(&n).unwrap().unwrap();
        assert!(e.message_consume(&n).unwrap().is_none());
    }

    #[test]
    fn message_queue_independent_per_node() {
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[], &[]);
        e.message_enqueue(&a, "for_a").unwrap();
        e.message_enqueue(&b, "for_b").unwrap();
        assert_eq!(e.message_consume(&a).unwrap().unwrap().content, "for_a");
        assert_eq!(e.message_consume(&b).unwrap().unwrap().content, "for_b");
    }

    // ── viz_data ──────────────────────────────────────────────────────────────

    #[test]
    fn viz_data_empty() {
        let (e, _d) = eng();
        let (nodes, edges) = e.viz_data().unwrap();
        assert!(nodes.is_empty());
        assert!(edges.is_empty());
    }

    #[test]
    fn viz_data_nodes_and_edges() {
        let (e, _d) = eng();
        let a = add(&e, "A", &[], &[]);
        let b = add(&e, "B", &[&a], &[]);
        let (nodes, edges) = e.viz_data().unwrap();
        let ids: Vec<_> = nodes.iter().map(|n| n.id.clone()).collect();
        assert!(ids.contains(&a));
        assert!(ids.contains(&b));
        assert!(edges
            .iter()
            .any(|e| e.predecessor_id == a && e.successor_id == b));
    }

    // ── RegistryEngine ────────────────────────────────────────────────────────

    fn registry_eng() -> (RegistryEngine, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let e = RegistryEngine::new(dir.path()).unwrap();
        (e, dir)
    }

    #[test]
    fn task_create_returns_uuid_and_creates_db() {
        let (reg, _d) = registry_eng();
        let id = reg.task_create("my task", "/work").unwrap();
        assert!(!id.is_empty());
        assert!(reg.task_dir(&id).join("task.db").exists());
    }

    #[test]
    fn task_list_returns_all_created() {
        let (reg, _d) = registry_eng();
        let id1 = reg.task_create("task1", "/tmp").unwrap();
        let id2 = reg.task_create("task2", "/tmp").unwrap();
        let ids: Vec<_> = reg.task_list().unwrap().into_iter().map(|t| t.id).collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn task_list_empty_initially() {
        let (reg, _d) = registry_eng();
        assert!(reg.task_list().unwrap().is_empty());
    }

    #[test]
    fn task_get_returns_correct_fields() {
        let (reg, _d) = registry_eng();
        let id = reg.task_create("my summary", "/the/workdir").unwrap();
        let task = reg.task_get(&id).unwrap();
        assert_eq!(task.summary, "my summary");
        assert_eq!(task.workdir, "/the/workdir");
        assert!(!task.created_at.is_empty());
    }

    #[test]
    fn task_get_not_found() {
        let (reg, _d) = registry_eng();
        assert!(matches!(
            reg.task_get("nonexistent"),
            Err(Error::TaskNotFound(_))
        ));
    }

    #[test]
    fn init_deploys_skill_files() {
        let (reg, _d) = registry_eng();
        reg.init().unwrap();
        let skill_dir = reg.skills_dir().join("kerrigan");
        assert!(skill_dir.join("SKILL.md").exists());
        assert!(skill_dir.join("guide.md").exists());
        assert!(skill_dir.join("guide-research.md").exists());
    }

    #[test]
    fn init_is_idempotent() {
        let (reg, _d) = registry_eng();
        reg.init().unwrap();
        reg.init().unwrap(); // second call must not fail
    }

    // ── Successor + cascade_ready integration ─────────────────────────────────

    #[test]
    fn successor_then_cascade_ready_full_pipeline() {
        // D → N → S; after D succeeds N becomes ready; after N succeeds S becomes ready
        let (e, _d) = eng();
        let d = add(&e, "D", &[], &[]);
        let s = add(&e, "S", &[], &[]);
        let n = add(&e, "N", &[&d], &[&s]);

        assert_eq!(status(&e, &n), NodeStatus::Pending);
        assert_eq!(status(&e, &s), NodeStatus::Pending); // demoted by cascade_pending

        succeed(&e, &d);
        assert_eq!(status(&e, &n), NodeStatus::Ready); // cascade_ready
        assert_eq!(status(&e, &s), NodeStatus::Pending); // N not yet succeeded

        succeed(&e, &n);
        assert_eq!(status(&e, &s), NodeStatus::Ready); // cascade_ready
    }
}
