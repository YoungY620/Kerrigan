use std::fs;
use std::path::Path;
use std::time::SystemTime;

/// Watches the board.md file for changes.
pub struct BoardWatcher {
    path: std::path::PathBuf,
    last_mtime: Option<SystemTime>,
    last_content: String,
}

impl BoardWatcher {
    pub fn new(task_dir: &Path) -> Self {
        Self {
            path: task_dir.join("board.md"),
            last_mtime: None,
            last_content: String::new(),
        }
    }

    /// Check if the board file has been modified since last check.
    /// Returns the new content if changed, None otherwise.
    pub fn check_for_changes(&mut self) -> Option<String> {
        let mtime = fs::metadata(&self.path).ok()?.modified().ok()?;

        if self.last_mtime == Some(mtime) {
            return None;
        }

        let content = fs::read_to_string(&self.path).ok()?;
        self.last_mtime = Some(mtime);

        if content.trim().is_empty() || content == self.last_content {
            return None;
        }

        self.last_content = content.clone();
        Some(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detects_new_content() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("board.md"), "").unwrap();
        let mut watcher = BoardWatcher::new(tmp.path());

        // Empty file → no change
        assert!(watcher.check_for_changes().is_none());

        // Write content
        std::thread::sleep(std::time::Duration::from_millis(50));
        fs::write(tmp.path().join("board.md"), "Please check node n3").unwrap();
        let change = watcher.check_for_changes();
        assert!(change.is_some());
        assert!(change.unwrap().contains("node n3"));

        // Same content → no change
        assert!(watcher.check_for_changes().is_none());
    }
}
