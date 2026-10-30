use crate::commands::dto::{
    CommitLegacyImportDto, EntityCountDto, ImportBatchStatusDto, ImportReportDto,
    ImportReviewItemDto, ImportWarningDto, RunLegacyImportDto,
};
use crate::error::AppError;
use crate::import::executor::{
    commit_legacy_import as executor_commit_legacy_import, preview_import, run_import,
    run_import_worker,
};
use crate::import::import_report::ImportReport;
use crate::repos::import_batch_repo::{ImportBatchRepo, SqliteImportBatchRepo};
use crate::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};
use crate::state::Db;
use tauri::State;

#[tauri::command]
pub fn run_legacy_import(
    db: State<'_, Db>,
    dto: RunLegacyImportDto,
) -> Result<ImportReportDto, AppError> {
    crate::log_command!(
        "run_legacy_import",
        db.with_tx(|conn| run_legacy_import_inner(conn, &dto))
    )
}

/// Start a non-blocking legacy import.
///
/// Creates one import batch (and records the workspace root path) in a short
/// transaction, returns the batch ID immediately, then runs the long
/// discovery/import transaction on a controlled blocking worker. The Tauri
/// command call and the React thread are never held open for the discovery
/// work; the caller polls [`get_import_batch_status`] until the batch reaches a
/// terminal state.
#[tauri::command]
pub async fn start_legacy_import(
    db: State<'_, Db>,
    dto: RunLegacyImportDto,
) -> Result<ImportBatchStatusDto, AppError> {
    validate_source_root(&dto.source_root)?;

    // Short, atomic phase: ensure the workspace, record the source root for
    // provenance, and create the batch. Returns immediately.
    let (batch_id, workspace_id, started_at) = db.with_tx(|conn| {
        let workspace = SqliteWorkspaceRepo.ensure_default(conn)?;
        SqliteWorkspaceRepo.set_root_path(conn, &workspace.id, &dto.source_root)?;
        let batch = SqliteImportBatchRepo.create(conn, &workspace.id, &dto.source_root)?;
        Ok((batch.id, workspace.id, batch.started_at))
    })?;

    // Hand a clone of the shared connection handle to the worker. The worker
    // owns this clone and never borrows Tauri `State`.
    let worker_db = db.clone_handle();
    let source_root = dto.source_root.clone();
    let include_paths = dto.include_paths.clone();
    let worker_batch_id = batch_id.clone();

    tauri::async_runtime::spawn_blocking(move || {
        run_import_worker(
            worker_db,
            workspace_id,
            source_root,
            include_paths,
            worker_batch_id,
        );
    });

    // Return the status already obtained while creating the batch. Do not
    // reacquire the database lock after spawning the worker: the worker may
    // have acquired it already, and doing so would make this supposedly
    // immediate command wait for the complete import transaction.
    Ok(ImportBatchStatusDto {
        batch_id,
        status: "running".to_string(),
        started_at,
        completed_at: None,
    })
}

/// Read-only discovery and classification preview. No import batch or entity
/// rows are created by this command.
#[tauri::command]
pub fn preview_legacy_import(
    db: State<'_, Db>,
    dto: RunLegacyImportDto,
) -> Result<ImportReportDto, AppError> {
    validate_source_root(&dto.source_root)?;
    db.with_conn(|conn| {
        let workspace = SqliteWorkspaceRepo.ensure_default(conn)?;
        preview_import(
            &workspace.id,
            &dto.source_root,
            dto.include_paths.as_deref(),
        )
        .map(|report| to_report_dto(&report))
    })
}

