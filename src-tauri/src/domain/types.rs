use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Operational state of an execution task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Inbox,
    Planned,
    InProgress,
    Blocked,
    Completed,
    Cancelled,
    Deferred,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Inbox => "inbox",
            TaskStatus::Planned => "planned",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Blocked => "blocked",
            TaskStatus::Completed => "completed",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::Deferred => "deferred",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Cancelled)
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TaskStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inbox" => Ok(TaskStatus::Inbox),
            "planned" => Ok(TaskStatus::Planned),
            "in_progress" => Ok(TaskStatus::InProgress),
            "blocked" => Ok(TaskStatus::Blocked),
            "completed" => Ok(TaskStatus::Completed),
            "cancelled" => Ok(TaskStatus::Cancelled),
            "deferred" => Ok(TaskStatus::Deferred),
            other => Err(AppError::validation(format!(
                "Unknown task status '{other}'."
            ))),
        }
    }
}

/// Status of a goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalStatus {
    Inbox,
    Active,
    Paused,
    Achieved,
    Abandoned,
    Archived,
}

impl GoalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            GoalStatus::Inbox => "inbox",
            GoalStatus::Active => "active",
            GoalStatus::Paused => "paused",
            GoalStatus::Achieved => "achieved",
            GoalStatus::Abandoned => "abandoned",
            GoalStatus::Archived => "archived",
        }
    }
}

impl fmt::Display for GoalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GoalStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inbox" => Ok(GoalStatus::Inbox),
            "active" => Ok(GoalStatus::Active),
            "paused" => Ok(GoalStatus::Paused),
            "achieved" => Ok(GoalStatus::Achieved),
            "abandoned" => Ok(GoalStatus::Abandoned),
            "archived" => Ok(GoalStatus::Archived),
            other => Err(AppError::validation(format!(
                "Unknown goal status '{other}'."
            ))),
        }
    }
}

/// Status of a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Inbox,
    Planned,
    Active,
    Paused,
    Completed,
    Cancelled,
    Archived,
}

impl ProjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectStatus::Inbox => "inbox",
            ProjectStatus::Planned => "planned",
            ProjectStatus::Active => "active",
            ProjectStatus::Paused => "paused",
            ProjectStatus::Completed => "completed",
            ProjectStatus::Cancelled => "cancelled",
            ProjectStatus::Archived => "archived",
        }
    }
}

impl fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ProjectStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inbox" => Ok(ProjectStatus::Inbox),
            "planned" => Ok(ProjectStatus::Planned),
            "active" => Ok(ProjectStatus::Active),
            "paused" => Ok(ProjectStatus::Paused),
            "completed" => Ok(ProjectStatus::Completed),
            "cancelled" => Ok(ProjectStatus::Cancelled),
            "archived" => Ok(ProjectStatus::Archived),
            other => Err(AppError::validation(format!(
                "Unknown project status '{other}'."
            ))),
        }
    }
}

/// Priority expresses relevance or need for attention, not effort.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    None,
    Low,
    Normal,
    High,
    Critical,
}

impl Priority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Priority::None => "none",
            Priority::Low => "low",
            Priority::Normal => "normal",
            Priority::High => "high",
            Priority::Critical => "critical",
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Priority {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Priority::None),
            "low" => Ok(Priority::Low),
            "normal" => Ok(Priority::Normal),
            "high" => Ok(Priority::High),
            "critical" => Ok(Priority::Critical),
            other => Err(AppError::validation(format!("Unknown priority '{other}'."))),
        }
    }
}

/// Effort size maps to internal effort points 1, 2, 3, 5, 8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffortSize {
    Trivial,
    Small,
    Medium,
    Large,
    VeryLarge,
}

impl EffortSize {
    pub fn as_str(&self) -> &'static str {
        match self {
            EffortSize::Trivial => "trivial",
            EffortSize::Small => "small",
            EffortSize::Medium => "medium",
            EffortSize::Large => "large",
            EffortSize::VeryLarge => "very_large",
        }
    }

    pub fn points(self) -> i64 {
        match self {
            EffortSize::Trivial => 1,
            EffortSize::Small => 2,
            EffortSize::Medium => 3,
            EffortSize::Large => 5,
            EffortSize::VeryLarge => 8,
        }
    }
}

