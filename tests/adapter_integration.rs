//! Integration tests that actually invoke agent CLIs.
//!
//! Run with: `cargo test --features integration`
//! Skipped in CI (no `integration` feature by default).

#![cfg(feature = "integration")]

use tempfile::TempDir;

// We can't import from the binary crate directly in integration tests,
// so we shell out to the agent CLIs directly to verify flags work.

/// Verify claude --print works with a simple prompt.
#[tokio::test]
async fn claude_print_simple_prompt() {
    let workdir = TempDir::new().unwrap();
    let output = tokio::process::Command::new("claude")
        .current_dir(workdir.path())
        .args([
            "--print",
            "--dangerously-skip-permissions",
            "Reply with exactly: KERRIGAN_TEST_OK",
        ])
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            println!("claude stdout: {stdout}");
            println!("claude stderr: {stderr}");
            println!("claude exit code: {}", out.status.code().unwrap_or(-1));
            assert!(out.status.success(), "claude exited with error: {stderr}");
            assert!(
                stdout.contains("KERRIGAN_TEST_OK"),
                "Expected KERRIGAN_TEST_OK in output, got: {stdout}"
            );
        }
        Err(e) => {
            panic!("Failed to execute claude: {e}. Is claude installed?");
        }
    }
}

/// Verify claude --session-id accepts a UUID.
#[tokio::test]
async fn claude_session_id_uuid() {
    let workdir = TempDir::new().unwrap();
    let session_id = uuid::Uuid::new_v4().to_string();
    let output = tokio::process::Command::new("claude")
        .current_dir(workdir.path())
        .args([
            "--print",
            "--dangerously-skip-permissions",
            "--session-id",
            &session_id,
            "Reply with exactly: SESSION_OK",
        ])
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            println!("claude stdout: {stdout}");
            assert!(out.status.success(), "claude failed with session-id");
            assert!(stdout.contains("SESSION_OK"));
        }
        Err(e) => {
            panic!("Failed to execute claude: {e}");
        }
    }
}

/// Verify claude --add-dir works (skills directory injection).
#[tokio::test]
async fn claude_add_dir_for_skills() {
    let workdir = TempDir::new().unwrap();
    let skills_dir = TempDir::new().unwrap();

    // Create a minimal file in the skills dir to verify it's accessible
    std::fs::write(skills_dir.path().join("test.txt"), "skills test content").unwrap();

    let output = tokio::process::Command::new("claude")
        .current_dir(workdir.path())
        .args([
            "--print",
            "--dangerously-skip-permissions",
            "--add-dir",
            &skills_dir.path().to_string_lossy(),
            "-p",
            "Reply with exactly: ADDDIR_OK",
        ])
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            println!("claude stdout: {stdout}");
            assert!(
                out.status.success(),
                "claude failed with --add-dir: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Err(e) => {
            panic!("Failed to execute claude: {e}");
        }
    }
}

/// Verify kimi --print --yolo works with a simple prompt.
#[tokio::test]
async fn kimi_print_simple_prompt() {
    let workdir = TempDir::new().unwrap();
    let output = tokio::process::Command::new("kimi")
        .current_dir(workdir.path())
        .args([
            "--print",
            "--yolo",
            "--prompt",
            "Reply with exactly: KERRIGAN_TEST_OK",
        ])
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            println!("kimi stdout: {stdout}");
            println!("kimi stderr: {stderr}");
            println!("kimi exit code: {}", out.status.code().unwrap_or(-1));
            assert!(out.status.success(), "kimi exited with error: {stderr}");
            assert!(
                stdout.contains("KERRIGAN_TEST_OK"),
                "Expected KERRIGAN_TEST_OK in output, got: {stdout}"
            );
        }
        Err(e) => {
            panic!("Failed to execute kimi: {e}. Is kimi installed?");
        }
    }
}

/// Verify kimi --session works.
#[tokio::test]
async fn kimi_session_id() {
    let workdir = TempDir::new().unwrap();
    let session_id = uuid::Uuid::new_v4().to_string();
    let output = tokio::process::Command::new("kimi")
        .current_dir(workdir.path())
        .args([
            "--print",
            "--yolo",
            "--session",
            &session_id,
            "--prompt",
            "Reply with exactly: SESSION_OK",
        ])
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            println!("kimi stdout: {stdout}");
            assert!(out.status.success(), "kimi failed with --session");
            assert!(stdout.contains("SESSION_OK"));
        }
        Err(e) => {
            panic!("Failed to execute kimi: {e}");
        }
    }
}

/// Verify kimi --skills-dir works.
#[tokio::test]
async fn kimi_skills_dir() {
    let workdir = TempDir::new().unwrap();
    let skills_dir = TempDir::new().unwrap();

    let output = tokio::process::Command::new("kimi")
        .current_dir(workdir.path())
        .args([
            "--print",
            "--yolo",
            "--skills-dir",
            &skills_dir.path().to_string_lossy(),
            "--prompt",
            "Reply with exactly: SKILLSDIR_OK",
        ])
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            println!("kimi stdout: {stdout}");
            assert!(
                out.status.success(),
                "kimi failed with --skills-dir: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Err(e) => {
            panic!("Failed to execute kimi: {e}");
        }
    }
}
