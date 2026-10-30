//! Controlled export worker (Phase 7.10 partial slice).
//!
//! This module owns the blocking, non-database work of an export job:
//! filesystem output, Markdown -> Typst conversion through the AST API, and
//! embedded Typst compilation. It is designed to run on a `spawn_blocking`
//! worker boundary, outside any SQLite transaction and off the Tauri command
//! thread.
//!
//! # Ownership boundary
//!
//! Every input is passed in as an owned value ([`WorkerInput`]); no `Connection`
//! or `State` crosses into this module. All filesystem writes are confined to
//! the caller-supplied, application-controlled per-job directory, and each write
//! is staged to a temporary file and atomically renamed into place so a
//! partially written file is never observed as a finished artifact.
//!
//! # Failure contract
//!
//! The worker is infallible at the top level: it always returns a terminal
//! [`RunOutcome`] (completed or failed) so the caller can persist a terminal
//! status. Markdown is always retained as the source artifact. On Typst/PDF
//! failure the `.typ` source is retained and no PDF is produced; a PDF is only
//! written after a fully successful compile, so a partial PDF can never be
//! reported as successful. Error messages are fixed, content-safe strings that
//! never contain source text, diagnostic text, or filesystem paths.
//!
//! # Limits enforcement
//!
//! The 30-second compile timeout is enforced by wrapping the Typst compilation
//! in a bounded blocking task. The Typst 0.15.1 library API has no cancellation
//! hook, so the timeout operates at the worker boundary: if the compile exceeds
//! the deadline, the worker returns a timeout failure and the partial output is
//! discarded. The caller persists a terminal `failed` status.
//!
//! # Cancellation
//!
//! A cooperative cancellation flag is checked at phase boundaries (after Markdown
//! generation, after Typst conversion, before and after compilation). If the job
//! was cancelled while the worker was running, the worker returns a `cancelled`
//! outcome with retained Markdown and `.typ` sources, and no PDF artifact.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::commands::dto::ExportWarningDto;
use crate::domain::types::{ExportJobStatus, ExportType};
use crate::export::ast::ConversionWarning;
use crate::export::compiler::{
    self, CompileDiagnostic, CompileError, CompileOutcome, CompileRequest, Severity,
};
use crate::export::diary;
use crate::export::template::{apply_template, resolve_template};
use crate::export::typst::{convert_diary_markdown, convert_markdown};

/// Compile timeout enforced at the worker boundary (30 seconds).
const COMPILE_TIMEOUT: Duration = Duration::from_secs(30);
const RETAIN_TYPST_ENV: &str = "JOI_EXPORT_RETAIN_TYPST";

/// Input owned by the worker; no database handle crosses this boundary.
pub struct WorkerInput {
    pub job_id: String,
    pub export_type: ExportType,
    pub markdown: String,
    pub diary_profile: bool,
    /// Safe compatibility diagnostics assembled from the resolved selection.
    pub initial_warnings: Vec<ExportWarningDto>,
    /// The stored template name, or `None` for direct compilation. Only the
    /// approved `default` template is supported; the command boundary rejects
    /// unknown or path-like names before a job is persisted.
    pub template_name: Option<String>,
    pub export_dir: PathBuf,
    /// When true, retain intermediate artifacts (`.typ` source, diagnostics)
    /// on successful completion for debugging. Default is false (clean on success).
    pub retain_debug_artifact: bool,
    /// Cooperative cancellation flag. The worker checks this at phase boundaries.
    pub cancelled: Arc<AtomicBool>,
}

/// A file produced by the worker, ready to be persisted as an export artifact.
#[derive(Debug, Clone)]
pub struct WrittenArtifact {
    pub format: String,
    pub path: String,
    pub checksum: String,
    pub size_bytes: i64,
}

/// The terminal outcome of a worker run.
#[derive(Debug)]
pub struct RunOutcome {
    pub job_id: String,
    pub status: ExportJobStatus,
    pub output_path: Option<String>,
    pub error_message: Option<String>,
    pub artifacts: Vec<WrittenArtifact>,
    /// Safe, structured warnings (conversion and compiler diagnostics). Never
    /// contain source text, therapy content, absolute paths, or secrets.
    pub warnings: Vec<ExportWarningDto>,
}