/// Reviewed-decision commit boundary for legacy import.
///
/// The caller must provide the exact `preview_fingerprint` returned by
/// `preview_legacy_import` and an explicit decision for every review item
/// from that preview. The backend recomputes the fingerprint and content
/// hashes; any mismatch (stale preview, missing/changed source file, or
/// incomplete decisions) causes the commit to be rejected. Therapy content
/// is only imported when the decision explicitly chooses `therapy_record`;
/// path-derived therapy classification is never auto-applied.
///
/// The commit runs atomically in a single transaction. On success, entities
/// are created with provenance and the batch is marked `completed`. On
/// failure, the transaction rolls back and the batch is marked `failed`
/// with a content-safe summary.
#[tauri::command]
pub fn commit_legacy_import(
    db: State<'_, Db>,
    dto: CommitLegacyImportDto,
) -> Result<ImportReportDto, AppError> {
    crate::log_command!(
        "commit_legacy_import",
        db.with_tx(|conn| commit_legacy_import_inner(conn, &dto))
    )
}

/// Core of `commit_legacy_import`, separated for testability. Runs inside the
/// caller's transaction.
fn commit_legacy_import_inner(
    conn: &rusqlite::Connection,
    dto: &CommitLegacyImportDto,
) -> Result<ImportReportDto, AppError> {
    let workspace = SqliteWorkspaceRepo.ensure_default(conn)?;
    if dto.workspace_id.trim().is_empty() {
        return Err(AppError::validation("workspace_id is required."));
    }
    if dto.workspace_id != workspace.id {
        return Err(AppError::validation(
            "The commit request references an unknown workspace.",
        ));
    }
    validate_source_root(&dto.source_root)?;
    SqliteWorkspaceRepo.set_root_path(conn, &workspace.id, &dto.source_root)?;
    let report = executor_commit_legacy_import(
        conn,
        &workspace.id,
        &dto.source_root,
        dto.include_paths.as_deref(),
        &dto.preview_fingerprint,
        &dto.decisions,
    )?;
    Ok(to_report_dto(&report))
}

/// Core of `run_legacy_import`, separated for testability. Runs inside the
/// caller's transaction.
fn run_legacy_import_inner(
    conn: &rusqlite::Connection,
    dto: &RunLegacyImportDto,
) -> Result<ImportReportDto, AppError> {
    let workspace = SqliteWorkspaceRepo.ensure_default(conn)?;
    // Persist the selected source root for provenance and future explicit local
    // consumers. Source files are never modified.
    validate_source_root(&dto.source_root)?;
    SqliteWorkspaceRepo.set_root_path(conn, &workspace.id, &dto.source_root)?;
    let report = run_import(
        conn,
        &workspace.id,
        &dto.source_root,
        dto.include_paths.as_deref(),
    )?;
    Ok(to_report_dto(&report))
}

