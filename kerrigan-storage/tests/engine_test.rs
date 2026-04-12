use kerrigan_storage::*;
use std::thread;
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
fn init_deploys_skill() {
    let (tmp, engine) = setup();
    let skill_path = tmp
        .path()
        .join("skills")
        .join("kerrigan-storage")
        .join("SKILL.md");
    assert!(skill_path.exists(), "SKILL.md should be deployed on init");
    let content = std::fs::read_to_string(&skill_path).unwrap();
    assert!(content.contains("name: kerrigan-storage"));
}

#[test]
fn init_updates_skill_on_reinit() {
    let (tmp, engine) = setup();
    let skill_path = tmp
        .path()
        .join("skills")
        .join("kerrigan-storage")
        .join("SKILL.md");
    // Overwrite with garbage
    std::fs::write(&skill_path, "old content").unwrap();
    // Re-init should restore
    engine.init().unwrap();
    let content = std::fs::read_to_string(&skill_path).unwrap();
    assert!(content.contains("name: kerrigan-storage"));
}

#[test]
fn init_deploys_all_guide_files() {
    let (tmp, _engine) = setup();
    let shared = tmp.path().join("shared");
    let expected_files = [
        "guide.md",
        "guide-software-development.md",
        "guide-research.md",
        "guide-decision-ab-testing.md",
        "guide-git-management.md",
    ];
    for name in &expected_files {
        let path = shared.join(name);
        assert!(path.exists(), "{name} should be deployed on init");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.is_empty(), "{name} should not be empty");
    }
}

#[test]
fn init_updates_guide_on_reinit() {
    let (tmp, engine) = setup();
    let guide_path = tmp.path().join("shared").join("guide.md");
    std::fs::write(&guide_path, "old content").unwrap();
    engine.init().unwrap();
    let content = std::fs::read_to_string(&guide_path).unwrap();
    assert!(
        content.contains("Kerrigan"),
        "guide.md should be restored on reinit"
    );
}

#[test]
fn shared_dir_returns_correct_path() {
    let (tmp, engine) = setup();
    assert_eq!(engine.shared_dir(), tmp.path().join("shared"));
}

#[test]
fn task_create_does_not_create_per_task_shared() {
    let (tmp, engine) = setup();
    let id = engine.task_create("test", "/tmp").unwrap();
    let task_shared = tmp.path().join("tasks").join(&id).join("shared");
    assert!(
        !task_shared.exists(),
        "per-task shared/ should not be created"
    );
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
    // Global shared/ dir (not per-task) is created by init()
    assert!(tmp.path().join("shared").is_dir());
    assert!(tmp.path().join("shared").join("guide.md").exists());
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
        .add_node(
            &task_id,
            NewNode {
                summary: "a".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();
    let id2 = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "b".into(),
                description: "".into(),
            },
            vec![id1.clone()],
        )
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
        .add_node(
            &task_id,
            NewNode {
                summary: "A".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();
    let b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "B".into(),
                description: "".into(),
            },
            vec![a.clone()],
        )
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
        .add_node(
            &task_id,
            NewNode {
                summary: "A".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();
    let b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "B".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();
    let c = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "C".into(),
                description: "".into(),
            },
            vec![a.clone(), b.clone()],
        )
        .unwrap();

    // C is Pending
    assert_eq!(
        engine.get_node(&task_id, &c).unwrap().status,
        NodeStatus::Pending
    );

    // Complete A only → C still Pending
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: a.clone(),
                status: Some(NodeStatus::Succeeded),
                metadata: Some(NodeMetadata {
                    result_summary: Some("ok".into()),
                    ..Default::default()
                }),
            }],
        )
        .unwrap();
    assert_eq!(
        engine.get_node(&task_id, &c).unwrap().status,
        NodeStatus::Pending
    );

    // Complete B → C becomes Ready
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: b.clone(),
                status: Some(NodeStatus::Succeeded),
                metadata: Some(NodeMetadata {
                    result_summary: Some("ok".into()),
                    ..Default::default()
                }),
            }],
        )
        .unwrap();
    assert_eq!(
        engine.get_node(&task_id, &c).unwrap().status,
        NodeStatus::Ready
    );
}