/// Run the blocking portion of an export job and return its terminal outcome.
///
/// This function is `Send`-safe and owns all of its inputs; it performs no
/// database access.
pub fn run_export_worker(input: WorkerInput) -> RunOutcome {
    log::info!(
        "export_worker: start job_id={} type={} template={} markdown_bytes={}",
        input.job_id,
        input.export_type,
        input.template_name.as_deref().unwrap_or("none"),
        input.markdown.len()
    );
    if let Err(message) = ensure_output_dir(&input.export_dir) {
        log::error!(
            "export_worker: output directory failed job_id={} error={}",
            input.job_id,
            message
        );
        return failed(&input.job_id, message, Vec::new(), input.initial_warnings);
    }

    // Markdown is always retained as the source snapshot.
    let md_path = input.export_dir.join("input.md");
    let md_artifact = match write_artifact(&md_path, input.markdown.as_bytes(), "md") {
        Ok(artifact) => {
            log::info!(
                "export_worker: markdown written job_id={} bytes={}",
                input.job_id,
                artifact.size_bytes
            );
            artifact
        }
        Err(message) => {
            log::error!(
                "export_worker: markdown write failed job_id={} error={}",
                input.job_id,
                message
            );
            return failed(&input.job_id, message, Vec::new(), input.initial_warnings);
        }
    };

    // Cancellation check after Markdown generation.
    if input.cancelled.load(Ordering::Acquire) {
        return cancelled(
            &input.job_id,
            vec![md_artifact],
            input.initial_warnings.clone(),
        );
    }

    match input.export_type {
        ExportType::Markdown => {
            log::info!(
                "export_worker: markdown-only complete job_id={}",
                input.job_id
            );
            RunOutcome {
                job_id: input.job_id,
                status: ExportJobStatus::Completed,
                output_path: Some(path_string(&md_path)),
                error_message: None,
                artifacts: vec![md_artifact],
                warnings: input.initial_warnings.clone(),
            }
        }
        ExportType::Typst | ExportType::Pdf => run_typst_or_pdf(&input, md_artifact),
        // Timesheet and metrics exports are not part of this slice; retain the
        // Markdown source and fail explicitly rather than silently treating the
        // job as a Markdown export.
        _ => failed(
            &input.job_id,
            "That export type is not supported yet.",
            vec![md_artifact],
            input.initial_warnings.clone(),
        ),
    }
}