impl fmt::Display for EffortSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EffortSize {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trivial" => Ok(EffortSize::Trivial),
            "small" => Ok(EffortSize::Small),
            "medium" => Ok(EffortSize::Medium),
            "large" => Ok(EffortSize::Large),
            "very_large" => Ok(EffortSize::VeryLarge),
            other => Err(AppError::validation(format!(
                "Unknown effort size '{other}'."
            ))),
        }
    }
}

/// Why a parent task with open children was closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClosureMode {
    Automatic,
    ManualNoLongerApplicable,
    ManualSolvedElsewhere,
    ManualAcceptedPartial,
    ManualOther,
}

impl ClosureMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClosureMode::Automatic => "automatic",
            ClosureMode::ManualNoLongerApplicable => "manual_no_longer_applicable",
            ClosureMode::ManualSolvedElsewhere => "manual_solved_elsewhere",
            ClosureMode::ManualAcceptedPartial => "manual_accepted_partial",
            ClosureMode::ManualOther => "manual_other",
        }
    }

    pub fn is_manual(self) -> bool {
        !matches!(self, ClosureMode::Automatic)
    }
}

impl fmt::Display for ClosureMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ClosureMode {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "automatic" => Ok(ClosureMode::Automatic),
            "manual_no_longer_applicable" => Ok(ClosureMode::ManualNoLongerApplicable),
            "manual_solved_elsewhere" => Ok(ClosureMode::ManualSolvedElsewhere),
            "manual_accepted_partial" => Ok(ClosureMode::ManualAcceptedPartial),
            "manual_other" => Ok(ClosureMode::ManualOther),
            other => Err(AppError::validation(format!(
                "Unknown closure mode '{other}'."
            ))),
        }
    }
}

/// Origin of a status-history entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistorySource {
    User,
    Session,
    Import,
    Agent,
    System,
}

impl HistorySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            HistorySource::User => "user",
            HistorySource::Session => "session",
            HistorySource::Import => "import",
            HistorySource::Agent => "agent",
            HistorySource::System => "system",
        }
    }
}

impl fmt::Display for HistorySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for HistorySource {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(HistorySource::User),
            "session" => Ok(HistorySource::Session),
            "import" => Ok(HistorySource::Import),
            "agent" => Ok(HistorySource::Agent),
            "system" => Ok(HistorySource::System),
            other => Err(AppError::validation(format!(
                "Unknown history source '{other}'."
            ))),
        }
    }
}

/// Lifecycle of a daily plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DailyPlanStatus {
    NotStarted,
    Planning,
    Planned,
    InReview,
    Closed,
}

impl DailyPlanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DailyPlanStatus::NotStarted => "not_started",
            DailyPlanStatus::Planning => "planning",
            DailyPlanStatus::Planned => "planned",
            DailyPlanStatus::InReview => "in_review",
            DailyPlanStatus::Closed => "closed",
        }
    }
}

impl fmt::Display for DailyPlanStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DailyPlanStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "not_started" => Ok(DailyPlanStatus::NotStarted),
            "planning" => Ok(DailyPlanStatus::Planning),
            "planned" => Ok(DailyPlanStatus::Planned),
            "in_review" => Ok(DailyPlanStatus::InReview),
            "closed" => Ok(DailyPlanStatus::Closed),
            other => Err(AppError::validation(format!(
                "Unknown daily plan status '{other}'."
            ))),
        }
    }
}

/// Why a task was selected for a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionReason {
    NextAction,
    Priority,
    Deadline,
    Dependency,
    Manual,
}

impl SelectionReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            SelectionReason::NextAction => "next_action",
            SelectionReason::Priority => "priority",
            SelectionReason::Deadline => "deadline",
            SelectionReason::Dependency => "dependency",
            SelectionReason::Manual => "manual",
        }
    }
}

impl fmt::Display for SelectionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SelectionReason {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "next_action" => Ok(SelectionReason::NextAction),
            "priority" => Ok(SelectionReason::Priority),
            "deadline" => Ok(SelectionReason::Deadline),
            "dependency" => Ok(SelectionReason::Dependency),
            "manual" => Ok(SelectionReason::Manual),
            other => Err(AppError::validation(format!(
                "Unknown selection reason '{other}'."
            ))),
        }
    }
}

