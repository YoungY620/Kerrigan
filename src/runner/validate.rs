use kerrigan_storage::{NodeStatus, StorageEngine};

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
/// - Nodes with no predecessors should be Ready (not Pending)
/// - Ready nodes should have no incomplete predecessors
/// - Succeeded nodes should have result_summary
/// - Failed nodes should have failure_reason
/// - All nodes should have non-empty summary
pub fn validate_dag(engine: &StorageEngine, task_id: &str) -> Vec<DagIssue> {
    let mut issues = Vec::new();

    let nodes = match engine.list_all(task_id) {
        Ok(n) => n,
        Err(_) => return issues,
    };

    for summary in &nodes {
        let node = match engine.get_node(task_id, &summary.id) {
            Ok(n) => n,
            Err(_) => continue,
        };

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

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use kerrigan_storage::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, StorageEngine, String) {
        let tmp = TempDir::new().unwrap();
        let engine = StorageEngine::new(tmp.path());
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
}
