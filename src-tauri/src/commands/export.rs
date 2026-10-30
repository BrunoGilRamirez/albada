use std::collections::HashMap;
use std::fs;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use tauri::Manager;
use tauri::State;

use crate::commands::dto::{
    CreateExportJobDto, ExportArtifactDto, ExportJobDto, ExportPreviewDto, ExportWarningDto,
    ResolvedSelectionSnapshotDto, SaveExportArtifactCopyResultDto, TemplateCapabilityDto,
};
use crate::domain::models::ExportJob;
use crate::domain::types::{ExportJobStatus, ExportScopeType, ExportType};
use crate::domain::validation::validate_title;
use crate::error::AppError;
use crate::export::markdown::{generate as generate_markdown, MarkdownInput};
use crate::export::runner::{run_export_worker, WorkerInput};
use crate::export::selection::{
    resolve_selection, revalidate_snapshot, selection_fingerprint, to_preview_dto,
    to_resolved_selection_dto, to_snapshot_dto,
};
use crate::repos::export_artifact_repo::{ExportArtifactRepo, SqliteExportArtifactRepo};
use crate::repos::export_job_repo::{ExportJobRepo, SqliteExportJobRepo};
use crate::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};
use crate::state::Db;
use crate::time;

/// In-memory cancellation registry for running export jobs.
///
/// The worker checks the associated `AtomicBool` at phase boundaries. The
/// `cancel_export_job` command sets the flag to request cooperative cancellation.
type ExportCancellationRegistry = Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>;

/// Tauri state wrapper for the export cancellation registry.
pub struct ExportCancellationState(ExportCancellationRegistry);

