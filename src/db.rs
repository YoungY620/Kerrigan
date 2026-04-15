use crate::error::{Error, Result};
use rusqlite::Connection;
use std::path::Path;

const REGISTRY_SCHEMA: &str = "
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    summary TEXT NOT NULL,
    workdir TEXT NOT NULL,
    created_at TEXT NOT NULL
);
";

const TASK_SCHEMA: &str = "
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;

CREATE TABLE IF NOT EXISTS nodes (
    id TEXT PRIMARY KEY,
    summary TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending','ready','running','succeeded','failed')),
    result_summary TEXT,
    failure_reason TEXT
);

CREATE TABLE IF NOT EXISTS node_edges (
    predecessor_id TEXT NOT NULL REFERENCES nodes(id),
    successor_id TEXT NOT NULL REFERENCES nodes(id),
    PRIMARY KEY(predecessor_id, successor_id)
);

CREATE TABLE IF NOT EXISTS node_sessions (
    node_id TEXT NOT NULL REFERENCES nodes(id),
    session_id TEXT NOT NULL,
    PRIMARY KEY(node_id, session_id)
);

CREATE TABLE IF NOT EXISTS spawned_tasks (
    node_id TEXT NOT NULL REFERENCES nodes(id),
    spawned_node_id TEXT NOT NULL,
    PRIMARY KEY(node_id, spawned_node_id)
);

CREATE TABLE IF NOT EXISTS processes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id TEXT NOT NULL REFERENCES nodes(id),
    pid INTEGER NOT NULL,
    session_id TEXT NOT NULL,
    adapter TEXT NOT NULL,
    log_path TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    exit_code INTEGER,
    status TEXT NOT NULL DEFAULT 'running'
        CHECK(status IN ('running','exited','killed'))
);

CREATE TABLE IF NOT EXISTS node_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    consumed INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes(status);
CREATE INDEX IF NOT EXISTS idx_edges_pred ON node_edges(predecessor_id);
CREATE INDEX IF NOT EXISTS idx_edges_succ ON node_edges(successor_id);
CREATE INDEX IF NOT EXISTS idx_processes_node ON processes(node_id);
CREATE INDEX IF NOT EXISTS idx_processes_status ON processes(status);
CREATE INDEX IF NOT EXISTS idx_node_messages ON node_messages(node_id, consumed);
";

/// Open (or create) the global registry database.
pub fn open_registry(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path).map_err(|e| Error::Db(format!("{path:?}: {e}")))?;
    conn.execute_batch(REGISTRY_SCHEMA)
        .map_err(|e| Error::Db(format!("registry migration: {e}")))?;
    Ok(conn)
}

/// Open (or create) a task-level database.
pub fn open_task(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path).map_err(|e| Error::Db(format!("{path:?}: {e}")))?;
    conn.execute_batch(TASK_SCHEMA)
        .map_err(|e| Error::Db(format!("task migration: {e}")))?;
    Ok(conn)
}

/// Open an in-memory task database (for testing).
pub fn open_task_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory().map_err(|e| Error::Db(format!("in-memory: {e}")))?;
    conn.execute_batch(TASK_SCHEMA)
        .map_err(|e| Error::Db(format!("task migration: {e}")))?;
    Ok(conn)
}

/// Open an in-memory registry database (for testing).
pub fn open_registry_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory().map_err(|e| Error::Db(format!("in-memory: {e}")))?;
    conn.execute_batch(REGISTRY_SCHEMA)
        .map_err(|e| Error::Db(format!("registry migration: {e}")))?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_creates_tasks_table() {
        let conn = open_registry_memory().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='tasks'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn task_db_creates_all_tables() {
        let conn = open_task_memory().unwrap();
        for table in [
            "nodes",
            "node_edges",
            "node_sessions",
            "processes",
            "node_messages",
        ] {
            let count: i64 = conn
                .query_row(
                    &format!(
                        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='{table}'"
                    ),
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "table {table} should exist");
        }
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = open_task_memory().unwrap();
        conn.execute_batch(TASK_SCHEMA).unwrap();
        conn.execute_batch(TASK_SCHEMA).unwrap();
    }
}