#[test]
fn cascade_chain() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

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
    let b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "B".into(),
                description: "".into(),
            },
            vec![a.clone()],
        )
        .unwrap();
    let c = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "C".into(),
                description: "".into(),
            },
            vec![b.clone()],
        )
        .unwrap();
    let d = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "D".into(),
                description: "".into(),
            },
            vec![c.clone()],
        )
        .unwrap();

    // Complete A → B becomes Ready
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: a,
                status: Some(NodeStatus::Succeeded),
                metadata: Some(NodeMetadata {
                    result_summary: Some("ok".into()),
                    ..Default::default()
                }),
            }],
        )
        .unwrap();
    assert_eq!(
        engine.get_node(&task_id, &b).unwrap().status,
        NodeStatus::Ready
    );
    assert_eq!(
        engine.get_node(&task_id, &c).unwrap().status,
        NodeStatus::Pending
    );
    assert_eq!(
        engine.get_node(&task_id, &d).unwrap().status,
        NodeStatus::Pending
    );

    // Complete B → C becomes Ready
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: b,
                status: Some(NodeStatus::Succeeded),
                metadata: Some(NodeMetadata {
                    result_summary: Some("ok".into()),
                    ..Default::default()
                }),
            }],
        )
        .unwrap();
    assert_eq!(
        engine.get_node(&task_id, &c).unwrap().status,
        NodeStatus::Ready
    );
    assert_eq!(
        engine.get_node(&task_id, &d).unwrap().status,
        NodeStatus::Pending
    );

    // Complete C → D becomes Ready
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: c,
                status: Some(NodeStatus::Succeeded),
                metadata: Some(NodeMetadata {
                    result_summary: Some("ok".into()),
                    ..Default::default()
                }),
            }],
        )
        .unwrap();
    assert_eq!(
        engine.get_node(&task_id, &d).unwrap().status,
        NodeStatus::Ready
    );
}

// ── DAG: Failure does not propagate ──

#[test]
fn failure_no_propagation() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

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
    let b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "B".into(),
                description: "".into(),
            },
            vec![a.clone()],
        )
        .unwrap();

    // Fail A → B stays Pending forever
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: a,
                status: Some(NodeStatus::Failed),
                metadata: Some(NodeMetadata {
                    failure_reason: Some("broken".into()),
                    ..Default::default()
                }),
            }],
        )
        .unwrap();

    assert_eq!(
        engine.get_node(&task_id, &b).unwrap().status,
        NodeStatus::Pending
    );
}

// ── DAG: Batch update ──

#[test]
fn batch_update() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

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
    let b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "B".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();
    let c = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "C".into(),
                description: "".into(),
            },
            vec![a.clone(), b.clone()],
        )
        .unwrap();

    // Complete both A and B in one call → C should become Ready
    engine
        .update_nodes(
            &task_id,
            vec![
                NodeUpdate {
                    id: a,
                    status: Some(NodeStatus::Succeeded),
                    metadata: Some(NodeMetadata {
                        result_summary: Some("ok".into()),
                        ..Default::default()
                    }),
                },
                NodeUpdate {
                    id: b,
                    status: Some(NodeStatus::Succeeded),
                    metadata: Some(NodeMetadata {
                        result_summary: Some("ok".into()),
                        ..Default::default()
                    }),
                },
            ],
        )
        .unwrap();

    assert_eq!(
        engine.get_node(&task_id, &c).unwrap().status,
        NodeStatus::Ready
    );
}

// ── DAG: list_ready ──

#[test]
fn list_ready_filters() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

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
    let _b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "B".into(),
                description: "".into(),
            },
            vec![a.clone()],
        )
        .unwrap();

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
            NewNode {
                summary: "A".into(),
                description: "".into(),
            },
            vec!["ghost".into()],
        )
        .unwrap_err();
    assert!(matches!(err, StorageError::NodeNotFound(_)));
}

