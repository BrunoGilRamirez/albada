use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::commands::dto::{
    ExportPreviewItemDto, ExportSelectionDto, ResolvedSelectionItemDto,
    ResolvedSelectionSnapshotDto,
};
use crate::domain::models::{Goal, JournalEntry, Project, Quarter, Task, TherapyRecord};
use crate::domain::types::ExportScopeType;
use crate::error::AppError;
use crate::repos::goal_repo::{GoalRepo, SqliteGoalRepo};
use crate::repos::journal_entry_repo::{JournalEntryRepo, SqliteJournalEntryRepo};
use crate::repos::project_repo::{ProjectRepo, SqliteProjectRepo};
use crate::repos::quarter_repo::{QuarterRepo, SqliteQuarterRepo};
use crate::repos::task_repo::{SqliteTaskRepo, TaskRepo};
use crate::repos::therapy_record_repo::{SqliteTherapyRecordRepo, TherapyRecordRepo};
use crate::time;

/// Page size for chunked resolution of large exports (export_all, wide date ranges).
const EXPORT_RESOLUTION_PAGE_SIZE: usize = 500;

/// A concrete set of entities selected for export.
#[derive(Debug)]
pub struct ResolvedSelection {
    pub journal_entries: Vec<JournalEntry>,
    pub therapy_records: Vec<TherapyRecord>,
    pub tasks: Vec<Task>,
    pub projects: Vec<Project>,
    pub goals: Vec<Goal>,
    pub quarters: Vec<Quarter>,
}

impl ResolvedSelection {
    pub fn is_empty(&self) -> bool {
        self.journal_entries.is_empty()
            && self.therapy_records.is_empty()
            && self.tasks.is_empty()
            && self.projects.is_empty()
            && self.goals.is_empty()
            && self.quarters.is_empty()
    }

    pub fn item_count(&self) -> i64 {
        (self.journal_entries.len()
            + self.therapy_records.len()
            + self.tasks.len()
            + self.projects.len()
            + self.goals.len()
            + self.quarters.len()) as i64
    }

    /// Normalize every entity group into a stable order (ascending stable ID).
    ///
    /// The stable UUID id, not the input order, defines the order so that the
    /// same selection always yields the same preview order, content hashes, and
    /// fingerprint.
    fn sort_for_determinism(&mut self) {
        self.journal_entries.sort_by(|a, b| {
            a.occurred_on
                .cmp(&b.occurred_on)
                .then_with(|| a.title.cmp(&b.title))
                .then_with(|| a.id.cmp(&b.id))
        });
        self.therapy_records.sort_by(|a, b| a.id.cmp(&b.id));
        self.tasks.sort_by(|a, b| a.id.cmp(&b.id));
        self.projects.sort_by(|a, b| a.id.cmp(&b.id));
        self.goals.sort_by(|a, b| a.id.cmp(&b.id));
        self.quarters.sort_by(|a, b| a.id.cmp(&b.id));
    }
}

/// Reject selections that cross privacy boundaries for a given scope.
///
/// Journal and therapy are separate, private collections: a therapy ID must
/// never be resolved through a generic (`task`/`project`/`goal`/`quarter`) or
/// journal export, and a journal ID must never be resolved through a generic or
/// therapy export. This is the defensive enforcement behind the ADR's rule that
/// generic scopes reject therapy IDs and never traverse journal/therapy
/// relationships implicitly.
pub fn validate_scope_id_fields(
    scope_type: ExportScopeType,
    selection: &ExportSelectionDto,
) -> Result<(), AppError> {
    let journal = has_ids(&selection.journal_entry_ids);
    let therapy = has_ids(&selection.therapy_record_ids);
    let task = has_ids(&selection.task_ids);
    let project = has_ids(&selection.project_ids);
    let goal = has_ids(&selection.goal_ids);
    let quarter = has_ids(&selection.quarter_ids);
    let has_date_range = selection.date_range.is_some();
    let export_all = selection.export_all.unwrap_or(false);

    // export_all is only valid for journal and therapy scopes.
    if export_all
        && !matches!(
            scope_type,
            ExportScopeType::Journal | ExportScopeType::Therapy
        )
    {
        return Err(AppError::validation(
            "export_all is only supported for journal and therapy export scopes.",
        ));
    }

    // export_all is mutually exclusive with date_range and explicit ID lists for the same scope.
    if export_all {
        match scope_type {
            ExportScopeType::Journal if has_date_range || journal => {
                return Err(AppError::validation(
                    "export_all cannot be combined with date_range or explicit journal_entry_ids.",
                ));
            }
            ExportScopeType::Therapy if has_date_range || therapy => {
                return Err(AppError::validation(
                    "export_all cannot be combined with date_range or explicit therapy_record_ids.",
                ));
            }
            _ => {}
        }
    }

    match scope_type {
        ExportScopeType::Journal => {
            if therapy || task || project || goal || quarter {
                return Err(AppError::validation(
                    "A journal export can only include journal entries.",
                ));
            }
        }
        ExportScopeType::Therapy => {
            if journal || task || project || goal || quarter {
                return Err(AppError::validation(
                    "A therapy export can only include therapy records.",
                ));
            }
        }
        ExportScopeType::Task
        | ExportScopeType::Project
        | ExportScopeType::Goal
        | ExportScopeType::Quarter => {
            if journal {
                return Err(AppError::validation(
                    "Journal entries cannot be included in this export scope.",
                ));
            }
            if therapy {
                return Err(AppError::validation(
                    "Therapy records cannot be included in a generic export.",
                ));
            }
        }
        ExportScopeType::Custom => {}
    }
    Ok(())
}

fn has_ids(ids: &Option<Vec<String>>) -> bool {
    ids.as_ref().is_some_and(|v| !v.is_empty())
}

