use crate::error::{Result, StorageError};
use crate::model::*;
use crate::persist;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;

pub struct StorageEngine {
    root: PathBuf,
}

impl StorageEngine {
    /// Create a new engine pointing at the given root directory.
    /// Typically `~/.kerrigan/`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Default root: `~/.kerrigan/`.
    pub fn default_root() -> Result<Self> {
        let home = std::env::var("HOME").map_err(|_| StorageError::NotInitialized)?;
        Ok(Self::new(PathBuf::from(home).join(".kerrigan")))
    }

    // ── Paths ──

    fn registry_path(&self) -> PathBuf {
        self.root.join("registry.json")
    }

    fn lock_path(&self) -> PathBuf {
        self.root.join("lock")
    }

    pub fn task_dir(&self, task_id: &str) -> PathBuf {
        self.root.join("tasks").join(task_id)
    }

    fn dag_path(&self, task_id: &str) -> PathBuf {
        self.task_dir(task_id).join("dag.json")
    }

    fn config_path(&self, task_id: &str) -> PathBuf {
        self.task_dir(task_id).join("config.json")
    }

    // ── Init ──

    // SKILL.md compiled into the binary
    const SKILL_MD: &str = include_str!("../skills/kerrigan-storage/SKILL.md");

    /// Initialize `~/.kerrigan/` directory structure. Idempotent.
    /// Also deploys the kerrigan-storage skill to `~/.kerrigan/skills/`.
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(self.root.join("tasks")).map_err(|e| StorageError::io(&self.root, e))?;

        let reg_path = self.registry_path();
        if !reg_path.exists() {
            persist::atomic_write(&reg_path, &Registry::default())?;
        }

        // Deploy bundled skill
        let skill_dir = self.skills_dir().join("kerrigan-storage");
        fs::create_dir_all(&skill_dir).map_err(|e| StorageError::io(&skill_dir, e))?;
        let skill_path = skill_dir.join("SKILL.md");
        fs::write(&skill_path, Self::SKILL_MD).map_err(|e| StorageError::io(&skill_path, e))?;

        Ok(())
    }

    /// Path to the skills directory: `~/.kerrigan/skills/`.
    pub fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    // ── Task management ──

    /// Create a new task, returning its id.
    pub fn task_create(&self, summary: &str, workdir: &str) -> Result<TaskId> {
        self.ensure_initialized()?;
        let _lock = persist::lock_exclusive(&self.lock_path())?;

        let task_id = uuid::Uuid::new_v4().to_string();
        let dir = self.task_dir(&task_id);
        fs::create_dir_all(dir.join("shared")).map_err(|e| StorageError::io(&dir, e))?;

        // Create initial files
        persist::atomic_write(&self.dag_path(&task_id), &Dag::default())?;
        persist::atomic_write(
            &self.config_path(&task_id),
            &TaskConfig {
                workdir: workdir.to_string(),
            },
        )?;
        persist::atomic_write(&dir.join("sessions.json"), &serde_json::json!([]))?;
        fs::write(dir.join("board.md"), "").map_err(|e| StorageError::io(&dir, e))?;

        // Register
        let mut registry: Registry = persist::read_json(&self.registry_path())?;
        registry.tasks.push(TaskEntry {
            id: task_id.clone(),
            summary: summary.to_string(),
            workdir: workdir.to_string(),
            created_at: now_iso(),
        });
        persist::atomic_write(&self.registry_path(), &registry)?;

        Ok(task_id)
    }

    /// List all registered tasks.
    pub fn task_list(&self) -> Result<Vec<TaskEntry>> {
        self.ensure_initialized()?;
        let _lock = persist::lock_exclusive(&self.lock_path())?;
        let registry: Registry = persist::read_json(&self.registry_path())?;
        Ok(registry.tasks)
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
        let dag_path = self.dag_path(task_id);
        let mut dag: Dag = persist::read_json(&dag_path)?;

        let node_id = uuid::Uuid::new_v4().to_string();

        // Validate dependencies exist
        for dep_id in &depends_on {
            if !dag.nodes.contains_key(dep_id) {
                return Err(StorageError::NodeNotFound(dep_id.clone()));
            }
        }

        // Determine initial status
        let all_predecessors_done = depends_on.iter().all(|dep_id| {
            dag.nodes
                .get(dep_id)
                .is_some_and(|n| n.status == NodeStatus::Succeeded)
        });
        let status = if depends_on.is_empty() || all_predecessors_done {
            NodeStatus::Ready
        } else {
            NodeStatus::Pending
        };

        let node = Node {
            id: node_id.clone(),
            summary: new_node.summary,
            description: new_node.description,
            predecessors: depends_on.clone(),
            successors: Vec::new(),
            status,
            metadata: NodeMetadata::default(),
        };

        // Check for cycles before inserting
        if would_create_cycle(&dag, &node_id, &depends_on) {
            return Err(StorageError::CycleDetected);
        }

        // Update successors of predecessor nodes
        for dep_id in &depends_on {
            if let Some(dep_node) = dag.nodes.get_mut(dep_id) {
                dep_node.successors.push(node_id.clone());
            }
        }

        dag.nodes.insert(node_id.clone(), node);
        persist::atomic_write(&dag_path, &dag)?;
        Ok(node_id)
    }

    /// List all nodes in a task's DAG (lightweight summaries).
    pub fn list_all(&self, task_id: &str) -> Result<Vec<NodeSummary>> {
        self.ensure_task_exists(task_id)?;
        let dag: Dag = persist::read_json(&self.dag_path(task_id))?;
        Ok(dag.nodes.values().map(NodeSummary::from).collect())
    }

    /// List all Ready nodes in a task's DAG.
    pub fn list_ready(&self, task_id: &str) -> Result<Vec<NodeSummary>> {
        self.ensure_task_exists(task_id)?;
        let dag: Dag = persist::read_json(&self.dag_path(task_id))?;
        Ok(dag
            .nodes
            .values()
            .filter(|n| n.status == NodeStatus::Ready)
            .map(NodeSummary::from)
            .collect())
    }

    /// Batch update nodes. Handles cascading Ready status.
    pub fn update_nodes(&self, task_id: &str, updates: Vec<NodeUpdate>) -> Result<()> {
        self.ensure_task_exists(task_id)?;
        let dag_path = self.dag_path(task_id);
        let mut dag: Dag = persist::read_json(&dag_path)?;

        // Apply updates
        let mut newly_succeeded = Vec::new();
        for update in &updates {
            let node = dag
                .nodes
                .get_mut(&update.id)
                .ok_or_else(|| StorageError::NodeNotFound(update.id.clone()))?;

            if let Some(new_status) = update.status {
                validate_transition(&node.id, node.status, new_status)?;
                if new_status == NodeStatus::Succeeded {
                    newly_succeeded.push(node.id.clone());
                }
                node.status = new_status;
            }

            if let Some(ref meta) = update.metadata {
                node.metadata = meta.clone();
            }
        }

        // Cascade: check successors of newly succeeded nodes
        cascade_ready(&mut dag, &newly_succeeded);

        persist::atomic_write(&dag_path, &dag)?;
        Ok(())
    }

    /// Get full details of a single node.
    pub fn get_node(&self, task_id: &str, node_id: &str) -> Result<Node> {
        self.ensure_task_exists(task_id)?;
        let dag: Dag = persist::read_json(&self.dag_path(task_id))?;
        dag.nodes
            .get(node_id)
            .cloned()
            .ok_or_else(|| StorageError::NodeNotFound(node_id.to_string()))
    }

    // ── Helpers ──

    fn ensure_initialized(&self) -> Result<()> {
        if !self.registry_path().exists() {
            return Err(StorageError::NotInitialized);
        }
        Ok(())
    }

    fn ensure_task_exists(&self, task_id: &str) -> Result<()> {
        self.ensure_initialized()?;
        if !self.dag_path(task_id).exists() {
            return Err(StorageError::TaskNotFound(task_id.to_string()));
        }
        Ok(())
    }
}