#[test]
fn invalid_state_transition() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();
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

    // Succeed A first
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: a.clone(),
                status: Some(NodeStatus::Succeeded),
                metadata: None,
            }],
        )
        .unwrap();

    // Try to transition Succeeded → Running (invalid)
    let err = engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: a,
                status: Some(NodeStatus::Running),
                metadata: None,
            }],
        )
        .unwrap_err();
    assert!(matches!(err, StorageError::InvalidStateTransition { .. }));
}

// ── Persistence roundtrip ──

#[test]
fn persistence_roundtrip() {
    let (tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();
    let a = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "A".into(),
                description: "desc".into(),
            },
            vec![],
        )
        .unwrap();
    let b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "B".into(),
                description: "".into(),
            },
            vec![a.clone()],
        )
        .unwrap();

    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: a.clone(),
                status: Some(NodeStatus::Succeeded),
                metadata: Some(NodeMetadata {
                    result_summary: Some("result".into()),
                    ..Default::default()
                }),
            }],
        )
        .unwrap();

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
    let b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "B".into(),
                description: "".into(),
            },
            vec![a.clone()],
        )
        .unwrap();

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
                id: a.clone(),
                status: Some(NodeStatus::Succeeded),
                metadata: None,
            }],
        )
        .unwrap();

    // Add B depending on already-succeeded A → B should be Ready immediately
    let b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "B".into(),
                description: "".into(),
            },
            vec![a],
        )
        .unwrap();
    assert_eq!(
        engine.get_node(&task_id, &b).unwrap().status,
        NodeStatus::Ready
    );
}

// ── Concurrent task_create via threads ──

#[test]
fn concurrent_task_create() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    let engine = StorageEngine::new(&root);
    engine.init().unwrap();

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let r = root.clone();
            thread::spawn(move || {
                let e = StorageEngine::new(&r);
                e.task_create(&format!("task-{i}"), "/tmp").unwrap()
            })
        })
        .collect();

    let ids: Vec<String> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    // All ids unique
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), 10);

    // Registry has all 10
    let tasks = engine.task_list().unwrap();
    assert_eq!(tasks.len(), 10);
}

// ── Init preserves existing tasks ──

#[test]
fn init_preserves_existing_data() {
    let (_tmp, engine) = setup();
    engine.task_create("existing task", "/tmp").unwrap();
    // Init again
    engine.init().unwrap();
    let tasks = engine.task_list().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].summary, "existing task");
}

// ── Empty DAG: list_all and list_ready return empty ──

#[test]
fn empty_dag() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();
    assert!(engine.list_all(&task_id).unwrap().is_empty());
    assert!(engine.list_ready(&task_id).unwrap().is_empty());
}

// ── Multiple independent ready nodes ──

#[test]
fn multiple_independent_ready_nodes() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

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
    let b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "B".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();
    let c = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "C".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();

    let ready = engine.list_ready(&task_id).unwrap();
    assert_eq!(ready.len(), 3);
    let ids: Vec<&str> = ready.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains(&a.as_str()));
    assert!(ids.contains(&b.as_str()));
    assert!(ids.contains(&c.as_str()));
}

// ── Running node is not Ready ──

#[test]
fn running_node_not_in_ready() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

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
                id: a.clone(),
                status: Some(NodeStatus::Running),
                metadata: None,
            }],
        )
        .unwrap();

    let ready = engine.list_ready(&task_id).unwrap();
    assert!(ready.is_empty());

    let node = engine.get_node(&task_id, &a).unwrap();
    assert_eq!(node.status, NodeStatus::Running);
}

// ── update_nodes: metadata-only update (no status change) ──

#[test]
fn metadata_only_update() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

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
                id: a.clone(),
                status: None,
                metadata: Some(NodeMetadata {
                    session_ids: vec!["sess_001".into()],
                    ..Default::default()
                }),
            }],
        )
        .unwrap();

    let node = engine.get_node(&task_id, &a).unwrap();
    assert_eq!(node.status, NodeStatus::Ready); // unchanged
    assert_eq!(node.metadata.session_ids, vec!["sess_001"]);
}

// ── Diamond DAG: A → B, A → C, B+C → D ──

