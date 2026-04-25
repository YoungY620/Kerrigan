use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

// ── Helper ────────────────────────────────────────────────────────────────────

fn kerrigan(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("kerrigan").unwrap();
    cmd.env("HOME", home.path());
    cmd
}

/// Run init, create a task, return (home, task_id).
fn setup() -> (TempDir, String) {
    let home = tempfile::tempdir().unwrap();
    kerrigan(&home).args(["init"]).assert().success();
    let out = kerrigan(&home)
        .args(["task", "create", "--summary", "test-task", "--workdir", "/tmp"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let task_id = json["data"]["task_id"].as_str().unwrap().to_string();
    (home, task_id)
}

/// Add a node to a task, return node_id.
fn add_node(home: &TempDir, task: &str, summary: &str, args: &[&str]) -> String {
    let mut cmd_args = vec!["dag", "add", "--task", task, "--summary", summary];
    cmd_args.extend_from_slice(args);
    let out = kerrigan(home).args(&cmd_args).output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    json["data"]["node_id"].as_str().unwrap().to_string()
}

fn update_node(home: &TempDir, task: &str, node: &str, status: &str) {
    let input = format!("[{{\"id\":\"{node}\",\"status\":\"{status}\"}}]");
    kerrigan(home)
        .args(["dag", "update", "--task", task, "--input", &input])
        .assert()
        .success();
}

// ── init ──────────────────────────────────────────────────────────────────────

#[test]
fn init_succeeds() {
    let home = tempfile::tempdir().unwrap();
    kerrigan(&home)
        .args(["init"])
        .assert()
        .success()
        .stdout(contains("ok"));
}

#[test]
fn init_is_idempotent() {
    let home = tempfile::tempdir().unwrap();
    kerrigan(&home).args(["init"]).assert().success();
    kerrigan(&home).args(["init"]).assert().success();
}

#[test]
fn init_deploys_skill_md() {
    let home = tempfile::tempdir().unwrap();
    kerrigan(&home).args(["init"]).assert().success();
    assert!(home.path().join(".kerrigan/skills/kerrigan/SKILL.md").exists());
}

// ── task ──────────────────────────────────────────────────────────────────────

#[test]
fn task_create_returns_task_id() {
    let home = tempfile::tempdir().unwrap();
    kerrigan(&home).args(["init"]).assert().success();
    kerrigan(&home)
        .args(["task", "create", "--summary", "hello", "--workdir", "/tmp"])
        .assert()
        .success()
        .stdout(contains("task_id"));
}

#[test]
fn task_list_empty() {
    let home = tempfile::tempdir().unwrap();
    kerrigan(&home).args(["init"]).assert().success();
    kerrigan(&home)
        .args(["task", "list"])
        .assert()
        .success()
        .stdout(contains("[]"));
}

#[test]
fn task_list_shows_created_task() {
    let (home, _task_id) = setup();
    kerrigan(&home)
        .args(["task", "list"])
        .assert()
        .success()
        .stdout(contains("test-task"));
}

// ── dag add ───────────────────────────────────────────────────────────────────

#[test]
fn dag_add_returns_node_id() {
    let (home, task) = setup();
    kerrigan(&home)
        .args(["dag", "add", "--task", &task, "--summary", "do something"])
        .assert()
        .success()
        .stdout(contains("node_id"));
}

#[test]
fn dag_add_with_description() {
    let (home, task) = setup();
    kerrigan(&home)
        .args([
            "dag", "add", "--task", &task,
            "--summary", "analyze",
            "--description", "Read all files carefully",
        ])
        .assert()
        .success();
}

#[test]
fn dag_add_depends_on_not_found() {
    let (home, task) = setup();
    kerrigan(&home)
        .args([
            "dag", "add", "--task", &task,
            "--summary", "n", "--depends-on", "bad-node-id",
        ])
        .assert()
        .failure()
        .stderr(contains("not found"));
}

#[test]
fn dag_add_successor_not_found() {
    let (home, task) = setup();
    kerrigan(&home)
        .args([
            "dag", "add", "--task", &task,
            "--summary", "n", "--successor", "bad-node-id",
        ])
        .assert()
        .failure()
        .stderr(contains("not found"));
}

#[test]
fn dag_add_cycle_detected() {
    let (home, task) = setup();
    let a = add_node(&home, &task, "A", &[]);
    let b = add_node(&home, &task, "B", &["--depends-on", &a]);
    // Try: C depends on B, makes A its successor → B→C→A→B cycle
    kerrigan(&home)
        .args([
            "dag", "add", "--task", &task,
            "--summary", "C",
            "--depends-on", &b,
            "--successor", &a,
        ])
        .assert()
        .failure()
        .stderr(contains("cycle"));
}

#[test]
fn dag_add_same_node_as_dep_and_successor_is_cycle() {
    let (home, task) = setup();
    let a = add_node(&home, &task, "A", &[]);
    kerrigan(&home)
        .args([
            "dag", "add", "--task", &task,
            "--summary", "N",
            "--depends-on", &a,
            "--successor", &a,
        ])
        .assert()
        .failure()
        .stderr(contains("cycle"));
}

#[test]
fn dag_add_with_successor_creates_edge() {
    let (home, task) = setup();
    let s = add_node(&home, &task, "S", &[]);
    let n = add_node(&home, &task, "N", &["--successor", &s]);

    // N's successors should include S
    let out = kerrigan(&home)
        .args(["dag", "get", "--task", &task, "--node", &n])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let succs: Vec<&str> = json["data"]["successors"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(succs.contains(&s.as_str()));
}

// ── dag list / ready / get ────────────────────────────────────────────────────

#[test]
fn dag_list_shows_nodes() {
    let (home, task) = setup();
    add_node(&home, &task, "node-one", &[]);
    kerrigan(&home)
        .args(["dag", "list", "--task", &task])
        .assert()
        .success()
        .stdout(contains("node-one"));
}

#[test]
fn dag_ready_shows_root_nodes() {
    let (home, task) = setup();
    let a = add_node(&home, &task, "A", &[]);
    let b = add_node(&home, &task, "B", &["--depends-on", &a]); // pending
    let c = add_node(&home, &task, "C", &[]); // ready

    let out = kerrigan(&home)
        .args(["dag", "ready", "--task", &task])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    // Parse actual node IDs from the ready list (not string-contains, to avoid UUID collisions)
    let ready_ids: Vec<&str> = json["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["id"].as_str())
        .collect();
    assert!(ready_ids.contains(&a.as_str()), "A should be ready");
    assert!(ready_ids.contains(&c.as_str()), "C should be ready");
    assert!(!ready_ids.contains(&b.as_str()), "B should not be ready");
}

#[test]
fn dag_get_includes_predecessor_results() {
    let (home, task) = setup();
    let a = add_node(&home, &task, "A", &[]);
    let b = add_node(&home, &task, "B", &["--depends-on", &a]);

    let input = format!(
        "[{{\"id\":\"{a}\",\"status\":\"succeeded\",\"metadata\":{{\"result_summary\":\"A output\"}}}}]"
    );
    kerrigan(&home)
        .args(["dag", "update", "--task", &task, "--input", &input])
        .assert()
        .success();

    // B can now get A's result via dag get
    let out = kerrigan(&home)
        .args(["dag", "get", "--task", &task, "--node", &b])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let preds = json["data"]["predecessors"].as_array().unwrap();
    assert!(preds.iter().any(|p| p.as_str() == Some(&a)));
}

// ── dag update ────────────────────────────────────────────────────────────────

#[test]
fn dag_update_succeeded_cascades_ready() {
    let (home, task) = setup();
    let a = add_node(&home, &task, "A", &[]);
    let b = add_node(&home, &task, "B", &["--depends-on", &a]);

    // B is pending initially — parse ids to avoid UUID appearing in successors list
    let ready_ids_before: Vec<String> = {
        let out = kerrigan(&home)
            .args(["dag", "ready", "--task", &task])
            .output()
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        json["data"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|n| n["id"].as_str().map(String::from))
            .collect()
    };
    assert!(!ready_ids_before.contains(&b), "B should start as pending");

    // Succeed A → B becomes ready
    update_node(&home, &task, &a, "succeeded");

    let ready_ids_after: Vec<String> = {
        let out2 = kerrigan(&home)
            .args(["dag", "ready", "--task", &task])
            .output()
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&out2.stdout).unwrap();
        json["data"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|n| n["id"].as_str().map(String::from))
            .collect()
    };
    assert!(ready_ids_after.contains(&b), "B should be ready after A succeeds");
}

#[test]
fn dag_update_invalid_transition_fails() {
    let (home, task) = setup();
    let n = add_node(&home, &task, "N", &[]);
    update_node(&home, &task, &n, "succeeded");

    // succeeded → running is invalid
    let input = format!("[{{\"id\":\"{n}\",\"status\":\"running\"}}]");
    kerrigan(&home)
        .args(["dag", "update", "--task", &task, "--input", &input])
        .assert()
        .failure()
        .stderr(contains("invalid state transition"));
}

#[test]
fn dag_update_reads_from_stdin() {
    let (home, task) = setup();
    let n = add_node(&home, &task, "N", &[]);
    let input = format!("[{{\"id\":\"{n}\",\"status\":\"succeeded\"}}]");

    kerrigan(&home)
        .args(["dag", "update", "--task", &task])
        .write_stdin(input)
        .assert()
        .success();

    let out = kerrigan(&home)
        .args(["dag", "get", "--task", &task, "--node", &n])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["data"]["status"].as_str(), Some("succeeded"));
}

#[test]
fn dag_update_cascade_pending_after_successor_added() {
    let (home, task) = setup();
    let dep = add_node(&home, &task, "dep", &[]);     // ready
    let s = add_node(&home, &task, "S", &[]);          // ready
    add_node(&home, &task, "N", &["--depends-on", &dep, "--successor", &s]);

    // S should now be pending (N is pending because dep not succeeded)
    let out = kerrigan(&home)
        .args(["dag", "get", "--task", &task, "--node", &s])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["data"]["status"].as_str(), Some("pending"));
}

// ── status ────────────────────────────────────────────────────────────────────

#[test]
fn status_no_args_lists_tasks() {
    let (home, _) = setup();
    kerrigan(&home)
        .args(["status"])
        .assert()
        .success()
        .stdout(contains("test-task"));
}

#[test]
fn status_with_task_id_shows_nodes() {
    let (home, task) = setup();
    add_node(&home, &task, "my-node", &[]);
    kerrigan(&home)
        .args(["status", &task])
        .assert()
        .success()
        .stdout(contains("my-node"));
}

// ── output format ─────────────────────────────────────────────────────────────

#[test]
fn output_has_data_key_no_reminder() {
    let (home, task) = setup();
    let out = kerrigan(&home)
        .args(["dag", "list", "--task", &task])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(json.get("data").is_some());
    assert!(json.get("reminder").is_none()); // removed
}

#[test]
fn error_goes_to_stderr_as_json() {
    let home = tempfile::tempdir().unwrap();
    kerrigan(&home).args(["init"]).assert().success();
    let err_out = kerrigan(&home)
        .args(["dag", "get", "--task", "nonexistent", "--node", "nonexistent"])
        .output()
        .unwrap();
    // Error output may be on stderr (JSON) or combined; check stderr first then stdout
    let stderr = String::from_utf8_lossy(&err_out.stderr);
    let to_parse = stderr.trim();
    assert!(!to_parse.is_empty(), "stderr should not be empty on error");
    let json: serde_json::Value =
        serde_json::from_str(to_parse).expect("stderr should be valid JSON");
    assert!(json.get("error").is_some(), "error key should be present");
}