/// Validate that the import source root is an existing directory. This is a
/// read-only check; it never creates, moves, or modifies source files.
fn validate_source_root(source_root: &str) -> Result<(), AppError> {
    let path = std::path::Path::new(source_root);
    if !path.is_dir() {
        return Err(AppError::validation(
            "The import source path is not a directory.",
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn get_import_batch(db: State<'_, Db>, batch_id: String) -> Result<ImportReportDto, AppError> {
    db.with_conn(|conn| {
        let batch = SqliteImportBatchRepo.get(conn, &batch_id)?;
        let summary = batch
            .summary_json
            .as_deref()
            .map(serde_json::from_str::<crate::import::import_report::ImportSummary>)
            .transpose()
            .map_err(|_| AppError::database("The import summary could not be read."))?
            .unwrap_or(crate::import::import_report::ImportSummary {
                counts: Vec::new(),
                warnings: Vec::new(),
                review_items: Vec::new(),
                preview_fingerprint: String::new(),
            });

        Ok(ImportReportDto {
            batch_id: batch.id,
            source_root: batch.source_root,
            status: batch.status,
            started_at: batch.started_at,
            completed_at: batch.completed_at,
            counts: summary
                .counts
                .into_iter()
                .map(|c| EntityCountDto {
                    entity_type: c.entity_type,
                    count: c.count,
                })
                .collect(),
            warnings: summary
                .warnings
                .into_iter()
                .map(|w| ImportWarningDto {
                    severity: w.severity,
                    source_path: w.source_path,
                    line: w.line,
                    message: w.message,
                })
                .collect(),
            review_items: summary
                .review_items
                .into_iter()
                .map(|r| ImportReviewItemDto {
                    id: r.id,
                    source_path: r.source_path,
                    proposed_entity_type: r.proposed_entity_type,
                    title: r.title,
                    snippet: r.snippet,
                    confidence: r.confidence,
                    source_hash: r.source_hash,
                })
                .collect(),
            preview_fingerprint: summary.preview_fingerprint,
        })
    })
}

/// Poll the status of an import batch without exposing any source content.
///
/// This is the safe polling contract for an in-flight import: it returns only
/// the batch ID, status, and timestamps. The full report (counts, warnings,
/// review items) is available from [`get_import_batch`] once the batch reaches
/// a terminal state.
#[tauri::command]
pub fn get_import_batch_status(
    db: State<'_, Db>,
    batch_id: String,
) -> Result<ImportBatchStatusDto, AppError> {
    db.with_conn(|conn| {
        let batch = SqliteImportBatchRepo.get(conn, &batch_id)?;
        Ok(ImportBatchStatusDto {
            batch_id: batch.id,
            status: batch.status,
            started_at: batch.started_at,
            completed_at: batch.completed_at,
        })
    })
}

fn to_report_dto(report: &ImportReport) -> ImportReportDto {
    ImportReportDto {
        batch_id: report.batch_id.clone(),
        source_root: report.source_root.clone(),
        status: report.status.clone(),
        started_at: report.started_at.clone(),
        completed_at: report.completed_at.clone(),
        counts: report
            .counts
            .iter()
            .map(|c| EntityCountDto {
                entity_type: c.entity_type.clone(),
                count: c.count,
            })
            .collect(),
        warnings: report
            .warnings
            .iter()
            .map(|w| ImportWarningDto {
                severity: w.severity.clone(),
                source_path: w.source_path.clone(),
                line: w.line,
                message: w.message.clone(),
            })
            .collect(),
        review_items: report
            .review_items
            .iter()
            .map(|r| ImportReviewItemDto {
                id: r.id.clone(),
                source_path: r.source_path.clone(),
                proposed_entity_type: r.proposed_entity_type.clone(),
                title: r.title.clone(),
                snippet: r.snippet.clone(),
                confidence: r.confidence.clone(),
                source_hash: r.source_hash.clone(),
            })
            .collect(),
        preview_fingerprint: report.preview_fingerprint.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_conn;
    use std::fs;
    use std::path::PathBuf;

    fn fixture_root() -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("joi-import-cmd-test-{}", crate::id::new_uuid()));
        dir
    }

    #[test]
    fn import_sets_workspace_root_path() {
        let root = fixture_root();
        fs::create_dir_all(&root).expect("create source root");
        fs::write(root.join("entry.md"), "# Diario\n\nContenido sintético.\n")
            .expect("write fixture");

        let conn = test_conn();
        let dto = RunLegacyImportDto {
            source_root: root.to_string_lossy().to_string(),
            include_paths: None,
        };

        let report = run_legacy_import_inner(&conn, &dto).expect("run import");
        assert_eq!(report.status, "completed");

        // The default workspace now records the source root so exports can
        // locate the project Python virtualenv.
        let workspace = SqliteWorkspaceRepo
            .ensure_default(&conn)
            .expect("ensure default");
        assert_eq!(
            workspace.root_path.as_deref(),
            Some(root.to_string_lossy().as_ref())
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn import_rejects_non_directory_source_root() {
        let conn = test_conn();
        let dto = RunLegacyImportDto {
            source_root: "C:/definitely/not/a/real/directory".to_string(),
            include_paths: None,
        };
        let result = run_legacy_import_inner(&conn, &dto);
        assert!(result.is_err());
    }
}