impl ExportCancellationState {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }

    fn register(&self, job_id: &str) -> Result<Arc<AtomicBool>, AppError> {
        let mut map = self.0.lock().map_err(|_| AppError::unknown())?;
        let flag = Arc::new(AtomicBool::new(false));
        map.insert(job_id.to_string(), Arc::clone(&flag));
        Ok(flag)
    }

    fn cancel(&self, job_id: &str) -> Result<bool, AppError> {
        let map = self.0.lock().map_err(|_| AppError::unknown())?;
        if let Some(flag) = map.get(job_id) {
            flag.store(true, std::sync::atomic::Ordering::Release);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn unregister(&self, job_id: &str) -> Result<(), AppError> {
        let mut map = self.0.lock().map_err(|_| AppError::unknown())?;
        map.remove(job_id);
        Ok(())
    }
}

#[tauri::command]
pub fn create_export_job(
    db: State<'_, Db>,
    dto: CreateExportJobDto,
) -> Result<ExportJobDto, AppError> {
    log::info!(
        "create_export_job: begin type={} scope={} workspace={}",
        dto.export_type,
        dto.scope_type,
        dto.workspace_id
    );
    let export_type = ExportType::from_str(&dto.export_type)?;
    let scope_type = ExportScopeType::from_str(&dto.scope_type)?;
    validate_title(&dto.title)?;
    validate_template_name(&dto.template_name)?;

    db.with_tx(|conn| {
        SqliteWorkspaceRepo.get(conn, &dto.workspace_id)?;
        let selection = resolve_selection(conn, &dto.workspace_id, scope_type, &dto.selection)?;
        log::info!(
            "create_export_job: selection resolved items={} type={} scope={}",
            selection.item_count(),
            export_type,
            scope_type
        );
        if matches!(
            scope_type,
            ExportScopeType::Journal | ExportScopeType::Therapy
        ) && selection.is_empty()
        {
            return Err(AppError::validation(
                "At least one item must be selected for this export scope.",
            ));
        }

        // Bind job creation to a reviewed preview snapshot. Typst/PDF always
        // require the fingerprint; other types validate it when present. The
        // backend recomputes the fingerprint from the current records, so a
        // client can never supply an unverified fingerprint.
        let fingerprint = selection_fingerprint(&selection);
        let requires_fingerprint = matches!(export_type, ExportType::Typst | ExportType::Pdf);
        match &dto.preview_fingerprint {
            Some(provided) if provided != &fingerprint => {
                return Err(AppError::validation(
                    "The export selection changed since it was previewed; preview again.",
                ));
            }
            None if requires_fingerprint => {
                return Err(AppError::validation(
                    "A preview confirmation is required for this export type.",
                ));
            }
            _ => {}
        }

        // Persist the resolved snapshot (IDs plus content hashes), not the
        // original filters, so a stale confirmation can be detected at run time.
        let snapshot = to_snapshot_dto(&selection);
        let snapshot_json = serde_json::to_string(&snapshot)
            .map_err(|e| AppError::validation(format!("Invalid selection: {e}")))?;

        let job = SqliteExportJobRepo.create(
            conn,
            &dto.workspace_id,
            export_type.as_str(),
            scope_type.as_str(),
            &dto.title,
            &dto.template_name,
            &snapshot_json,
            &fingerprint,
            &dto.options,
        )?;
        let result = job_to_dto(job);
        log::info!(
            "create_export_job: queued job_id={} type={} scope={}",
            result.id,
            result.export_type,
            result.scope_type
        );
        Ok(result)
    })
}

/// List export jobs for a workspace, newest first, including their generated
/// artifacts so the user can reopen a previous PDF/Typst/Markdown result.
#[tauri::command]
pub fn list_export_jobs(
    db: State<'_, Db>,
    workspace_id: String,
) -> Result<Vec<ExportJobDto>, AppError> {
    crate::id::parse_uuid(&workspace_id)?;
    db.with_conn(|conn| {
        SqliteWorkspaceRepo.get(conn, &workspace_id)?;
        let jobs = SqliteExportJobRepo
            .list_for_workspace(conn, &workspace_id)
            .map(|jobs| jobs.into_iter().map(job_to_dto).collect::<Vec<_>>())?;
        log::info!(
            "list_export_jobs: workspace={} jobs={}",
            workspace_id,
            jobs.len()
        );
        Ok(jobs)
    })
}

/// Permanently remove one export job and its application-owned artifact directory.
/// The workspace and job IDs are both validated so a caller cannot delete an
/// export from another workspace or address an arbitrary filesystem path.
#[tauri::command]
pub fn delete_export_job(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    workspace_id: String,
    job_id: String,
) -> Result<(), AppError> {
    crate::id::parse_uuid(&workspace_id)?;
    crate::id::parse_uuid(&job_id)?;
    let export_dir = app.path().app_data_dir()?.join("exports").join(&job_id);

    db.with_tx(|conn| {
        SqliteWorkspaceRepo.get(conn, &workspace_id)?;
        SqliteExportJobRepo.delete_for_workspace(conn, &workspace_id, &job_id)
    })?;

    if export_dir.exists() {
        fs::remove_dir_all(export_dir)?;
    }
    log::info!(
        "delete_export_job: deleted job_id={} workspace={}",
        job_id,
        workspace_id
    );
    Ok(())
}

#[tauri::command]
pub async fn run_export_job(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    cancellation_state: State<'_, ExportCancellationState>,
    job_id: String,
) -> Result<ExportJobDto, AppError> {
    // Export directories are derived from UUID-backed job identities. Rejecting
    // malformed IDs before joining the path prevents separators or traversal
    // segments from escaping the application data directory.
    crate::id::parse_uuid(&job_id)?;

    // Application-controlled per-job export directory. Never user-supplied.
    let export_dir = app.path().app_data_dir()?.join("exports").join(&job_id);
    log::info!("run_export_job: begin job_id={}", job_id);

    // Phase 1 (short, atomic): transition the job to `running` and prepare the
    // Markdown snapshot. Markdown generation is a fast read (progress queries)
    // and must stay on the connection; Typst conversion, compilation, and all
    // filesystem output happen later on a blocking worker thread.
    let prepared_result = db.with_tx(|conn| {
        // Atomically claim the job: only a `queued` job may start. A terminal
        // job (completed/failed/cancelled) is rejected here before any work, so
        // a rerun can never produce duplicate artifacts or resurrect a terminal
        // state.
        SqliteExportJobRepo.transition_status(
            conn,
            &job_id,
            ExportJobStatus::Running,
            &None,
            &None,
            &None,
        )?;

        let job = SqliteExportJobRepo.get(conn, &job_id)?;
        let scope_type = ExportScopeType::from_str(&job.scope_type)?;
        let export_type = ExportType::from_str(&job.export_type)?;
        let selection_json = load_selection_json(conn, &job)?;
        // The stored selection is a resolved snapshot (IDs plus content hashes).
        // Old jobs persisted the original filters and fail safely here: a
        // snapshot parse failure or a content-hash mismatch stops generation
        // rather than exporting changed journal/therapy content under a stale
        // confirmation.
        let snapshot: ResolvedSelectionSnapshotDto = serde_json::from_str(&selection_json)
            .map_err(|_| {
                AppError::validation(
                    "The export selection is in an unsupported format; preview again.",
                )
            })?;
        let selection = revalidate_snapshot(conn, &job.workspace_id, scope_type, &snapshot)?;

        let markdown = generate_markdown(
            conn,
            &MarkdownInput {
                scope_type: job.scope_type.as_str(),
                title: &job.title,
            },
            &selection,
        )?;
        let compatibility_warnings = crate::export::markdown::journal_compatibility_warnings(
            job.scope_type.as_str(),
            &selection,
        );

        // Extract retain_debug_artifact from options (default false).
        let retain_debug_artifact = job
            .options_json
            .as_deref()
            .and_then(|o| serde_json::from_str::<serde_json::Value>(o).ok())
            .and_then(|v| v.get("retain_debug_artifact").cloned())
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok((
            PreparedJob {
                job_id: job.id,
                export_type,
                template_name: job.template_name,
                markdown,
                scope_type: job.scope_type,
                compatibility_warnings,
            },
            retain_debug_artifact,
        ))
    });

    // If preparation failed, record a terminal failed state (best-effort) and
    // surface the error. The running state is already committed; leaving the
    // job stuck in `running` would violate the terminal-state contract.
    let (prepared, retain_debug_artifact) = match prepared_result {
        Ok(p) => p,
        Err(err) => {
            log::error!(
                "run_export_job: preparation failed job_id={} error={:?}",
                job_id,
                err
            );
            let _ = db.with_conn(|conn| {
                SqliteExportJobRepo.transition_status(
                    conn,
                    &job_id,
                    ExportJobStatus::Failed,
                    &None,
                    &None,
                    &Some("The export could not be prepared.".to_string()),
                )
            });
            return Err(err);
        }
    };

    // Register cancellation flag for this job.
    let cancelled = cancellation_state.register(&job_id)?;

    // Phase 2: conversion, compilation, and filesystem output on a controlled
    // blocking worker. No SQLite transaction or handle is held here.
    let outcome = match tauri::async_runtime::spawn_blocking(move || {
        run_export_worker(WorkerInput {
            job_id: prepared.job_id,
            export_type: prepared.export_type,
            markdown: prepared.markdown,
            diary_profile: prepared.scope_type == "journal",
            initial_warnings: prepared.compatibility_warnings,
            template_name: prepared.template_name,
            export_dir,
            retain_debug_artifact,
            cancelled,
        })
    })
    .await
    {
        Ok(outcome) => outcome,
        Err(_) => {
            log::error!("run_export_job: worker join failed job_id={}", job_id);
            // A worker join failure must not leave a committed job in the
            // running state. Keep the user-facing error fixed and safe.
            let _ = db.with_conn(|conn| {
                SqliteExportJobRepo.transition_status(
                    conn,
                    &job_id,
                    ExportJobStatus::Failed,
                    &None,
                    &None,
                    &Some("The export worker stopped unexpectedly.".to_string()),
                )
            });
            let _ = cancellation_state.unregister(&job_id);
            return Err(AppError::unknown());
        }
    };

    // Unregister cancellation flag now that the worker has completed.
    cancellation_state.unregister(&job_id)?;
    log::info!(
        "run_export_job: worker finished job_id={} status={} artifacts={} warnings={} output={}",
        outcome.job_id,
        outcome.status,
        outcome.artifacts.len(),
        outcome.warnings.len(),
        outcome.output_path.is_some()
    );
    for warning in &outcome.warnings {
        log::warn!(
            "run_export_job: warning job_id={} code={} severity={} message={}",
            outcome.job_id,
            warning.code,
            warning.severity,
            warning.message
        );
    }

    // Phase 3 (short, atomic): persist written artifacts and the terminal
    // status. A failed compilation is persisted as `failed`, never as a
    // successful PDF. The terminal transition is conditional on `running`, so a
    // job that was cancelled or already completed while the worker ran is
    // rejected and the whole transaction (including any artifact inserts)
    // rolls back, preventing duplicate artifact rows.
    let persisted = db.with_tx(|conn| {
        for artifact in &outcome.artifacts {
            SqliteExportArtifactRepo.create(
                conn,
                &outcome.job_id,
                &artifact.format,
                &artifact.path,
                &Some(artifact.checksum.clone()),
                &Some(artifact.size_bytes),
            )?;
        }
        let generated_at = (outcome.status == ExportJobStatus::Completed).then(time::now_rfc3339);
        let diagnostics_json = serde_json::to_string(&outcome.warnings)
            .map_err(|e| AppError::database(format!("Could not encode diagnostics: {e}")))?;
        SqliteExportJobRepo.transition_status(
            conn,
            &outcome.job_id,
            outcome.status,
            &outcome.output_path,
            &generated_at,
            &outcome.error_message,
        )?;
        SqliteExportJobRepo.record_diagnostics(conn, &outcome.job_id, &diagnostics_json)?;
        SqliteExportJobRepo.get(conn, &outcome.job_id)
    });

    let job = match persisted {
        Ok(job) => {
            log::info!(
                "run_export_job: persisted job_id={} status={} artifacts={}",
                job.id,
                job.status,
                job.artifacts.len()
            );
            job
        }
        Err(err) => {
            log::error!(
                "run_export_job: persistence failed job_id={} error={:?}",
                job_id,
                err
            );
            return Err(err);
        }
    };

    Ok(job_to_dto(job))
}

/// The owned data the command needs to hand off to the blocking worker.
struct PreparedJob {
    job_id: String,
    export_type: ExportType,
    template_name: Option<String>,
    markdown: String,
    scope_type: String,
    compatibility_warnings: Vec<crate::commands::dto::ExportWarningDto>,
}

#[tauri::command]
pub fn preview_export_selection(
    db: State<'_, Db>,
    dto: CreateExportJobDto,
) -> Result<ExportPreviewDto, AppError> {
    let scope_type = ExportScopeType::from_str(&dto.scope_type)?;
    validate_template_name(&dto.template_name)?;

    db.with_conn(|conn| {
        SqliteWorkspaceRepo.get(conn, &dto.workspace_id)?;
        let selection = resolve_selection(conn, &dto.workspace_id, scope_type, &dto.selection)?;
        let all_items = to_preview_dto(&selection);
        let total_count = all_items.len() as i64;
        let truncated = total_count > 200;
        let has_therapy_private = all_items
            .iter()
            .any(|i| i.privacy_scope == "therapy_private");
        let items = all_items.into_iter().take(200).collect::<Vec<_>>();
        let resolved_selection = to_resolved_selection_dto(&selection);
        let selection_fingerprint = selection_fingerprint(&selection);
        Ok(ExportPreviewDto {
            item_count: total_count,
            total_count,
            items,
            truncated,
            resolved_selection,
            selection_fingerprint,
            has_therapy_private,
        })
    })
}

/// Local template capability snapshot (Phase 7.10.7).
///
/// Reports the shipped reviewed template name, version, validation status,
/// and unavailable resources. Reads no database and exposes no content.
#[tauri::command]
pub fn export_template_capability() -> Result<TemplateCapabilityDto, AppError> {
    let valid = crate::export::template::bundled_package_is_valid()
        && crate::export::template::resolve_template(&Some(
            crate::export::compiler::APPROVED_TEMPLATE_NAME.to_string(),
        ))
        .is_ok();
    Ok(TemplateCapabilityDto {
        name: crate::export::compiler::APPROVED_TEMPLATE_NAME.to_string(),
        version: crate::export::template::DEFAULT_TEMPLATE_VERSION.to_string(),
        valid,
        compiler_version: crate::export::compiler::EMBEDDED_COMPILER_VERSION.to_string(),
        unavailable_resources: if valid {
            vec![]
        } else {
            vec!["default template".to_string()]
        },
    })
}

/// Return the locally installed, validated journal-template catalog.
///
/// The catalog is intentionally bundled and read-only in this phase. No URL,
/// filesystem path, or executable package content crosses the IPC boundary.
#[tauri::command]
pub fn export_template_catalog() -> Result<Vec<crate::export::manifest::TemplateManifest>, AppError>
{
    let manifest = crate::export::manifest::foundation_manifest();
    if !crate::export::template::bundled_package_is_valid() {
        return Err(AppError::validation(
            "The bundled journal template is invalid.",
        ));
    }
    Ok(vec![manifest])
}

/// Validate a stored template name at the command boundary.///
/// Only the approved `default` template is supported for the MVP. Unknown or
/// path-like names (including `diary`, absolute paths, separators, and
/// traversal) are rejected here rather than silently ignored, so a job is never
/// persisted with a template that would later be dropped. `None` means direct
/// compilation and is always valid.
fn validate_template_name(template_name: &Option<String>) -> Result<(), AppError> {
    match template_name {
        None => Ok(()),
        Some(name) => crate::export::compiler::validate_template_name(name)
            .map_err(|err| AppError::validation(err.message())),
    }
}

fn load_selection_json(conn: &rusqlite::Connection, job: &ExportJob) -> Result<String, AppError> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        "SELECT selection_json FROM export_jobs WHERE id = ?1",
        [&job.id],
        |row| row.get::<_, String>(0),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("The export job selection is missing."))
}