/// Convert the generated Markdown through the AST, compile the resulting Typst,
/// and write the deliverable. Retains Markdown and `.typ` on failure.
fn run_typst_or_pdf(input: &WorkerInput, md_artifact: WrittenArtifact) -> RunOutcome {
    log::info!(
        "export_worker: markdown-to-typst begin job_id={}",
        input.job_id
    );
    let normalization = if input.diary_profile {
        let rules = crate::export::manifest::foundation_manifest().cleanup;
        Some(diary::normalize_with_rules(&input.markdown, &rules))
    } else {
        None
    };
    let normalized = normalization
        .as_ref()
        .map(|result| result.markdown.clone())
        .unwrap_or_else(|| input.markdown.clone());
    let mut warnings = input.initial_warnings.clone();
    let mut normalized_artifact = None;
    if input.diary_profile {
        let normalized_path = input.export_dir.join("normalized.md");
        if let Ok(artifact) =
            write_artifact(&normalized_path, normalized.as_bytes(), "normalized_md")
        {
            warnings.push(ExportWarningDto {
                code: "diary_normalized".to_string(),
                severity: "info".to_string(),
                message: normalization
                    .as_ref()
                    .map(|result| {
                        format!(
                            "Diary normalization removed {} comments, {} prompts, {} placeholders, and {} images; skipped {} empty-day markers and {} entries without content.",
                            result.stats.removed_comments,
                            result.stats.removed_prompts,
                            result.stats.removed_placeholders,
                            result.stats.removed_images,
                            result.stats.removed_empty_day_markers,
                            result.stats.removed_empty_entries,
                        )
                    })
                    .unwrap_or_else(|| "Diary normalization completed.".to_string()),
                source: Some("normalized.md".to_string()),
                phase: Some("normalization".to_string()),
                category: Some("diary_compatibility".to_string()),
                line: None,
                column: None,
            });
            normalized_artifact = Some(artifact);
        }
    }
    let conversion = if input.diary_profile {
        convert_diary_markdown(&normalized)
    } else {
        convert_markdown(&normalized)
    };
    warnings.extend(conversion_warnings(&conversion.warnings));
    log::info!(
        "export_worker: markdown-to-typst complete job_id={} source_lines={} source_bytes={} typst_bytes={} warnings={}",
        input.job_id,
        normalized.lines().count(),
        normalized.len(),
        conversion.typst.len(),
        warnings.len()
    );
    log_warnings(&input.job_id, &warnings);

    // Cancellation check after Typst conversion.
    if input.cancelled.load(Ordering::Acquire) {
        return cancelled(
            &input.job_id,
            artifacts_with_normalized(&md_artifact, normalized_artifact.as_ref()),
            warnings,
        );
    }

    // Resolve the stored template name into the approved compiler template.
    // Only `default` is supported; unknown or path-like names are rejected here
    // as a last line of defense (the command boundary rejects them earlier).
    let template = match resolve_template(&input.template_name) {
        Ok(template) => {
            log::info!(
                "export_worker: template resolved job_id={} template={}",
                input.job_id,
                input.template_name.as_deref().unwrap_or("none")
            );
            template
        }
        Err(err) => {
            log::error!(
                "export_worker: template failed job_id={} error={}",
                input.job_id,
                err.message()
            );
            return failed(
                &input.job_id,
                err.message(),
                artifacts_with_normalized(&md_artifact, normalized_artifact.as_ref()),
                warnings,
            );
        }
    };

    // Apply the template: when present, the generated source imports the fixed
    // compiler TEMPLATE_PATH; otherwise the body is compiled directly.
    let source = apply_template(&conversion.typst, &template);

    let typ_path = input.export_dir.join("input.typ");
    let typ_artifact = match write_artifact(&typ_path, source.as_bytes(), "typ") {
        Ok(artifact) => {
            log::info!(
                "export_worker: typst source written job_id={} bytes={}",
                input.job_id,
                artifact.size_bytes
            );
            artifact
        }
        Err(message) => {
            log::error!(
                "export_worker: typst source write failed job_id={} error={}",
                input.job_id,
                message
            );
            return failed(
                &input.job_id,
                message,
                artifacts_with_normalized(&md_artifact, normalized_artifact.as_ref()),
                warnings,
            );
        }
    };

    // Cancellation check before compilation.
    if input.cancelled.load(Ordering::Acquire) {
        return cancelled(
            &input.job_id,
            artifacts_with_normalized_and_typ(
                &md_artifact,
                normalized_artifact.as_ref(),
                &typ_artifact,
            ),
            warnings,
        );
    }

    let request = CompileRequest {
        source,
        template,
        attachments: Vec::new(),
    };

    // Compile with a 30-second timeout enforced at the worker boundary.
    // The Typst 0.15.1 library API has no cancellation hook, so we run the
    // compile in a separate blocking task and apply the timeout there.
    let compile_result = compile_with_timeout(request);
    log::info!("export_worker: compiler finished job_id={}", input.job_id);

    // Cancellation check after compilation.
    if input.cancelled.load(Ordering::Acquire) {
        return cancelled(&input.job_id, vec![md_artifact, typ_artifact], warnings);
    }

    let (pdf, compile_warnings) = match compile_result {
        Err(err) => {
            log::error!(
                "export_worker: compiler infrastructure failed job_id={} error={}",
                input.job_id,
                err.message()
            );
            let artifacts = with_diagnostics(
                artifacts_with_normalized_and_typ(
                    &md_artifact,
                    normalized_artifact.as_ref(),
                    &typ_artifact,
                ),
                &input.export_dir,
                &warnings,
            );
            return failed(&input.job_id, err.message(), artifacts, warnings);
        }
        Ok(CompileOutcome::Failure { diagnostics }) => {
            warnings.extend(diagnostics.into_iter().map(compile_warning));
            log::error!(
                "export_worker: typst compilation failed job_id={} diagnostics={}",
                input.job_id,
                warnings.len()
            );
            log_warnings(&input.job_id, &warnings);
            let artifacts = with_diagnostics(
                artifacts_with_normalized_and_typ(
                    &md_artifact,
                    normalized_artifact.as_ref(),
                    &typ_artifact,
                ),
                &input.export_dir,
                &warnings,
            );
            return failed(
                &input.job_id,
                "The document could not be compiled.",
                artifacts,
                warnings,
            );
        }
        Ok(CompileOutcome::Success { pdf, warnings: w }) => (pdf, w),
    };
    warnings.extend(compile_warnings.into_iter().map(compile_warning));
    log::info!(
        "export_worker: diagnostics summary job_id={} errors={} warnings={} info={}",
        input.job_id,
        warnings
            .iter()
            .filter(|warning| warning.severity == "error")
            .count(),
        warnings
            .iter()
            .filter(|warning| warning.severity == "warning")
            .count(),
        warnings
            .iter()
            .filter(|warning| warning.severity == "info")
            .count(),
    );
    log_warnings(&input.job_id, &warnings);

    if matches!(input.export_type, ExportType::Typst) {
        log::info!(
            "export_worker: typst export complete job_id={}",
            input.job_id
        );
        // A Typst export delivers the `.typ` source; successful compilation
        // above is validation that the generated source is well-formed.
        // The `.typ` file is the primary deliverable and is always retained.
        let artifacts = artifacts_with_normalized_and_typ(
            &md_artifact,
            normalized_artifact.as_ref(),
            &typ_artifact,
        );
        return RunOutcome {
            job_id: input.job_id.clone(),
            status: ExportJobStatus::Completed,
            output_path: Some(path_string(&typ_path)),
            error_message: None,
            artifacts,
            warnings,
        };
    }

    let pdf_path = input.export_dir.join("output.pdf");
    match write_artifact(&pdf_path, &pdf, "pdf") {
        Ok(pdf_artifact) => {
            log::info!(
                "export_worker: pdf written job_id={} bytes={}",
                input.job_id,
                pdf_artifact.size_bytes
            );
            let mut artifacts = artifacts_with_normalized_and_typ(
                &md_artifact,
                normalized_artifact.as_ref(),
                &typ_artifact,
            );
            artifacts.push(pdf_artifact.clone());
            // Clean up .typ on success unless retain_debug_artifact is set.
            // For PDF exports, .typ is an intermediate artifact.
            if !should_retain_debug_artifact(input.retain_debug_artifact) {
                let _ = std::fs::remove_file(&typ_path);
                artifacts.retain(|artifact| {
                    artifact.format != "typ" && artifact.format != "normalized_md"
                });
            }
            RunOutcome {
                job_id: input.job_id.clone(),
                status: ExportJobStatus::Completed,
                output_path: Some(path_string(&pdf_path)),
                error_message: None,
                artifacts,
                warnings,
            }
        }
        Err(message) => failed(
            &input.job_id,
            message,
            artifacts_with_normalized_and_typ(
                &md_artifact,
                normalized_artifact.as_ref(),
                &typ_artifact,
            ),
            warnings,
        ),
    }
}

