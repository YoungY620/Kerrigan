use serde::{Deserialize, Serialize};

pub type TaskId = String;
pub type NodeId = String;

// ── Task registry ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEntry {
    pub id: TaskId,
    pub summary: String,
    pub workdir: String,
    pub created_at: String,
}

// ── DAG ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Ready,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub spawned_tasks: Vec<NodeId>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub session_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub summary: String,
    pub description: String,
    pub predecessors: Vec<NodeId>,
    pub successors: Vec<NodeId>,
    pub status: NodeStatus,
    pub metadata: NodeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSummary {
    pub id: NodeId,
    pub summary: String,
    pub successors: Vec<NodeId>,
    pub status: NodeStatus,
}

impl From<&Node> for NodeSummary {
    fn from(node: &Node) -> Self {
        Self {
            id: node.id.clone(),
            summary: node.summary.clone(),
            successors: node.successors.clone(),
            status: node.status,
        }
    }
}

// ── Input types ──

pub struct NewNode {
    pub summary: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeUpdate {
    pub id: NodeId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<NodeStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<NodeMetadata>,
}

// ── Process tracking ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessStatus {
    Running,
    Exited,
    Killed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEntry {
    pub id: i64,
    pub node_id: NodeId,
    pub pid: u32,
    pub session_id: String,
    pub adapter: String,
    pub log_path: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub exit_code: Option<i32>,
    pub status: ProcessStatus,
}

// ── Viz types ──

#[derive(Debug, Clone, Serialize)]
pub struct VizNodeData {
    pub id: NodeId,
    pub summary: String,
    pub status: NodeStatus,
    pub result_summary: Option<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VizEdgeData {
    pub predecessor_id: NodeId,
    pub successor_id: NodeId,
}

// ── Node messages ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMessage {
    pub id: i64,
    pub node_id: NodeId,
    pub content: String,
    pub created_at: String,
}