fn job_to_dto(job: ExportJob) -> ExportJobDto {
    let warnings = job
        .diagnostics_json
        .as_deref()
        .map(parse_warnings)
        .unwrap_or_default();
    ExportJobDto {
        id: job.id,
        workspace_id: job.workspace_id,
        export_type: job.export_type,
        scope_type: job.scope_type,
        title: job.title,
        template_name: job.template_name,
        status: job.status,
        output_path: job.output_path,
        generated_at: job.generated_at,
        error_message: job.error_message,
        selection_fingerprint: job.selection_fingerprint,
        warnings,
        created_at: job.created_at,
        updated_at: job.updated_at,
        artifacts: job.artifacts.into_iter().map(artifact_to_dto).collect(),
    }
}

/// Parse the persisted diagnostics JSON into safe warning DTOs. A malformed
/// blob yields an empty list rather than failing the whole response.
fn parse_warnings(json: &str) -> Vec<ExportWarningDto> {
    serde_json::from_str::<Vec<ExportWarningDto>>(json).unwrap_or_default()
}

fn artifact_to_dto(artifact: crate::domain::models::ExportArtifact) -> ExportArtifactDto {
    ExportArtifactDto {
        id: artifact.id,
        format: artifact.format,
        path: artifact.path,
        checksum: artifact.checksum,
        size_bytes: artifact.size_bytes,
    }
}

