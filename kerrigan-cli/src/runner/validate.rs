use kerrigan_storage::{NodeStatus, StorageEngine};
use std::collections::{HashMap, HashSet, VecDeque};

/// A single DAG consistency issue.
#[derive(Debug, Clone)]
pub struct DagIssue {
    pub node_id: String,
    pub description: String,
}

impl std::fmt::Display for DagIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.node_id, self.description)
    }
}

/// Validate DAG consistency after an agent round.
///
/// Checks:
/// - Duplicate node IDs
/// - Predecessor references to non-existent nodes
/// - Successor references to non-existent nodes
/// - Predecessor/successor bidirectional consistency
/// - Nodes with no predecessors should be Ready (not Pending)
/// - Ready nodes should have no incomplete predecessors
/// - Succeeded nodes should have result_summary
/// - Failed nodes should have failure_reason
/// - All nodes should have non-empty summary
pub fn validate_dag(engine: &StorageEngine, task_id: &str) -> Vec<DagIssue> {
    let mut issues = Vec::new();

    let summaries = match engine.list_all(task_id) {
        Ok(n) => n,
        Err(_) => return issues,
    };

    // Collect all node IDs for reference checks
    let all_ids: HashSet<String> = summaries.iter().map(|s| s.id.clone()).collect();

    // Check for duplicate IDs (list_all uses HashMap so unlikely, but defensive)
    if all_ids.len() != summaries.len() {
        issues.push(DagIssue {
            node_id: "*".into(),
            description: format!(
                "Duplicate node IDs detected: {} unique IDs for {} nodes",
                all_ids.len(),
                summaries.len()
            ),
        });
    }

    for summary in &summaries {
        let node = match engine.get_node(task_id, &summary.id) {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Predecessor references must exist
        for pred_id in &node.predecessors {
            if !all_ids.contains(pred_id) {
                issues.push(DagIssue {
                    node_id: node.id.clone(),
                    description: format!("Predecessor {pred_id} does not exist in the DAG"),
                });
            }
        }

        // Successor references must exist
        for succ_id in &node.successors {
            if !all_ids.contains(succ_id) {
                issues.push(DagIssue {
                    node_id: node.id.clone(),
                    description: format!("Successor {succ_id} does not exist in the DAG"),
                });
            }
        }

        // Bidirectional consistency: if A lists B as successor, B must list A as predecessor
        for succ_id in &node.successors {
            if let Ok(succ) = engine.get_node(task_id, succ_id)
                && !succ.predecessors.contains(&node.id)
            {
                issues.push(DagIssue {
                    node_id: node.id.clone(),
                    description: format!(
                        "Node lists {succ_id} as successor, but {succ_id} does not list this node as predecessor"
                    ),
                });
            }
        }

        // Nodes with no predecessors should not be Pending
        if node.predecessors.is_empty() && node.status == NodeStatus::Pending {
            issues.push(DagIssue {
                node_id: node.id.clone(),
                description: "Node has no predecessors but is Pending (should be Ready)".into(),
            });
        }

        // Ready nodes should have all predecessors Succeeded
        if node.status == NodeStatus::Ready {
            for pred_id in &node.predecessors {
                if let Ok(pred) = engine.get_node(task_id, pred_id)
                    && pred.status != NodeStatus::Succeeded
                {
                    issues.push(DagIssue {
                        node_id: node.id.clone(),
                        description: format!(
                            "Node is Ready but predecessor {pred_id} is {:?}",
                            pred.status
                        ),
                    });
                }
            }
        }

        // Succeeded nodes should have result_summary
        if node.status == NodeStatus::Succeeded && node.metadata.result_summary.is_none() {
            issues.push(DagIssue {
                node_id: node.id.clone(),
                description: "Node is Succeeded but has no result_summary".into(),
            });
        }

        // Failed nodes should have failure_reason
        if node.status == NodeStatus::Failed && node.metadata.failure_reason.is_none() {
            issues.push(DagIssue {
                node_id: node.id.clone(),
                description: "Node is Failed but has no failure_reason".into(),
            });
        }

        // All nodes should have non-empty summary
        if node.summary.trim().is_empty() {
            issues.push(DagIssue {
                node_id: node.id.clone(),
                description: "Node has empty summary".into(),
            });
        }
    }

    // Cycle detection via topological sort (Kahn's algorithm)
    // Build adjacency from successors
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for s in &summaries {
        in_degree.entry(s.id.clone()).or_insert(0);
        adj.entry(s.id.clone()).or_default();
    }
    for s in &summaries {
        if let Ok(node) = engine.get_node(task_id, &s.id) {
            for succ_id in &node.successors {
                if all_ids.contains(succ_id) {
                    *in_degree.entry(succ_id.clone()).or_insert(0) += 1;
                    adj.entry(node.id.clone())
                        .or_default()
                        .push(succ_id.clone());
                }
            }
        }
    }
    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut visited_count = 0;
    while let Some(id) = queue.pop_front() {
        visited_count += 1;
        if let Some(neighbors) = adj.get(&id) {
            for n in neighbors {
                if let Some(deg) = in_degree.get_mut(n) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(n.clone());
                    }
                }
            }
        }
    }
    if visited_count < summaries.len() {
        let cycle_nodes: Vec<String> = in_degree
            .iter()
            .filter(|(_, deg)| **deg > 0)
            .map(|(id, _)| id.clone())
            .collect();
        issues.push(DagIssue {
            node_id: "*".into(),
            description: format!(
                "Cycle detected involving {} nodes: {}",
                cycle_nodes.len(),
                cycle_nodes.join(", ")
            ),
        });
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use kerrigan_storage::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, StorageEngine, String) {
        let tmp = TempDir::new().unwrap();
        let engine = StorageEngine::new(tmp.path()).unwrap();
        engine.init().unwrap();
        let task_id = engine.task_create("test", "/tmp").unwrap();
        (tmp, engine, task_id)
    }

    #[test]
    fn valid_dag_no_issues() {
        let (_tmp, engine, task_id) = setup();
        let a = engine
            .add_node(
                &task_id,
                NewNode {
                    summary: "A".into(),
                    description: "do A".into(),
                },
                vec![],
            )
            .unwrap();
        engine
            .update_nodes(
                &task_id,
                vec![NodeUpdate {
                    id: a,
                    status: Some(NodeStatus::Succeeded),
                    metadata: Some(NodeMetadata {
                        result_summary: Some("done".into()),
                        ..Default::default()
                    }),
                }],
            )
            .unwrap();

        let issues = validate_dag(&engine, &task_id);
        assert!(issues.is_empty(), "Expected no issues, got: {issues:?}");
    }

    #[test]
    fn succeeded_without_result_summary() {
        let (_tmp, engine, task_id) = setup();
        let a = engine
            .add_node(
                &task_id,
                NewNode {
                    summary: "A".into(),
                    description: "".into(),
                },
                vec![],
            )
            .unwrap();
        engine
            .update_nodes(
                &task_id,
                vec![NodeUpdate {
                    id: a,
                    status: Some(NodeStatus::Succeeded),
                    metadata: None,
                }],
            )
            .unwrap();

        let issues = validate_dag(&engine, &task_id);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].description.contains("result_summary"));
    }

    #[test]
    fn failed_without_failure_reason() {
        let (_tmp, engine, task_id) = setup();
        let a = engine
            .add_node(
                &task_id,
                NewNode {
                    summary: "A".into(),
                    description: "".into(),
                },
                vec![],
            )
            .unwrap();
        engine
            .update_nodes(
                &task_id,
                vec![NodeUpdate {
                    id: a,
                    status: Some(NodeStatus::Failed),
                    metadata: None,
                }],
            )
            .unwrap();

        let issues = validate_dag(&engine, &task_id);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].description.contains("failure_reason"));
    }

    // NOTE: The following corruption tests (dangling references, bidirectional inconsistency,
    // cycles, status inconsistency) were removed during the JSON→SQLite migration.
    // SQLite's foreign keys and the engine's transaction-based operations prevent these
    // invalid states from occurring. The validator still checks for them as a defense-in-depth
    // measure, but they cannot be triggered through the normal API.

    #[test]
    fn empty_dag_no_issues() {
        let (_tmp, engine, task_id) = setup();
        let issues = validate_dag(&engine, &task_id);
        assert!(issues.is_empty());
    }
}
