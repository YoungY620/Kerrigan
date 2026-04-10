use kerrigan_storage::*;
use tempfile::TempDir;

fn setup() -> (TempDir, StorageEngine) {
    let tmp = TempDir::new().unwrap();
    let engine = StorageEngine::new(tmp.path());
    engine.init().unwrap();
    (tmp, engine)
}

// ── Init ──

#[test]
fn init_idempotent() {
    let (_tmp, engine) = setup();
    // Second init should not fail or corrupt data
    engine.init().unwrap();
    let tasks = engine.task_list().unwrap();
    assert!(tasks.is_empty());
}

#[test]
fn not_initialized_error() {
    let tmp = TempDir::new().unwrap();
    let engine = StorageEngine::new(tmp.path());
    // Should fail before init
    let err = engine.task_list().unwrap_err();
    assert!(matches!(err, StorageError::NotInitialized));
}

// ── Task management ──

#[test]
fn task_create_and_list() {
    let (_tmp, engine) = setup();
    let id = engine.task_create("test task", "/tmp/work").unwrap();
    let tasks = engine.task_list().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, id);
    assert_eq!(tasks[0].summary, "test task");
    assert_eq!(tasks[0].workdir, "/tmp/work");
}

#[test]
fn task_create_multiple() {
    let (_tmp, engine) = setup();
    let id1 = engine.task_create("task 1", "/tmp/a").unwrap();
    let id2 = engine.task_create("task 2", "/tmp/b").unwrap();
    assert_ne!(id1, id2);
    let tasks = engine.task_list().unwrap();
    assert_eq!(tasks.len(), 2);
}

#[test]
fn task_directory_structure() {
    let (tmp, engine) = setup();
    let id = engine.task_create("test", "/tmp").unwrap();
    let dir = tmp.path().join("tasks").join(&id);
    assert!(dir.join("dag.json").exists());
    assert!(dir.join("config.json").exists());
    assert!(dir.join("sessions.json").exists());
    assert!(dir.join("board.md").exists());
    assert!(dir.join("shared").is_dir());
}

// ── DAG: Basic CRUD ──

#[test]
fn add_and_get_node() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

    let node_id = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "do something".into(),
                description: "detailed desc".into(),
            },
            vec![],
        )
        .unwrap();

    let node = engine.get_node(&task_id, &node_id).unwrap();
    assert_eq!(node.summary, "do something");
    assert_eq!(node.description, "detailed desc");
    assert_eq!(node.status, NodeStatus::Ready); // no predecessors → Ready
    assert!(node.predecessors.is_empty());
    assert!(node.successors.is_empty());
}

#[test]
fn list_all_consistency() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

    let id1 = engine
        .add_node(&task_id, NewNode { summary: "a".into(), description: "".into() }, vec![])
        .unwrap();
    let id2 = engine
        .add_node(&task_id, NewNode { summary: "b".into(), description: "".into() }, vec![id1.clone()])
        .unwrap();

    let all = engine.list_all(&task_id).unwrap();
    assert_eq!(all.len(), 2);

    let ids: Vec<&str> = all.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains(&id1.as_str()));
    assert!(ids.contains(&id2.as_str()));
}

// ── DAG: Dependency resolution ──

#[test]
fn dependency_pending_then_ready() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

    let a = engine
        .add_node(&task_id, NewNode { summary: "A".into(), description: "".into() }, vec![])
        .unwrap();
    let b = engine
        .add_node(&task_id, NewNode { summary: "B".into(), description: "".into() }, vec![a.clone()])
        .unwrap();

    // B should be Pending (A not done yet)
    let b_node = engine.get_node(&task_id, &b).unwrap();
    assert_eq!(b_node.status, NodeStatus::Pending);

    // Complete A
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: a.clone(),
                status: Some(NodeStatus::Succeeded),
                metadata: Some(NodeMetadata {
                    result_summary: Some("done".into()),
                    ..Default::default()
                }),
            }],
        )
        .unwrap();

    // B should now be Ready (cascade)
    let b_node = engine.get_node(&task_id, &b).unwrap();
    assert_eq!(b_node.status, NodeStatus::Ready);
}