/// Cancel a running export job.
///
/// The job must be in `queued` or `running` state. If `queued`, it transitions
/// directly to `cancelled`. If `running`, the cooperative cancellation flag is
/// set and the worker will observe it at the next phase boundary, returning a
/// `cancelled` outcome with retained Markdown and `.typ` sources. Jobs in
/// terminal states (`completed`, `failed`, `cancelled`) cannot be cancelled.
#[tauri::command]
pub async fn cancel_export_job(
    db: State<'_, Db>,
    cancellation_state: State<'_, ExportCancellationState>,
    job_id: String,
) -> Result<ExportJobDto, AppError> {
    crate::id::parse_uuid(&job_id)?;

    // Check current status and attempt cancellation.
    let job = db.with_tx(|conn| {
        let job = SqliteExportJobRepo.get(conn, &job_id)?;
        let current_status = ExportJobStatus::from_str(&job.status)?;

        match current_status {
            ExportJobStatus::Queued => {
                // Queued job: transition directly to cancelled.
                SqliteExportJobRepo.transition_status(
                    conn,
                    &job_id,
                    ExportJobStatus::Cancelled,
                    &None,
                    &None,
                    &Some("The export was cancelled before it started.".to_string()),
                )?;
            }
            ExportJobStatus::Running => {
                // Running job: set the cooperative cancellation flag.
                // The worker will observe it at the next phase boundary.
                if !cancellation_state.cancel(&job_id)? {
                    // No cancellation flag registered (worker may have already finished).
                    // Check if the job is still running; if so, we can't cancel it.
                    return Err(AppError::validation(
                        "The export job is no longer cancellable.",
                    ));
                }
                // The worker will handle the transition to cancelled when it observes the flag.
                // We don't transition here to avoid racing with the worker.
            }
            ExportJobStatus::Completed | ExportJobStatus::Failed | ExportJobStatus::Cancelled => {
                return Err(AppError::validation(
                    "The export job cannot be cancelled from its current status.",
                ));
            }
        }

        SqliteExportJobRepo.get(conn, &job_id)
    })?;

    Ok(job_to_dto(job))
}