/// Intended outcome of a selected task within a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannedStatus {
    Selected,
    Started,
    Completed,
    Deferred,
    Skipped,
}

impl PlannedStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlannedStatus::Selected => "selected",
            PlannedStatus::Started => "started",
            PlannedStatus::Completed => "completed",
            PlannedStatus::Deferred => "deferred",
            PlannedStatus::Skipped => "skipped",
        }
    }
}

impl fmt::Display for PlannedStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PlannedStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "selected" => Ok(PlannedStatus::Selected),
            "started" => Ok(PlannedStatus::Started),
            "completed" => Ok(PlannedStatus::Completed),
            "deferred" => Ok(PlannedStatus::Deferred),
            "skipped" => Ok(PlannedStatus::Skipped),
            other => Err(AppError::validation(format!(
                "Unknown planned status '{other}'."
            ))),
        }
    }
}

/// Kind of intention a block represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockType {
    Planning,
    Focus,
    Review,
    Journaling,
    Rest,
    Personal,
}

impl BlockType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockType::Planning => "planning",
            BlockType::Focus => "focus",
            BlockType::Review => "review",
            BlockType::Journaling => "journaling",
            BlockType::Rest => "rest",
            BlockType::Personal => "personal",
        }
    }

    pub fn all() -> &'static [BlockType] {
        &[
            BlockType::Planning,
            BlockType::Focus,
            BlockType::Review,
            BlockType::Journaling,
            BlockType::Rest,
            BlockType::Personal,
        ]
    }
}

impl fmt::Display for BlockType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BlockType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "planning" => Ok(BlockType::Planning),
            "focus" => Ok(BlockType::Focus),
            "review" => Ok(BlockType::Review),
            "journaling" => Ok(BlockType::Journaling),
            "rest" => Ok(BlockType::Rest),
            "personal" => Ok(BlockType::Personal),
            other => Err(AppError::validation(format!(
                "Unknown block type '{other}'."
            ))),
        }
    }
}

/// Lifecycle of a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockStatus {
    Planned,
    Active,
    Completed,
    Skipped,
    Cancelled,
}

impl BlockStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockStatus::Planned => "planned",
            BlockStatus::Active => "active",
            BlockStatus::Completed => "completed",
            BlockStatus::Skipped => "skipped",
            BlockStatus::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for BlockStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BlockStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "planned" => Ok(BlockStatus::Planned),
            "active" => Ok(BlockStatus::Active),
            "completed" => Ok(BlockStatus::Completed),
            "skipped" => Ok(BlockStatus::Skipped),
            "cancelled" => Ok(BlockStatus::Cancelled),
            other => Err(AppError::validation(format!(
                "Unknown block status '{other}'."
            ))),
        }
    }
}

/// Lifecycle of a block or block recurrence, separate from its work status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockLifecycleState {
    Active,
    Inactive,
    Deleted,
}

impl BlockLifecycleState {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockLifecycleState::Active => "active",
            BlockLifecycleState::Inactive => "inactive",
            BlockLifecycleState::Deleted => "deleted",
        }
    }
}

impl fmt::Display for BlockLifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BlockLifecycleState {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(BlockLifecycleState::Active),
            "inactive" => Ok(BlockLifecycleState::Inactive),
            "deleted" => Ok(BlockLifecycleState::Deleted),
            other => Err(AppError::validation(format!(
                "Unknown block lifecycle state '{other}'."
            ))),
        }
    }
}

/// How rigid a block's intended time is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Flexibility {
    Fixed,
    Flexible,
    Approximate,
}

impl Flexibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Flexibility::Fixed => "fixed",
            Flexibility::Flexible => "flexible",
            Flexibility::Approximate => "approximate",
        }
    }
}

impl fmt::Display for Flexibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Flexibility {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fixed" => Ok(Flexibility::Fixed),
            "flexible" => Ok(Flexibility::Flexible),
            "approximate" => Ok(Flexibility::Approximate),
            other => Err(AppError::validation(format!(
                "Unknown flexibility '{other}'."
            ))),
        }
    }
}