#[test]
fn multi_predecessor() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

    let a = engine
        .add_node(&task_id, NewNode { summary: "A".into(), description: "".into() }, vec![])
        .unwrap();
    let b = engine
        .add_node(&task_id, NewNode { summary: "B".into(), description: "".into() }, vec![])
        .unwrap();
    let c = engine
        .add_node(&task_id, NewNode { summary: "C".into(), description: "".into() }, vec![a.clone(), b.clone()])
        .unwrap();

    // C is Pending
    assert_eq!(engine.get_node(&task_id, &c).unwrap().status, NodeStatus::Pending);

    // Complete A only → C still Pending
    engine.update_nodes(&task_id, vec![NodeUpdate {
        id: a.clone(), status: Some(NodeStatus::Succeeded),
        metadata: Some(NodeMetadata { result_summary: Some("ok".into()), ..Default::default() }),
    }]).unwrap();
    assert_eq!(engine.get_node(&task_id, &c).unwrap().status, NodeStatus::Pending);

    // Complete B → C becomes Ready
    engine.update_nodes(&task_id, vec![NodeUpdate {
        id: b.clone(), status: Some(NodeStatus::Succeeded),
        metadata: Some(NodeMetadata { result_summary: Some("ok".into()), ..Default::default() }),
    }]).unwrap();
    assert_eq!(engine.get_node(&task_id, &c).unwrap().status, NodeStatus::Ready);
}

#[test]
fn cascade_chain() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

    let a = engine.add_node(&task_id, NewNode { summary: "A".into(), description: "".into() }, vec![]).unwrap();
    let b = engine.add_node(&task_id, NewNode { summary: "B".into(), description: "".into() }, vec![a.clone()]).unwrap();
    let c = engine.add_node(&task_id, NewNode { summary: "C".into(), description: "".into() }, vec![b.clone()]).unwrap();
    let d = engine.add_node(&task_id, NewNode { summary: "D".into(), description: "".into() }, vec![c.clone()]).unwrap();

    // Complete A → B becomes Ready
    engine.update_nodes(&task_id, vec![NodeUpdate {
        id: a, status: Some(NodeStatus::Succeeded),
        metadata: Some(NodeMetadata { result_summary: Some("ok".into()), ..Default::default() }),
    }]).unwrap();
    assert_eq!(engine.get_node(&task_id, &b).unwrap().status, NodeStatus::Ready);
    assert_eq!(engine.get_node(&task_id, &c).unwrap().status, NodeStatus::Pending);
    assert_eq!(engine.get_node(&task_id, &d).unwrap().status, NodeStatus::Pending);

    // Complete B → C becomes Ready
    engine.update_nodes(&task_id, vec![NodeUpdate {
        id: b, status: Some(NodeStatus::Succeeded),
        metadata: Some(NodeMetadata { result_summary: Some("ok".into()), ..Default::default() }),
    }]).unwrap();
    assert_eq!(engine.get_node(&task_id, &c).unwrap().status, NodeStatus::Ready);
    assert_eq!(engine.get_node(&task_id, &d).unwrap().status, NodeStatus::Pending);

    // Complete C → D becomes Ready
    engine.update_nodes(&task_id, vec![NodeUpdate {
        id: c, status: Some(NodeStatus::Succeeded),
        metadata: Some(NodeMetadata { result_summary: Some("ok".into()), ..Default::default() }),
    }]).unwrap();
    assert_eq!(engine.get_node(&task_id, &d).unwrap().status, NodeStatus::Ready);
}

// ── DAG: Failure does not propagate ──

#[test]
fn failure_no_propagation() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

    let a = engine.add_node(&task_id, NewNode { summary: "A".into(), description: "".into() }, vec![]).unwrap();
    let b = engine.add_node(&task_id, NewNode { summary: "B".into(), description: "".into() }, vec![a.clone()]).unwrap();

    // Fail A → B stays Pending forever
    engine.update_nodes(&task_id, vec![NodeUpdate {
        id: a, status: Some(NodeStatus::Failed),
        metadata: Some(NodeMetadata { failure_reason: Some("broken".into()), ..Default::default() }),
    }]).unwrap();

    assert_eq!(engine.get_node(&task_id, &b).unwrap().status, NodeStatus::Pending);
}

// ── DAG: Batch update ──

#[test]
fn batch_update() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

    let a = engine.add_node(&task_id, NewNode { summary: "A".into(), description: "".into() }, vec![]).unwrap();
    let b = engine.add_node(&task_id, NewNode { summary: "B".into(), description: "".into() }, vec![]).unwrap();
    let c = engine.add_node(&task_id, NewNode { summary: "C".into(), description: "".into() }, vec![a.clone(), b.clone()]).unwrap();

    // Complete both A and B in one call → C should become Ready
    engine.update_nodes(&task_id, vec![
        NodeUpdate {
            id: a, status: Some(NodeStatus::Succeeded),
            metadata: Some(NodeMetadata { result_summary: Some("ok".into()), ..Default::default() }),
        },
        NodeUpdate {
            id: b, status: Some(NodeStatus::Succeeded),
            metadata: Some(NodeMetadata { result_summary: Some("ok".into()), ..Default::default() }),
        },
    ]).unwrap();

    assert_eq!(engine.get_node(&task_id, &c).unwrap().status, NodeStatus::Ready);
}

