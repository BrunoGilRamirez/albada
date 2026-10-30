use std::str::FromStr;

use rusqlite::{params, Connection, OptionalExtension};
use serde_json;

use crate::domain::models::ExportJob;
use crate::domain::types::ExportJobStatus;
use crate::error::AppError;
use crate::id;
use crate::repos::export_artifact_repo::ExportArtifactRepo;
use crate::time;

pub trait ExportJobRepo {
    /// Create a queued job. `selection_json` is the pre-serialized resolved
    /// selection snapshot (IDs plus content hashes); `selection_fingerprint` is
    /// its deterministic fingerprint.
    #[allow(clippy::too_many_arguments)]
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        export_type: &str,
        scope_type: &str,
        title: &str,
        template_name: &Option<String>,
        selection_json: &str,
        selection_fingerprint: &str,
        options: &Option<serde_json::Value>,
    ) -> Result<ExportJob, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<ExportJob, AppError>;
    /// Atomically move a job to `to`, enforcing the Phase 7.10 state machine.
    ///
    /// The current status is read and validated against `to` via
    /// [`ExportJobStatus::can_transition_to`], then the update is applied with a
    /// conditional `WHERE id = ?1 AND status = ?current` so a concurrent change
    /// cannot slip through. Terminal states have no outgoing transitions, so a
    /// terminal job can never be moved again and a retried terminal update is a
    /// no-op that returns a fixed, safe error.
    fn transition_status(
        &self,
        conn: &Connection,
        id: &str,
        to: ExportJobStatus,
        output_path: &Option<String>,
        generated_at: &Option<String>,
        error_message: &Option<String>,
    ) -> Result<ExportJob, AppError>;
    /// Persist structured, safe diagnostics (a JSON array of warning DTOs).
    fn record_diagnostics(
        &self,
        conn: &Connection,
        id: &str,
        diagnostics_json: &str,
    ) -> Result<(), AppError>;
    fn list_for_workspace(
        &self,
        conn: &Connection,
        workspace_id: &str,
    ) -> Result<Vec<ExportJob>, AppError>;
    fn delete_for_workspace(
        &self,
        conn: &Connection,
        workspace_id: &str,
        id: &str,
    ) -> Result<(), AppError>;
}

const SELECT_COLS: &str = "id, workspace_id, type, scope_type, title, template_name, status, output_path, generated_at, error_message, selection_fingerprint, diagnostics_json, options_json, created_at, updated_at";

fn row_to_job(row: &rusqlite::Row<'_>) -> Result<ExportJob, rusqlite::Error> {
    Ok(ExportJob {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        export_type: row.get(2)?,
        scope_type: row.get(3)?,
        title: row.get(4)?,
        template_name: row.get(5)?,
        status: row.get(6)?,
        output_path: row.get(7)?,
        generated_at: row.get(8)?,
        error_message: row.get(9)?,
        selection_fingerprint: row.get(10)?,
        diagnostics_json: row.get(11)?,
        options_json: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        artifacts: Vec::new(),
    })
}

pub struct SqliteExportJobRepo;

