use crate::error::{Result, StorageError};
use rusqlite::Connection;
use std::path::Path;

const SCHEMA: &str = "
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    summary TEXT NOT NULL,
    workdir TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS nodes (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id),
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
    task_id TEXT NOT NULL REFERENCES tasks(id),
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

CREATE TABLE IF NOT EXISTS board_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    read INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_nodes_task ON nodes(task_id);
CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes(task_id, status);
CREATE INDEX IF NOT EXISTS idx_edges_pred ON node_edges(predecessor_id);
CREATE INDEX IF NOT EXISTS idx_edges_succ ON node_edges(successor_id);
CREATE INDEX IF NOT EXISTS idx_board_task_unread ON board_messages(task_id, read);
";

/// Open (or create) the database at the given path and apply migrations.
pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path).map_err(|e| StorageError::Db(format!("{path:?}: {e}")))?;
    migrate(&conn)?;
    Ok(conn)
}

/// Open an in-memory database (for testing).
pub fn open_memory() -> Result<Connection> {
    let conn =
        Connection::open_in_memory().map_err(|e| StorageError::Db(format!("in-memory: {e}")))?;
    migrate(&conn)?;
    Ok(conn)
}

/// Apply schema migrations. Idempotent (CREATE IF NOT EXISTS).
fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA)
        .map_err(|e| StorageError::Db(format!("migration: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_memory_creates_tables() {
        let conn = open_memory().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='nodes'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn wal_mode_enabled() {
        let conn = open_memory().unwrap();
        // In-memory doesn't actually use WAL, but the PRAGMA should not fail.
        // For file-backed DBs, verify WAL mode.
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        // In-memory returns "memory", file-backed returns "wal"
        assert!(mode == "memory" || mode == "wal");
    }

    #[test]
    fn migrate_is_idempotent() {
        let conn = open_memory().unwrap();
        // Second migration should not fail
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();
    }
}