/// Keep intermediate Typst and normalized Markdown artifacts when explicitly
/// requested by the job or, during development builds, by setting
/// `JOI_EXPORT_RETAIN_TYPST=1` (or `true`). Production builds ignore the
/// environment flag so successful PDF exports remain clean by default.
fn should_retain_debug_artifact(explicit: bool) -> bool {
    explicit
        || (cfg!(debug_assertions)
            && std::env::var(RETAIN_TYPST_ENV)
                .map(|value| {
                    matches!(
                        value.trim().to_ascii_lowercase().as_str(),
                        "1" | "true" | "yes"
                    )
                })
                .unwrap_or(false))
}

fn log_warnings(job_id: &str, warnings: &[ExportWarningDto]) {
    for warning in warnings {
        log::warn!(
            "export_worker: warning job_id={} code={} severity={} message={}",
            job_id,
            warning.code,
            warning.severity,
            warning.message
        );
    }
}

fn artifacts_with_normalized(
    md: &WrittenArtifact,
    normalized: Option<&WrittenArtifact>,
) -> Vec<WrittenArtifact> {
    let mut artifacts = vec![md.clone()];
    if let Some(artifact) = normalized {
        artifacts.push(artifact.clone());
    }
    artifacts
}

fn artifacts_with_normalized_and_typ(
    md: &WrittenArtifact,
    normalized: Option<&WrittenArtifact>,
    typ: &WrittenArtifact,
) -> Vec<WrittenArtifact> {
    let mut artifacts = artifacts_with_normalized(md, normalized);
    artifacts.push(typ.clone());
    artifacts
}