/// Resolve a front-end selection into real entities.
///
/// For `journal` and `therapy` scopes, a `date_range` is expanded into the
/// matching records. The resolved set must be non-empty for those scopes. The
/// result is normalized into a deterministic order (sorted by stable ID within
/// each entity group) so preview order, content hashes, and the fingerprint are
/// stable across calls regardless of the input ordering.
pub fn resolve_selection(
    conn: &Connection,
    workspace_id: &str,
    scope_type: ExportScopeType,
    selection: &ExportSelectionDto,
) -> Result<ResolvedSelection, AppError> {
    validate_scope_id_fields(scope_type, selection)?;

    let mut result = ResolvedSelection {
        journal_entries: Vec::new(),
        therapy_records: Vec::new(),
        tasks: Vec::new(),
        projects: Vec::new(),
        goals: Vec::new(),
        quarters: Vec::new(),
    };

    match scope_type {
        ExportScopeType::Journal => {
            result.journal_entries = resolve_journal_entries(conn, workspace_id, selection)?;
            if result.journal_entries.is_empty() {
                return Err(AppError::validation(
                    "At least one journal entry must be selected for export.",
                ));
            }
        }
        ExportScopeType::Therapy => {
            result.therapy_records = resolve_therapy_records(conn, workspace_id, selection)?;
            if result.therapy_records.is_empty() {
                return Err(AppError::validation(
                    "At least one therapy record must be selected for export.",
                ));
            }
        }
        ExportScopeType::Task => {
            result.tasks = resolve_tasks(conn, selection)?;
        }
        ExportScopeType::Project => {
            result.projects = resolve_projects(conn, selection)?;
        }
        ExportScopeType::Goal => {
            result.goals = resolve_goals(conn, selection)?;
        }
        ExportScopeType::Quarter => {
            result.quarters = resolve_quarters(conn, selection)?;
        }
        ExportScopeType::Custom => {
            result.journal_entries = resolve_journal_entries(conn, workspace_id, selection)?;
            result.therapy_records = resolve_therapy_records(conn, workspace_id, selection)?;
            result.tasks = resolve_tasks(conn, selection)?;
            result.projects = resolve_projects(conn, selection)?;
            result.goals = resolve_goals(conn, selection)?;
            result.quarters = resolve_quarters(conn, selection)?;
        }
    }

    result.sort_for_determinism();
    Ok(result)
}

/// Map a resolved selection to the preview DTO exposed to the UI.
///
/// Items are emitted in deterministic entity-type order (journal, therapy,
/// task, project, goal, quarter), each group already sorted by ID by
/// [`resolve_selection`]. Each item carries its stable ID, type, title, date,
/// privacy scope, and SHA-256 content hash; the content itself is never exposed.
pub fn to_preview_dto(selection: &ResolvedSelection) -> Vec<ExportPreviewItemDto> {
    let mut items = Vec::new();
    for entry in &selection.journal_entries {
        items.push(ExportPreviewItemDto {
            id: entry.id.clone(),
            entity_type: "journal_entry".to_string(),
            title: entry.title.clone(),
            occurred_on: Some(entry.occurred_on.clone()),
            privacy_scope: if entry.export_eligible {
                "export_eligible"
            } else {
                "private"
            }
            .to_string(),
            content_hash: journal_content_hash(entry),
        });
    }
    for record in &selection.therapy_records {
        items.push(ExportPreviewItemDto {
            id: record.id.clone(),
            entity_type: "therapy_record".to_string(),
            title: record.title.clone(),
            occurred_on: record.occurred_on.clone(),
            privacy_scope: record.privacy_scope.to_string(),
            content_hash: therapy_content_hash(record),
        });
    }
    for task in &selection.tasks {
        items.push(ExportPreviewItemDto {
            id: task.id.clone(),
            entity_type: "task".to_string(),
            title: task.title.clone(),
            occurred_on: None,
            privacy_scope: "export_eligible".to_string(),
            content_hash: task_content_hash(task),
        });
    }
    for project in &selection.projects {
        items.push(ExportPreviewItemDto {
            id: project.id.clone(),
            entity_type: "project".to_string(),
            title: project.title.clone(),
            occurred_on: None,
            privacy_scope: "export_eligible".to_string(),
            content_hash: project_content_hash(project),
        });
    }
    for goal in &selection.goals {
        items.push(ExportPreviewItemDto {
            id: goal.id.clone(),
            entity_type: "goal".to_string(),
            title: goal.title.clone(),
            occurred_on: None,
            privacy_scope: "export_eligible".to_string(),
            content_hash: goal_content_hash(goal),
        });
    }
    for quarter in &selection.quarters {
        items.push(ExportPreviewItemDto {
            id: quarter.id.clone(),
            entity_type: "quarter".to_string(),
            title: format!(
                "{} ({} to {})",
                quarter.code, quarter.start_date, quarter.end_date
            ),
            occurred_on: None,
            privacy_scope: "export_eligible".to_string(),
            content_hash: quarter_content_hash(quarter),
        });
    }
    items
}

/// Build the persisted selection snapshot (IDs plus content hashes) for a
/// resolved selection, in deterministic order.
pub fn to_snapshot_dto(selection: &ResolvedSelection) -> ResolvedSelectionSnapshotDto {
    ResolvedSelectionSnapshotDto {
        items: ordered_items(selection)
            .into_iter()
            .map(|item| ResolvedSelectionItemDto {
                entity_type: item.entity_type.to_string(),
                id: item.id,
                content_hash: item.content_hash,
            })
            .collect(),
    }
}

/// Compute the deterministic selection fingerprint over the ordered
/// (entity type, ID, content hash) triples.
pub fn selection_fingerprint(selection: &ResolvedSelection) -> String {
    let items = ordered_items(selection);
    let mut hasher = Sha256::new();
    for item in &items {
        for field in [
            item.entity_type,
            item.id.as_str(),
            item.content_hash.as_str(),
        ] {
            hasher.update((field.len() as u64).to_le_bytes());
            hasher.update(field.as_bytes());
        }
    }
    hex::encode(hasher.finalize())
}

/// Reconstruct a resolved selection from a stored snapshot and verify that the
/// current records' content hashes still match. Returns the resolved selection
/// on success, or a validation error when the snapshot is stale or incomplete.
///
/// Used by `run_export_job` to refuse generation when a journal or therapy
/// record (or any other selected record) changed after confirmation.
pub fn revalidate_snapshot(
    conn: &Connection,
    workspace_id: &str,
    scope_type: ExportScopeType,
    snapshot: &ResolvedSelectionSnapshotDto,
) -> Result<ResolvedSelection, AppError> {
    let selection = snapshot_to_selection_dto(snapshot)?;
    let resolved = resolve_selection(conn, workspace_id, scope_type, &selection)?;
    if to_snapshot_dto(&resolved) != *snapshot {
        return Err(AppError::validation(
            "The export selection is stale; preview again before exporting.",
        ));
    }
    Ok(resolved)
}