// ── DAG: list_ready ──

#[test]
fn list_ready_filters() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

    let a = engine.add_node(&task_id, NewNode { summary: "A".into(), description: "".into() }, vec![]).unwrap();
    let _b = engine.add_node(&task_id, NewNode { summary: "B".into(), description: "".into() }, vec![a.clone()]).unwrap();

    let ready = engine.list_ready(&task_id).unwrap();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, a);
}

// ── DAG: Error cases ──

#[test]
fn task_not_found() {
    let (_tmp, engine) = setup();
    let err = engine.list_all("nonexistent").unwrap_err();
    assert!(matches!(err, StorageError::TaskNotFound(_)));
}

#[test]
fn node_not_found() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();
    let err = engine.get_node(&task_id, "nonexistent").unwrap_err();
    assert!(matches!(err, StorageError::NodeNotFound(_)));
}

#[test]
fn depends_on_nonexistent_node() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();
    let err = engine
        .add_node(
            &task_id,
            NewNode { summary: "A".into(), description: "".into() },
            vec!["ghost".into()],
        )
        .unwrap_err();
    assert!(matches!(err, StorageError::NodeNotFound(_)));
}

#[test]
fn invalid_state_transition() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();
    let a = engine.add_node(&task_id, NewNode { summary: "A".into(), description: "".into() }, vec![]).unwrap();

    // Succeed A first
    engine.update_nodes(&task_id, vec![NodeUpdate {
        id: a.clone(), status: Some(NodeStatus::Succeeded), metadata: None,
    }]).unwrap();

    // Try to transition Succeeded → Running (invalid)
    let err = engine.update_nodes(&task_id, vec![NodeUpdate {
        id: a, status: Some(NodeStatus::Running), metadata: None,
    }]).unwrap_err();
    assert!(matches!(err, StorageError::InvalidStateTransition { .. }));
}

// ── Persistence roundtrip ──

#[test]
fn persistence_roundtrip() {
    let (tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();
    let a = engine.add_node(&task_id, NewNode { summary: "A".into(), description: "desc".into() }, vec![]).unwrap();
    let b = engine.add_node(&task_id, NewNode { summary: "B".into(), description: "".into() }, vec![a.clone()]).unwrap();

    engine.update_nodes(&task_id, vec![NodeUpdate {
        id: a.clone(), status: Some(NodeStatus::Succeeded),
        metadata: Some(NodeMetadata { result_summary: Some("result".into()), ..Default::default() }),
    }]).unwrap();

    // Create a new engine instance pointing to the same directory
    let engine2 = StorageEngine::new(tmp.path());
    let node_a = engine2.get_node(&task_id, &a).unwrap();
    assert_eq!(node_a.status, NodeStatus::Succeeded);
    assert_eq!(node_a.metadata.result_summary.as_deref(), Some("result"));

    let node_b = engine2.get_node(&task_id, &b).unwrap();
    assert_eq!(node_b.status, NodeStatus::Ready); // cascaded
}

// ── Predecessor / Successor bidirectional ──

#[test]
fn bidirectional_edges() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

    let a = engine.add_node(&task_id, NewNode { summary: "A".into(), description: "".into() }, vec![]).unwrap();
    let b = engine.add_node(&task_id, NewNode { summary: "B".into(), description: "".into() }, vec![a.clone()]).unwrap();

    let a_node = engine.get_node(&task_id, &a).unwrap();
    assert!(a_node.successors.contains(&b));

    let b_node = engine.get_node(&task_id, &b).unwrap();
    assert!(b_node.predecessors.contains(&a));
}

// ── Add node to already-completed predecessors → Ready immediately ──

#[test]
fn add_node_with_completed_predecessors() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

    let a = engine.add_node(&task_id, NewNode { summary: "A".into(), description: "".into() }, vec![]).unwrap();
    engine.update_nodes(&task_id, vec![NodeUpdate {
        id: a.clone(), status: Some(NodeStatus::Succeeded), metadata: None,
    }]).unwrap();

    // Add B depending on already-succeeded A → B should be Ready immediately
    let b = engine.add_node(&task_id, NewNode { summary: "B".into(), description: "".into() }, vec![a]).unwrap();
    assert_eq!(engine.get_node(&task_id, &b).unwrap().status, NodeStatus::Ready);
}