/// Save a copy of an export artifact to a user-chosen location via native save dialog.
///
/// Validates that the workspace, job, and artifact IDs are valid UUIDs and that the
/// job belongs to the workspace and the artifact belongs to the job. Resolves the
/// source path from the stored artifact record and verifies it sits under the
/// application's per-job export directory. Opens a native save dialog with a
/// sensible default file name derived from the artifact's basename. Copies the
/// file on user confirmation. User cancellation returns `cancelled: true` without
/// error.
#[tauri::command]
pub async fn save_export_artifact_copy(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    workspace_id: String,
    job_id: String,
    artifact_id: String,
) -> Result<SaveExportArtifactCopyResultDto, AppError> {
    // Validate all IDs as UUIDs.
    crate::id::parse_uuid(&workspace_id)?;
    crate::id::parse_uuid(&job_id)?;
    crate::id::parse_uuid(&artifact_id)?;

    // Verify workspace/job/artifact scoping and fetch the artifact record.
    let artifact = db.with_conn(|conn| {
        SqliteWorkspaceRepo.get(conn, &workspace_id)?;
        let job = SqliteExportJobRepo.get(conn, &job_id)?;
        if job.workspace_id != workspace_id {
            return Err(AppError::not_found("The export job was not found."));
        }
        let artifact = SqliteExportArtifactRepo.get(conn, &artifact_id)?;
        if artifact.export_job_id != job_id {
            return Err(AppError::not_found("The export artifact was not found."));
        }
        Ok(artifact)
    })?;

    // Resolve and validate the source path: must be under app_data_dir/exports/<job_id>/.
    let app_data_dir = app.path().app_data_dir()?;
    let expected_job_dir = app_data_dir.join("exports").join(&job_id);
    let source_path = std::path::Path::new(&artifact.path);

    // Canonicalize both paths to resolve symlinks and normalize.
    let canonical_source = source_path
        .canonicalize()
        .map_err(|_| AppError::io("The artifact file could not be accessed."))?;
    let canonical_job_dir = expected_job_dir
        .canonicalize()
        .map_err(|_| AppError::io("The export directory could not be accessed."))?;

    // Verify the source is within the expected per-job directory.
    if !canonical_source.starts_with(&canonical_job_dir) {
        log::warn!(
            "save_export_artifact_copy: artifact path escapes job directory job_id={}",
            job_id
        );
        return Err(AppError::validation(
            "The artifact file is not in the expected location.",
        ));
    }

    // Derive a safe default file name from the artifact's basename.
    let default_file_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("export")
        .to_string();

    // Prepare document directory for the dialog (optional).
    let document_dir = app.path().document_dir().ok();

    // Open native save dialog (blocking collection).
    let dialog_result = tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_dialog::DialogExt;
        let mut dialog = app.dialog().file().set_file_name(&default_file_name);
        if let Some(dir) = document_dir {
            dialog = dialog.set_directory(dir);
        }
        dialog.blocking_save_file()
    })
    .await
    .map_err(|_| AppError::unknown())?;

    match dialog_result {
        Some(dest_path) => {
            // Copy the file. dest_path is rfd::FilePath; use its as_path() method.
            let dest_path_buf = dest_path
                .as_path()
                .map(|p| p.to_path_buf())
                .ok_or_else(|| AppError::io("The selected destination path is invalid."))?;
            std::fs::copy(&canonical_source, &dest_path_buf)?;
            let size = std::fs::metadata(&dest_path_buf)?.len() as i64;
            log::info!(
                "save_export_artifact_copy: copied artifact_id={} size={}",
                artifact_id,
                size
            );
            Ok(SaveExportArtifactCopyResultDto {
                cancelled: false,
                destination_path: Some(dest_path_buf.to_string_lossy().to_string()),
                size_bytes: Some(size),
            })
        }
        None => {
            // User cancelled the dialog.
            log::info!(
                "save_export_artifact_copy: user cancelled artifact_id={}",
                artifact_id
            );
            Ok(SaveExportArtifactCopyResultDto {
                cancelled: true,
                destination_path: None,
                size_bytes: None,
            })
        }
    }
}

