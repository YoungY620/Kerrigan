use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Registry {
    pub tasks: Vec<TaskEntry>,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dag {
    pub nodes: HashMap<NodeId, Node>,
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

// ── Task config ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub workdir: String,
}

// ── Board messages ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardMessage {
    pub id: i64,
    pub content: String,
    pub created_at: String,
}
