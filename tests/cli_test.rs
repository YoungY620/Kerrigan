use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd(root: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("kerrigan-storage").unwrap();
    c.env("HOME", root.parent().unwrap_or(root));
    // We need to set HOME so that default_root() resolves to our temp dir.
    // But default_root() uses $HOME/.kerrigan, so we set HOME to the temp dir's parent
    // and name the temp dir ".kerrigan". Alternatively, we just set HOME to the temp dir
    // itself so it creates .kerrigan inside it.
    c.env("HOME", root);
    c
}

fn setup() -> TempDir {
    let tmp = TempDir::new().unwrap();
    cmd(tmp.path()).args(["init"]).assert().success();
    tmp
}

// ── Init ──

#[test]
fn cli_init() {
    let tmp = TempDir::new().unwrap();
    cmd(tmp.path())
        .args(["init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\""));
}

#[test]
fn cli_init_idempotent() {
    let tmp = setup();
    cmd(tmp.path())
        .args(["init"])
        .assert()
        .success();
}

// ── Help ──

#[test]
fn cli_help() {
    let tmp = TempDir::new().unwrap();
    cmd(tmp.path())
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DAG engine"));
}

#[test]
fn cli_task_help() {
    let tmp = TempDir::new().unwrap();
    cmd(tmp.path())
        .args(["task", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("list"));
}

#[test]
fn cli_add_help() {
    let tmp = TempDir::new().unwrap();
    cmd(tmp.path())
        .args(["add", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--task"))
        .stdout(predicate::str::contains("--summary"));
}

// ── Task create and list ──

#[test]
fn cli_task_create_and_list() {
    let tmp = setup();

    // Create
    let out = cmd(tmp.path())
        .args(["task", "create", "--summary", "my task", "--workdir", "/tmp"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let task_id = v["task_id"].as_str().unwrap().to_string();
    assert!(!task_id.is_empty());

    // List
    let out = cmd(tmp.path())
        .args(["task", "list"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let tasks: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(tasks.as_array().unwrap().len(), 1);
    assert_eq!(tasks[0]["summary"], "my task");
}

// ── Node operations ──

fn create_task(tmp: &TempDir) -> String {
    let out = cmd(tmp.path())
        .args(["task", "create", "--summary", "t", "--workdir", "/tmp"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    v["task_id"].as_str().unwrap().to_string()
}

fn add_node(tmp: &TempDir, task_id: &str, summary: &str, depends_on: &str) -> String {
    let mut args = vec!["add", "--task", task_id, "--summary", summary];
    if !depends_on.is_empty() {
        args.push("--depends-on");
        args.push(depends_on);
    }
    let out = cmd(tmp.path()).args(&args).assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    v["node_id"].as_str().unwrap().to_string()
}

#[test]
fn cli_add_and_get_node() {
    let tmp = setup();
    let task_id = create_task(&tmp);
    let node_id = add_node(&tmp, &task_id, "do something", "");

    let out = cmd(tmp.path())
        .args(["get", "--task", &task_id, "--node", &node_id])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let node: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(node["summary"], "do something");
    assert_eq!(node["status"], "ready");
}

#[test]
fn cli_list_and_ready() {
    let tmp = setup();
    let task_id = create_task(&tmp);
    let a = add_node(&tmp, &task_id, "A", "");
    let _b = add_node(&tmp, &task_id, "B", &a);

    // list: 2 nodes
    let out = cmd(tmp.path())
        .args(["list", "--task", &task_id])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let nodes: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap();
    assert_eq!(nodes.len(), 2);

    // ready: 1 node (A)
    let out = cmd(tmp.path())
        .args(["ready", "--task", &task_id])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let ready: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0]["id"], a);
}

#[test]
fn cli_update_via_input_flag() {
    let tmp = setup();
    let task_id = create_task(&tmp);
    let a = add_node(&tmp, &task_id, "A", "");
    let b = add_node(&tmp, &task_id, "B", &a);

    // Update A to succeeded via --input
    let json = format!(
        r#"[{{"id":"{}","status":"succeeded","metadata":{{"result_summary":"done"}}}}]"#,
        a
    );
    cmd(tmp.path())
        .args(["update", "--task", &task_id, "--input", &json])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\""));

    // B should now be ready
    let out = cmd(tmp.path())
        .args(["get", "--task", &task_id, "--node", &b])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let node: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(node["status"], "ready");
}

#[test]
fn cli_update_via_stdin() {
    let tmp = setup();
    let task_id = create_task(&tmp);
    let a = add_node(&tmp, &task_id, "A", "");

    let json = format!(
        r#"[{{"id":"{}","status":"succeeded"}}]"#,
        a
    );
    cmd(tmp.path())
        .args(["update", "--task", &task_id])
        .write_stdin(json)
        .assert()
        .success();

    let out = cmd(tmp.path())
        .args(["get", "--task", &task_id, "--node", &a])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let node: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(node["status"], "succeeded");
}

// ── Error cases ──

#[test]
fn cli_error_task_not_found() {
    let tmp = setup();
    cmd(tmp.path())
        .args(["list", "--task", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("task not found"));
}

#[test]
fn cli_error_node_not_found() {
    let tmp = setup();
    let task_id = create_task(&tmp);
    cmd(tmp.path())
        .args(["get", "--task", &task_id, "--node", "ghost"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("node not found"));
}

#[test]
fn cli_error_invalid_json() {
    let tmp = setup();
    let task_id = create_task(&tmp);
    cmd(tmp.path())
        .args(["update", "--task", &task_id, "--input", "not json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn cli_error_not_initialized() {
    let tmp = TempDir::new().unwrap();
    cmd(tmp.path())
        .args(["task", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not initialized"));
}