#[cfg(test)]
mod save_export_artifact_copy_tests {
    use crate::db::connection;
    use crate::db::migrations::run_migrations;
    use crate::repos::export_artifact_repo::{ExportArtifactRepo, SqliteExportArtifactRepo};
    use crate::repos::export_job_repo::{ExportJobRepo, SqliteExportJobRepo};
    use crate::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};
    use std::fs;
    use std::path::PathBuf;

    fn create_test_setup() -> (rusqlite::Connection, String, String, String, PathBuf) {
        let conn = connection::open_in_memory().expect("open in-memory connection");
        run_migrations(&conn).expect("run migrations");

        // Create workspace
        let ws_id = SqliteWorkspaceRepo
            .create(&conn, "Test Workspace")
            .expect("create workspace")
            .id;

        // Create export job
        let (selection_json, fingerprint) = sample_snapshot();
        let job = SqliteExportJobRepo
            .create(
                &conn,
                &ws_id,
                "markdown",
                "journal",
                "Test Export",
                &None,
                &selection_json,
                &fingerprint,
                &None,
            )
            .expect("create job");
        let job_id = job.id;

        // Create a temporary directory to simulate the export directory
        let temp_dir = std::env::temp_dir().join(format!("joi_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("create temp dir");
        let export_dir = temp_dir.join("exports").join(&job_id);
        fs::create_dir_all(&export_dir).expect("create export dir");

        // Create a test artifact file
        let artifact_file = export_dir.join("test-export.md");
        fs::write(&artifact_file, "# Test Export\n\nContent").expect("write artifact file");

        // Create artifact record
        let artifact = SqliteExportArtifactRepo
            .create(
                &conn,
                &job_id,
                "md",
                artifact_file.to_str().unwrap(),
                &Some("abc123".to_string()),
                &Some(100),
            )
            .expect("create artifact");
        let artifact_id = artifact.id;

        (conn, ws_id, job_id, artifact_id, export_dir)
    }

    fn sample_snapshot() -> (String, String) {
        let selection_json =
            r#"{"items":[{"entity_type":"journal_entry","id":"j1","content_hash":"abc123"}]}"#
                .to_string();
        let fingerprint = "deadbeef".to_string();
        (selection_json, fingerprint)
    }

    #[test]
    fn artifact_path_escaping_job_dir_is_rejected() {
        let conn = connection::open_in_memory().expect("open in-memory connection");
        run_migrations(&conn).expect("run migrations");

        let ws_id = SqliteWorkspaceRepo
            .create(&conn, "Test Workspace")
            .expect("create workspace")
            .id;

        let (selection_json, fingerprint) = sample_snapshot();
        let job = SqliteExportJobRepo
            .create(
                &conn,
                &ws_id,
                "markdown",
                "journal",
                "Test Export",
                &None,
                &selection_json,
                &fingerprint,
                &None,
            )
            .expect("create job");
        let job_id = job.id;

        // Create an artifact record with a path OUTSIDE the job directory
        let outside_dir =
            std::env::temp_dir().join(format!("joi_test_outside_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&outside_dir).expect("create outside dir");
        let outside_file = outside_dir.join("malicious.md");
        fs::write(&outside_file, "malicious").expect("write file");

        let artifact = SqliteExportArtifactRepo
            .create(
                &conn,
                &job_id,
                "md",
                outside_file.to_str().unwrap(),
                &None,
                &None,
            )
            .expect("create artifact");
        let _artifact_id = artifact.id;

        // The validation logic: canonicalize and check if it starts with job dir
        let app_data_dir = std::env::temp_dir(); // Use temp as mock app_data_dir
        let expected_job_dir = app_data_dir.join("exports").join(&job_id);
        fs::create_dir_all(&expected_job_dir).expect("create expected job dir");

        let source_path = PathBuf::from(&artifact.path);
        let canonical_source = source_path.canonicalize().expect("canonicalize source");
        let canonical_job_dir = expected_job_dir
            .canonicalize()
            .expect("canonicalize job dir");

        // This should be false (path escapes)
        assert!(
            !canonical_source.starts_with(&canonical_job_dir),
            "Path should escape the job directory"
        );

        // Cleanup
        let _ = fs::remove_dir_all(outside_dir);
        let _ = fs::remove_dir_all(expected_job_dir);
    }

    #[test]
    fn workspace_job_artifact_scoping_mismatch_rejected() {
        let conn = connection::open_in_memory().expect("open in-memory connection");
        run_migrations(&conn).expect("run migrations");

        // Create two workspaces
        let ws1_id = SqliteWorkspaceRepo
            .create(&conn, "Workspace 1")
            .expect("create workspace 1")
            .id;
        let ws2_id = SqliteWorkspaceRepo
            .create(&conn, "Workspace 2")
            .expect("create workspace 2")
            .id;

        let (selection_json, fingerprint) = sample_snapshot();

        // Create job in workspace 1
        let job1 = SqliteExportJobRepo
            .create(
                &conn,
                &ws1_id,
                "markdown",
                "journal",
                "Test Export 1",
                &None,
                &selection_json,
                &fingerprint,
                &None,
            )
            .expect("create job 1");
        let job1_id = job1.id;

        // Create job in workspace 2
        let job2 = SqliteExportJobRepo
            .create(
                &conn,
                &ws2_id,
                "markdown",
                "journal",
                "Test Export 2",
                &None,
                &selection_json,
                &fingerprint,
                &None,
            )
            .expect("create job 2");
        let job2_id = job2.id;

        // Create artifact for job1
        let export_dir1 =
            std::env::temp_dir().join(format!("joi_test_export1_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&export_dir1).expect("create export dir 1");
        let artifact_file = export_dir1.join("test.md");
        fs::write(&artifact_file, "content").expect("write artifact");

        let artifact = SqliteExportArtifactRepo
            .create(
                &conn,
                &job1_id,
                "md",
                artifact_file.to_str().unwrap(),
                &None,
                &None,
            )
            .expect("create artifact");
        let artifact_id = artifact.id;

        // Verify: artifact belongs to job1, not job2
        let fetched_artifact = SqliteExportArtifactRepo
            .get(&conn, &artifact_id)
            .expect("get artifact");
        assert_eq!(fetched_artifact.export_job_id, job1_id);
        assert_ne!(fetched_artifact.export_job_id, job2_id);

        // Verify: job1 belongs to ws1, not ws2
        let fetched_job1 = SqliteExportJobRepo.get(&conn, &job1_id).expect("get job1");
        assert_eq!(fetched_job1.workspace_id, ws1_id);
        assert_ne!(fetched_job1.workspace_id, ws2_id);

        // Verify: job2 belongs to ws2
        let fetched_job2 = SqliteExportJobRepo.get(&conn, &job2_id).expect("get job2");
        assert_eq!(fetched_job2.workspace_id, ws2_id);

        // Cleanup
        let _ = fs::remove_dir_all(export_dir1);
    }

    #[test]
    fn artifact_path_within_job_dir_is_accepted() {
        let (conn, _ws_id, _job_id, artifact_id, export_dir) = create_test_setup();

        let artifact = SqliteExportArtifactRepo
            .get(&conn, &artifact_id)
            .expect("get artifact");
        let source_path = PathBuf::from(&artifact.path);

        let canonical_source = source_path.canonicalize().expect("canonicalize source");
        let canonical_job_dir = export_dir.canonicalize().expect("canonicalize job dir");

        // This should be true (path is within job directory)
        assert!(
            canonical_source.starts_with(&canonical_job_dir),
            "Path should be within the job directory"
        );
    }

    #[test]
    fn artifact_basename_used_for_default_filename() {
        let (conn, _ws_id, _job_id, artifact_id, _export_dir) = create_test_setup();

        let artifact = SqliteExportArtifactRepo
            .get(&conn, &artifact_id)
            .expect("get artifact");
        let source_path = PathBuf::from(&artifact.path);

        let default_file_name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("export")
            .to_string();

        assert_eq!(default_file_name, "test-export.md");
    }
}