/// Check if adding a node with the given predecessors would create a cycle.
/// Since we're adding a new node (not yet in the graph) that depends on `depends_on`,
/// a cycle would only occur if any node reachable from the new node's successors
/// can reach back to one of the predecessors. But since the new node has no successors
/// yet, it cannot create a cycle. This function is here for future-proofing when
/// we allow editing dependencies.
fn would_create_cycle(_dag: &Dag, _new_node_id: &str, _depends_on: &[NodeId]) -> bool {
    // A brand-new node with no successors cannot create a cycle.
    // Cycle detection becomes relevant when re-wiring existing edges.
    false
}

/// After nodes become Succeeded, check their successors.
/// If all predecessors of a successor are Succeeded, mark it Ready.
fn cascade_ready(dag: &mut Dag, newly_succeeded: &[NodeId]) {
    let mut queue: VecDeque<NodeId> = newly_succeeded.iter().cloned().collect();
    let mut visited: HashSet<NodeId> = HashSet::new();

    while let Some(node_id) = queue.pop_front() {
        if !visited.insert(node_id.clone()) {
            continue;
        }

        let successor_ids: Vec<NodeId> = dag
            .nodes
            .get(&node_id)
            .map(|n| n.successors.clone())
            .unwrap_or_default();

        for succ_id in successor_ids {
            let all_preds_succeeded = {
                let succ = match dag.nodes.get(&succ_id) {
                    Some(n) => n,
                    None => continue,
                };
                if succ.status != NodeStatus::Pending {
                    continue;
                }
                succ.predecessors.iter().all(|pid| {
                    dag.nodes
                        .get(pid)
                        .is_some_and(|p| p.status == NodeStatus::Succeeded)
                })
            };

            if all_preds_succeeded {
                if let Some(succ) = dag.nodes.get_mut(&succ_id) {
                    succ.status = NodeStatus::Ready;
                }
                // Continue cascading in case this unlocks further nodes
                queue.push_back(succ_id);
            }
        }
    }
}

fn validate_transition(node_id: &str, from: NodeStatus, to: NodeStatus) -> Result<()> {
    let valid = matches!(
        (from, to),
        (NodeStatus::Ready, NodeStatus::Running)
            | (NodeStatus::Running, NodeStatus::Succeeded)
            | (NodeStatus::Running, NodeStatus::Failed)
            // Allow direct Ready -> Succeeded/Failed for simple tasks
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
    let secs = duration.as_secs();
    // Simple ISO-ish format without chrono dependency
    format!("{secs}")
}
