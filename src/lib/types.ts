export const TASK_STATUSES = [
  "inbox",
  "planned",
  "in_progress",
  "blocked",
  "completed",
  "cancelled",
  "deferred",
] as const;
export type TaskStatus = (typeof TASK_STATUSES)[number];

export const PRIORITIES = ["none", "low", "normal", "high", "critical"] as const;
export type Priority = (typeof PRIORITIES)[number];

export const EFFORT_SIZES = ["trivial", "small", "medium", "large", "very_large"] as const;
export type EffortSize = (typeof EFFORT_SIZES)[number];

export const CLOSURE_MODES = [
  "automatic",
  "manual_no_longer_applicable",
  "manual_solved_elsewhere",
  "manual_accepted_partial",
  "manual_other",
] as const;
export type ClosureMode = (typeof CLOSURE_MODES)[number];

export const GOAL_STATUSES = [
  "inbox",
  "active",
  "paused",
  "achieved",
  "abandoned",
  "archived",
] as const;
export type GoalStatus = (typeof GOAL_STATUSES)[number];

export const PROJECT_STATUSES = [
  "inbox",
  "planned",
  "active",
  "paused",
  "completed",
  "cancelled",
  "archived",
] as const;
export type ProjectStatus = (typeof PROJECT_STATUSES)[number];

export const HISTORY_SOURCES = ["user", "session", "import", "agent", "system"] as const;
export type HistorySource = (typeof HISTORY_SOURCES)[number];

export const DEADLINE_STATUSES = [
  "none",
  "normal",
  "approaching",
  "overdue",
  "blocked_risk",
] as const;
export type DeadlineStatus = (typeof DEADLINE_STATUSES)[number];

export const DEADLINE_LABELS: Record<DeadlineStatus, string> = {
  none: "No deadline",
  normal: "On track",
  approaching: "Approaching",
  overdue: "Overdue",
  blocked_risk: "Blocked at risk",
};

export interface ProgressContributor {
  task_id: string;
  title: string;
  status: TaskStatus;
  effort_points: number;
  completed: boolean;
}

export interface ProgressDetails {
  percentage: number | null;
  label: string;
  completed_points: number;
  total_points: number;
  contributors: ProgressContributor[];
}

export interface DeadlineTaskInfo {
  task_id: string;
  title: string;
  status: string;
  deadline_at: string | null;
  deadline_status: DeadlineStatus;
}

export interface DeadlineStatusDto {
  status: DeadlineStatus;
  at_risk_tasks: DeadlineTaskInfo[];
  warning_window_days: number;
}

export const EFFORT_POINTS: Record<EffortSize, number> = {
  trivial: 1,
  small: 2,
  medium: 3,
  large: 5,
  very_large: 8,
};

export const EFFORT_LABELS: Record<EffortSize, string> = {
  trivial: "Trivial",
  small: "Small",
  medium: "Medium",
  large: "Large",
  very_large: "Very large",
};

export const PRIORITY_LABELS: Record<Priority, string> = {
  none: "None",
  low: "Low",
  normal: "Normal",
  high: "High",
  critical: "Critical",
};

export const STATUS_LABELS: Record<TaskStatus, string> = {
  inbox: "Inbox",
  planned: "Planned",
  in_progress: "In progress",
  blocked: "Blocked",
  completed: "Completed",
  cancelled: "Cancelled",
  deferred: "Deferred",
};

export interface Workspace {
  id: string;
  name: string;
  root_path: string | null;
  timezone: string;
  locale: string;
  default_privacy_scope: string;
  current_year_theme_id: string | null;
  settings_json: string | null;
  created_at: string;
  updated_at: string;
}

export interface YearTheme {
  id: string;
  workspace_id: string;
  year: number;
  name: string;
  description_markdown: string | null;
  status: string;
  created_at: string;
  updated_at: string;
  archived_at: string | null;
}