fn with_diagnostics(
    mut artifacts: Vec<WrittenArtifact>,
    export_dir: &Path,
    warnings: &[ExportWarningDto],
) -> Vec<WrittenArtifact> {
    if let Ok(json) = serde_json::to_vec_pretty(warnings) {
        let path = export_dir.join("diagnostics.json");
        if let Ok(artifact) = write_artifact(&path, &json, "diagnostics") {
            artifacts.push(artifact);
        }
    }
    artifacts
}

/// Run the Typst compilation with a 30-second timeout.
///
/// The Typst 0.15.1 library API has no cancellation hook, so the timeout is
/// enforced by running the compile in a separate thread and waiting for the
/// result with a timeout. If the timeout expires, the thread is left to run
/// (it cannot be forcibly stopped) but the result is discarded and a timeout
/// error is returned. No partial PDF is produced.
fn compile_with_timeout(request: CompileRequest) -> Result<CompileOutcome, CompileError> {
    compile_with_timeout_in(request, COMPILE_TIMEOUT)
}

fn compile_with_timeout_in(
    request: CompileRequest,
    timeout: std::time::Duration,
) -> Result<CompileOutcome, CompileError> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = compiler::compile(&request);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(CompileError::CompileTimeout),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            // Thread panicked or exited without sending.
            Err(CompileError::CompileTimeout)
        }
    }
}

/// Build a terminal failed outcome with a fixed, content-safe message.
fn failed(
    job_id: &str,
    message: &str,
    artifacts: Vec<WrittenArtifact>,
    warnings: Vec<ExportWarningDto>,
) -> RunOutcome {
    RunOutcome {
        job_id: job_id.to_string(),
        status: ExportJobStatus::Failed,
        output_path: None,
        error_message: Some(message.to_string()),
        artifacts,
        warnings,
    }
}

/// Build a terminal cancelled outcome with retained artifacts.
fn cancelled(
    job_id: &str,
    artifacts: Vec<WrittenArtifact>,
    warnings: Vec<ExportWarningDto>,
) -> RunOutcome {
    RunOutcome {
        job_id: job_id.to_string(),
        status: ExportJobStatus::Cancelled,
        output_path: None,
        error_message: Some("The export was cancelled.".to_string()),
        artifacts,
        warnings,
    }
}

/// Map AST conversion warnings to the safe export warning DTO.
fn conversion_warnings(warnings: &[ConversionWarning]) -> Vec<ExportWarningDto> {
    warnings
        .iter()
        .map(|w| ExportWarningDto {
            code: w.code.as_str().to_string(),
            severity: "warning".to_string(),
            message: w.message.clone(),
            source: None,
            phase: Some("conversion".to_string()),
            category: Some("markdown_conversion".to_string()),
            line: None,
            column: None,
        })
        .collect()
}

