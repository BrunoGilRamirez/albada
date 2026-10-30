use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::domain::models::{Block, BlockRecurrence};

/// Frontend error diagnostics accepted by the local logging command.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendErrorReportDto {
    pub source: String,
    pub message: String,
    pub name: Option<String>,
    pub stack: Option<String>,
    pub component_stack: Option<String>,
}

/// Boot milestone diagnostics accepted by the local startup logger.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendStartupReportDto {
    pub phase: FrontendStartupPhase,
    pub elapsed_ms: u64,
    pub command: Option<String>,
}

/// Stable startup milestones reported by the frontend.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FrontendStartupPhase {
    ModuleEval,
    FirstCommit,
    FirstInvoke,
}

/// Input DTOs for Tauri commands. These carry frontend form data across the
/// command boundary. Domain models are the output DTOs (they are `Serialize`).
/// Decision for a single review item during legacy import commit.
#[derive(Debug, Deserialize)]
pub struct ReviewDecisionDto {
    /// The review item ID from the preview report.
    pub review_item_id: String,
    /// The entity type to import as, or "ignore" to skip this item.
    /// Valid entity types: "journal_entry", "therapy_record", "task", "goal",
    /// "project", "year_theme", "quarter", "quarter_focus", "note".
    pub entity_type: String,
}