impl ExportJobRepo for SqliteExportJobRepo {
    #[allow(clippy::too_many_arguments)]
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        export_type: &str,
        scope_type: &str,
        title: &str,
        template_name: &Option<String>,
        selection_json: &str,
        selection_fingerprint: &str,
        options: &Option<serde_json::Value>,
    ) -> Result<ExportJob, AppError> {
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        let options_json = options
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| AppError::validation(format!("Invalid options: {e}")))?;

        conn.execute(
            "INSERT INTO export_jobs (id, workspace_id, type, scope_type, title, template_name, selection_json, selection_fingerprint, options_json, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
            params![
                id,
                workspace_id,
                export_type,
                scope_type,
                title.trim(),
                template_name,
                selection_json,
                selection_fingerprint,
                options_json,
                ExportJobStatus::Queued.to_string(),
                now
            ],
        )?;
        self.get(conn, &id)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<ExportJob, AppError> {
        let mut job = conn
            .query_row(
                &format!("SELECT {SELECT_COLS} FROM export_jobs WHERE id = ?1"),
                [id],
                row_to_job,
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("The export job was not found."))?;
        job.artifacts =
            crate::repos::export_artifact_repo::SqliteExportArtifactRepo.list_for_job(conn, id)?;
        Ok(job)
    }

    fn transition_status(
        &self,
        conn: &Connection,
        id: &str,
        to: ExportJobStatus,
        output_path: &Option<String>,
        generated_at: &Option<String>,
        error_message: &Option<String>,
    ) -> Result<ExportJob, AppError> {
        // Read the current status and validate the transition against the
        // Phase 7.10 state machine before writing.
        let current: String = conn
            .query_row(
                "SELECT status FROM export_jobs WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("The export job was not found."))?;
        let current_status = ExportJobStatus::from_str(&current)?;

        if !current_status.can_transition_to(to) {
            return Err(AppError::validation(
                "The export job cannot change state from its current status.",
            ));
        }

        // Conditional update keyed on the exact current status so a concurrent
        // change cannot slip through between the read and the write.
        let updated = conn.execute(
            "UPDATE export_jobs
             SET status = ?2, output_path = ?3, generated_at = ?4, error_message = ?5, updated_at = ?6
             WHERE id = ?1 AND status = ?7",
            params![
                id,
                to.to_string(),
                output_path,
                generated_at,
                error_message,
                time::now_rfc3339(),
                current
            ],
        )?;
        if updated == 0 {
            return Err(AppError::validation(
                "The export job cannot change state from its current status.",
            ));
        }
        self.get(conn, id)
    }

    fn record_diagnostics(
        &self,
        conn: &Connection,
        id: &str,
        diagnostics_json: &str,
    ) -> Result<(), AppError> {
        let updated = conn.execute(
            "UPDATE export_jobs SET diagnostics_json = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, diagnostics_json, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The export job was not found."));
        }
        Ok(())
    }

    fn list_for_workspace(
        &self,
        conn: &Connection,
        workspace_id: &str,
    ) -> Result<Vec<ExportJob>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM export_jobs WHERE workspace_id = ?1 ORDER BY created_at DESC"
        ))?;
        let rows = stmt.query_map([workspace_id], row_to_job)?;
        let mut result = Vec::new();
        for row in rows {
            let mut job = row?;
            job.artifacts = crate::repos::export_artifact_repo::SqliteExportArtifactRepo
                .list_for_job(conn, &job.id)?;
            result.push(job);
        }
        Ok(result)
    }

    fn delete_for_workspace(
        &self,
        conn: &Connection,
        workspace_id: &str,
        id: &str,
    ) -> Result<(), AppError> {
        let status: String = conn
            .query_row(
                "SELECT status FROM export_jobs WHERE id = ?1 AND workspace_id = ?2",
                params![id, workspace_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("The export job was not found."))?;
        if status == ExportJobStatus::Running.to_string() {
            return Err(AppError::validation(
                "A running export cannot be deleted; cancel it first.",
            ));
        }
        conn.execute(
            "DELETE FROM export_jobs WHERE id = ?1 AND workspace_id = ?2",
            params![id, workspace_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection;
    use crate::db::migrations::run_migrations;
    use crate::repos::export_artifact_repo::{ExportArtifactRepo, SqliteExportArtifactRepo};
    use crate::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};

    fn create_tables(conn: &Connection) {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS export_jobs (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
                type TEXT NOT NULL,
                scope_type TEXT NOT NULL,
                title TEXT NOT NULL,
                template_name TEXT,
                selection_json TEXT NOT NULL,
                selection_fingerprint TEXT,
                options_json TEXT,
                status TEXT NOT NULL DEFAULT 'queued',
                output_path TEXT,
                generated_at TEXT,
                error_message TEXT,
                diagnostics_json TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS export_artifacts (
                id TEXT PRIMARY KEY,
                export_job_id TEXT NOT NULL REFERENCES export_jobs(id) ON DELETE CASCADE,
                format TEXT NOT NULL,
                path TEXT NOT NULL,
                checksum TEXT,
                size_bytes INTEGER,
                created_at TEXT NOT NULL,
                deleted_at TEXT
            );
            "#,
        )
        .expect("create export tables");
    }

    fn test_conn() -> (Connection, String) {
        let conn = connection::open_in_memory().expect("open in-memory connection");
        run_migrations(&conn).expect("run migrations");
        create_tables(&conn);
        let ws_id = SqliteWorkspaceRepo
            .create(&conn, "Personal")
            .expect("ws")
            .id;
        (conn, ws_id)
    }

    fn sample_snapshot() -> (String, String) {
        let selection_json =
            r#"{"items":[{"entity_type":"journal_entry","id":"j1","content_hash":"abc123"}]}"#
                .to_string();
        let fingerprint = "deadbeef".to_string();
        (selection_json, fingerprint)
    }

    #[test]
    fn job_records_artifacts_and_errors() {
        let (conn, ws_id) = test_conn();
        let (selection_json, fingerprint) = sample_snapshot();
        let job = SqliteExportJobRepo
            .create(
                &conn,
                &ws_id,
                "markdown",
                "journal",
                "Journal export",
                &Some("diary".to_string()),
                &selection_json,
                &fingerprint,
                &None,
            )
            .expect("create job");

        assert_eq!(job.selection_fingerprint.as_deref(), Some("deadbeef"));
        assert!(job.diagnostics_json.is_none());

        let artifact = SqliteExportArtifactRepo
            .create(
                &conn,
                &job.id,
                "md",
                "/tmp/exports/job/input.md",
                &Some("abc123".to_string()),
                &Some(123),
            )
            .expect("create artifact");
        assert_eq!(artifact.format, "md");

        let updated = SqliteExportJobRepo
            .transition_status(
                &conn,
                &job.id,
                ExportJobStatus::Failed,
                &Some("/tmp/exports/job/input.md".to_string()),
                &Some(time::now_rfc3339()),
                &Some("Python script not found.".to_string()),
            )
            .expect("update status");

        assert_eq!(updated.status, "failed");
        assert_eq!(
            updated.error_message.as_deref(),
            Some("Python script not found.")
        );
        assert_eq!(updated.artifacts.len(), 1);
        assert_eq!(updated.artifacts[0].format, "md");
    }

    #[test]
    fn terminal_status_cannot_transition_again() {
        let (conn, ws_id) = test_conn();
        let (selection_json, fingerprint) = sample_snapshot();
        let job = SqliteExportJobRepo
            .create(
                &conn,
                &ws_id,
                "markdown",
                "journal",
                "Journal export",
                &None,
                &selection_json,
                &fingerprint,
                &None,
            )
            .expect("create job");

        // queued -> running -> completed is the legal happy path.
        SqliteExportJobRepo
            .transition_status(
                &conn,
                &job.id,
                ExportJobStatus::Running,
                &None,
                &None,
                &None,
            )
            .expect("queued -> running");
        SqliteExportJobRepo
            .transition_status(
                &conn,
                &job.id,
                ExportJobStatus::Completed,
                &Some("/tmp/exports/job/input.md".to_string()),
                &Some(time::now_rfc3339()),
                &None,
            )
            .expect("running -> completed");

        // A completed job must not return to running or change again.
        let rerun = SqliteExportJobRepo.transition_status(
            &conn,
            &job.id,
            ExportJobStatus::Running,
            &None,
            &None,
            &None,
        );
        assert!(rerun.is_err(), "completed job must not return to running");
        assert_eq!(rerun.unwrap_err().code, crate::error::ErrorCode::Validation);

        let re_fail = SqliteExportJobRepo.transition_status(
            &conn,
            &job.id,
            ExportJobStatus::Failed,
            &None,
            &None,
            &Some("late failure".to_string()),
        );
        assert!(re_fail.is_err(), "completed job must not change again");

        // The stored status is unchanged.
        let stored = SqliteExportJobRepo.get(&conn, &job.id).expect("get job");
        assert_eq!(stored.status, "completed");
    }

    #[test]
    fn failed_job_cannot_be_rerun() {
        let (conn, ws_id) = test_conn();
        let (selection_json, fingerprint) = sample_snapshot();
        let job = SqliteExportJobRepo
            .create(
                &conn,
                &ws_id,
                "markdown",
                "journal",
                "Journal export",
                &None,
                &selection_json,
                &fingerprint,
                &None,
            )
            .expect("create job");

        SqliteExportJobRepo
            .transition_status(
                &conn,
                &job.id,
                ExportJobStatus::Running,
                &None,
                &None,
                &None,
            )
            .expect("queued -> running");
        SqliteExportJobRepo
            .transition_status(
                &conn,
                &job.id,
                ExportJobStatus::Failed,
                &None,
                &None,
                &Some("boom".to_string()),
            )
            .expect("running -> failed");

        // A failed job cannot be claimed for a rerun.
        let rerun = SqliteExportJobRepo.transition_status(
            &conn,
            &job.id,
            ExportJobStatus::Running,
            &None,
            &None,
            &None,
        );
        assert!(rerun.is_err(), "failed job must not be rerun");
    }

    #[test]
    fn illegal_transition_from_queued_to_completed_is_rejected() {
        let (conn, ws_id) = test_conn();
        let (selection_json, fingerprint) = sample_snapshot();
        let job = SqliteExportJobRepo
            .create(
                &conn,
                &ws_id,
                "markdown",
                "journal",
                "Journal export",
                &None,
                &selection_json,
                &fingerprint,
                &None,
            )
            .expect("create job");

        // queued -> completed is not a legal transition (must pass through
        // running).
        let result = SqliteExportJobRepo.transition_status(
            &conn,
            &job.id,
            ExportJobStatus::Completed,
            &None,
            &None,
            &None,
        );
        assert!(result.is_err(), "queued -> completed must be rejected");
    }

    #[test]
    fn transition_on_missing_job_is_not_found() {
        let (conn, _ws_id) = test_conn();
        let result = SqliteExportJobRepo.transition_status(
            &conn,
            "missing-job",
            ExportJobStatus::Running,
            &None,
            &None,
            &None,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, crate::error::ErrorCode::NotFound);
    }

    #[test]
    fn create_stores_resolved_snapshot_and_fingerprint() {
        let (conn, ws_id) = test_conn();
        let (selection_json, fingerprint) = sample_snapshot();
        let job = SqliteExportJobRepo
            .create(
                &conn,
                &ws_id,
                "markdown",
                "journal",
                "Journal export",
                &None,
                &selection_json,
                &fingerprint,
                &None,
            )
            .expect("create job");

        // The resolved snapshot (IDs plus hashes) is persisted verbatim, not the
        // original front-end filters.
        let stored: String = conn
            .query_row(
                "SELECT selection_json FROM export_jobs WHERE id = ?1",
                [&job.id],
                |row| row.get(0),
            )
            .expect("read selection_json");
        assert_eq!(stored, selection_json);
        assert_eq!(job.selection_fingerprint.as_deref(), Some("deadbeef"));
    }

    #[test]
    fn record_diagnostics_persists_safe_warnings() {
        let (conn, ws_id) = test_conn();
        let (selection_json, fingerprint) = sample_snapshot();
        let job = SqliteExportJobRepo
            .create(
                &conn,
                &ws_id,
                "markdown",
                "journal",
                "Journal export",
                &None,
                &selection_json,
                &fingerprint,
                &None,
            )
            .expect("create job");

        let diagnostics = r#"[{"code":"unsafe_url","severity":"warning","message":"Link uses a disallowed URL scheme and was rendered as text."}]"#;
        SqliteExportJobRepo
            .record_diagnostics(&conn, &job.id, diagnostics)
            .expect("record diagnostics");

        let reloaded = SqliteExportJobRepo.get(&conn, &job.id).expect("get job");
        assert_eq!(reloaded.diagnostics_json.as_deref(), Some(diagnostics));
    }

    #[test]
    fn deleting_export_job_removes_job_and_cascades_artifacts() {
        let (conn, ws_id) = test_conn();
        let (selection_json, fingerprint) = sample_snapshot();
        let job = SqliteExportJobRepo
            .create(
                &conn,
                &ws_id,
                "markdown",
                "journal",
                "Journal export",
                &None,
                &selection_json,
                &fingerprint,
                &None,
            )
            .expect("create job");
        SqliteExportArtifactRepo
            .create(&conn, &job.id, "md", "input.md", &None, &None)
            .expect("create artifact");

        SqliteExportJobRepo
            .delete_for_workspace(&conn, &ws_id, &job.id)
            .expect("delete job");
        assert!(SqliteExportJobRepo.get(&conn, &job.id).is_err());
        let artifact_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM export_artifacts WHERE export_job_id = ?1",
                [&job.id],
                |row| row.get(0),
            )
            .expect("count artifacts");
        assert_eq!(artifact_count, 0);
    }
}