/// Convert a stored snapshot back into a concrete-ID selection DTO.
pub fn snapshot_to_selection_dto(
    snapshot: &ResolvedSelectionSnapshotDto,
) -> Result<ExportSelectionDto, AppError> {
    let mut dto = ExportSelectionDto {
        journal_entry_ids: None,
        therapy_record_ids: None,
        task_ids: None,
        project_ids: None,
        goal_ids: None,
        quarter_ids: None,
        date_range: None,
        export_all: None,
    };
    for item in &snapshot.items {
        let ids = match item.entity_type.as_str() {
            "journal_entry" => dto.journal_entry_ids.get_or_insert_with(Vec::new),
            "therapy_record" => dto.therapy_record_ids.get_or_insert_with(Vec::new),
            "task" => dto.task_ids.get_or_insert_with(Vec::new),
            "project" => dto.project_ids.get_or_insert_with(Vec::new),
            "goal" => dto.goal_ids.get_or_insert_with(Vec::new),
            "quarter" => dto.quarter_ids.get_or_insert_with(Vec::new),
            other => {
                return Err(AppError::database(format!(
                    "The export selection references an unknown entity type '{other}'."
                )))
            }
        };
        ids.push(item.id.clone());
    }
    Ok(dto)
}

/// Build the resolved-selection DTO (concrete IDs) for the preview response.
pub fn to_resolved_selection_dto(selection: &ResolvedSelection) -> ExportSelectionDto {
    ExportSelectionDto {
        journal_entry_ids: ids_or_none(&selection.journal_entries),
        therapy_record_ids: ids_or_none(&selection.therapy_records),
        task_ids: ids_or_none(&selection.tasks),
        project_ids: ids_or_none(&selection.projects),
        goal_ids: ids_or_none(&selection.goals),
        quarter_ids: ids_or_none(&selection.quarters),
        date_range: None,
        export_all: None,
    }
}

fn ids_or_none<T: HasId>(items: &[T]) -> Option<Vec<String>> {
    if items.is_empty() {
        None
    } else {
        Some(items.iter().map(|item| item.id().to_string()).collect())
    }
}

/// Minimal id accessor so the resolved-selection DTO can be built generically.
trait HasId {
    fn id(&self) -> &str;
}

impl HasId for JournalEntry {
    fn id(&self) -> &str {
        &self.id
    }
}
impl HasId for TherapyRecord {
    fn id(&self) -> &str {
        &self.id
    }
}
impl HasId for Task {
    fn id(&self) -> &str {
        &self.id
    }
}
impl HasId for Project {
    fn id(&self) -> &str {
        &self.id
    }
}
impl HasId for Goal {
    fn id(&self) -> &str {
        &self.id
    }
}
impl HasId for Quarter {
    fn id(&self) -> &str {
        &self.id
    }
}

/// An ordered (entity type, ID, content hash) triple used for fingerprinting.
struct OrderedItem {
    entity_type: &'static str,
    id: String,
    content_hash: String,
}

/// Produce the deterministic ordered item list for a resolved selection.
///
/// Order is fixed by entity type (journal, therapy, task, project, goal,
/// quarter), matching generation order; within each group items are already
/// sorted by ID.
fn ordered_items(selection: &ResolvedSelection) -> Vec<OrderedItem> {
    let mut items = Vec::with_capacity(selection.item_count() as usize);
    for entry in &selection.journal_entries {
        items.push(OrderedItem {
            entity_type: "journal_entry",
            id: entry.id.clone(),
            content_hash: journal_content_hash(entry),
        });
    }
    for record in &selection.therapy_records {
        items.push(OrderedItem {
            entity_type: "therapy_record",
            id: record.id.clone(),
            content_hash: therapy_content_hash(record),
        });
    }
    for task in &selection.tasks {
        items.push(OrderedItem {
            entity_type: "task",
            id: task.id.clone(),
            content_hash: task_content_hash(task),
        });
    }
    for project in &selection.projects {
        items.push(OrderedItem {
            entity_type: "project",
            id: project.id.clone(),
            content_hash: project_content_hash(project),
        });
    }
    for goal in &selection.goals {
        items.push(OrderedItem {
            entity_type: "goal",
            id: goal.id.clone(),
            content_hash: goal_content_hash(goal),
        });
    }
    for quarter in &selection.quarters {
        items.push(OrderedItem {
            entity_type: "quarter",
            id: quarter.id.clone(),
            content_hash: quarter_content_hash(quarter),
        });
    }
    items
}