#[test]
fn diamond_dag() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

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
    let b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "B".into(),
                description: "".into(),
            },
            vec![a.clone()],
        )
        .unwrap();
    let c = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "C".into(),
                description: "".into(),
            },
            vec![a.clone()],
        )
        .unwrap();
    let d = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "D".into(),
                description: "".into(),
            },
            vec![b.clone(), c.clone()],
        )
        .unwrap();

    // Only A is Ready
    assert_eq!(engine.list_ready(&task_id).unwrap().len(), 1);

    // Complete A → B and C become Ready
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
    let ready = engine.list_ready(&task_id).unwrap();
    assert_eq!(ready.len(), 2);
    assert_eq!(
        engine.get_node(&task_id, &d).unwrap().status,
        NodeStatus::Pending
    );

    // Complete B → D still Pending (C not done)
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: b,
                status: Some(NodeStatus::Succeeded),
                metadata: None,
            }],
        )
        .unwrap();
    assert_eq!(
        engine.get_node(&task_id, &d).unwrap().status,
        NodeStatus::Pending
    );

    // Complete C → D becomes Ready
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: c,
                status: Some(NodeStatus::Succeeded),
                metadata: None,
            }],
        )
        .unwrap();
    assert_eq!(
        engine.get_node(&task_id, &d).unwrap().status,
        NodeStatus::Ready
    );
}

// ── All state transitions ──

#[test]
fn valid_state_transitions() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

    // Ready → Running → Succeeded
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
                id: a.clone(),
                status: Some(NodeStatus::Running),
                metadata: None,
            }],
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

    // Ready → Running → Failed
    let b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "B".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: b.clone(),
                status: Some(NodeStatus::Running),
                metadata: None,
            }],
        )
        .unwrap();
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: b,
                status: Some(NodeStatus::Failed),
                metadata: None,
            }],
        )
        .unwrap();

    // Ready → Succeeded (direct)
    let c = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "C".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: c,
                status: Some(NodeStatus::Succeeded),
                metadata: None,
            }],
        )
        .unwrap();

    // Ready → Failed (direct)
    let d = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "D".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: d,
                status: Some(NodeStatus::Failed),
                metadata: None,
            }],
        )
        .unwrap();
}

#[test]
fn invalid_transitions() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

    // Pending → Running (invalid, must be Ready first)
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
    let b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "B".into(),
                description: "".into(),
            },
            vec![a.clone()],
        )
        .unwrap();
    assert_eq!(
        engine.get_node(&task_id, &b).unwrap().status,
        NodeStatus::Pending
    );
    let err = engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: b,
                status: Some(NodeStatus::Running),
                metadata: None,
            }],
        )
        .unwrap_err();
    assert!(matches!(err, StorageError::InvalidStateTransition { .. }));

    // Succeeded → anything (invalid)
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: a.clone(),
                status: Some(NodeStatus::Succeeded),
                metadata: None,
            }],
        )
        .unwrap();
    let err = engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: a.clone(),
                status: Some(NodeStatus::Failed),
                metadata: None,
            }],
        )
        .unwrap_err();
    assert!(matches!(err, StorageError::InvalidStateTransition { .. }));

    // Failed → anything (invalid)
    let c = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "C".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: c.clone(),
                status: Some(NodeStatus::Failed),
                metadata: None,
            }],
        )
        .unwrap();
    let err = engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: c,
                status: Some(NodeStatus::Ready),
                metadata: None,
            }],
        )
        .unwrap_err();
    assert!(matches!(err, StorageError::InvalidStateTransition { .. }));
}

// ── NodeSummary has correct fields ──

#[test]
fn node_summary_fields() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

    let a = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "do A".into(),
                description: "long desc".into(),
            },
            vec![],
        )
        .unwrap();
    let b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "do B".into(),
                description: "".into(),
            },
            vec![a.clone()],
        )
        .unwrap();

    let all = engine.list_all(&task_id).unwrap();
    let a_summary = all.iter().find(|n| n.id == a).unwrap();
    assert_eq!(a_summary.summary, "do A");
    assert_eq!(a_summary.successors, vec![b.clone()]);
    assert_eq!(a_summary.status, NodeStatus::Ready);

    let b_summary = all.iter().find(|n| n.id == b).unwrap();
    assert_eq!(b_summary.summary, "do B");
    assert!(b_summary.successors.is_empty());
    assert_eq!(b_summary.status, NodeStatus::Pending);
}

