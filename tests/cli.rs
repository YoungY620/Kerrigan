use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

// ── Helper ────────────────────────────────────────────────────────────────────

/// Minimal valid contract with no predecessors and a simple output schema.
/// Declares "result" as required so subsumption checks pass when this node
/// is used as a predecessor with contract_with_pred().
const CONTRACT_ROOT: &str =
    r#"{"output":{"type":"object","required":["result"],"properties":{"result":{"type":"string"}}}}"#;

/// Minimal valid contract for a node with one predecessor: requires the predecessor
/// to have a "result" string field; promises its own "result" field.
fn contract_with_pred(pred_id: &str) -> String {
    format!(
        r#"{{"predecessor":{{"{pred_id}":{{"type":"object","required":["result"],"properties":{{"result":{{"type":"string"}}}}}}}},"output":{{"type":"object","required":["result"],"properties":{{"result":{{"type":"string"}}}}}}}}"#
    )
}

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
/// If args does not include --contract, CONTRACT_ROOT is appended automatically.
fn add_node(home: &TempDir, task: &str, summary: &str, args: &[&str]) -> String {
    let has_contract = args.iter().any(|a| *a == "--contract");
    // Build full arg list; keep default_contract alive for the lifetime of cmd_args
    let default_contract = CONTRACT_ROOT.to_string();
    let mut cmd_args = vec!["dag", "add", "--task", task, "--summary", summary];
    cmd_args.extend_from_slice(args);
    if !has_contract {
        cmd_args.extend_from_slice(&["--contract", &default_contract]);
    }
    let out = kerrigan(home).args(&cmd_args).output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    json["data"]["node_id"].as_str().unwrap().to_string()
}

fn update_node(home: &TempDir, task: &str, node: &str, status: &str) {
    // When marking succeeded, provide a valid JSON result_summary matching CONTRACT_ROOT
    let input = if status == "succeeded" {
        format!(
            "[{{\"id\":\"{node}\",\"status\":\"succeeded\",\"metadata\":{{\"result_summary\":\"{{\\\"result\\\":\\\"ok\\\"}}\"}}}}]"
        )
    } else {
        format!("[{{\"id\":\"{node}\",\"status\":\"{status}\"}}]")
    };
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
        .args(["dag", "add", "--task", &task, "--summary", "do something",
               "--contract", CONTRACT_ROOT])
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
            "--contract", CONTRACT_ROOT,
        ])
        .assert()
        .success();
}

#[test]
fn dag_add_depends_on_not_found() {
    let (home, task) = setup();
    // Supply a contract that covers the bad-node-id predecessor (bijection OK),
    // so the contract passes and the node-existence check fires → not found
    let contract = contract_with_pred("bad-node-id");
    kerrigan(&home)
        .args([
            "dag", "add", "--task", &task,
            "--summary", "n", "--depends-on", "bad-node-id",
            "--contract", &contract,
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
            "--contract", CONTRACT_ROOT,
        ])
        .assert()
        .failure()
        .stderr(contains("not found"));
}

#[test]
fn dag_add_cycle_detected() {
    let (home, task) = setup();
    let a = add_node(&home, &task, "A", &[]);
    let b_contract = contract_with_pred(&a);
    let b = add_node(&home, &task, "B", &["--depends-on", &a, "--contract", &b_contract]);
    // Try: C depends on B, makes A its successor → B→C→A→B cycle
    let c_contract = contract_with_pred(&b);
    kerrigan(&home)
        .args([
            "dag", "add", "--task", &task,
            "--summary", "C",
            "--depends-on", &b,
            "--successor", &a,
            "--contract", &c_contract,
        ])
        .assert()
        .failure()
        .stderr(contains("cycle"));
}