/// Map a reduced compiler diagnostic to the safe export warning DTO.
fn compile_warning(diagnostic: CompileDiagnostic) -> ExportWarningDto {
    let severity = match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    let code = match diagnostic.severity {
        Severity::Error => "compile_error",
        Severity::Warning => "compile_warning",
    };
    ExportWarningDto {
        code: code.to_string(),
        severity: severity.to_string(),
        message: diagnostic.message,
        source: Some(diagnostic.source),
        phase: Some(diagnostic.phase),
        category: Some(diagnostic.category),
        line: diagnostic.line,
        column: diagnostic.column,
    }
}

/// Ensure the per-job output directory exists.
fn ensure_output_dir(dir: &Path) -> Result<(), &'static str> {
    std::fs::create_dir_all(dir).map_err(|_| "The export output directory could not be created.")
}

/// Stage bytes to a temporary file and atomically rename it into `path`.
///
/// The checksum is computed over the exact bytes written, so the persisted
/// checksum always matches the file content.
fn write_artifact(
    path: &Path,
    bytes: &[u8],
    format: &str,
) -> Result<WrittenArtifact, &'static str> {
    let dir = path
        .parent()
        .ok_or("The output directory is unavailable.")?;
    ensure_output_dir(dir)?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let tmp = dir.join(format!(".{file_name}.tmp-{}", crate::id::new_uuid()));

    std::fs::write(&tmp, bytes).map_err(|_| "The output file could not be written.")?;
    if let Err(_e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err("The output file could not be finalized.");
    }

    Ok(WrittenArtifact {
        format: format.to_string(),
        path: path_string(path),
        checksum: sha256_hex(bytes),
        size_bytes: bytes.len() as i64,
    })
}

