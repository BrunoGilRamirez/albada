//! Shared test helpers. Only compiled in test builds.

use rusqlite::Connection;

use crate::db::connection;
use crate::db::migrations::run_migrations;
use crate::time;

/// Create an in-memory connection with migrations applied and a workspace
/// seeded as `w1`.
pub fn test_conn() -> Connection {
    let conn = connection::open_in_memory().expect("open in-memory connection");
    run_migrations(&conn).expect("run migrations");
    let now = time::now_rfc3339();
    conn.execute(
        "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES ('w1', 'Test', ?1, ?1)",
        [&now],
    )
    .expect("seed workspace");
    conn
}

/// Seed a workspace returning its id.
pub fn seed_workspace(conn: &Connection) -> String {
    let now = time::now_rfc3339();
    conn.execute(
        "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES ('w1', 'Test', ?1, ?1)",
        [&now],
    )
    .expect("seed workspace");
    "w1".to_string()
}