export interface Quarter {
  id: string;
  year_theme_id: string;
  code: string;
  start_date: string;
  end_date: string;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface QuarterFocus {
  id: string;
  quarter_id: string;
  goal_id: string | null;
  project_id: string | null;
  priority: Priority;
  intention_markdown: string | null;
  success_signal_markdown: string | null;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface Goal {
  id: string;
  workspace_id: string;
  year_theme_id: string | null;
  title: string;
  description_markdown: string | null;
  why_it_matters_markdown: string | null;
  success_definition_markdown: string | null;
  status: GoalStatus;
  priority: Priority;
  target_date: string | null;
  created_at: string;
  updated_at: string;
  archived_at: string | null;
}

export interface Project {
  id: string;
  workspace_id: string;
  goal_id: string | null;
  title: string;
  description_markdown: string | null;
  why_it_matters_markdown: string | null;
  definition_of_done_markdown: string | null;
  status: ProjectStatus;
  priority: Priority;
  target_date: string | null;
  created_at: string;
  updated_at: string;
  archived_at: string | null;
}

export interface Task {
  id: string;
  workspace_id: string;
  project_id: string | null;
  goal_id: string | null;
  parent_task_id: string | null;
  title: string;
  description_markdown: string | null;
  notes_markdown: string | null;
  blocking_note_markdown: string | null;
  status: TaskStatus;
  priority: Priority;
  effort_size: EffortSize | null;
  effort_points: number;
  target_date: string | null;
  scheduled_date: string | null;
  completed_at: string | null;
  cancelled_at: string | null;
  closure_mode: ClosureMode | null;
  closure_reason_markdown: string | null;
  created_at: string;
  updated_at: string;
  archived_at: string | null;
}

export interface PrerequisiteTask {
  dependency_id: string;
  task_id: string;
  title: string;
  status: string;
}

export interface TaskTree {
  task: Task;
  children: TaskTree[];
  blocked_by: PrerequisiteTask[];
}

export interface TaskDependency {
  id: string;
  blocked_task_id: string;
  prerequisite_task_id: string;
  dependency_type: string;
  note_markdown: string | null;
  created_at: string;
  removed_at: string | null;
}

export interface StatusHistoryEntry {
  id: string;
  task_id: string;
  from_status: string | null;
  to_status: string;
  reason_markdown: string | null;
  source: HistorySource;
  metadata_json: string | null;
  occurred_at: string;
}

export interface BackupDto {
  path: string;
  created_at: string;
  size_bytes: number;
  checksum: string | null;
  schema_version: number | null;
  source_workspace_id: string | null;
  kind: "user" | "recovery";
  validation_status: "valid" | "unvalidated" | "invalid";
}

export interface BackupValidationDto {
  status: "valid" | "invalid";
  checksum: string | null;
  schema_version: number | null;
  size_bytes: number | null;
  kind: "user" | "recovery" | null;
  message: string;
}

export interface BackupReminderPreference {
  enabled: boolean;
  interval_days: number;
  last_successful_backup_at: string | null;
}

export const DAILY_PLAN_STATUSES = [
  "not_started",
  "planning",
  "planned",
  "in_review",
  "closed",
] as const;
export type DailyPlanStatus = (typeof DAILY_PLAN_STATUSES)[number];

export const SELECTION_REASONS = [
  "next_action",
  "priority",
  "deadline",
  "dependency",
  "manual",
] as const;
export type SelectionReason = (typeof SELECTION_REASONS)[number];

export const PLANNED_STATUSES = [
  "selected",
  "started",
  "completed",
  "deferred",
  "skipped",
] as const;
export type PlannedStatus = (typeof PLANNED_STATUSES)[number];

export const BLOCK_TYPES = [
  "planning",
  "focus",
  "review",
  "journaling",
  "rest",
  "personal",
] as const;
export type BlockType = (typeof BLOCK_TYPES)[number];

export const BLOCK_STATUSES = ["planned", "active", "completed", "skipped", "cancelled"] as const;
export type BlockStatus = (typeof BLOCK_STATUSES)[number];

export const BLOCK_LIFECYCLE_STATES = ["active", "inactive", "deleted"] as const;
export type BlockLifecycleState = (typeof BLOCK_LIFECYCLE_STATES)[number];

export const FLEXIBILITIES = ["fixed", "flexible", "approximate"] as const;
export type Flexibility = (typeof FLEXIBILITIES)[number];

export const SESSION_STATUSES = ["planned", "started", "finished", "paused", "skipped"] as const;
export type SessionStatus = (typeof SESSION_STATUSES)[number];

export const FIXED_COMMITMENT_TYPES = [
  "sleep",
  "meal",
  "work",
  "appointment",
  "exercise",
  "other",
] as const;
export type FixedCommitmentType = (typeof FIXED_COMMITMENT_TYPES)[number];

export const REPLAN_DECISIONS = [
  "move_to_next_day",
  "return_to_planned",
  "remove_date",
  "defer",
  "divide",
] as const;
export type ReplanDecision = (typeof REPLAN_DECISIONS)[number];

export const TIME_WINDOW_LABELS = ["morning", "afternoon", "evening", "after_meal"] as const;
export type TimeWindowLabel = (typeof TIME_WINDOW_LABELS)[number];

export const WEEKDAY_NAMES = [
  "Sunday",
  "Monday",
  "Tuesday",
  "Wednesday",
  "Thursday",
  "Friday",
  "Saturday",
] as const;

export const BLOCK_TYPE_LABELS: Record<BlockType, string> = {
  planning: "Planning",
  focus: "Focus",
  review: "Review",
  journaling: "Journaling",
  rest: "Rest",
  personal: "Personal",
};

export const BLOCK_STATUS_LABELS: Record<BlockStatus, string> = {
  planned: "Planned",
  active: "Active",
  completed: "Completed",
  skipped: "Skipped",
  cancelled: "Cancelled",
};

export const SESSION_STATUS_LABELS: Record<SessionStatus, string> = {
  planned: "Planned",
  started: "Started",
  finished: "Finished",
  paused: "Paused",
  skipped: "Skipped",
};

export const SELECTION_REASON_LABELS: Record<SelectionReason, string> = {
  next_action: "Next action",
  priority: "Priority",
  deadline: "Deadline",
  dependency: "Dependency",
  manual: "Manual",
};

export const PLANNED_STATUS_LABELS: Record<PlannedStatus, string> = {
  selected: "Selected",
  started: "Started",
  completed: "Completed",
  deferred: "Deferred",
  skipped: "Skipped",
};

export const REPLAN_DECISION_LABELS: Record<ReplanDecision, string> = {
  move_to_next_day: "Move to next day",
  return_to_planned: "Return to planned",
  remove_date: "Remove date",
  defer: "Defer",
  divide: "Divide into children",
};

export const FIXED_COMMITMENT_TYPE_LABELS: Record<FixedCommitmentType, string> = {
  sleep: "Sleep",
  meal: "Meal",
  work: "Work",
  appointment: "Appointment",
  exercise: "Exercise",
  other: "Other",
};

export interface DailyPlan {
  id: string;
  workspace_id: string;
  date: string;
  status: DailyPlanStatus;
  intention_markdown: string | null;
  capacity_note_markdown: string | null;
  review_markdown: string | null;
  planning_block_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface DailyPlanTask {
  id: string;
  daily_plan_id: string;
  task_id: string;
  order_index: number;
  selection_reason: SelectionReason;
  planned_status: PlannedStatus;
  carryover_from_daily_plan_id: string | null;
  decision_note_markdown: string | null;
  created_at: string;
  updated_at: string;
}

export interface Block {
  id: string;
  workspace_id: string;
  daily_plan_id: string | null;
  date: string;
  block_type: BlockType;
  status: BlockStatus;
  label: string | null;
  planned_start_at: string | null;
  planned_end_at: string | null;
  estimated_duration_seconds: number | null;
  recurrence_id: string | null;
  recurrence_version_id: string | null;
  local_date: string | null;
  local_start_time: string | null;
  timezone_id: string | null;
  resolved_start_at: string | null;
  time_window_label: TimeWindowLabel | null;
  actual_start_at: string | null;
  actual_end_at: string | null;
  flexibility: Flexibility;
  note_markdown: string | null;
  created_at: string;
  updated_at: string;
  lifecycle_state: BlockLifecycleState;
  task_id: string | null;
  reactivated_from_block_id: string | null;
}

export interface BlockRecurrence {
  id: string;
  workspace_id: string;
  version_id: string;
  weekdays: number[];
  local_start_time: string;
  estimated_duration_seconds: number;
  block_type: BlockType;
  label: string | null;
  flexibility: Flexibility;
  note_markdown: string | null;
  enabled: boolean;
  effective_from: string;
  effective_until: string | null;
  created_at: string;
  updated_at: string;
  lifecycle_state: BlockLifecycleState;
  task_id: string | null;
}

export interface ScheduledBlockRecurrence {
  recurrence: BlockRecurrence;
  next_occurrence_date: string;
}

/** One recorded active interval of a series; `until` is null while active. */
export interface RecurrenceAnalyticsCoverageInterval {
  from: string;
  until: string | null;
}

/** Coverage statement: first active date plus the recorded intervals. */
export interface RecurrenceAnalyticsCoverage {
  start: string | null;
  intervals: RecurrenceAnalyticsCoverageInterval[];
}

/**
 * Outcome counts of a series (or of one ISO week of it). `unrecorded` means an
 * expected date without a recorded result, never a confirmed failure.
 */
export interface RecurrenceAnalyticsCounts {
  scheduled: number;
  completed: number;
  started: number;
  skipped: number;
  cancelled: number;
  unrecorded: number;
}

/** Completion rate; absent when the denominator is zero. */
export interface RecurrenceAnalyticsRate {
  denominator: number;
  completed: number;
  percent: number;
}

/** One contiguous ISO week of the trend, labelled by its Monday. */
export interface RecurrenceAnalyticsWeekBucket {
  week_start: string;
  scheduled: number;
  completed: number;
  started: number;
  skipped: number;
  cancelled: number;
  unrecorded: number;
}

/** Read-only analytics for one recurring block series (phase 8.7.12). */
export interface RecurrenceAnalyticsSeries {
  /** `block_recurrences.id`; version edits never split the history. */
  series_id: string;
  display_label: string | null;
  /** True when the range had no in-coverage date and the label is out-of-range context. */
  display_label_is_current_fallback: boolean;
  coverage: RecurrenceAnalyticsCoverage;
  counts: RecurrenceAnalyticsCounts;
  rate: RecurrenceAnalyticsRate | null;
  weekly: RecurrenceAnalyticsWeekBucket[];
}

/** Read-only habit analytics response; `effective_to` is clamped to today. */
export interface RecurrenceAnalyticsResponse {
  effective_from: string;
  effective_to: string;
  series: RecurrenceAnalyticsSeries[];
}

export interface TaskSession {
  id: string;
  task_id: string;
  block_id: string | null;
  daily_plan_task_id: string | null;
  started_at: string | null;
  ended_at: string | null;
  status: SessionStatus;
  outcome_markdown: string | null;
  next_step_markdown: string | null;
  created_at: string;
  updated_at: string;
}

export interface FixedCommitment {
  id: string;
  workspace_id: string;
  title: string;
  description_markdown: string | null;
  commitment_type: FixedCommitmentType;
  start_at: string | null;
  end_at: string | null;
  duration_minutes: number | null;
  recurrence_rule: string | null;
  movable: boolean;
  active: boolean;
  privacy_scope: string;
  created_at: string;
  updated_at: string;
}

export interface AvailabilityProfile {
  id: string;
  workspace_id: string;
  weekday: number;
  start_time: string;
  end_time: string;
  preferred_block_length_minutes: number | null;
  energy_expectation: string | null;
  active_from: string | null;
  active_until: string | null;
  note_markdown: string | null;
  created_at: string;
  updated_at: string;
}

export const JOURNAL_SOURCES = ["user", "import", "agent"] as const;
export type JournalSource = (typeof JOURNAL_SOURCES)[number];

export const JOURNAL_REFERENCE_ENTITY_TYPES = [
  "task",
  "project",
  "goal",
  "block",
  "session",
] as const;
export type JournalReferenceEntityType = (typeof JOURNAL_REFERENCE_ENTITY_TYPES)[number];

export const JOURNAL_REFERENCE_RELATIONS = [
  "mentions",
  "related_to",
  "reflects_on",
  "result_of",
] as const;
export type JournalReferenceRelation = (typeof JOURNAL_REFERENCE_RELATIONS)[number];

// Structured, template-specific values keyed by approved manifest field ID.
// The Rust boundary validates keys against the bundled manifest; the wire
// shape is an arbitrary JSON object.
export type JournalTemplateValues = Record<string, unknown>;

export interface JournalEntry {
  id: string;
  workspace_id: string;
  occurred_on: string;
  title: string;
  content_markdown: string;
  template_id: string | null;
  template_version: string | null;
  template_values: JournalTemplateValues;
  mood_label: string | null;
  energy_label: string | null;
  emotions: string[];
  export_eligible: boolean;
  source: JournalSource;
  created_at: string;
  updated_at: string;
  archived_at: string | null;
}

export interface JournalTaskReference {
  id: string;
  journal_entry_id: string;
  entity_type: JournalReferenceEntityType;
  entity_id: string;
  relation: JournalReferenceRelation;
  created_by: string;
  confidence: number | null;
  created_at: string;
  removed_at: string | null;
}

export const THERAPY_RECORD_TYPES = [
  "note",
  "exercise",
  "session_note",
  "resource",
  "reflection",
] as const;
export type TherapyRecordType = (typeof THERAPY_RECORD_TYPES)[number];

export const THERAPY_RECORD_STATUSES = ["open", "in_progress", "completed", "archived"] as const;
export type TherapyRecordStatus = (typeof THERAPY_RECORD_STATUSES)[number];

export const THERAPY_PRIVACY_SCOPES = ["therapy_private"] as const;
export type TherapyPrivacyScope = (typeof THERAPY_PRIVACY_SCOPES)[number];

export const THERAPY_REFERENCE_RELATIONS = [
  "supports",
  "prompted_by",
  "evidence_for",
  "related_to",
] as const;
export type TherapyReferenceRelation = (typeof THERAPY_REFERENCE_RELATIONS)[number];

export interface TherapyCollection {
  id: string;
  workspace_id: string;
  name: string;
  privacy_scope: TherapyPrivacyScope;
  encryption_status: string | null;
  created_at: string;
  updated_at: string;
  archived_at: string | null;
}

export interface TherapyRecord {
  id: string;
  therapy_collection_id: string;
  record_type: TherapyRecordType;
  title: string;
  content_markdown: string;
  occurred_on: string | null;
  status: TherapyRecordStatus;
  privacy_scope: TherapyPrivacyScope;
  created_at: string;
  updated_at: string;
  archived_at: string | null;
}

export interface TherapyTaskReference {
  id: string;
  therapy_record_id: string;
  task_id: string | null;
  relation: TherapyReferenceRelation;
  note_markdown: string | null;
  created_by: string;
  created_at: string;
  removed_at: string | null;
}

export const JOURNAL_SOURCE_LABELS: Record<JournalSource, string> = {
  user: "User",
  import: "Import",
  agent: "Agent",
};

export const JOURNAL_REFERENCE_RELATION_LABELS: Record<JournalReferenceRelation, string> = {
  mentions: "Mentions",
  related_to: "Related to",
  reflects_on: "Reflects on",
  result_of: "Result of",
};

export const THERAPY_RECORD_TYPE_LABELS: Record<TherapyRecordType, string> = {
  note: "Note",
  exercise: "Exercise",
  session_note: "Session note",
  resource: "Resource",
  reflection: "Reflection",
};

export const THERAPY_RECORD_STATUS_LABELS: Record<TherapyRecordStatus, string> = {
  open: "Open",
  in_progress: "In progress",
  completed: "Completed",
  archived: "Archived",
};

export const THERAPY_REFERENCE_RELATION_LABELS: Record<TherapyReferenceRelation, string> = {
  supports: "Supports",
  prompted_by: "Prompted by",
  evidence_for: "Evidence for",
  related_to: "Related to",
};

export interface CapacitySuggestion {
  start_time: string;
  end_time: string;
  preferred_block_length_minutes: number | null;
  energy_expectation: string | null;
  profile_id: string | null;
}

export const EXPORT_TYPES = ["markdown", "typst", "pdf", "timesheet", "metrics"] as const;
export type ExportType = (typeof EXPORT_TYPES)[number];

export const EXPORT_SCOPE_TYPES = [
  "journal",
  "therapy",
  "task",
  "project",
  "goal",
  "quarter",
  "custom",
] as const;
export type ExportScopeType = (typeof EXPORT_SCOPE_TYPES)[number];

export const EXPORT_JOB_STATUSES = [
  "queued",
  "running",
  "completed",
  "failed",
  "cancelled",
] as const;
export type ExportJobStatus = (typeof EXPORT_JOB_STATUSES)[number];

export interface DateRange {
  from: string;
  to: string;
}

export interface ExportSelection {
  journal_entry_ids?: string[] | null;
  therapy_record_ids?: string[] | null;
  task_ids?: string[] | null;
  project_ids?: string[] | null;
  goal_ids?: string[] | null;
  quarter_ids?: string[] | null;
  date_range?: DateRange | null;
  export_all?: boolean | null;
}

export interface ExportArtifact {
  id: string;
  format: string;
  path: string;
  checksum: string | null;
  size_bytes: number | null;
}

export interface SaveExportArtifactCopyResult {
  cancelled: boolean;
  destination_path: string;
  size_bytes: number;
}

export interface ExportWarning {
  code: string;
  severity: "info" | "warning" | "error";
  message: string;
  source?: string | null;
  phase?: string | null;
  category?: string | null;
  line?: number | null;
  column?: number | null;
}

export interface ExportJob {
  id: string;
  workspace_id: string;
  export_type: ExportType;
  scope_type: ExportScopeType;
  title: string;
  template_name: string | null;
  status: ExportJobStatus;
  output_path: string | null;
  generated_at: string | null;
  error_message: string | null;
  selection_fingerprint: string | null;
  warnings: ExportWarning[];
  created_at: string;
  updated_at: string;
  artifacts: ExportArtifact[];
}

export interface ExportPreviewItem {
  id: string;
  entity_type: string;
  title: string;
  occurred_on: string | null;
  privacy_scope: string;
  content_hash: string;
}

export interface ExportPreview {
  item_count: number;
  total_count?: number;
  truncated?: boolean;
  items: ExportPreviewItem[];
  resolved_selection: ExportSelection;
  selection_fingerprint: string;
  has_therapy_private: boolean;
}

export interface TemplateCapability {
  name: string;
  version: string;
  valid: boolean;
  compiler_version: string;
  unavailable_resources: string[];
}

export interface EnvironmentDiagnostics {
  sqlite_version: string;
  tauri_version: string;
  os_platform: string;
  os_version: string;
  typst_compiler_version: string;
  template_name: string;
  template_version: string;
  template_valid: boolean;
  bundled_font_count: number;
}

export type JournalTemplateFieldType =
  | "short_text"
  | "markdown"
  | "textarea"
  | "single_select"
  | "multi_select"
  | "custom_option"
  | "bounded_scale";

export interface TemplateManifestField {
  id: string;
  field_type: JournalTemplateFieldType;
  required: boolean;
  options?: string[];
}

export interface TemplateManifestSection {
  id: string;
  heading: string;
  fields: string[];
}

export interface TemplateManifest {
  id: string;
  version: string;
  display_name: string;
  locale: string;
  description: string;
  min_app_version: string;
  fields: TemplateManifestField[];
  sections: TemplateManifestSection[];
  cleanup: {
    remove_comments: boolean;
    remove_guide_blockquotes: boolean;
    remove_response_markers: boolean;
    remove_placeholders: boolean;
    remove_images: boolean;
    remove_empty_day_markers: boolean;
  };
  files: string[];
}

// Import / Export

export const IMPORT_WARNING_SEVERITIES = ["info", "warning", "error"] as const;
export type ImportWarningSeverity = (typeof IMPORT_WARNING_SEVERITIES)[number];

export const IMPORT_CONFIDENCES = ["high", "medium", "low", "review"] as const;
export type ImportConfidence = (typeof IMPORT_CONFIDENCES)[number];

export interface EntityCountDto {
  entity_type: string;
  count: number;
}

export interface ImportWarningDto {
  severity: ImportWarningSeverity;
  source_path: string;
  line: number | null;
  message: string;
}

export interface ImportReviewItemDto {
  id: string;
  source_path: string;
  proposed_entity_type: string;
  title: string | null;
  snippet: string | null;
  confidence: ImportConfidence;
  /// SHA-256 over the exact source file content, hex-encoded. Used for
  /// fingerprint verification at commit time.
  source_hash: string;
}

export interface ImportReportDto {
  batch_id: string;
  source_root: string;
  status: string;
  started_at: string;
  completed_at: string | null;
  counts: EntityCountDto[];
  warnings: ImportWarningDto[];
  review_items: ImportReviewItemDto[];
  /// Deterministic fingerprint over workspace_id + canonical source_root +
  /// include_paths + ordered (relative_path, content_hash, proposed_entity, confidence).
  /// Must be provided to commit_legacy_import to prove the preview has not changed.
  preview_fingerprint: string;
}

export interface ImportBatchStatusDto {
  batch_id: string;
  status: string;
  started_at: string;
  completed_at: string | null;
}

export interface ReviewDecisionDto {
  review_item_id: string;
  entity_type: string;
}

export interface CommitLegacyImportDto {
  workspace_id: string;
  source_root: string;
  include_paths: string[] | null;
  preview_fingerprint: string;
  decisions: ReviewDecisionDto[];
}

export const IMPORT_WARNING_SEVERITY_LABELS: Record<ImportWarningSeverity, string> = {
  info: "Info",
  warning: "Warning",
  error: "Error",
};

export const IMPORT_CONFIDENCE_LABELS: Record<ImportConfidence, string> = {
  high: "High confidence",
  medium: "Medium confidence",
  low: "Low confidence",
  review: "Needs review",
};

export const REMINDER_TYPES = ["planning", "focus", "journaling", "review", "deadline"] as const;
export type ReminderType = (typeof REMINDER_TYPES)[number];

export const NOTIFICATION_CHANNELS = ["desktop", "mobile", "both"] as const;
export type NotificationChannel = (typeof NOTIFICATION_CHANNELS)[number];

export const REMINDER_ESCALATIONS = ["none", "gentle"] as const;
export type ReminderEscalation = (typeof REMINDER_ESCALATIONS)[number];

export const NOTIFICATION_DELIVERY_STATUSES = [
  "scheduled",
  "delivered",
  "snoozed",
  "dismissed",
  "failed",
] as const;
export type NotificationDeliveryStatus = (typeof NOTIFICATION_DELIVERY_STATUSES)[number];
export type NotificationDeliveryKind = "reminder_rule" | "backup" | "block";

export interface ReminderRule {
  id: string;
  workspace_id: string;
  reminder_type: ReminderType;
  enabled: boolean;
  recurrence_rule: string | null;
  preferred_time: string | null;
  time_window_start: string | null;
  time_window_end: string | null;
  quiet_hours_start: string | null;
  quiet_hours_end: string | null;
  deadline_notification_window_days: number;
  max_reminders_per_day: number;
  snooze_minutes: number[];
  channel: NotificationChannel;
  escalation: ReminderEscalation;
  privacy_safe_message: string;
  deliveries_today: number;
  has_delivery_today: boolean;
}

export const NOTIFICATION_TARGET_ENTITY_TYPES = [
  "task",
  "project",
  "goal",
  "block",
  "session",
] as const;
export type NotificationTargetEntityType = (typeof NOTIFICATION_TARGET_ENTITY_TYPES)[number];

export interface NotificationTarget {
  entity_type: NotificationTargetEntityType;
  entity_id: string;
}

export interface NotificationDelivery {
  id: string;
  workspace_id: string;
  delivery_kind: NotificationDeliveryKind;
  // Matches the native serialization exactly: backup deliveries are scoped to
  // the workspace with no rule and no target, so `reminder_rule_id` is null for
  // `backup` rows, and `cycle_key` (the effective scheduled slot) is set only
  // for `backup` rows. Rule deliveries always keep `cycle_key` null.
  reminder_rule_id: string | null;
  target: NotificationTarget | null;
  scheduled_at: string;
  delivered_at: string | null;
  status: NotificationDeliveryStatus;
  channel: NotificationChannel;
  message: string;
  error_message: string | null;
  created_at: string;
  cycle_key: string | null;
}

export const NOTIFICATION_DELIVERY_STATUS_LABELS: Record<NotificationDeliveryStatus, string> = {
  scheduled: "Scheduled",
  delivered: "Delivered",
  snoozed: "Snoozed",
  dismissed: "Dismissed",
  failed: "Failed",
};

export const REMINDER_TYPE_LABELS: Record<ReminderType, string> = {
  planning: "Daily planning",
  focus: "Focus block",
  journaling: "Journaling",
  review: "Weekly review",
  deadline: "Deadline check",
};

export const NOTIFICATION_CHANNEL_LABELS: Record<NotificationChannel, string> = {
  desktop: "Desktop",
  mobile: "Mobile",
  both: "Desktop and mobile",
};

export const REMINDER_ESCALATION_LABELS: Record<ReminderEscalation, string> = {
  none: "No escalation",
  gentle: "Gentle follow-up",
};
