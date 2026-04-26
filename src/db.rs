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

/// Apply incremental migrations to a task DB.
///
/// Uses PRAGMA user_version as a monotone schema version counter.
/// Each migration runs exactly once: version guard prevents re-execution.
///
/// Version history:
///   0 → base schema (nodes, edges, processes, messages)
///   1 → contract_json column + schema_validation_log table (v1.1.0)
pub fn migrate_task_db(conn: &Connection) -> Result<()> {
    let v: i32 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|e| Error::Db(format!("user_version read: {e}")))?;

    if v < 1 {
        conn.execute_batch(
            "
            ALTER TABLE nodes ADD COLUMN contract_json TEXT;

            CREATE TABLE IF NOT EXISTS schema_validation_log (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id    TEXT NOT NULL,
                phase      TEXT NOT NULL
                               CHECK(phase IN ('static','runtime','override')),
                result     TEXT NOT NULL
                               CHECK(result IN ('ok','fail','override')),
                details    TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_svl_node
                ON schema_validation_log(node_id);
            CREATE INDEX IF NOT EXISTS idx_svl_result
                ON schema_validation_log(node_id, result);

            PRAGMA user_version = 1;
            ",
        )
        .map_err(|e| Error::Db(format!("migration 0→1: {e}")))?;
    }

    // Future migrations: if v < 2 { ... }

    Ok(())
}

/// Open (or create) the global registry database.
pub fn open_registry(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path).map_err(|e| Error::Db(format!("{path:?}: {e}")))?;
    conn.execute_batch(REGISTRY_SCHEMA)
        .map_err(|e| Error::Db(format!("registry migration: {e}")))?;
    Ok(conn)
}

/// Open (or create) a task-level database, applying any pending migrations.
pub fn open_task(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path).map_err(|e| Error::Db(format!("{path:?}: {e}")))?;
    conn.execute_batch(TASK_SCHEMA)
        .map_err(|e| Error::Db(format!("task migration: {e}")))?;
    migrate_task_db(&conn)?;
    Ok(conn)
}

/// Open an in-memory task database (for testing).
pub fn open_task_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory().map_err(|e| Error::Db(format!("in-memory: {e}")))?;
    conn.execute_batch(TASK_SCHEMA)
        .map_err(|e| Error::Db(format!("task migration: {e}")))?;
    migrate_task_db(&conn)?;
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
    fn task_db_creates_all_base_tables() {
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
    fn migration_adds_contract_json_column() {
        let conn = open_task_memory().unwrap();
        // Column must exist after migration
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM pragma_table_info('nodes') WHERE name='contract_json'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "contract_json column should exist after migration");
    }

    #[test]
    fn migration_creates_schema_validation_log_table() {
        let conn = open_task_memory().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='schema_validation_log'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "schema_validation_log table should exist");
    }

    #[test]
    fn migration_sets_user_version_to_1() {
        let conn = open_task_memory().unwrap();
        let v: i32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, 1);
    }

    #[test]
    fn migration_is_idempotent() {
        let conn = open_task_memory().unwrap();
        // Running migration again must not fail
        migrate_task_db(&conn).unwrap();
        migrate_task_db(&conn).unwrap();
        // user_version still 1
        let v: i32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, 1);
    }

    #[test]
    fn migration_runs_on_existing_v0_db() {
        // Simulate a v1.0.x DB: apply base schema without migration
        let conn =
            Connection::open_in_memory().map_err(|e| Error::Db(format!("{e}"))).unwrap();
        conn.execute_batch(TASK_SCHEMA).unwrap();
        // user_version = 0 (default), contract_json column absent
        let v0: i32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v0, 0);
        let col_count_before: i64 = conn
            .query_row(
                "SELECT count(*) FROM pragma_table_info('nodes') WHERE name='contract_json'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(col_count_before, 0, "column should not exist before migration");

        // Apply migration (simulates upgrading from v1.0.x)
        migrate_task_db(&conn).unwrap();

        let v1: i32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v1, 1);
        let col_count_after: i64 = conn
            .query_row(
                "SELECT count(*) FROM pragma_table_info('nodes') WHERE name='contract_json'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(col_count_after, 1, "column should exist after migration");
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = open_task_memory().unwrap();
        conn.execute_batch(TASK_SCHEMA).unwrap();
        migrate_task_db(&conn).unwrap();
    }
}
