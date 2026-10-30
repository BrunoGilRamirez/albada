use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::domain::types::{
    BlockLifecycleState, BlockStatus, BlockType, ClosureMode, DailyPlanStatus, EffortSize,
    FixedCommitmentType, Flexibility, GoalStatus, JournalReferenceEntityType,
    JournalReferenceRelation, JournalSource, PlannedStatus, Priority, ProjectStatus,
    SelectionReason, SessionStatus, TaskStatus, TherapyPrivacyScope, TherapyRecordStatus,
    TherapyRecordType, TherapyReferenceRelation,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub root_path: Option<String>,
    pub timezone: String,
    pub locale: String,
    pub default_privacy_scope: String,
    pub current_year_theme_id: Option<String>,
    pub settings_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YearTheme {
    pub id: String,
    pub workspace_id: String,
    pub year: i64,
    pub name: String,
    pub description_markdown: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quarter {
    pub id: String,
    pub year_theme_id: String,
    pub code: String,
    pub start_date: String,
    pub end_date: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuarterFocus {
    pub id: String,
    pub quarter_id: String,
    pub goal_id: Option<String>,
    pub project_id: Option<String>,
    pub priority: Priority,
    pub intention_markdown: Option<String>,
    pub success_signal_markdown: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub workspace_id: String,
    pub year_theme_id: Option<String>,
    pub title: String,
    pub description_markdown: Option<String>,
    pub why_it_matters_markdown: Option<String>,
    pub success_definition_markdown: Option<String>,
    pub status: GoalStatus,
    pub priority: Priority,
    pub target_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub workspace_id: String,
    pub goal_id: Option<String>,
    pub title: String,
    pub description_markdown: Option<String>,
    pub why_it_matters_markdown: Option<String>,
    pub definition_of_done_markdown: Option<String>,
    pub status: ProjectStatus,
    pub priority: Priority,
    pub target_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub goal_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub title: String,
    pub description_markdown: Option<String>,
    pub notes_markdown: Option<String>,
    pub blocking_note_markdown: Option<String>,
    pub status: TaskStatus,
    pub priority: Priority,
    pub effort_size: Option<EffortSize>,
    pub effort_points: i64,
    pub target_date: Option<String>,
    pub scheduled_date: Option<String>,
    pub completed_at: Option<String>,
    pub cancelled_at: Option<String>,
    pub closure_mode: Option<ClosureMode>,
    pub closure_reason_markdown: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDependency {
    pub id: String,
    pub blocked_task_id: String,
    pub prerequisite_task_id: String,
    pub dependency_type: String,
    pub note_markdown: Option<String>,
    pub created_at: String,
    pub removed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusHistoryEntry {
    pub id: String,
    pub task_id: String,
    pub from_status: Option<String>,
    pub to_status: String,
    pub reason_markdown: Option<String>,
    pub source: String,
    pub metadata_json: Option<String>,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyPlan {
    pub id: String,
    pub workspace_id: String,
    pub date: String,
    pub status: DailyPlanStatus,
    pub intention_markdown: Option<String>,
    pub capacity_note_markdown: Option<String>,
    pub review_markdown: Option<String>,
    pub planning_block_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyPlanTask {
    pub id: String,
    pub daily_plan_id: String,
    pub task_id: String,
    pub order_index: i64,
    pub selection_reason: SelectionReason,
    pub planned_status: PlannedStatus,
    pub carryover_from_daily_plan_id: Option<String>,
    pub decision_note_markdown: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    pub workspace_id: String,
    pub daily_plan_id: Option<String>,
    pub date: String,
    pub block_type: BlockType,
    pub status: BlockStatus,
    pub label: Option<String>,
    pub planned_start_at: Option<String>,
    pub planned_end_at: Option<String>,
    pub estimated_duration_seconds: Option<i64>,
    pub recurrence_id: Option<String>,
    pub recurrence_version_id: Option<String>,
    pub local_date: Option<String>,
    pub local_start_time: Option<String>,
    pub timezone_id: Option<String>,
    pub resolved_start_at: Option<String>,
    pub time_window_label: Option<String>,
    pub actual_start_at: Option<String>,
    pub actual_end_at: Option<String>,
    pub flexibility: Flexibility,
    pub note_markdown: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub lifecycle_state: BlockLifecycleState,
    pub task_id: Option<String>,
    pub reactivated_from_block_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockRecurrence {
    pub id: String,
    pub workspace_id: String,
    pub version_id: String,
    pub weekdays: Vec<i64>,
    pub local_start_time: String,
    pub estimated_duration_seconds: i64,
    pub block_type: BlockType,
    pub label: Option<String>,
    pub flexibility: Flexibility,
    pub note_markdown: Option<String>,
    pub enabled: bool,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub lifecycle_state: BlockLifecycleState,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledBlockRecurrence {
    pub recurrence: BlockRecurrence,
    pub next_occurrence_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSession {
    pub id: String,
    pub task_id: String,
    pub block_id: Option<String>,
    pub daily_plan_task_id: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub status: SessionStatus,
    pub outcome_markdown: Option<String>,
    pub next_step_markdown: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixedCommitment {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub description_markdown: Option<String>,
    pub commitment_type: FixedCommitmentType,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub duration_minutes: Option<i64>,
    pub recurrence_rule: Option<String>,
    pub movable: bool,
    pub active: bool,
    pub privacy_scope: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailabilityProfile {
    pub id: String,
    pub workspace_id: String,
    pub weekday: i64,
    pub start_time: String,
    pub end_time: String,
    pub preferred_block_length_minutes: Option<i64>,
    pub energy_expectation: Option<String>,
    pub active_from: Option<String>,
    pub active_until: Option<String>,
    pub note_markdown: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: String,
    pub workspace_id: String,
    pub occurred_on: String,
    pub title: String,
    pub content_markdown: String,
    pub template_id: Option<String>,
    pub template_version: Option<String>,
    pub template_values: BTreeMap<String, Value>,
    pub mood_label: Option<String>,
    pub energy_label: Option<String>,
    pub emotions: Vec<String>,
    pub export_eligible: bool,
    pub source: JournalSource,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalTaskReference {
    pub id: String,
    pub journal_entry_id: String,
    pub entity_type: JournalReferenceEntityType,
    pub entity_id: String,
    pub relation: JournalReferenceRelation,
    pub created_by: String,
    pub confidence: Option<f64>,
    pub created_at: String,
    pub removed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TherapyCollection {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub privacy_scope: TherapyPrivacyScope,
    pub encryption_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TherapyRecord {
    pub id: String,
    pub therapy_collection_id: String,
    pub record_type: TherapyRecordType,
    pub title: String,
    pub content_markdown: String,
    pub occurred_on: Option<String>,
    pub status: TherapyRecordStatus,
    pub privacy_scope: TherapyPrivacyScope,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TherapyTaskReference {
    pub id: String,
    pub therapy_record_id: String,
    pub task_id: Option<String>,
    pub relation: TherapyReferenceRelation,
    pub note_markdown: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub removed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportJob {
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
    pub diagnostics_json: Option<String>,
    pub options_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub artifacts: Vec<ExportArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportArtifact {
    pub id: String,
    pub export_job_id: String,
    pub format: String,
    pub path: String,
    pub checksum: Option<String>,
    pub size_bytes: Option<i64>,
    pub created_at: String,
    pub deleted_at: Option<String>,
}