/// SHA-256 over a length-prefixed sequence of fields, hex-encoded.
fn hash_fields(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn journal_content_hash(entry: &JournalEntry) -> String {
    let emotions = serde_json::to_string(&entry.emotions).unwrap_or_default();
    let template_values = serde_json::to_string(&entry.template_values).unwrap_or_default();
    let manifest =
        serde_json::to_string(&crate::export::manifest::foundation_manifest()).unwrap_or_default();
    hash_fields(&[
        "journal_entry",
        "diary-export-projection-v1",
        &entry.title,
        &entry.occurred_on,
        &entry.content_markdown,
        entry.mood_label.as_deref().unwrap_or(""),
        entry.energy_label.as_deref().unwrap_or(""),
        &emotions,
        if entry.export_eligible {
            "true"
        } else {
            "false"
        },
        entry.template_id.as_deref().unwrap_or(""),
        entry.template_version.as_deref().unwrap_or(""),
        &template_values,
        &manifest,
    ])
}

fn therapy_content_hash(record: &TherapyRecord) -> String {
    hash_fields(&[
        "therapy_record",
        &record.title,
        record.occurred_on.as_deref().unwrap_or(""),
        &record.content_markdown,
    ])
}

fn task_content_hash(task: &Task) -> String {
    hash_fields(&[
        "task",
        &task.title,
        &task.status.to_string(),
        task.description_markdown.as_deref().unwrap_or(""),
    ])
}

fn project_content_hash(project: &Project) -> String {
    hash_fields(&[
        "project",
        &project.title,
        &project.status.to_string(),
        project.description_markdown.as_deref().unwrap_or(""),
    ])
}

fn goal_content_hash(goal: &Goal) -> String {
    hash_fields(&[
        "goal",
        &goal.title,
        &goal.status.to_string(),
        goal.description_markdown.as_deref().unwrap_or(""),
    ])
}

fn quarter_content_hash(quarter: &Quarter) -> String {
    hash_fields(&[
        "quarter",
        &quarter.code,
        &quarter.start_date,
        &quarter.end_date,
        &quarter.status,
    ])
}

fn resolve_journal_entries(
    conn: &Connection,
    workspace_id: &str,
    selection: &ExportSelectionDto,
) -> Result<Vec<JournalEntry>, AppError> {
    let mut entries = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let export_all = selection.export_all.unwrap_or(false);

    if export_all {
        // Chunked resolution: page through all non-archived entries in chronological order.
        let mut offset = 0;
        loop {
            let page = SqliteJournalEntryRepo.list_page(
                conn,
                workspace_id,
                EXPORT_RESOLUTION_PAGE_SIZE,
                offset,
            )?;
            if page.is_empty() {
                break;
            }
            for entry in page {
                if entry.export_eligible && seen.insert(entry.id.clone()) {
                    entries.push(entry);
                }
            }
            offset += EXPORT_RESOLUTION_PAGE_SIZE;
        }
    } else if let Some(range) = &selection.date_range {
        time::parse_date(&range.from)?;
        time::parse_date(&range.to)?;
        // Chunked resolution: page through non-archived entries in the date range.
        let mut offset = 0;
        loop {
            let page = SqliteJournalEntryRepo.list_range_page(
                conn,
                workspace_id,
                &range.from,
                &range.to,
                EXPORT_RESOLUTION_PAGE_SIZE,
                offset,
            )?;
            if page.is_empty() {
                break;
            }
            for entry in page {
                if entry.export_eligible && seen.insert(entry.id.clone()) {
                    entries.push(entry);
                }
            }
            offset += EXPORT_RESOLUTION_PAGE_SIZE;
        }
    }

    if let Some(ids) = &selection.journal_entry_ids {
        for id in ids {
            if !seen.contains(id) {
                let entry = SqliteJournalEntryRepo.get(conn, id)?;
                if entry.workspace_id != workspace_id || entry.archived_at.is_some() {
                    return Err(AppError::not_found(
                        "One or more selected journal entries are unavailable.",
                    ));
                }
                if !entry.export_eligible {
                    return Err(AppError::validation(
                        "Private journal entries cannot be included in an export.",
                    ));
                }
                entries.push(entry);
                seen.insert(id.clone());
            }
        }
    }

    Ok(entries)
}

fn resolve_therapy_records(
    conn: &Connection,
    workspace_id: &str,
    selection: &ExportSelectionDto,
) -> Result<Vec<TherapyRecord>, AppError> {
    let mut records = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let export_all = selection.export_all.unwrap_or(false);

    if export_all {
        // Chunked resolution: page through all non-archived records in chronological order.
        let mut offset = 0;
        loop {
            let page = SqliteTherapyRecordRepo.list_page(
                conn,
                workspace_id,
                EXPORT_RESOLUTION_PAGE_SIZE,
                offset,
            )?;
            if page.is_empty() {
                break;
            }
            for record in page {
                if seen.insert(record.id.clone()) {
                    records.push(record);
                }
            }
            offset += EXPORT_RESOLUTION_PAGE_SIZE;
        }
    } else if let Some(range) = &selection.date_range {
        time::parse_date(&range.from)?;
        time::parse_date(&range.to)?;
        // Chunked resolution: page through non-archived records in the date range.
        // Records with NULL occurred_on are excluded by the repo method.
        let mut offset = 0;
        loop {
            let page = SqliteTherapyRecordRepo.list_range_page(
                conn,
                workspace_id,
                &range.from,
                &range.to,
                EXPORT_RESOLUTION_PAGE_SIZE,
                offset,
            )?;
            if page.is_empty() {
                break;
            }
            for record in page {
                if seen.insert(record.id.clone()) {
                    records.push(record);
                }
            }
            offset += EXPORT_RESOLUTION_PAGE_SIZE;
        }
    }

    if let Some(ids) = &selection.therapy_record_ids {
        for id in ids {
            if !seen.contains(id) {
                records.push(SqliteTherapyRecordRepo.get(conn, id)?);
                seen.insert(id.clone());
            }
        }
    }

    Ok(records)
}

fn resolve_tasks(conn: &Connection, selection: &ExportSelectionDto) -> Result<Vec<Task>, AppError> {
    let mut tasks = Vec::new();
    if let Some(ids) = &selection.task_ids {
        for id in ids {
            tasks.push(SqliteTaskRepo.get(conn, id)?);
        }
    }
    Ok(tasks)
}

fn resolve_projects(
    conn: &Connection,
    selection: &ExportSelectionDto,
) -> Result<Vec<Project>, AppError> {
    let mut projects = Vec::new();
    if let Some(ids) = &selection.project_ids {
        for id in ids {
            projects.push(SqliteProjectRepo.get(conn, id)?);
        }
    }
    Ok(projects)
}

fn resolve_goals(conn: &Connection, selection: &ExportSelectionDto) -> Result<Vec<Goal>, AppError> {
    let mut goals = Vec::new();
    if let Some(ids) = &selection.goal_ids {
        for id in ids {
            goals.push(SqliteGoalRepo.get(conn, id)?);
        }
    }
    Ok(goals)
}

fn resolve_quarters(
    conn: &Connection,
    selection: &ExportSelectionDto,
) -> Result<Vec<Quarter>, AppError> {
    let mut quarters = Vec::new();
    if let Some(ids) = &selection.quarter_ids {
        for id in ids {
            quarters.push(SqliteQuarterRepo.get(conn, id)?);
        }
    }
    Ok(quarters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::dto::ExportSelectionDto;
    use crate::domain::types::{
        ExportScopeType, JournalSource, TherapyPrivacyScope, TherapyRecordType,
    };
    use crate::repos::journal_entry_repo::{JournalEntryRepo, SqliteJournalEntryRepo};
    use crate::repos::therapy_collection_repo::{
        SqliteTherapyCollectionRepo, TherapyCollectionRepo,
    };
    use crate::repos::therapy_record_repo::{SqliteTherapyRecordRepo, TherapyRecordRepo};
    use crate::test_utils::test_conn;

    fn empty_selection() -> ExportSelectionDto {
        ExportSelectionDto {
            journal_entry_ids: None,
            therapy_record_ids: None,
            task_ids: None,
            project_ids: None,
            goal_ids: None,
            quarter_ids: None,
            date_range: None,
            export_all: None,
        }
    }

    fn create_entry(conn: &Connection, title: &str, content: &str) -> JournalEntry {
        create_entry_on(conn, "2026-09-04", title, content)
    }

    fn create_entry_on(
        conn: &Connection,
        occurred_on: &str,
        title: &str,
        content: &str,
    ) -> JournalEntry {
        SqliteJournalEntryRepo
            .create(
                conn,
                "w1",
                occurred_on,
                title,
                content,
                &None,
                &None,
                &std::collections::BTreeMap::new(),
                &None,
                &None,
                &[],
                true,
                JournalSource::User,
            )
            .expect("create entry")
    }

    fn create_record(conn: &Connection, title: &str, content: &str) -> TherapyRecord {
        let collection = SqliteTherapyCollectionRepo
            .create(
                conn,
                "w1",
                "Private",
                TherapyPrivacyScope::TherapyPrivate,
                &None,
            )
            .expect("collection");
        SqliteTherapyRecordRepo
            .create(
                conn,
                &collection.id,
                TherapyRecordType::Note,
                title,
                content,
                &None,
            )
            .expect("record")
    }

    #[test]
    fn empty_journal_selection_returns_validation_error() {
        let conn = test_conn();
        let result = resolve_selection(&conn, "w1", ExportScopeType::Journal, &empty_selection());
        assert!(result.is_err());
        let message = result.unwrap_err().message;
        assert!(message.to_lowercase().contains("journal entry"));
    }

    #[test]
    fn journal_selection_resolves_explicit_ids() {
        let conn = test_conn();
        let entry = create_entry(&conn, "A quiet day", "Some reflection.");

        let selection = ExportSelectionDto {
            journal_entry_ids: Some(vec![entry.id.clone()]),
            ..empty_selection()
        };
        let resolved =
            resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection).expect("resolve");
        assert_eq!(resolved.journal_entries.len(), 1);
        assert_eq!(resolved.journal_entries[0].id, entry.id);
    }

    #[test]
    fn private_entries_are_rejected_explicitly_and_excluded_from_bulk_resolution() {
        let conn = test_conn();
        let eligible = create_entry(&conn, "Eligible", "visible synthetic body");
        let private = create_entry(&conn, "Private", "private synthetic body");
        conn.execute(
            "UPDATE journal_entries SET export_eligible = 0 WHERE id = ?1",
            [&private.id],
        )
        .expect("make synthetic entry private");

        for scope_type in [ExportScopeType::Journal, ExportScopeType::Custom] {
            let selection = ExportSelectionDto {
                journal_entry_ids: Some(vec![private.id.clone()]),
                ..empty_selection()
            };
            let error = resolve_selection(&conn, "w1", scope_type, &selection)
                .expect_err("explicit private selection is rejected");
            assert!(error.message.contains("Private journal entries"));
        }

        for selection in [
            ExportSelectionDto {
                export_all: Some(true),
                ..empty_selection()
            },
            ExportSelectionDto {
                date_range: Some(crate::commands::dto::DateRangeDto {
                    from: "2026-01-01".to_string(),
                    to: "2026-12-31".to_string(),
                }),
                ..empty_selection()
            },
        ] {
            let resolved = resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection)
                .expect("bulk resolution excludes private entries");
            assert_eq!(resolved.journal_entries.len(), 1);
            assert_eq!(resolved.journal_entries[0].id, eligible.id);
            assert!(!to_preview_dto(&resolved)
                .iter()
                .any(|item| item.id == private.id));
        }
    }

    #[test]
    fn journal_fingerprint_covers_every_structured_output_field() {
        let conn = test_conn();
        let values = std::collections::BTreeMap::new();
        let entry = SqliteJournalEntryRepo
            .create(
                &conn,
                "w1",
                "2026-09-04",
                "Synthetic",
                "Authored source",
                &Some("foundation-journal".to_owned()),
                &Some("1.0.1".to_owned()),
                &values,
                &Some("good".to_owned()),
                &Some("7".to_owned()),
                &["calm".to_owned()],
                true,
                JournalSource::User,
            )
            .expect("create synthetic template entry");
        conn.execute(
            "UPDATE journal_entries
             SET template_values_json = '{\"future.note\":\"Synthetic structured response\"}'
             WHERE id = ?1",
            [&entry.id],
        )
        .expect("seed synthetic future template value");
        let entry = SqliteJournalEntryRepo
            .get(&conn, &entry.id)
            .expect("reload synthetic template entry");
        let original = journal_content_hash(&entry);

        let selection = ExportSelectionDto {
            journal_entry_ids: Some(vec![entry.id.clone()]),
            ..empty_selection()
        };
        let resolved = resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection)
            .expect("resolve synthetic preview selection");
        let snapshot = to_snapshot_dto(&resolved);
        conn.execute(
            "UPDATE journal_entries SET mood_label = 'low' WHERE id = ?1",
            [&entry.id],
        )
        .expect("mutate structured mood after preview");
        let stale = revalidate_snapshot(&conn, "w1", ExportScopeType::Journal, &snapshot)
            .expect_err("structured metadata mutation invalidates preview");
        assert!(stale.message.contains("stale"));
        conn.execute(
            "UPDATE journal_entries SET mood_label = 'good' WHERE id = ?1",
            [&entry.id],
        )
        .expect("restore baseline mood");

        for (sql, reset_sql, value) in [
            (
                "UPDATE journal_entries SET mood_label = 'low' WHERE id = ?1",
                "UPDATE journal_entries SET mood_label = 'good' WHERE id = ?1",
                "mood",
            ),
            (
                "UPDATE journal_entries SET energy_label = '8' WHERE id = ?1",
                "UPDATE journal_entries SET energy_label = '7' WHERE id = ?1",
                "energy",
            ),
            (
                "UPDATE journal_entries SET emotions_json = '[\"joy\"]' WHERE id = ?1",
                "UPDATE journal_entries SET emotions_json = '[\"calm\"]' WHERE id = ?1",
                "emotions",
            ),
            (
                "UPDATE journal_entries SET template_values_json = '{\"future.note\":\"Changed response\"}' WHERE id = ?1",
                "UPDATE journal_entries SET template_values_json = '{\"future.note\":\"Synthetic structured response\"}' WHERE id = ?1",
                "template values",
            ),
            (
                "UPDATE journal_entries SET template_id = 'changed-template' WHERE id = ?1",
                "UPDATE journal_entries SET template_id = 'foundation-journal' WHERE id = ?1",
                "template ID",
            ),
            (
                "UPDATE journal_entries SET template_version = '1.0.2' WHERE id = ?1",
                "UPDATE journal_entries SET template_version = '1.0.1' WHERE id = ?1",
                "template version",
            ),
            (
                "UPDATE journal_entries SET export_eligible = 0 WHERE id = ?1",
                "UPDATE journal_entries SET export_eligible = 1 WHERE id = ?1",
                "eligibility",
            ),
        ] {
            conn.execute(sql, [&entry.id]).expect("mutate synthetic metadata");
            let changed = SqliteJournalEntryRepo
                .get(&conn, &entry.id)
                .expect("reload synthetic entry");
            assert_ne!(
                journal_content_hash(&changed),
                original,
                "journal hash must cover {value}"
            );
            conn.execute(reset_sql, [&entry.id])
                .expect("restore baseline metadata");
        }
    }

    #[test]
    fn deterministic_order_is_independent_of_input_order() {
        let conn = test_conn();
        let a = create_entry(&conn, "Alpha", "content a");
        let b = create_entry(&conn, "Beta", "content b");
        let c = create_entry(&conn, "Gamma", "content c");

        // Input order is deliberately shuffled.
        let shuffled = ExportSelectionDto {
            journal_entry_ids: Some(vec![c.id.clone(), a.id.clone(), b.id.clone()]),
            ..empty_selection()
        };
        let sorted = ExportSelectionDto {
            journal_entry_ids: Some(vec![a.id.clone(), b.id.clone(), c.id.clone()]),
            ..empty_selection()
        };

        let resolved_shuffled =
            resolve_selection(&conn, "w1", ExportScopeType::Journal, &shuffled).expect("resolve");
        let resolved_sorted =
            resolve_selection(&conn, "w1", ExportScopeType::Journal, &sorted).expect("resolve");

        let ids_shuffled: Vec<&str> = resolved_shuffled
            .journal_entries
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        let ids_sorted: Vec<&str> = resolved_sorted
            .journal_entries
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        assert_eq!(
            ids_shuffled, ids_sorted,
            "order must be stable by date/title/id"
        );

        // The fingerprint is identical regardless of input order.
        assert_eq!(
            selection_fingerprint(&resolved_shuffled),
            selection_fingerprint(&resolved_sorted)
        );
    }

    #[test]
    fn journal_entries_are_ordered_chronologically_for_preview_and_export() {
        let conn = test_conn();
        let later = create_entry_on(&conn, "2026-09-12", "Later", "later");
        let earlier = create_entry_on(&conn, "2026-09-02", "Earlier", "earlier");
        let middle = create_entry_on(&conn, "2026-09-07", "Middle", "middle");
        let selection = ExportSelectionDto {
            journal_entry_ids: Some(vec![later.id, middle.id, earlier.id]),
            ..empty_selection()
        };

        let resolved =
            resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection).expect("resolve");
        let dates: Vec<&str> = resolved
            .journal_entries
            .iter()
            .map(|entry| entry.occurred_on.as_str())
            .collect();
        assert_eq!(dates, vec!["2026-09-02", "2026-09-07", "2026-09-12"]);
    }

    #[test]
    fn fingerprint_changes_when_content_changes() {
        let conn = test_conn();
        let entry = create_entry(&conn, "A quiet day", "original content");

        let selection = ExportSelectionDto {
            journal_entry_ids: Some(vec![entry.id.clone()]),
            ..empty_selection()
        };
        let before =
            resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection).expect("resolve");
        let fp_before = selection_fingerprint(&before);

        // Mutate the entry content.
        SqliteJournalEntryRepo
            .update(
                &conn,
                &entry.id,
                "2026-09-04",
                "A quiet day",
                "changed content",
                &None,
                &None,
                None,
                &None,
                &None,
                None,
                None,
            )
            .expect("update entry");

        let after =
            resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection).expect("resolve");
        let fp_after = selection_fingerprint(&after);

        assert_ne!(
            fp_before, fp_after,
            "fingerprint must reflect content change"
        );
    }

    #[test]
    fn content_hash_is_stable_and_does_not_expose_content() {
        let conn = test_conn();
        let entry = create_entry(&conn, "A quiet day", "secret reflection text");

        let selection = ExportSelectionDto {
            journal_entry_ids: Some(vec![entry.id.clone()]),
            ..empty_selection()
        };
        let resolved =
            resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection).expect("resolve");
        let items = to_preview_dto(&resolved);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].content_hash.len(), 64, "SHA-256 hex length");
        assert!(!items[0].content_hash.contains("secret"));
        assert!(!items[0].content_hash.contains("reflection"));
    }

    #[test]
    fn snapshot_round_trips_and_revalidates() {
        let conn = test_conn();
        let entry = create_entry(&conn, "A quiet day", "content");

        let selection = ExportSelectionDto {
            journal_entry_ids: Some(vec![entry.id.clone()]),
            ..empty_selection()
        };
        let resolved =
            resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection).expect("resolve");
        let snapshot = to_snapshot_dto(&resolved);

        // Revalidation against unchanged content succeeds.
        let revalidated = revalidate_snapshot(&conn, "w1", ExportScopeType::Journal, &snapshot)
            .expect("revalidate");
        assert_eq!(revalidated.journal_entries.len(), 1);

        // Mutate content; revalidation must now fail as stale.
        SqliteJournalEntryRepo
            .update(
                &conn,
                &entry.id,
                "2026-09-04",
                "A quiet day",
                "changed",
                &None,
                &None,
                None,
                &None,
                &None,
                None,
                None,
            )
            .expect("update entry");
        let stale = revalidate_snapshot(&conn, "w1", ExportScopeType::Journal, &snapshot);
        assert!(stale.is_err(), "stale snapshot must be rejected");
    }

    #[test]
    fn therapy_ids_are_rejected_in_generic_scope() {
        let conn = test_conn();
        let record = create_record(&conn, "Therapy note", "private content");

        let selection = ExportSelectionDto {
            therapy_record_ids: Some(vec![record.id.clone()]),
            ..empty_selection()
        };
        let result = resolve_selection(&conn, "w1", ExportScopeType::Task, &selection);
        assert!(
            result.is_err(),
            "therapy IDs must be rejected in a generic scope"
        );
    }

    #[test]
    fn journal_ids_are_rejected_in_therapy_scope() {
        let conn = test_conn();
        let entry = create_entry(&conn, "Journal", "content");

        let selection = ExportSelectionDto {
            journal_entry_ids: Some(vec![entry.id.clone()]),
            ..empty_selection()
        };
        let result = resolve_selection(&conn, "w1", ExportScopeType::Therapy, &selection);
        assert!(
            result.is_err(),
            "journal IDs must be rejected in a therapy scope"
        );
    }

    #[test]
    fn therapy_scope_requires_explicit_ids_not_range_only() {
        let conn = test_conn();
        // A date range with no resolved IDs must not satisfy the therapy scope.
        let selection = ExportSelectionDto {
            date_range: Some(crate::commands::dto::DateRangeDto {
                from: "2026-01-01".to_string(),
                to: "2026-12-31".to_string(),
            }),
            ..empty_selection()
        };
        let result = resolve_selection(&conn, "w1", ExportScopeType::Therapy, &selection);
        assert!(
            result.is_err(),
            "range-only therapy selection must be rejected"
        );
    }

    #[test]
    fn malformed_snapshot_entity_type_is_rejected() {
        let snapshot = ResolvedSelectionSnapshotDto {
            items: vec![ResolvedSelectionItemDto {
                entity_type: "unknown_entity".to_string(),
                id: "x".to_string(),
                content_hash: "abc".to_string(),
            }],
        };
        let result = snapshot_to_selection_dto(&snapshot);
        assert!(result.is_err(), "unknown entity type must be rejected");
    }

    #[test]
    fn export_all_journal_resolves_all_non_archived_entries() {
        let conn = test_conn();
        let _e1 = create_entry_on(&conn, "2026-01-01", "First", "content 1");
        let _e2 = create_entry_on(&conn, "2026-01-05", "Second", "content 2");
        let _e3 = create_entry_on(&conn, "2026-02-10", "Third", "content 3");
        // Archived entry should not be included.
        let archived = create_entry_on(&conn, "2026-03-01", "Archived", "content 4");
        SqliteJournalEntryRepo
            .archive(&conn, &archived.id)
            .expect("archive entry");

        let selection = ExportSelectionDto {
            export_all: Some(true),
            ..empty_selection()
        };
        let resolved =
            resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection).expect("resolve");
        assert_eq!(resolved.journal_entries.len(), 3);
        let titles: Vec<&str> = resolved
            .journal_entries
            .iter()
            .map(|e| e.title.as_str())
            .collect();
        assert_eq!(titles, vec!["First", "Second", "Third"]);
    }

    #[test]
    fn export_all_therapy_resolves_all_non_archived_records() {
        let conn = test_conn();
        let collection = SqliteTherapyCollectionRepo
            .create(
                &conn,
                "w1",
                "Private",
                TherapyPrivacyScope::TherapyPrivate,
                &None,
            )
            .expect("collection");
        let _r1 = SqliteTherapyRecordRepo
            .create(
                &conn,
                &collection.id,
                TherapyRecordType::Note,
                "First",
                "content 1",
                &Some("2026-01-01".to_string()),
            )
            .expect("record 1");
        let _r2 = SqliteTherapyRecordRepo
            .create(
                &conn,
                &collection.id,
                TherapyRecordType::Note,
                "Second",
                "content 2",
                &Some("2026-01-05".to_string()),
            )
            .expect("record 2");
        let _r3 = SqliteTherapyRecordRepo
            .create(
                &conn,
                &collection.id,
                TherapyRecordType::Note,
                "Third",
                "content 3",
                &Some("2026-02-10".to_string()),
            )
            .expect("record 3");
        // Archived record should not be included.
        let archived = SqliteTherapyRecordRepo
            .create(
                &conn,
                &collection.id,
                TherapyRecordType::Note,
                "Archived",
                "content 4",
                &Some("2026-03-01".to_string()),
            )
            .expect("archived record");
        SqliteTherapyRecordRepo
            .archive(&conn, &archived.id)
            .expect("archive record");

        let selection = ExportSelectionDto {
            export_all: Some(true),
            ..empty_selection()
        };
        let resolved =
            resolve_selection(&conn, "w1", ExportScopeType::Therapy, &selection).expect("resolve");
        assert_eq!(resolved.therapy_records.len(), 3);
        let mut titles: Vec<&str> = resolved
            .therapy_records
            .iter()
            .map(|r| r.title.as_str())
            .collect();
        titles.sort();
        assert_eq!(titles, vec!["First", "Second", "Third"]);
    }

    #[test]
    fn export_all_rejects_combination_with_date_range() {
        let conn = test_conn();
        let selection = ExportSelectionDto {
            export_all: Some(true),
            date_range: Some(crate::commands::dto::DateRangeDto {
                from: "2026-01-01".to_string(),
                to: "2026-12-31".to_string(),
            }),
            ..empty_selection()
        };
        let result = resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection);
        assert!(
            result.is_err(),
            "export_all with date_range must be rejected"
        );
        let msg = result.unwrap_err().message;
        assert!(msg.contains("export_all cannot be combined with date_range"));
    }

    #[test]
    fn export_all_rejects_combination_with_explicit_ids() {
        let conn = test_conn();
        let entry = create_entry(&conn, "Entry", "content");
        let selection = ExportSelectionDto {
            export_all: Some(true),
            journal_entry_ids: Some(vec![entry.id]),
            ..empty_selection()
        };
        let result = resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection);
        assert!(
            result.is_err(),
            "export_all with explicit IDs must be rejected"
        );
        let msg = result.unwrap_err().message;
        assert!(msg.contains("export_all cannot be combined"));
    }

    #[test]
    fn export_all_invalid_for_generic_scope() {
        let conn = test_conn();
        let selection = ExportSelectionDto {
            export_all: Some(true),
            ..empty_selection()
        };
        let result = resolve_selection(&conn, "w1", ExportScopeType::Task, &selection);
        assert!(
            result.is_err(),
            "export_all must be rejected for generic scopes"
        );
        let msg = result.unwrap_err().message;
        assert!(msg.contains("export_all is only supported"));
    }

    #[test]
    fn wide_date_range_resolves_only_existing_entries_no_filler() {
        let conn = test_conn();
        let _e1 = create_entry_on(&conn, "2026-01-01", "First", "content 1");
        let _e2 = create_entry_on(&conn, "2026-01-05", "Second", "content 2");
        let _e3 = create_entry_on(&conn, "2026-02-10", "Third", "content 3");

        let selection = ExportSelectionDto {
            date_range: Some(crate::commands::dto::DateRangeDto {
                from: "2026-01-01".to_string(),
                to: "2026-12-31".to_string(),
            }),
            ..empty_selection()
        };
        let resolved =
            resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection).expect("resolve");
        assert_eq!(resolved.journal_entries.len(), 3);
        let titles: Vec<&str> = resolved
            .journal_entries
            .iter()
            .map(|e| e.title.as_str())
            .collect();
        assert_eq!(titles, vec!["First", "Second", "Third"]);
    }

    #[test]
    fn date_range_resolves_entries_in_chronological_order_excludes_out_of_range() {
        let conn = test_conn();
        let _e1 = create_entry_on(&conn, "2025-12-31", "Before Range", "content");
        let _e2 = create_entry_on(&conn, "2026-01-01", "First In Range", "content");
        let _e3 = create_entry_on(&conn, "2026-06-15", "Middle In Range", "content");
        let _e4 = create_entry_on(&conn, "2026-12-31", "Last In Range", "content");
        let _e5 = create_entry_on(&conn, "2027-01-01", "After Range", "content");

        let selection = ExportSelectionDto {
            date_range: Some(crate::commands::dto::DateRangeDto {
                from: "2026-01-01".to_string(),
                to: "2026-12-31".to_string(),
            }),
            ..empty_selection()
        };
        let resolved =
            resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection).expect("resolve");
        assert_eq!(resolved.journal_entries.len(), 3);
        let titles: Vec<&str> = resolved
            .journal_entries
            .iter()
            .map(|e| e.title.as_str())
            .collect();
        assert_eq!(
            titles,
            vec!["First In Range", "Middle In Range", "Last In Range"]
        );
        let dates: Vec<&str> = resolved
            .journal_entries
            .iter()
            .map(|e| e.occurred_on.as_str())
            .collect();
        assert_eq!(dates, vec!["2026-01-01", "2026-06-15", "2026-12-31"]);
    }

    #[test]
    fn therapy_date_range_resolves_records_in_chronological_order_excludes_out_of_range() {
        let conn = test_conn();
        let collection = SqliteTherapyCollectionRepo
            .create(
                &conn,
                "w1",
                "Private",
                TherapyPrivacyScope::TherapyPrivate,
                &None,
            )
            .expect("collection");
        let _r1 = SqliteTherapyRecordRepo
            .create(
                &conn,
                &collection.id,
                TherapyRecordType::Note,
                "Before Range",
                "content",
                &Some("2025-12-31".to_string()),
            )
            .expect("record 1");
        let _r2 = SqliteTherapyRecordRepo
            .create(
                &conn,
                &collection.id,
                TherapyRecordType::Note,
                "First In Range",
                "content",
                &Some("2026-01-01".to_string()),
            )
            .expect("record 2");
        let _r3 = SqliteTherapyRecordRepo
            .create(
                &conn,
                &collection.id,
                TherapyRecordType::Note,
                "Middle In Range",
                "content",
                &Some("2026-06-15".to_string()),
            )
            .expect("record 3");
        let _r4 = SqliteTherapyRecordRepo
            .create(
                &conn,
                &collection.id,
                TherapyRecordType::Note,
                "Last In Range",
                "content",
                &Some("2026-12-31".to_string()),
            )
            .expect("record 4");
        let _r5 = SqliteTherapyRecordRepo
            .create(
                &conn,
                &collection.id,
                TherapyRecordType::Note,
                "After Range",
                "content",
                &Some("2027-01-01".to_string()),
            )
            .expect("record 5");
        // Record with NULL occurred_on should be excluded.
        let _r6 = SqliteTherapyRecordRepo
            .create(
                &conn,
                &collection.id,
                TherapyRecordType::Note,
                "No Date",
                "content",
                &None,
            )
            .expect("record 6");

        let selection = ExportSelectionDto {
            date_range: Some(crate::commands::dto::DateRangeDto {
                from: "2026-01-01".to_string(),
                to: "2026-12-31".to_string(),
            }),
            ..empty_selection()
        };
        let resolved =
            resolve_selection(&conn, "w1", ExportScopeType::Therapy, &selection).expect("resolve");
        assert_eq!(resolved.therapy_records.len(), 3);
        // Therapy records are sorted by ID for determinism (not by date).
        // Verify the correct records are included (by title set) and dates are correct.
        let titles: std::collections::HashSet<&str> = resolved
            .therapy_records
            .iter()
            .map(|r| r.title.as_str())
            .collect();
        assert_eq!(
            titles,
            std::collections::HashSet::from(["First In Range", "Middle In Range", "Last In Range"])
        );
        let dates: std::collections::HashSet<Option<&str>> = resolved
            .therapy_records
            .iter()
            .map(|r| r.occurred_on.as_deref())
            .collect();
        assert_eq!(
            dates,
            std::collections::HashSet::from([
                Some("2026-01-01"),
                Some("2026-06-15"),
                Some("2026-12-31")
            ])
        );
    }

    #[test]
    fn preview_truncation_caps_items_at_200() {
        let conn = test_conn();
        // Create 250 entries.
        for i in 0..250 {
            let date = format!("2026-{:02}-{:02}", (i / 28) + 1, (i % 28) + 1);
            create_entry_on(&conn, &date, &format!("Entry {}", i), "content");
        }

        let selection = ExportSelectionDto {
            export_all: Some(true),
            ..empty_selection()
        };
        let resolved =
            resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection).expect("resolve");
        let all_items = to_preview_dto(&resolved);
        assert_eq!(all_items.len(), 250);

        // Simulate preview truncation logic (as done in preview_export_selection command).
        let total_count = all_items.len() as i64;
        let truncated = total_count > 200;
        let items = all_items.into_iter().take(200).collect::<Vec<_>>();

        assert_eq!(total_count, 250);
        assert!(truncated);
        assert_eq!(items.len(), 200);
        // Items should be in deterministic chronological order.
        assert_eq!(items[0].title, "Entry 0");
        assert_eq!(items[199].title, "Entry 199");
    }
}