// ── Spawned tasks in metadata ──

#[test]
fn spawned_tasks_metadata() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();

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

    // Fail A with spawned retry task info
    let retry = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "retry A".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: a.clone(),
                status: Some(NodeStatus::Failed),
                metadata: Some(NodeMetadata {
                    failure_reason: Some("timeout".into()),
                    spawned_tasks: vec![retry.clone()],
                    ..Default::default()
                }),
            }],
        )
        .unwrap();

    let node = engine.get_node(&task_id, &a).unwrap();
    assert_eq!(node.metadata.failure_reason.as_deref(), Some("timeout"));
    assert_eq!(node.metadata.spawned_tasks, vec![retry]);
}

// ── Persistence roundtrip: full DAG state survives reload ──

#[test]
fn persistence_full_dag_state() {
    let (tmp, engine) = setup();
    let task_id = engine.task_create("project", "/work").unwrap();

    let a = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "A".into(),
                description: "desc A".into(),
            },
            vec![],
        )
        .unwrap();
    let b = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "B".into(),
                description: "desc B".into(),
            },
            vec![a.clone()],
        )
        .unwrap();
    let c = engine
        .add_node(
            &task_id,
            NewNode {
                summary: "C".into(),
                description: "desc C".into(),
            },
            vec![a.clone()],
        )
        .unwrap();

    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: a.clone(),
                status: Some(NodeStatus::Running),
                metadata: None,
            }],
        )
        .unwrap();
    engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: a.clone(),
                status: Some(NodeStatus::Succeeded),
                metadata: Some(NodeMetadata {
                    result_summary: Some("A done".into()),
                    session_ids: vec!["sess_1".into()],
                    ..Default::default()
                }),
            }],
        )
        .unwrap();

    // Reload from disk
    let engine2 = StorageEngine::new(tmp.path());

    // Task registry intact
    let tasks = engine2.task_list().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].summary, "project");

    // Node A state
    let na = engine2.get_node(&task_id, &a).unwrap();
    assert_eq!(na.status, NodeStatus::Succeeded);
    assert_eq!(na.metadata.result_summary.as_deref(), Some("A done"));
    assert_eq!(na.metadata.session_ids, vec!["sess_1"]);
    assert_eq!(na.successors.len(), 2);

    // Nodes B and C should be Ready (cascaded after A succeeded)
    assert_eq!(
        engine2.get_node(&task_id, &b).unwrap().status,
        NodeStatus::Ready
    );
    assert_eq!(
        engine2.get_node(&task_id, &c).unwrap().status,
        NodeStatus::Ready
    );

    // list_ready should return B and C
    let ready = engine2.list_ready(&task_id).unwrap();
    assert_eq!(ready.len(), 2);
}

// ── update_nodes on nonexistent node ──

#[test]
fn update_nonexistent_node() {
    let (_tmp, engine) = setup();
    let task_id = engine.task_create("t", "/tmp").unwrap();
    let err = engine
        .update_nodes(
            &task_id,
            vec![NodeUpdate {
                id: "ghost".into(),
                status: Some(NodeStatus::Running),
                metadata: None,
            }],
        )
        .unwrap_err();
    assert!(matches!(err, StorageError::NodeNotFound(_)));
}

// ── Cross-task isolation: nodes in one task don't affect another ──

#[test]
fn cross_task_isolation() {
    let (_tmp, engine) = setup();
    let t1 = engine.task_create("task 1", "/tmp").unwrap();
    let t2 = engine.task_create("task 2", "/tmp").unwrap();

    engine
        .add_node(
            &t1,
            NewNode {
                summary: "A in T1".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();
    engine
        .add_node(
            &t1,
            NewNode {
                summary: "B in T1".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();
    engine
        .add_node(
            &t2,
            NewNode {
                summary: "X in T2".into(),
                description: "".into(),
            },
            vec![],
        )
        .unwrap();

    assert_eq!(engine.list_all(&t1).unwrap().len(), 2);
    assert_eq!(engine.list_all(&t2).unwrap().len(), 1);
}
