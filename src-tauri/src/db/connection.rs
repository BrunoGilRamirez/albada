use std::path::Path;

use rusqlite::Connection;

use crate::error::AppError;

/// Open a SQLite connection at the given path and apply the required pragmas.
pub fn open(path: &Path) -> Result<Connection, AppError> {
    let conn = Connection::open(path)?;
    configure(&conn)?;
    Ok(conn)
}

/// Open an in-memory SQLite connection for tests and diagnostics.
pub fn open_in_memory() -> Result<Connection, AppError> {
    let conn = Connection::open_in_memory()?;
    configure(&conn)?;
    Ok(conn)
}

/// Apply the pragmas required on every connection.
pub fn configure(conn: &Connection) -> Result<(), AppError> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    Ok(())
}

/// Convenience alias for starting a deferred transaction around multi-record
/// mutations. Callers commit explicitly; dropped transactions roll back.
pub fn begin_tx(conn: &mut Connection) -> Result<rusqlite::Transaction<'_>, AppError> {
    Ok(conn.transaction()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreign_keys_are_enabled() {
        let conn = open_in_memory().expect("open in-memory connection");
        let value: i64 = conn
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .expect("read foreign_keys pragma");
        assert_eq!(value, 1);
    }

    #[test]
    fn wal_journal_mode_is_set() {
        let dir = std::env::temp_dir().join(format!("joi-conn-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("test.db");
        {
            let conn = open(&path).expect("open connection");
            let value: String = conn
                .pragma_query_value(None, "journal_mode", |row| row.get(0))
                .expect("read journal_mode pragma");
            assert_eq!(value.to_lowercase(), "wal");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