/// Lifecycle of a task session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Planned,
    Started,
    Finished,
    Paused,
    Skipped,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Planned => "planned",
            SessionStatus::Started => "started",
            SessionStatus::Finished => "finished",
            SessionStatus::Paused => "paused",
            SessionStatus::Skipped => "skipped",
        }
    }
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SessionStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "planned" => Ok(SessionStatus::Planned),
            "started" => Ok(SessionStatus::Started),
            "finished" => Ok(SessionStatus::Finished),
            "paused" => Ok(SessionStatus::Paused),
            "skipped" => Ok(SessionStatus::Skipped),
            other => Err(AppError::validation(format!(
                "Unknown session status '{other}'."
            ))),
        }
    }
}

/// Category of an immovable commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixedCommitmentType {
    Sleep,
    Meal,
    Work,
    Appointment,
    Exercise,
    Other,
}

impl FixedCommitmentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FixedCommitmentType::Sleep => "sleep",
            FixedCommitmentType::Meal => "meal",
            FixedCommitmentType::Work => "work",
            FixedCommitmentType::Appointment => "appointment",
            FixedCommitmentType::Exercise => "exercise",
            FixedCommitmentType::Other => "other",
        }
    }
}

impl fmt::Display for FixedCommitmentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for FixedCommitmentType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sleep" => Ok(FixedCommitmentType::Sleep),
            "meal" => Ok(FixedCommitmentType::Meal),
            "work" => Ok(FixedCommitmentType::Work),
            "appointment" => Ok(FixedCommitmentType::Appointment),
            "exercise" => Ok(FixedCommitmentType::Exercise),
            "other" => Ok(FixedCommitmentType::Other),
            other => Err(AppError::validation(format!(
                "Unknown fixed commitment type '{other}'."
            ))),
        }
    }
}

/// Explicit replanning decision for a missed or changed intention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplanDecision {
    MoveToNextDay,
    ReturnToPlanned,
    RemoveDate,
    Defer,
    Divide,
}

impl ReplanDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReplanDecision::MoveToNextDay => "move_to_next_day",
            ReplanDecision::ReturnToPlanned => "return_to_planned",
            ReplanDecision::RemoveDate => "remove_date",
            ReplanDecision::Defer => "defer",
            ReplanDecision::Divide => "divide",
        }
    }
}

impl fmt::Display for ReplanDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ReplanDecision {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "move_to_next_day" => Ok(ReplanDecision::MoveToNextDay),
            "return_to_planned" => Ok(ReplanDecision::ReturnToPlanned),
            "remove_date" => Ok(ReplanDecision::RemoveDate),
            "defer" => Ok(ReplanDecision::Defer),
            "divide" => Ok(ReplanDecision::Divide),
            other => Err(AppError::validation(format!(
                "Unknown replan decision '{other}'."
            ))),
        }
    }
}

/// Origin of a journal entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalSource {
    User,
    Import,
    Agent,
}

impl JournalSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            JournalSource::User => "user",
            JournalSource::Import => "import",
            JournalSource::Agent => "agent",
        }
    }
}

impl fmt::Display for JournalSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for JournalSource {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(JournalSource::User),
            "import" => Ok(JournalSource::Import),
            "agent" => Ok(JournalSource::Agent),
            other => Err(AppError::validation(format!(
                "Unknown journal source '{other}'."
            ))),
        }
    }
}

/// Kind of entity a journal reference points to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalReferenceEntityType {
    Task,
    Project,
    Goal,
    Block,
    Session,
}

impl JournalReferenceEntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            JournalReferenceEntityType::Task => "task",
            JournalReferenceEntityType::Project => "project",
            JournalReferenceEntityType::Goal => "goal",
            JournalReferenceEntityType::Block => "block",
            JournalReferenceEntityType::Session => "session",
        }
    }
}

impl fmt::Display for JournalReferenceEntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for JournalReferenceEntityType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "task" => Ok(JournalReferenceEntityType::Task),
            "project" => Ok(JournalReferenceEntityType::Project),
            "goal" => Ok(JournalReferenceEntityType::Goal),
            "block" => Ok(JournalReferenceEntityType::Block),
            "session" => Ok(JournalReferenceEntityType::Session),
            other => Err(AppError::validation(format!(
                "Unknown journal reference entity type '{other}'."
            ))),
        }
    }
}

