use std::collections::HashSet;

use rusqlite::{params, Connection};

use crate::error::AppError;

include!(concat!(env!("OUT_DIR"), "/migration_sources.rs"));

/// Records migrations applied by this public database lineage.
const CREATE_MIGRATIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS _migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

/// Return the latest ordered migration version supported by this application.
pub fn migration_head() -> Result<i64, AppError> {
    Ok(migration_files()?
        .last()
        .map(|(version, _, _)| *version)
        .unwrap_or(0))
}

/// Parse the leading integer version from a migration file name.
fn version_from_file_name(file_name: &str) -> Option<i64> {
    file_name.split('_').next()?.parse().ok()
}

/// Return the strictly ordered, contiguous runtime migration sequence.
fn migration_files() -> Result<Vec<(i64, &'static str, &'static str)>, AppError> {
    let mut files = MIGRATIONS
        .iter()
        .map(|(file_name, sql)| {
            let version = version_from_file_name(file_name)
                .ok_or_else(|| AppError::io("A migration file has an invalid version prefix."))?;
            Ok((version, *file_name, *sql))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    files.sort_by_key(|(version, _, _)| *version);

    let mut seen = HashSet::new();
    for (index, (version, _, _)) in files.iter().enumerate() {
        if !seen.insert(*version) {
            return Err(AppError::io("Migration version numbers must be unique."));
        }
        if *version != index as i64 + 1 {
            return Err(AppError::io(
                "Runtime migration versions must be contiguous and start at 001.",
            ));
        }
    }
    Ok(files)
}

/// Reject databases created by a newer application before changing the schema
/// or connection settings.
pub fn ensure_supported_version(conn: &Connection) -> Result<(), AppError> {
    let head = migration_head()?;
    if !table_exists(conn, "_migrations")? {
        return Ok(());
    }
    let mut stmt = conn.prepare("SELECT version FROM _migrations")?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    for version in rows {
        if version? > head {
            return Err(AppError::validation(
                "This database was created by a newer application version and cannot be opened safely.",
            ));
        }
    }
    Ok(())
}

/// Apply pending public migrations in version order, each inside its own
/// transaction, and record them in `_migrations`.
pub fn run_migrations(conn: &Connection) -> Result<(), AppError> {
    let files = migration_files()?;
    ensure_supported_version(conn)?;

    let mut applied = HashSet::new();
    if table_exists(conn, "_migrations")? {
        let mut stmt = conn.prepare("SELECT version FROM _migrations")?;
        let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
        for version in rows {
            applied.insert(version?);
        }
    }

    conn.execute_batch(CREATE_MIGRATIONS_TABLE)?;
    for (version, file_name, sql) in files {
        if applied.contains(&version) {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (?1, ?2)",
            params![version, file_name],
        )?;
        tx.commit()?;
        log::info!("Applied migration {version} ({file_name})");
    }
    Ok(())
}

/// Assert that a table exists. Used by schema verification tests.
pub fn table_exists(conn: &Connection, table: &str) -> Result<bool, AppError> {
    let mut stmt =
        conn.prepare("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1")?;
    let count: i64 = stmt.query_row(params![table], |row| row.get(0))?;
    Ok(count > 0)
}

/// Assert that a column exists on a table. Used by schema verification tests.
pub fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, AppError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for name in rows {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn schema_objects(conn: &Connection) -> Vec<(String, String, String)> {
        let mut stmt = conn
            .prepare(
                "SELECT type, name, sql FROM sqlite_master
                 WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%'
                   AND name <> '_migrations'
                 ORDER BY type, name",
            )
            .expect("prepare schema snapshot");
        let rows = stmt
            .query_map([], |row| {
                let sql: String = row.get(2)?;
                Ok((row.get(0)?, row.get(1)?, sql.replace("\r\n", "\n")))
            })
            .expect("query schema snapshot");
        rows.collect::<Result<_, _>>()
            .expect("collect schema snapshot")
    }

    #[test]
    fn runtime_head_is_one_canonical_migration() {
        assert_eq!(migration_head().expect("read migration head"), 1);
        assert_eq!(
            MIGRATIONS.len(),
            1,
            "archive is not a runtime migration set"
        );
        assert_eq!(MIGRATIONS[0].0, "001_initial_schema.sql");
        assert_eq!(
            migration_files()
                .expect("discover runtime migrations")
                .iter()
                .map(|(version, _, _)| *version)
                .collect::<Vec<_>>(),
            vec![1]
        );
    }

    #[test]
    fn canonical_baseline_matches_archived_001_through_042_schema() {
        let baseline = Connection::open_in_memory().expect("open baseline database");
        crate::db::connection::configure(&baseline).expect("configure baseline database");
        run_migrations(&baseline).expect("apply canonical baseline");
        let table_count: i64 = baseline
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .expect("count baseline tables");
        assert_eq!(table_count, 34, "33 domain tables plus migration history");

        // The archive is read only by this explicit in-memory equivalence test;
        // build.rs discovers only direct .sql children of migrations/.
        let archive = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("migrations")
            .join("archive");
        let mut archived_files = std::fs::read_dir(archive)
            .expect("read archived SQL migrations")
            .map(|entry| entry.expect("read archived migration entry").path())
            .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("sql"))
            .collect::<Vec<_>>();
        archived_files.sort();
        assert_eq!(
            archived_files.len(),
            42,
            "archive retains migrations 001-042"
        );
        for (index, path) in archived_files.iter().enumerate() {
            let expected = format!("{:03}_", index + 1);
            assert!(
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(&expected)),
                "archived migration {index} should have prefix {expected}"
            );
        }

        let historical = Connection::open_in_memory().expect("open historical schema database");
        historical
            .pragma_update(None, "foreign_keys", "ON")
            .expect("enable historical foreign keys");
        for path in archived_files {
            let sql = std::fs::read_to_string(path).expect("read archived migration SQL");
            historical
                .execute_batch(&sql)
                .expect("apply archived migration to empty in-memory database");
        }
        assert_eq!(
            schema_objects(&baseline),
            schema_objects(&historical),
            "baseline tables, columns, defaults, constraints, indexes, and triggers match archived history"
        );

        let recorded: Vec<(i64, String)> = {
            let mut stmt = baseline
                .prepare("SELECT version, name FROM _migrations ORDER BY version")
                .expect("prepare applied migration query");
            let rows = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .expect("query applied migration");
            rows.collect::<Result<_, _>>()
                .expect("collect applied migrations")
        };
        assert_eq!(recorded, vec![(1, "001_initial_schema.sql".to_owned())]);
    }

    #[test]
    fn canonical_baseline_is_idempotent() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        crate::db::connection::configure(&conn).expect("configure in-memory database");
        run_migrations(&conn).expect("apply baseline first time");
        let before = schema_objects(&conn);
        run_migrations(&conn).expect("apply baseline second time");
        assert_eq!(schema_objects(&conn), before);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
            .expect("count migration markers");
        assert_eq!(count, 1);
    }

    #[test]
    fn future_database_version_is_rejected_without_mutating_existing_schema() {
        let conn = Connection::open_in_memory().expect("open in-memory connection");
        conn.execute_batch(
            "CREATE TABLE _migrations (
                 version INTEGER PRIMARY KEY,
                 name TEXT NOT NULL,
                 applied_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE TABLE preserved_data (value TEXT NOT NULL);
             INSERT INTO preserved_data VALUES ('synthetic sentinel');
             INSERT INTO _migrations (version, name) VALUES (999, '999_future.sql');",
        )
        .expect("create synthetic future database");
        let table_count_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
                [],
                |row| row.get(0),
            )
            .expect("count tables before");

        assert!(
            run_migrations(&conn).is_err(),
            "future version must be rejected"
        );
        let table_count_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
                [],
                |row| row.get(0),
            )
            .expect("count tables after");
        let sentinel: String = conn
            .query_row("SELECT value FROM preserved_data", [], |row| row.get(0))
            .expect("read preserved sentinel");
        assert_eq!(table_count_after, table_count_before);
        assert_eq!(sentinel, "synthetic sentinel");
        assert!(!table_exists(&conn, "workspaces").expect("check no app schema"));
    }

    #[test]
    fn lifecycle_intervals_reject_overlap_and_preserve_ordering_constraints() {
        let conn = Connection::open_in_memory().expect("open in-memory connection");
        crate::db::connection::configure(&conn).expect("configure in-memory database");
        run_migrations(&conn).expect("run canonical baseline");
        for workspace_id in ["w1", "w2"] {
            conn.execute(
                "INSERT INTO workspaces (id, name, created_at, updated_at)
                 VALUES (?1, 'Synthetic', '2026-01-01', '2026-01-01')",
                [workspace_id],
            )
            .expect("insert synthetic workspace");
        }
        for (recurrence_id, workspace_id) in [("r1", "w1"), ("r2", "w1"), ("r3", "w2")] {
            conn.execute(
                "INSERT INTO block_recurrences (id, workspace_id, created_at, updated_at)
                 VALUES (?1, ?2, '2026-01-01', '2026-01-01')",
                params![recurrence_id, workspace_id],
            )
            .expect("insert synthetic recurrence");
        }
        let insert_interval = |id: &str,
                               workspace_id: &str,
                               recurrence_id: &str,
                               from: &str,
                               until: Option<&str>,
                               source: &str| {
            conn.execute(
                "INSERT INTO block_recurrence_lifecycle_intervals (
                    id, workspace_id, recurrence_id, active_from_local_date,
                    active_until_local_date, source, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '2026-05-01')",
                params![id, workspace_id, recurrence_id, from, until, source],
            )
        };

        assert!(insert_interval("i1", "w1", "r1", "2026-03-01", None, "explicit").is_ok());
        assert!(insert_interval("i2", "w1", "r1", "2026-04-01", None, "explicit").is_err());
        assert!(insert_interval(
            "i3",
            "w1",
            "r1",
            "2026-03-15",
            Some("2026-04-15"),
            "explicit"
        )
        .is_err());
        assert!(insert_interval(
            "i4",
            "w1",
            "r1",
            "2026-02-01",
            Some("2026-02-10"),
            "explicit"
        )
        .is_err());
        conn.execute(
            "UPDATE block_recurrence_lifecycle_intervals
             SET active_until_local_date = '2026-05-01' WHERE id = 'i1'",
            [],
        )
        .expect("close open interval");
        assert!(insert_interval("i5", "w1", "r1", "2026-05-01", None, "explicit").is_ok());
        assert!(conn
            .execute(
                "UPDATE block_recurrence_lifecycle_intervals
                 SET active_from_local_date = '2026-04-01' WHERE id = 'i5'",
                [],
            )
            .is_err());
        assert!(insert_interval(
            "i6",
            "w1",
            "r2",
            "2026-06-01",
            Some("2026-05-31"),
            "explicit"
        )
        .is_err());
        assert!(insert_interval(
            "i7",
            "w1",
            "r2",
            "2026-06-01",
            Some("2026-06-30"),
            "guessed"
        )
        .is_err());
        assert!(insert_interval("i8", "w2", "r3", "2026-03-01", None, "explicit").is_ok());
        assert!(insert_interval("i9", "w1", "r2", "2026-06-01", None, "explicit").is_ok());
    }
}