/// Compute the SHA-256 checksum of `bytes`, hex-encoded.
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Convert a path to a UTF-8 string for storage. Paths under the app data
/// directory are expected to be valid UTF-8; a lossy conversion is only a
/// defensive fallback and never appears in a user-facing message.
fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "joi-export-runner-{name}-{}",
            crate::id::new_uuid()
        ));
        dir
    }

    fn markdown_input(export_type: ExportType, dir: PathBuf) -> WorkerInput {
        WorkerInput {
            job_id: "job-1".to_string(),
            export_type,
            markdown: "# Synthetic title\n\nA synthetic paragraph.\n".to_string(),
            diary_profile: false,
            template_name: Some("default".to_string()),
            export_dir: dir,
            retain_debug_artifact: false,
            initial_warnings: Vec::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    #[test]
    fn markdown_job_writes_source_and_completes() {
        let dir = temp_dir("md");
        let outcome = run_export_worker(markdown_input(ExportType::Markdown, dir.clone()));

        assert_eq!(outcome.status, ExportJobStatus::Completed);
        assert!(outcome.error_message.is_none());
        assert!(outcome.output_path.is_some());
        assert_eq!(outcome.artifacts.len(), 1);
        assert_eq!(outcome.artifacts[0].format, "md");

        let bytes = std::fs::read(dir.join("input.md")).expect("read input.md");
        assert_eq!(
            String::from_utf8(bytes).expect("utf8"),
            "# Synthetic title\n\nA synthetic paragraph.\n"
        );
        assert!(outcome.artifacts[0].checksum.len() == 64);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pdf_job_compiles_and_writes_pdf_with_checksum() {
        let dir = temp_dir("pdf");
        let input = WorkerInput {
            job_id: "job-1".to_string(),
            export_type: ExportType::Pdf,
            markdown: "# Synthetic title\n\nA synthetic paragraph.\n".to_string(),
            diary_profile: false,
            template_name: Some("default".to_string()),
            export_dir: dir.clone(),
            retain_debug_artifact: true,
            initial_warnings: Vec::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        let outcome = run_export_worker(input);

        assert_eq!(outcome.status, ExportJobStatus::Completed);
        assert!(outcome.error_message.is_none());
        assert!(outcome
            .output_path
            .as_deref()
            .unwrap()
            .ends_with("output.pdf"));

        let formats: Vec<&str> = outcome
            .artifacts
            .iter()
            .map(|a| a.format.as_str())
            .collect();
        assert!(formats.contains(&"md"));
        assert!(formats.contains(&"typ"));
        assert!(formats.contains(&"pdf"));

        let pdf_bytes = std::fs::read(dir.join("output.pdf")).expect("read output.pdf");
        assert_eq!(&pdf_bytes[..5], b"%PDF-");
        assert!(outcome
            .artifacts
            .iter()
            .all(|a| a.checksum.len() == 64 && a.size_bytes > 0));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn typst_job_delivers_typ_source() {
        let dir = temp_dir("typst");
        let outcome = run_export_worker(markdown_input(ExportType::Typst, dir.clone()));

        assert_eq!(outcome.status, ExportJobStatus::Completed);
        assert!(outcome
            .output_path
            .as_deref()
            .unwrap()
            .ends_with("input.typ"));
        assert!(std::fs::read(dir.join("input.typ")).is_ok());
        assert!(!outcome.artifacts.iter().any(|a| a.format == "pdf"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unsupported_export_type_fails_retaining_markdown() {
        let dir = temp_dir("unsupported");
        let outcome = run_export_worker(markdown_input(ExportType::Timesheet, dir.clone()));

        assert_eq!(outcome.status, ExportJobStatus::Failed);
        assert!(outcome.error_message.is_some());
        assert_eq!(outcome.artifacts.len(), 1);
        assert_eq!(outcome.artifacts[0].format, "md");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn failure_retains_markdown_and_typ_without_pdf() {
        // Content that renders to Typst that references a missing image will
        // fail compilation inside the sandbox, exercising the failure path
        // while the generated Markdown itself remains valid.
        let dir = temp_dir("fail");
        let input = WorkerInput {
            job_id: "job-1".to_string(),
            export_type: ExportType::Pdf,
            markdown: "# Synthetic\n\n![missing](assets/does-not-exist.png)\n".to_string(),
            diary_profile: false,
            template_name: Some("default".to_string()),
            export_dir: dir.clone(),
            retain_debug_artifact: false,
            initial_warnings: Vec::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        let outcome = run_export_worker(input);

        assert_eq!(outcome.status, ExportJobStatus::Failed);
        assert!(outcome.error_message.is_some());
        // Safe message: no source text, path, or diagnostic internals.
        let message = outcome.error_message.as_deref().unwrap();
        assert!(!message.contains("does-not-exist"));
        assert!(!message.contains("assets"));

        let formats: Vec<&str> = outcome
            .artifacts
            .iter()
            .map(|a| a.format.as_str())
            .collect();
        assert!(formats.contains(&"md"));
        assert!(formats.contains(&"typ"));
        assert!(formats.contains(&"diagnostics"));
        assert!(!formats.contains(&"pdf"));

        // Retained files exist; no PDF was written.
        assert!(std::fs::read(dir.join("input.md")).is_ok());
        assert!(std::fs::read(dir.join("input.typ")).is_ok());
        assert!(std::fs::read(dir.join("diagnostics.json")).is_ok());
        assert!(!dir.join("output.pdf").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_template_is_applied_to_generated_typst() {
        let dir = temp_dir("template-applied");
        let input = WorkerInput {
            job_id: "job-1".to_string(),
            export_type: ExportType::Pdf,
            markdown: "# Synthetic title\n\nA synthetic paragraph.\n".to_string(),
            template_name: Some("default".to_string()),
            diary_profile: false,
            export_dir: dir.clone(),
            retain_debug_artifact: true,
            initial_warnings: Vec::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        let outcome = run_export_worker(input);

        assert_eq!(outcome.status, ExportJobStatus::Completed);

        // The generated `.typ` source must import the fixed template path, so
        // the reviewed default template is actually applied rather than
        // silently ignored.
        let typ = std::fs::read_to_string(dir.join("input.typ")).expect("read input.typ");
        assert!(
            typ.starts_with("#import \"/templates/foundation-journal.typ\": *\n"),
            "generated source must import the fixed template path, got: {typ}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unsupported_template_name_fails_without_pdf() {
        let dir = temp_dir("template-unsupported");
        let input = WorkerInput {
            job_id: "job-1".to_string(),
            export_type: ExportType::Pdf,
            markdown: "# Synthetic\n\nBody.\n".to_string(),
            diary_profile: false,
            template_name: Some("diary".to_string()),
            export_dir: dir.clone(),
            retain_debug_artifact: false,
            initial_warnings: Vec::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        let outcome = run_export_worker(input);

        assert_eq!(outcome.status, ExportJobStatus::Failed);
        assert!(outcome.error_message.is_some());
        // No PDF is produced for an unsupported template.
        assert!(!dir.join("output.pdf").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn path_like_template_name_fails_without_pdf() {
        let dir = temp_dir("template-pathlike");
        let input = WorkerInput {
            job_id: "job-1".to_string(),
            export_type: ExportType::Pdf,
            markdown: "# Synthetic\n\nBody.\n".to_string(),
            diary_profile: false,
            template_name: Some("C:\\templates\\default".to_string()),
            export_dir: dir.clone(),
            retain_debug_artifact: false,
            initial_warnings: Vec::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        let outcome = run_export_worker(input);

        assert_eq!(outcome.status, ExportJobStatus::Failed);
        assert!(!dir.join("output.pdf").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_template_compiles_directly() {
        let dir = temp_dir("template-none");
        let input = WorkerInput {
            job_id: "job-1".to_string(),
            export_type: ExportType::Pdf,
            markdown: "# Synthetic\n\nBody.\n".to_string(),
            diary_profile: false,
            template_name: None,
            export_dir: dir.clone(),
            retain_debug_artifact: true,
            initial_warnings: Vec::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        let outcome = run_export_worker(input);

        assert_eq!(outcome.status, ExportJobStatus::Completed);
        let typ = std::fs::read_to_string(dir.join("input.typ")).expect("read input.typ");
        assert!(
            !typ.contains("#import"),
            "no-template export must compile directly, got: {typ}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn successful_pdf_does_not_duplicate_or_retain_intermediate_artifacts() {
        let dir = temp_dir("pdf-artifacts");
        let input = WorkerInput {
            job_id: "job-1".to_string(),
            export_type: ExportType::Pdf,
            markdown: "# Synthetic\n\nBody.\n".to_string(),
            diary_profile: false,
            template_name: None,
            export_dir: dir.clone(),
            retain_debug_artifact: false,
            initial_warnings: Vec::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        let outcome = run_export_worker(input);

        assert_eq!(outcome.status, ExportJobStatus::Completed);
        assert_eq!(
            outcome
                .artifacts
                .iter()
                .filter(|artifact| artifact.format == "pdf")
                .count(),
            1
        );
        assert!(outcome
            .artifacts
            .iter()
            .all(|artifact| artifact.format != "typ"));
        assert!(!dir.join("input.typ").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pre_cancelled_worker_returns_cancelled_with_markdown_retained() {
        let dir = temp_dir("cancelled");
        let input = WorkerInput {
            job_id: "job-1".to_string(),
            export_type: ExportType::Pdf,
            markdown: "# Synthetic\n\nBody.\n".to_string(),
            diary_profile: false,
            template_name: Some("default".to_string()),
            export_dir: dir.clone(),
            retain_debug_artifact: false,
            initial_warnings: Vec::new(),
            cancelled: Arc::new(AtomicBool::new(true)),
        };
        let outcome = run_export_worker(input);

        assert_eq!(outcome.status, ExportJobStatus::Cancelled);
        assert!(outcome.output_path.is_none(), "no PDF may be reported");
        assert!(
            outcome.artifacts.iter().any(|a| a.format == "md"),
            "Markdown must be retained on cancellation"
        );
        assert!(
            outcome.artifacts.iter().all(|a| a.format != "pdf"),
            "a partial PDF must never be registered"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zero_timeout_compile_fails_with_timeout_error() {
        let request = CompileRequest {
            source: "= Synthetic\n\nBody.\n".to_string(),
            template: None,
            attachments: Vec::new(),
        };
        let result = compile_with_timeout_in(request, std::time::Duration::from_nanos(0));

        assert!(
            matches!(result, Err(CompileError::CompileTimeout)),
            "an elapsed deadline must surface as CompileTimeout"
        );
    }
}