#[test]
fn dag_add_same_node_as_dep_and_successor_is_cycle() {
    let (home, task) = setup();
    let a = add_node(&home, &task, "A", &[]);
    let n_contract = contract_with_pred(&a);
    kerrigan(&home)
        .args([
            "dag", "add", "--task", &task,
            "--summary", "N",
            "--depends-on", &a,
            "--successor", &a,
            "--contract", &n_contract,
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
    let b_contract = contract_with_pred(&a);
    let b = add_node(&home, &task, "B", &["--depends-on", &a, "--contract", &b_contract]); // pending
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
    let b_contract = contract_with_pred(&a);
    let b = add_node(&home, &task, "B", &["--depends-on", &a, "--contract", &b_contract]);

    // result_summary must be valid JSON with "result" field (matching CONTRACT_ROOT output.schema)
    let input = format!(
        "[{{\"id\":\"{a}\",\"status\":\"succeeded\",\"metadata\":{{\"result_summary\":\"{{\\\"result\\\":\\\"A output\\\"}}\"}}}}]"
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
    let b_contract = contract_with_pred(&a);
    let b = add_node(&home, &task, "B", &["--depends-on", &a, "--contract", &b_contract]);

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
    // result_summary must be valid JSON matching output.schema (required: ["result"])
    let input = format!(
        "[{{\"id\":\"{n}\",\"status\":\"succeeded\",\"metadata\":{{\"result_summary\":\"{{\\\"result\\\":\\\"ok\\\"}}\"}}}}]"
    );

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
    let n_contract = contract_with_pred(&dep);
    add_node(&home, &task, "N", &["--depends-on", &dep, "--successor", &s, "--contract", &n_contract]);

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

// ── contract validation (CLI) ─────────────────────────────────────────────────

#[test]
fn dag_add_missing_contract_is_error() {
    let (home, task) = setup();
    // --contract is required; omitting it should cause a parse/missing-arg error
    let out = kerrigan(&home)
        .args(["dag", "add", "--task", &task, "--summary", "no-contract"])
        .output()
        .unwrap();
    // clap will fail with "required argument" error
    assert!(!out.status.success());
}

#[test]
fn dag_add_invalid_contract_json_is_error() {
    let (home, task) = setup();
    kerrigan(&home)
        .args([
            "dag", "add", "--task", &task,
            "--summary", "bad-contract",
            "--contract", "{not valid json",
        ])
        .assert()
        .failure()
        .stderr(contains("error")); // json parse error
}

#[test]
fn dag_add_contract_missing_output_schema_is_error() {
    let (home, task) = setup();
    // Valid JSON but missing output.schema
    kerrigan(&home)
        .args([
            "dag", "add", "--task", &task,
            "--summary", "no-output",
            "--contract", r#"{"predecessor":null}"#,
        ])
        .assert()
        .failure()
        .stderr(contains("output schema is required"));
}

#[test]
fn dag_add_contract_bijection_missing_predecessor_schema_is_error() {
    let (home, task) = setup();
    let a = add_node(&home, &task, "A", &[]);
    // Depends on A but predecessor map is empty
    let bad_contract = format!(
        r#"{{"predecessor":{{}},"output":{{"type":"object","required":["result"],"properties":{{"result":{{"type":"string"}}}}}}}}"#
    );
    kerrigan(&home)
        .args([
            "dag", "add", "--task", &task,
            "--summary", "bad-bijection",
            "--depends-on", &a,
            "--contract", &bad_contract,
        ])
        .assert()
        .failure()
        .stderr(contains("has no schema in predecessor map"));
}

// ── runtime validation (CLI) ──────────────────────────────────────────────────

#[test]
fn dag_update_bad_json_result_summary_is_error() {
    let (home, task) = setup();
    let n = add_node(&home, &task, "N", &[]);
    // result_summary is not valid JSON → schema validation fails
    let input = format!(
        "[{{\"id\":\"{n}\",\"status\":\"succeeded\",\"metadata\":{{\"result_summary\":\"not json\"}}}}]"
    );
    kerrigan(&home)
        .args(["dag", "update", "--task", &task, "--input", &input])
        .assert()
        .failure()
        .stderr(contains("output schema validation failed"));
}

#[test]
fn dag_update_missing_required_field_is_error() {
    let (home, task) = setup();
    let n = add_node(&home, &task, "N", &[]);
    // JSON but missing "result" field
    let input = format!(
        "[{{\"id\":\"{n}\",\"status\":\"succeeded\",\"metadata\":{{\"result_summary\":\"{{\\\"wrong\\\":\\\"field\\\"}}\"}}}}]"
    );
    kerrigan(&home)
        .args(["dag", "update", "--task", &task, "--input", &input])
        .assert()
        .failure()
        .stderr(contains("output schema validation failed"));
}

#[test]
fn dag_update_error_message_includes_override_hint() {
    let (home, task) = setup();
    let n = add_node(&home, &task, "N", &[]);
    let input = format!(
        "[{{\"id\":\"{n}\",\"status\":\"succeeded\",\"metadata\":{{\"result_summary\":\"bad\"}}}}]"
    );
    let err_out = kerrigan(&home)
        .args(["dag", "update", "--task", &task, "--input", &input])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&err_out.stderr);
    assert!(stderr.contains("dag override"), "error should mention dag override: {stderr}");
}