/// How a journal reference relates to the referenced entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalReferenceRelation {
    Mentions,
    RelatedTo,
    ReflectsOn,
    ResultOf,
}

impl JournalReferenceRelation {
    pub fn as_str(&self) -> &'static str {
        match self {
            JournalReferenceRelation::Mentions => "mentions",
            JournalReferenceRelation::RelatedTo => "related_to",
            JournalReferenceRelation::ReflectsOn => "reflects_on",
            JournalReferenceRelation::ResultOf => "result_of",
        }
    }
}

impl fmt::Display for JournalReferenceRelation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for JournalReferenceRelation {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mentions" => Ok(JournalReferenceRelation::Mentions),
            "related_to" => Ok(JournalReferenceRelation::RelatedTo),
            "reflects_on" => Ok(JournalReferenceRelation::ReflectsOn),
            "result_of" => Ok(JournalReferenceRelation::ResultOf),
            other => Err(AppError::validation(format!(
                "Unknown journal reference relation '{other}'."
            ))),
        }
    }
}

/// Serialize an enum via its snake_case string representation for SQLite rows.
pub fn enum_to_db<E: fmt::Display>(value: E) -> String {
    value.to_string()
}

/// Parse a snake_case string from the database into an enum.
pub fn enum_from_db<E: FromStr<Err = AppError>>(value: &str) -> Result<E, AppError> {
    E::from_str(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TherapyRecordType {
    Note,
    Exercise,
    SessionNote,
    Resource,
    Reflection,
}

impl TherapyRecordType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Exercise => "exercise",
            Self::SessionNote => "session_note",
            Self::Resource => "resource",
            Self::Reflection => "reflection",
        }
    }
}
impl fmt::Display for TherapyRecordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl FromStr for TherapyRecordType {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "note" => Ok(Self::Note),
            "exercise" => Ok(Self::Exercise),
            "session_note" => Ok(Self::SessionNote),
            "resource" => Ok(Self::Resource),
            "reflection" => Ok(Self::Reflection),
            other => Err(AppError::validation(format!(
                "Unknown therapy record type '{other}'."
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TherapyRecordStatus {
    Open,
    InProgress,
    Completed,
    Archived,
}
impl TherapyRecordStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }
}
impl fmt::Display for TherapyRecordStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl FromStr for TherapyRecordStatus {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "open" => Ok(Self::Open),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "archived" => Ok(Self::Archived),
            other => Err(AppError::validation(format!(
                "Unknown therapy record status '{other}'."
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TherapyPrivacyScope {
    TherapyPrivate,
}
impl TherapyPrivacyScope {
    pub fn as_str(&self) -> &'static str {
        "therapy_private"
    }
}
impl fmt::Display for TherapyPrivacyScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl FromStr for TherapyPrivacyScope {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "therapy_private" => Ok(Self::TherapyPrivate),
            other => Err(AppError::validation(format!(
                "Unknown therapy privacy scope '{other}'."
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TherapyReferenceRelation {
    Supports,
    PromptedBy,
    EvidenceFor,
    RelatedTo,
}
impl TherapyReferenceRelation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Supports => "supports",
            Self::PromptedBy => "prompted_by",
            Self::EvidenceFor => "evidence_for",
            Self::RelatedTo => "related_to",
        }
    }
}
impl fmt::Display for TherapyReferenceRelation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl FromStr for TherapyReferenceRelation {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "supports" => Ok(Self::Supports),
            "prompted_by" => Ok(Self::PromptedBy),
            "evidence_for" => Ok(Self::EvidenceFor),
            "related_to" => Ok(Self::RelatedTo),
            other => Err(AppError::validation(format!(
                "Unknown therapy reference relation '{other}'."
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportType {
    Markdown,
    Typst,
    Pdf,
    Timesheet,
    Metrics,
}

impl ExportType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExportType::Markdown => "markdown",
            ExportType::Typst => "typst",
            ExportType::Pdf => "pdf",
            ExportType::Timesheet => "timesheet",
            ExportType::Metrics => "metrics",
        }
    }
}

impl fmt::Display for ExportType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ExportType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "markdown" => Ok(ExportType::Markdown),
            "typst" => Ok(ExportType::Typst),
            "pdf" => Ok(ExportType::Pdf),
            "timesheet" => Ok(ExportType::Timesheet),
            "metrics" => Ok(ExportType::Metrics),
            other => Err(AppError::validation(format!(
                "Unknown export type '{other}'."
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportScopeType {
    Journal,
    Therapy,
    Task,
    Project,
    Goal,
    Quarter,
    Custom,
}

impl ExportScopeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExportScopeType::Journal => "journal",
            ExportScopeType::Therapy => "therapy",
            ExportScopeType::Task => "task",
            ExportScopeType::Project => "project",
            ExportScopeType::Goal => "goal",
            ExportScopeType::Quarter => "quarter",
            ExportScopeType::Custom => "custom",
        }
    }
}

impl fmt::Display for ExportScopeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ExportScopeType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "journal" => Ok(ExportScopeType::Journal),
            "therapy" => Ok(ExportScopeType::Therapy),
            "task" => Ok(ExportScopeType::Task),
            "project" => Ok(ExportScopeType::Project),
            "goal" => Ok(ExportScopeType::Goal),
            "quarter" => Ok(ExportScopeType::Quarter),
            "custom" => Ok(ExportScopeType::Custom),
            other => Err(AppError::validation(format!(
                "Unknown export scope type '{other}'."
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl ExportJobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExportJobStatus::Queued => "queued",
            ExportJobStatus::Running => "running",
            ExportJobStatus::Completed => "completed",
            ExportJobStatus::Failed => "failed",
            ExportJobStatus::Cancelled => "cancelled",
        }
    }

    /// Terminal states never transition again and never return to `running`.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ExportJobStatus::Completed | ExportJobStatus::Failed | ExportJobStatus::Cancelled
        )
    }

    /// Whether `self -> next` is a legal transition under the Phase 7.10 state
    /// machine:
    /// - `queued -> running`
    /// - `queued/running -> failed`
    /// - `queued/running -> cancelled`
    /// - `running -> completed`
    ///
    /// Terminal states have no outgoing transitions.
    pub fn can_transition_to(&self, next: ExportJobStatus) -> bool {
        matches!(
            (self, next),
            (ExportJobStatus::Queued, ExportJobStatus::Running)
                | (ExportJobStatus::Queued, ExportJobStatus::Failed)
                | (ExportJobStatus::Queued, ExportJobStatus::Cancelled)
                | (ExportJobStatus::Running, ExportJobStatus::Completed)
                | (ExportJobStatus::Running, ExportJobStatus::Failed)
                | (ExportJobStatus::Running, ExportJobStatus::Cancelled)
        )
    }
}

impl fmt::Display for ExportJobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ExportJobStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(ExportJobStatus::Queued),
            "running" => Ok(ExportJobStatus::Running),
            "completed" => Ok(ExportJobStatus::Completed),
            "failed" => Ok(ExportJobStatus::Failed),
            "cancelled" => Ok(ExportJobStatus::Cancelled),
            other => Err(AppError::validation(format!(
                "Unknown export job status '{other}'."
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effort_points_map_correctly() {
        assert_eq!(EffortSize::Trivial.points(), 1);
        assert_eq!(EffortSize::Small.points(), 2);
        assert_eq!(EffortSize::Medium.points(), 3);
        assert_eq!(EffortSize::Large.points(), 5);
        assert_eq!(EffortSize::VeryLarge.points(), 8);
    }

    #[test]
    fn task_status_round_trips() {
        for status in [
            TaskStatus::Inbox,
            TaskStatus::Planned,
            TaskStatus::InProgress,
            TaskStatus::Blocked,
            TaskStatus::Completed,
            TaskStatus::Cancelled,
            TaskStatus::Deferred,
        ] {
            let parsed = TaskStatus::from_str(&status.to_string()).expect("parse status");
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn unknown_status_is_rejected() {
        assert!(TaskStatus::from_str("overdue").is_err());
    }
}
