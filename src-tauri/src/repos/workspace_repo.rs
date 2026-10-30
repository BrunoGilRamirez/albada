use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::models::Workspace;
use crate::error::AppError;
use crate::id;
use crate::time;

pub trait WorkspaceRepo {
    fn create(&self, conn: &Connection, name: &str) -> Result<Workspace, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<Workspace, AppError>;
    fn list(&self, conn: &Connection) -> Result<Vec<Workspace>, AppError>;
    fn update_name(&self, conn: &Connection, id: &str, name: &str) -> Result<Workspace, AppError>;
    fn set_root_path(
        &self,
        conn: &Connection,
        id: &str,
        root_path: &str,
    ) -> Result<Workspace, AppError>;
    fn update_locale(
        &self,
        conn: &Connection,
        id: &str,
        locale: &str,
    ) -> Result<Workspace, AppError>;
    fn ensure_default(&self, conn: &Connection) -> Result<Workspace, AppError>;
}

fn row_to_workspace(row: &rusqlite::Row<'_>) -> Result<Workspace, rusqlite::Error> {
    Ok(Workspace {
        id: row.get(0)?,
        name: row.get(1)?,
        root_path: row.get(2)?,
        timezone: row.get(3)?,
        locale: row.get(4)?,
        default_privacy_scope: row.get(5)?,
        current_year_theme_id: row.get(6)?,
        settings_json: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub struct SqliteWorkspaceRepo;

impl WorkspaceRepo for SqliteWorkspaceRepo {
    fn create(&self, conn: &Connection, name: &str) -> Result<Workspace, AppError> {
        crate::domain::validation::validate_title(name)?;
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        // The SQLite compatibility default remains en-US; app-created
        // workspaces explicitly start in system mode instead.
        conn.execute(
            "INSERT INTO workspaces (id, name, timezone, locale, default_privacy_scope, created_at, updated_at)
             VALUES (?1, ?2, 'UTC', 'system', 'private', ?3, ?3)",
            params![id, name.trim(), now],
        )?;
        self.get(conn, &id)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<Workspace, AppError> {
        conn.query_row(
            "SELECT id, name, root_path, timezone, locale, default_privacy_scope,
                    current_year_theme_id, settings_json, created_at, updated_at
             FROM workspaces WHERE id = ?1",
            [id],
            row_to_workspace,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The workspace was not found."))
    }

    fn list(&self, conn: &Connection) -> Result<Vec<Workspace>, AppError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, root_path, timezone, locale, default_privacy_scope,
                    current_year_theme_id, settings_json, created_at, updated_at
             FROM workspaces ORDER BY created_at",
        )?;
        let rows = stmt.query_map([], row_to_workspace)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn update_name(&self, conn: &Connection, id: &str, name: &str) -> Result<Workspace, AppError> {
        crate::domain::validation::validate_title(name)?;
        let updated = conn.execute(
            "UPDATE workspaces SET name = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, name.trim(), time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The workspace was not found."));
        }
        self.get(conn, id)
    }

    fn set_root_path(
        &self,
        conn: &Connection,
        id: &str,
        root_path: &str,
    ) -> Result<Workspace, AppError> {
        let trimmed = root_path.trim();
        if trimmed.is_empty() {
            return Err(AppError::validation("A workspace root path is required."));
        }
        let updated = conn.execute(
            "UPDATE workspaces SET root_path = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, trimmed, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The workspace was not found."));
        }
        self.get(conn, id)
    }

    fn update_locale(
        &self,
        conn: &Connection,
        id: &str,
        locale: &str,
    ) -> Result<Workspace, AppError> {
        crate::domain::validation::validate_workspace_locale(locale)?;
        let updated = conn.execute(
            "UPDATE workspaces SET locale = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, locale, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The workspace was not found."));
        }
        self.get(conn, id)
    }

    fn ensure_default(&self, conn: &Connection) -> Result<Workspace, AppError> {
        let list = self.list(conn)?;
        if let Some(first) = list.first() {
            // Seeding runs on every ensure so an existing workspace with no
            // rules receives defaults. Existing or user-edited rules are kept.
            crate::db::seed::seed_default_reminder_rules(conn, &first.id)?;
            return Ok(first.clone());
        }
        let workspace = self.create(conn, "My Workspace")?;
        crate::db::seed::seed_default_reminder_rules(conn, &workspace.id)?;
        Ok(workspace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection;
    use crate::db::migrations::run_migrations;

    fn test_conn() -> Connection {
        let conn = connection::open_in_memory().expect("open in-memory connection");
        run_migrations(&conn).expect("run migrations");
        conn
    }

    #[test]
    fn create_and_get_workspace() {
        let conn = test_conn();
        let repo = SqliteWorkspaceRepo;
        let ws = repo.create(&conn, "Personal").expect("create workspace");
        assert_eq!(ws.name, "Personal");
        assert_eq!(ws.locale, "system");
        let fetched = repo.get(&conn, &ws.id).expect("get workspace");
        assert_eq!(fetched.id, ws.id);
    }

    #[test]
    fn ensure_default_creates_once() {
        let conn = test_conn();
        let repo = SqliteWorkspaceRepo;
        let first = repo.ensure_default(&conn).expect("ensure default");
        let second = repo.ensure_default(&conn).expect("ensure default again");
        assert_eq!(first.id, second.id);
    }

    #[test]
    fn ensure_default_seeds_reminder_rules_once() {
        let repo = SqliteWorkspaceRepo;

        // Fresh database: the first ensure creates the workspace and seeds the
        // five default reminder rules for it.
        let conn = test_conn();
        let ws = repo.ensure_default(&conn).expect("ensure default");
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM reminder_rules WHERE workspace_id = ?1",
                [&ws.id],
                |row| row.get(0),
            )
            .expect("count reminder rules");
        assert_eq!(count, 5, "five default reminder rules seeded");

        // Repeated ensures do not duplicate rules.
        repo.ensure_default(&conn).expect("ensure default again");
        repo.ensure_default(&conn)
            .expect("ensure default a third time");
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM reminder_rules WHERE workspace_id = ?1",
                [&ws.id],
                |row| row.get(0),
            )
            .expect("count reminder rules");
        assert_eq!(count, 5, "seeding is idempotent");
    }

    #[test]
    fn ensure_default_seeds_existing_workspace_without_duplicates() {
        // An existing workspace with no rules still receives defaults.
        let conn = test_conn();
        let repo = SqliteWorkspaceRepo;
        let ws = repo
            .create(&conn, "My Workspace")
            .expect("create workspace");

        let ws2 = repo.ensure_default(&conn).expect("ensure default upgrades");
        assert_eq!(ws.id, ws2.id, "existing workspace is kept");
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM reminder_rules WHERE workspace_id = ?1",
                [&ws.id],
                |row| row.get(0),
            )
            .expect("count reminder rules");
        assert_eq!(count, 5, "upgrade path seeds the default rules");
    }

    #[test]
    fn empty_name_is_rejected() {
        let conn = test_conn();
        let repo = SqliteWorkspaceRepo;
        assert!(repo.create(&conn, "  ").is_err());
    }

    #[test]
    fn set_root_path_persists_and_rejects_empty() {
        let conn = test_conn();
        let repo = SqliteWorkspaceRepo;
        let ws = repo.create(&conn, "Personal").expect("create workspace");
        assert!(ws.root_path.is_none());

        let updated = repo
            .set_root_path(&conn, &ws.id, "C:/legacy")
            .expect("set root path");
        assert_eq!(updated.root_path.as_deref(), Some("C:/legacy"));

        let fetched = repo.get(&conn, &ws.id).expect("get workspace");
        assert_eq!(fetched.root_path.as_deref(), Some("C:/legacy"));

        assert!(repo.set_root_path(&conn, &ws.id, "   ").is_err());
    }

    #[test]
    fn locale_update_persists_allowlisted_value_and_rejects_invalid_input() {
        let conn = test_conn();
        let repo = SqliteWorkspaceRepo;
        let ws = repo.create(&conn, "Personal").expect("create workspace");

        let updated = repo
            .update_locale(&conn, &ws.id, "es-ES")
            .expect("persist locale override");
        assert_eq!(updated.locale, "es-ES");
        assert_eq!(repo.get(&conn, &ws.id).unwrap().locale, "es-ES");

        let error = repo
            .update_locale(&conn, &ws.id, "pt-BR")
            .expect_err("reject unsupported locale");
        assert_eq!(error.code, crate::error::ErrorCode::Validation);
        assert_eq!(repo.get(&conn, &ws.id).unwrap().locale, "es-ES");
    }

    #[test]
    fn archived_migration_038_normalizes_synthetic_legacy_workspace_locales() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        conn.execute_batch(
            "CREATE TABLE workspaces (id TEXT PRIMARY KEY, locale TEXT);
             INSERT INTO workspaces VALUES
                 ('legacy-default', 'en-US'),
                 ('spanish', 'es-ES'),
                 ('german', 'de-DE'),
                 ('french', 'fr-FR'),
                 ('system', 'system'),
                 ('unsupported', 'pt-BR'),
                 ('empty', ''),
                 ('null', NULL);",
        )
        .expect("create synthetic legacy workspace locales");
        let archived_migration = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("migrations")
            .join("archive")
            .join("038_workspace_locale_system.sql");
        let sql = std::fs::read_to_string(archived_migration).expect("read archived migration 038");
        conn.execute_batch(&sql)
            .expect("apply archived migration to synthetic fixture");

        let rows: Vec<(String, String)> = {
            let mut stmt = conn
                .prepare("SELECT id, locale FROM workspaces ORDER BY id")
                .expect("prepare migrated locales query");
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .expect("query migrated locales")
                .collect::<Result<_, _>>()
                .expect("collect migrated locales")
        };
        assert_eq!(
            rows,
            vec![
                ("empty".into(), "system".into()),
                ("french".into(), "fr-FR".into()),
                ("german".into(), "de-DE".into()),
                ("legacy-default".into(), "system".into()),
                ("null".into(), "system".into()),
                ("spanish".into(), "es-ES".into()),
                ("system".into(), "system".into()),
                ("unsupported".into(), "system".into()),
            ]
        );
    }
}