/// Input for the reviewed-decision commit boundary.
#[derive(Debug, Deserialize)]
pub struct CommitLegacyImportDto {
    pub workspace_id: String,
    pub source_root: String,
    /// Optional relative paths to restrict the commit to (same semantics as preview).
    pub include_paths: Option<Vec<String>>,
    /// The fingerprint returned by preview_legacy_import. Must match exactly.
    pub preview_fingerprint: String,
    /// One decision per discovered review item. Every review item from the
    /// preview must have an explicit decision (entity_type or "ignore").
    pub decisions: Vec<ReviewDecisionDto>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceDto {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkspaceDto {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBackupReminderPreferenceDto {
    pub enabled: bool,
    pub interval_days: u32,
}

#[derive(Debug, Deserialize)]
pub struct CreateYearThemeDto {
    pub workspace_id: String,
    pub year: i64,
    pub name: String,
    pub description_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateYearThemeDto {
    pub name: String,
    pub description_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateQuarterDto {
    pub year_theme_id: String,
    pub code: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateQuarterDto {
    pub start_date: String,
    pub end_date: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateQuarterFocusDto {
    pub quarter_id: String,
    pub goal_id: Option<String>,
    pub project_id: Option<String>,
    pub priority: String,
    pub intention_markdown: Option<String>,
    pub success_signal_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateQuarterFocusDto {
    pub priority: String,
    pub intention_markdown: Option<String>,
    pub success_signal_markdown: Option<String>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateGoalDto {
    pub workspace_id: String,
    pub year_theme_id: Option<String>,
    pub title: String,
    pub description_markdown: Option<String>,
    pub why_it_matters_markdown: Option<String>,
    pub success_definition_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGoalDto {
    pub title: String,
    pub description_markdown: Option<String>,
    pub why_it_matters_markdown: Option<String>,
    pub success_definition_markdown: Option<String>,
    pub status: String,
    pub priority: String,
    pub target_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectDto {
    pub workspace_id: String,
    pub goal_id: Option<String>,
    pub title: String,
    pub description_markdown: Option<String>,
    pub why_it_matters_markdown: Option<String>,
    pub definition_of_done_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectDto {
    pub title: String,
    pub description_markdown: Option<String>,
    pub why_it_matters_markdown: Option<String>,
    pub definition_of_done_markdown: Option<String>,
    pub status: String,
    pub priority: String,
    pub target_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskDto {
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub goal_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub title: String,
    pub description_markdown: Option<String>,
    pub notes_markdown: Option<String>,
    pub priority: String,
    pub effort_size: Option<String>,
    pub target_date: Option<String>,
    pub scheduled_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChildTaskDto {
    pub parent_id: String,
    pub title: String,
    pub description_markdown: Option<String>,
    pub notes_markdown: Option<String>,
    pub priority: String,
    pub effort_size: Option<String>,
    pub target_date: Option<String>,
    pub scheduled_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskDto {
    pub title: String,
    pub description_markdown: Option<String>,
    pub notes_markdown: Option<String>,
    pub priority: String,
    pub effort_size: Option<String>,
    pub target_date: Option<String>,
    pub scheduled_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MoveTaskDto {
    pub project_id: Option<String>,
    pub goal_id: Option<String>,
    pub parent_task_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TransitionTaskStatusDto {
    pub to_status: String,
    pub reason_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CloseParentManuallyDto {
    pub closure_mode: String,
    pub reason_markdown: String,
}

#[derive(Debug, Deserialize)]
pub struct AddDependencyDto {
    pub blocked_task_id: String,
    pub prerequisite_task_id: String,
    pub note_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RemoveDependencyDto {
    pub reason_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDailyPlanDto {
    pub intention_markdown: Option<String>,
    pub capacity_note_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDailyPlanReviewDto {
    pub review_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SelectTaskForDayDto {
    pub daily_plan_id: String,
    pub task_id: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ReplanTaskDto {
    pub decision: String,
    pub note_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBlockDto {
    pub workspace_id: String,
    pub daily_plan_id: Option<String>,
    pub date: String,
    pub block_type: String,
    pub label: Option<String>,
    pub planned_start_at: Option<String>,
    /// Accepted only to return a clear validation error to pre-8.7.2 clients.
    pub planned_end_at: Option<String>,
    pub estimated_duration_seconds: Option<i64>,
    pub time_window_label: Option<String>,
    pub flexibility: String,
    pub note_markdown: Option<String>,
    pub task_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBlockDto {
    pub daily_plan_id: Option<String>,
    pub date: String,
    pub block_type: String,
    pub label: Option<String>,
    pub planned_start_at: Option<String>,
    pub estimated_duration_seconds: Option<i64>,
    pub time_window_label: Option<String>,
    pub flexibility: String,
    pub note_markdown: Option<String>,
    pub task_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBlockRecurrenceDto {
    pub workspace_id: String,
    pub weekdays: Option<Vec<i64>>,
    pub local_start_time: Option<String>,
    pub estimated_duration_seconds: Option<i64>,
    pub block_type: String,
    pub label: Option<String>,
    pub flexibility: String,
    pub note_markdown: Option<String>,
    pub enabled: bool,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub task_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBlockRecurrenceDto {
    pub workspace_id: String,
    pub weekdays: Option<Vec<i64>>,
    pub local_start_time: Option<String>,
    pub estimated_duration_seconds: Option<i64>,
    pub block_type: String,
    pub label: Option<String>,
    pub flexibility: String,
    pub note_markdown: Option<String>,
    pub enabled: bool,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub task_id: Option<String>,
}

/// Input for explicit reactivation of an inactive one-off block: the user
/// chooses an ISO weekday and either a single new occurrence or a new weekly
/// series.
#[derive(Debug, Deserialize)]
pub struct ReactivateBlockDto {
    pub workspace_id: String,
    pub block_id: String,
    pub schedule: String,
    pub weekday: i64,
    pub local_start_time: Option<String>,
    pub estimated_duration_seconds: Option<i64>,
}

/// Reactivation result: exactly one of the new one-off block or the new series.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactivateBlockResult {
    pub block: Option<Block>,
    pub recurrence: Option<BlockRecurrence>,
}

/// Read-only habit analytics for recurring block series (Phase 8.7.12).
///
/// Settled contract: `docs/data-entities-modeling.md` section 8.9. The response
/// echoes the effective range (its end clamped to the workspace-local today)
/// and carries one entry per non-deleted series of the workspace, ordered by
/// series id. Counts are a pure read of persisted occurrence rows plus a
/// derived expected-date set, so repeated queries over unchanged data return
/// identical results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecurrenceAnalyticsResponse {
    pub effective_from: String,
    pub effective_to: String,
    pub series: Vec<RecurrenceAnalyticsSeriesDto>,
}

/// One recurring block series in the analytics response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecurrenceAnalyticsSeriesDto {
    /// `block_recurrences.id` — never a version id, so renames and weekday
    /// edits never split series history.
    pub series_id: String,
    /// Snapshot label of the version covering the latest expected or observed
    /// in-range date of the series.
    pub display_label: Option<String>,
    /// True when the range had no in-coverage date and the label therefore
    /// comes from `current_version_id` as out-of-range context.
    pub display_label_is_current_fallback: bool,
    pub coverage: RecurrenceAnalyticsCoverageDto,
    pub counts: RecurrenceAnalyticsCountsDto,
    /// Hidden (`None`) when the rate denominator is zero.
    pub rate: Option<RecurrenceAnalyticsRateDto>,
    /// Contiguous ISO weeks (Monday-labelled) from the range-start week through
    /// the week containing `effective_to`, including zero-count weeks.
    pub weekly: Vec<RecurrenceAnalyticsWeekBucketDto>,
}

/// Coverage statement of a series: the first local date it was active plus
/// every recorded active interval (closed intervals are the paused ranges).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecurrenceAnalyticsCoverageDto {
    pub start: Option<String>,
    pub intervals: Vec<RecurrenceAnalyticsCoverageIntervalDto>,
}

/// One active lifecycle interval; `until` is absent while the series is active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecurrenceAnalyticsCoverageIntervalDto {
    pub from: String,
    pub until: Option<String>,
}

/// Outcome counts for the effective range or for one ISO week of it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecurrenceAnalyticsCountsDto {
    /// Size of the expected set `E` (scheduled dates inside the range).
    pub scheduled: i64,
    pub completed: i64,
    /// Started but not completed (`active`).
    pub started: i64,
    /// Skipped manually or by the elapsed-window expiry sweep.
    pub skipped: i64,
    /// Withdrawn (manual cancel, deactivation, or a rule edit), including rows
    /// on out-of-schedule dates.
    pub cancelled: i64,
    /// Expected dates without a recorded result; never a confirmed failure.
    pub unrecorded: i64,
}

/// Completion rate with the contract-defined exclusions made explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecurrenceAnalyticsRateDto {
    /// `|E|` minus the expected dates withdrawn by a cancelled row.
    pub denominator: i64,
    pub completed: i64,
    /// Rounded percentage.
    pub percent: i64,
}

/// One ISO week of the weekly trend, labelled by its Monday.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecurrenceAnalyticsWeekBucketDto {
    pub week_start: String,
    pub scheduled: i64,
    pub completed: i64,
    pub started: i64,
    pub skipped: i64,
    pub cancelled: i64,
    pub unrecorded: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionDto {
    pub task_id: String,
    pub block_id: Option<String>,
    pub daily_plan_task_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FinishSessionDto {
    pub outcome_markdown: Option<String>,
    pub next_step_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFixedCommitmentDto {
    pub workspace_id: String,
    pub title: String,
    pub description_markdown: Option<String>,
    pub commitment_type: String,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub duration_minutes: Option<i64>,
    pub recurrence_rule: Option<String>,
    pub movable: bool,
    pub privacy_scope: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFixedCommitmentDto {
    pub title: String,
    pub description_markdown: Option<String>,
    pub commitment_type: String,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub duration_minutes: Option<i64>,
    pub recurrence_rule: Option<String>,
    pub movable: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateAvailabilityProfileDto {
    pub workspace_id: String,
    pub weekday: i64,
    pub start_time: String,
    pub end_time: String,
    pub preferred_block_length_minutes: Option<i64>,
    pub energy_expectation: Option<String>,
    pub active_from: Option<String>,
    pub active_until: Option<String>,
    pub note_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAvailabilityProfileDto {
    pub weekday: i64,
    pub start_time: String,
    pub end_time: String,
    pub preferred_block_length_minutes: Option<i64>,
    pub energy_expectation: Option<String>,
    pub active_from: Option<String>,
    pub active_until: Option<String>,
    pub note_markdown: Option<String>,
}

/// Editable reminder-rule settings (Phase 8.4). Time-of-day values are `HH:MM`
/// strings; absent profile values are preserved when already configured and
/// receive the default notification profile when an enabled rule has no value.
/// IDs, type, recurrence, and audit timestamps are system-owned and not
/// editable through this DTO. Native domain validation is authoritative.
#[derive(Debug, Deserialize)]
pub struct UpdateReminderRuleDto {
    pub enabled: bool,
    pub preferred_time: Option<String>,
    pub time_window_start: Option<String>,
    pub time_window_end: Option<String>,
    pub quiet_hours_start: Option<String>,
    pub quiet_hours_end: Option<String>,
    pub deadline_notification_window_days: u32,
    pub max_reminders_per_day: u32,
    pub snooze_minutes: Vec<u32>,
    pub channel: String,
    pub escalation: String,
    pub privacy_safe_message: String,
}

/// Filters for notification delivery history (Phase 8.4). Workspace scoping is
/// required; `from`/`to` are inclusive UTC calendar days (`YYYY-MM-DD`);
/// `status` is one of the delivery statuses validated natively.
#[derive(Debug, Deserialize)]
pub struct NotificationDeliveryFiltersDto {
    pub workspace_id: String,
    pub reminder_rule_id: Option<String>,
    pub delivery_kind: Option<String>,
    pub status: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateJournalEntryDto {
    pub workspace_id: String,
    pub occurred_on: String,
    pub title: String,
    pub content_markdown: String,
    pub template_id: Option<String>,
    pub template_version: Option<String>,
    pub template_values: Option<BTreeMap<String, Value>>,
    pub mood_label: Option<String>,
    pub energy_label: Option<String>,
    pub emotions: Option<Vec<String>>,
    pub export_eligible: Option<bool>,
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateJournalEntryDto {
    pub occurred_on: String,
    pub title: String,
    pub content_markdown: String,
    pub template_id: Option<String>,
    pub template_version: Option<String>,
    pub template_values: Option<BTreeMap<String, Value>>,
    pub mood_label: Option<String>,
    pub energy_label: Option<String>,
    pub emotions: Option<Vec<String>>,
    pub export_eligible: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTherapyCollectionDto {
    pub workspace_id: String,
    pub name: String,
    pub privacy_scope: Option<String>,
    pub encryption_status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTherapyCollectionDto {
    pub name: String,
    pub privacy_scope: Option<String>,
    pub encryption_status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTherapyRecordDto {
    pub therapy_collection_id: String,
    pub record_type: String,
    pub title: String,
    pub content_markdown: String,
    pub occurred_on: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTherapyRecordDto {
    pub record_type: String,
    pub title: String,
    pub content_markdown: String,
    pub occurred_on: Option<String>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct RunLegacyImportDto {
    pub source_root: String,
    /// Optional relative paths (directories or files) to restrict the import to.
    /// When `None`, the whole source root is imported. Paths must be relative to
    /// `source_root` and must not contain `..` or be absolute.
    pub include_paths: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct ImportReportDto {
    pub batch_id: String,
    pub source_root: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub counts: Vec<EntityCountDto>,
    pub warnings: Vec<ImportWarningDto>,
    pub review_items: Vec<ImportReviewItemDto>,
    /// Deterministic fingerprint over workspace_id + canonical source_root +
    /// include_paths + ordered (relative_path, content_hash, proposed_entity, confidence).
    /// Must be provided to commit_legacy_import to prove the preview has not changed.
    pub preview_fingerprint: String,
}

/// A minimal, content-safe status snapshot for polling an in-flight import.
///
/// Unlike [`ImportReportDto`], this carries no source paths, snippets, counts,
/// or warnings, so it is safe to return while a batch is still running and its
/// summary has not been finalized.
#[derive(Debug, Serialize)]
pub struct ImportBatchStatusDto {
    pub batch_id: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EntityCountDto {
    pub entity_type: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct ImportWarningDto {
    pub severity: String,
    pub source_path: String,
    pub line: Option<i64>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ImportReviewItemDto {
    pub id: String,
    pub source_path: String,
    pub proposed_entity_type: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
    pub confidence: String,
    /// SHA-256 over the exact source file content, hex-encoded. Used for
    /// fingerprint verification at commit time.
    pub source_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateExportJobDto {
    pub workspace_id: String,
    pub export_type: String,
    pub scope_type: String,
    pub title: String,
    pub template_name: Option<String>,
    pub selection: ExportSelectionDto,
    pub options: Option<Value>,
    /// Binds job creation to a previously reviewed preview snapshot. Required
    /// for Typst/PDF and validated for every export type when present.
    pub preview_fingerprint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportSelectionDto {
    pub journal_entry_ids: Option<Vec<String>>,
    pub therapy_record_ids: Option<Vec<String>>,
    pub task_ids: Option<Vec<String>>,
    pub project_ids: Option<Vec<String>>,
    pub goal_ids: Option<Vec<String>>,
    pub quarter_ids: Option<Vec<String>>,
    pub date_range: Option<DateRangeDto>,
    /// When true, expands to all non-archived entries/records for the workspace
    /// (journal scope -> all journal entries; therapy scope -> all therapy records).
    /// Mutually exclusive with date_range and explicit ID lists for the same scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export_all: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DateRangeDto {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
pub struct ExportJobDto {
    pub id: String,
    pub workspace_id: String,
    pub export_type: String,
    pub scope_type: String,
    pub title: String,
    pub template_name: Option<String>,
    pub status: String,
    pub output_path: Option<String>,
    pub generated_at: Option<String>,
    pub error_message: Option<String>,
    pub selection_fingerprint: Option<String>,
    pub warnings: Vec<ExportWarningDto>,
    pub created_at: String,
    pub updated_at: String,
    pub artifacts: Vec<ExportArtifactDto>,
}

/// A safe, structured warning attached to an export job.
///
/// `code` is a stable machine-readable identifier, `severity` is one of
/// `info | warning | error`, and `message` is a fixed, content-safe template.
/// Warnings never contain source text, therapy content, absolute filesystem
/// paths, or secrets.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ExportWarningDto {
    pub code: String,
    pub severity: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ExportArtifactDto {
    pub id: String,
    pub format: String,
    pub path: String,
    pub checksum: Option<String>,
    pub size_bytes: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ExportPreviewDto {
    /// Total number of resolved items (100% coverage, not capped).
    /// Kept under the historical name so existing clients keep working.
    pub item_count: i64,
    /// Alias of item_count for clarity in new clients.
    pub total_count: i64,
    /// Preview items, capped at 200 in deterministic order.
    pub items: Vec<ExportPreviewItemDto>,
    /// Whether the items list was truncated (total_count > 200).
    pub truncated: bool,
    /// The resolved selection as concrete entity IDs (date ranges already
    /// expanded), in the same deterministic order used by generation.
    pub resolved_selection: ExportSelectionDto,
    /// A deterministic fingerprint over the ordered IDs and content hashes.
    pub selection_fingerprint: String,
    pub has_therapy_private: bool,
}

#[derive(Debug, Serialize)]
pub struct ExportPreviewItemDto {
    pub id: String,
    pub entity_type: String,
    pub title: String,
    pub occurred_on: Option<String>,
    pub privacy_scope: String,
    /// SHA-256 over the exact export-relevant content, hex-encoded.
    pub content_hash: String,
}

/// A resolved export selection snapshot persisted in `export_jobs.selection_json`.
///
/// Unlike the front-end filters ([`ExportSelectionDto`]), this carries the
/// concrete entity IDs plus their content hashes, so a stale confirmation can be
/// detected before generation without re-reading original filters.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ResolvedSelectionSnapshotDto {
    pub items: Vec<ResolvedSelectionItemDto>,
}

/// One resolved item in a stored selection snapshot.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ResolvedSelectionItemDto {
    pub entity_type: String,
    pub id: String,
    pub content_hash: String,
}

/// Local template capability snapshot (Phase 7.10.7).
///
/// Names the shipped reviewed template, its version, whether it validates,
/// and which resources (if any) are unavailable. Contains no content.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TemplateCapabilityDto {
    pub name: String,
    pub version: String,
    pub valid: bool,
    pub compiler_version: String,
    pub unavailable_resources: Vec<String>,
}

/// Local environment diagnostics snapshot (Phase 9.7).
///
/// Read-only version and status strings for the settings diagnostics section:
/// SQLite version, Tauri runtime version, OS platform/version, embedded
/// Typst compiler version, bundled template status, and the bundled font
/// blob count (font availability signal; fonts ship embedded via
/// `typst-assets`, with no host font dependency). Reads no database and
/// exposes no content, paths, or secrets.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DiagnosticsDto {
    pub sqlite_version: String,
    pub tauri_version: String,
    pub os_platform: String,
    pub os_version: String,
    pub typst_compiler_version: String,
    pub template_name: String,
    pub template_version: String,
    pub template_valid: bool,
    pub bundled_font_count: usize,
}

/// Result of saving a copy of an export artifact via native save dialog.
///
/// `cancelled` is true when the user dismissed the dialog; in that case
/// `destination_path` and `size_bytes` are None. On success, `cancelled` is
/// false and both fields are populated.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SaveExportArtifactCopyResultDto {
    pub cancelled: bool,
    pub destination_path: Option<String>,
    pub size_bytes: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_task_dto_deserializes_from_frontend_payload() {
        let json = r#"{
            "workspace_id": "ws-1",
            "project_id": "p-1",
            "title": "A task",
            "description_markdown": null,
            "notes_markdown": null,
            "priority": "normal",
            "effort_size": null,
            "target_date": null,
            "scheduled_date": null
        }"#;
        let dto: CreateTaskDto = serde_json::from_str(json).expect("deserialize CreateTaskDto");
        assert_eq!(dto.title, "A task");
        assert_eq!(dto.workspace_id, "ws-1");
        assert!(dto.effort_size.is_none());
        assert!(dto.project_id.is_some());
    }

    #[test]
    fn create_task_dto_with_effort_size() {
        let json = r#"{
            "workspace_id": "ws-1",
            "project_id": "p-1",
            "title": "A task",
            "priority": "high",
            "effort_size": "medium"
        }"#;
        let dto: CreateTaskDto = serde_json::from_str(json).expect("deserialize CreateTaskDto");
        assert_eq!(dto.effort_size.as_deref(), Some("medium"));
        assert_eq!(dto.priority, "high");
    }

    #[test]
    fn block_recurrence_dto_keeps_missing_required_values_for_domain_validation() {
        let json = r#"{
            "workspace_id": "ws-1",
            "block_type": "focus",
            "flexibility": "flexible",
            "enabled": true,
            "effective_from": "2026-10-19"
        }"#;
        let dto: CreateBlockRecurrenceDto =
            serde_json::from_str(json).expect("deserialize recurrence with missing fields");
        assert!(dto.weekdays.is_none());
        assert!(dto.local_start_time.is_none());
        assert!(dto.estimated_duration_seconds.is_none());
    }

    #[test]
    fn run_legacy_import_dto_deserializes_with_include_paths() {
        let json = r#"{
            "source_root": "C:/legacy",
            "include_paths": ["salud-mental", "records/2026-Foundation/Q2/diary/personal-notes-raw"]
        }"#;
        let dto: RunLegacyImportDto =
            serde_json::from_str(json).expect("deserialize RunLegacyImportDto");
        assert_eq!(dto.source_root, "C:/legacy");
        assert_eq!(
            dto.include_paths,
            Some(vec![
                "salud-mental".to_string(),
                "records/2026-Foundation/Q2/diary/personal-notes-raw".to_string()
            ])
        );
    }

    #[test]
    fn run_legacy_import_dto_deserializes_without_include_paths() {
        let json = r#"{ "source_root": "C:/legacy" }"#;
        let dto: RunLegacyImportDto =
            serde_json::from_str(json).expect("deserialize RunLegacyImportDto");
        assert_eq!(dto.source_root, "C:/legacy");
        assert!(dto.include_paths.is_none());
    }

    /// Safe progress payloads (Phase 7.10.8): the polling DTO carries IDs,
    /// status, and timestamps only — no source paths, snippets, counts,
    /// warnings, or content of any kind.
    #[test]
    fn import_batch_status_payload_contains_no_content_fields() {
        let dto = ImportBatchStatusDto {
            batch_id: "batch-1".to_string(),
            status: "running".to_string(),
            started_at: "2026-09-23T00:00:00Z".to_string(),
            completed_at: None,
        };
        let value = serde_json::to_value(&dto).expect("serialize status DTO");
        let obj = value.as_object().expect("status DTO is an object");
        let keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        assert_eq!(keys.len(), 4, "status payload must not gain content fields");
        for key in ["batch_id", "status", "started_at", "completed_at"] {
            assert!(obj.contains_key(key), "missing key {key}");
        }
        let raw = value.to_string().to_lowercase();
        for forbidden in [
            "source_path",
            "snippet",
            "warning",
            "content",
            "therapy",
            "journal",
            "C:\\",
            "/home/",
        ] {
            assert!(
                !raw.contains(&forbidden.to_lowercase()),
                "payload leaks {forbidden}"
            );
        }
    }

    /// Export warnings are fixed safe templates (Phase 7.10.8): code,
    /// severity, and message only — never source text, paths, or secrets.
    #[test]
    fn export_warning_payload_contains_no_content_fields() {
        let dto = ExportWarningDto {
            code: "unsafe_url".to_string(),
            severity: "warning".to_string(),
            message: "An unsafe link target was rendered as plain text.".to_string(),
            source: None,
            phase: None,
            category: None,
            line: None,
            column: None,
        };
        let value = serde_json::to_value(&dto).expect("serialize warning DTO");
        let obj = value.as_object().expect("warning DTO is an object");
        assert_eq!(obj.len(), 3, "warning payload must not gain content fields");
    }

    /// Habit analytics (Phase 8.7.12) keeps the snake_case field names of the
    /// existing output DTOs and model types, so Phase 8.7.13's UI mirrors the
    /// native payload key for key. Nulls stay explicit: a series without a
    /// label and a hidden (zero-denominator) rate are both visible as null.
    #[test]
    fn recurrence_analytics_payload_uses_documented_field_names() {
        let response = RecurrenceAnalyticsResponse {
            effective_from: "2026-10-01".to_string(),
            effective_to: "2026-10-22".to_string(),
            series: vec![RecurrenceAnalyticsSeriesDto {
                series_id: "series-1".to_string(),
                display_label: Some("Morning walk".to_string()),
                display_label_is_current_fallback: false,
                coverage: RecurrenceAnalyticsCoverageDto {
                    start: Some("2026-03-04".to_string()),
                    intervals: vec![RecurrenceAnalyticsCoverageIntervalDto {
                        from: "2026-03-04".to_string(),
                        until: Some("2026-07-01".to_string()),
                    }],
                },
                counts: RecurrenceAnalyticsCountsDto {
                    scheduled: 3,
                    completed: 1,
                    started: 1,
                    skipped: 0,
                    cancelled: 1,
                    unrecorded: 1,
                },
                rate: Some(RecurrenceAnalyticsRateDto {
                    denominator: 2,
                    completed: 1,
                    percent: 50,
                }),
                weekly: vec![RecurrenceAnalyticsWeekBucketDto {
                    week_start: "2026-09-28".to_string(),
                    scheduled: 3,
                    completed: 1,
                    started: 1,
                    skipped: 0,
                    cancelled: 1,
                    unrecorded: 1,
                }],
            }],
        };

        let value = serde_json::to_value(&response).expect("serialize analytics response");
        assert_eq!(value["effective_from"], "2026-10-01");
        assert_eq!(value["effective_to"], "2026-10-22");
        assert_eq!(value["series"][0]["series_id"], "series-1");
        assert_eq!(value["series"][0]["display_label"], "Morning walk");
        assert_eq!(
            value["series"][0]["display_label_is_current_fallback"].as_bool(),
            Some(false)
        );
        assert_eq!(value["series"][0]["coverage"]["start"], "2026-03-04");
        assert_eq!(
            value["series"][0]["coverage"]["intervals"][0]["until"],
            "2026-07-01"
        );
        assert_eq!(value["series"][0]["counts"]["unrecorded"], 1);
        assert_eq!(value["series"][0]["rate"]["denominator"], 2);
        assert_eq!(value["series"][0]["rate"]["percent"], 50);
        assert_eq!(value["series"][0]["weekly"][0]["week_start"], "2026-09-28");
        assert_eq!(value["series"][0]["weekly"][0]["scheduled"], 3);

        let mut hidden = response.clone();
        hidden.series[0].display_label = None;
        hidden.series[0].rate = None;
        let value = serde_json::to_value(&hidden).expect("serialize analytics response with nulls");
        assert!(value["series"][0]["display_label"].is_null());
        assert!(value["series"][0]["rate"].is_null());
    }
}
