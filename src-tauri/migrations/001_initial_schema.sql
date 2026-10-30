-- Canonical empty-database schema, equivalent to archived migrations 001-042.
-- Data backfills from historical upgrades are intentionally not included.

-- table: availability_profiles
CREATE TABLE availability_profiles (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    weekday INTEGER NOT NULL CHECK(weekday BETWEEN 0 AND 6),
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    preferred_block_length_minutes INTEGER,
    energy_expectation TEXT,
    active_from TEXT,
    active_until TEXT,
    note_markdown TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- table: block_recurrence_lifecycle_intervals
CREATE TABLE block_recurrence_lifecycle_intervals (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    recurrence_id TEXT NOT NULL REFERENCES block_recurrences(id) ON DELETE CASCADE,
    active_from_local_date TEXT NOT NULL,
    active_until_local_date TEXT,
    source TEXT NOT NULL CHECK (source IN ('seed', 'explicit')),
    created_at TEXT NOT NULL,
    CHECK (active_until_local_date IS NULL OR active_until_local_date >= active_from_local_date)
);

-- table: block_recurrence_version_weekdays
CREATE TABLE block_recurrence_version_weekdays (
    version_id TEXT NOT NULL REFERENCES block_recurrence_versions(id) ON DELETE CASCADE,
    weekday INTEGER NOT NULL CHECK (weekday BETWEEN 1 AND 7),
    PRIMARY KEY (version_id, weekday)
);

-- table: block_recurrence_versions
CREATE TABLE block_recurrence_versions (
    id TEXT PRIMARY KEY,
    recurrence_id TEXT NOT NULL REFERENCES block_recurrences(id) ON DELETE CASCADE,
    local_start_time TEXT NOT NULL CHECK (
        length(local_start_time) = 5 AND
        local_start_time GLOB '[0-2][0-9]:[0-5][0-9]' AND
        substr(local_start_time, 1, 2) < '24'
    ),
    estimated_duration_seconds INTEGER NOT NULL CHECK (estimated_duration_seconds > 0),
    task_id TEXT REFERENCES tasks(id) ON DELETE RESTRICT,
    type TEXT NOT NULL CHECK (type IN ('planning', 'focus', 'review', 'journaling', 'rest', 'personal')),
    label TEXT,
    flexibility TEXT NOT NULL CHECK (flexibility IN ('fixed', 'flexible', 'approximate')),
    note_markdown TEXT,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    effective_from TEXT NOT NULL,
    effective_until TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(recurrence_id, id),
    CHECK (effective_until IS NULL OR effective_until >= effective_from)
);

-- table: block_recurrences
CREATE TABLE block_recurrences (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('active', 'inactive', 'deleted')),
    current_version_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(workspace_id, id)
);

-- table: blocks
CREATE TABLE blocks (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    daily_plan_id TEXT REFERENCES daily_plans(id) ON DELETE SET NULL,
    date TEXT NOT NULL,
    type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'planned',
    label TEXT,
    planned_start_at TEXT,
    planned_end_at TEXT,
    time_window_label TEXT,
    actual_start_at TEXT,
    actual_end_at TEXT,
    flexibility TEXT NOT NULL DEFAULT 'flexible',
    note_markdown TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
, estimated_duration_seconds INTEGER
    CHECK (estimated_duration_seconds IS NULL OR estimated_duration_seconds > 0), recurrence_id TEXT REFERENCES block_recurrences(id) ON DELETE CASCADE, recurrence_version_id TEXT REFERENCES block_recurrence_versions(id), local_date TEXT, local_start_time TEXT, timezone_id TEXT, resolved_start_at TEXT, lifecycle_state TEXT NOT NULL DEFAULT 'active'
    CHECK (lifecycle_state IN ('active', 'inactive', 'deleted')), task_id TEXT REFERENCES tasks(id) ON DELETE RESTRICT, reactivated_from_block_id TEXT
    REFERENCES blocks(id) ON DELETE RESTRICT);

-- table: daily_plan_tasks
CREATE TABLE daily_plan_tasks (
    id TEXT PRIMARY KEY,
    daily_plan_id TEXT NOT NULL REFERENCES daily_plans(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    order_index INTEGER NOT NULL,
    selection_reason TEXT NOT NULL DEFAULT 'manual',
    planned_status TEXT NOT NULL DEFAULT 'selected',
    carryover_from_daily_plan_id TEXT REFERENCES daily_plans(id) ON DELETE SET NULL,
    decision_note_markdown TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(daily_plan_id, task_id)
);

-- table: daily_plans
CREATE TABLE daily_plans (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    date TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'not_started',
    intention_markdown TEXT,
    capacity_note_markdown TEXT,
    review_markdown TEXT,
    planning_block_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(workspace_id, date)
);

-- table: export_artifacts
CREATE TABLE export_artifacts (
    id TEXT PRIMARY KEY,
    export_job_id TEXT NOT NULL REFERENCES export_jobs(id) ON DELETE CASCADE,
    format TEXT NOT NULL,
    path TEXT NOT NULL,
    checksum TEXT,
    size_bytes INTEGER,
    created_at TEXT NOT NULL,
    deleted_at TEXT
);

-- table: export_jobs
CREATE TABLE export_jobs (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    type TEXT NOT NULL,
    scope_type TEXT NOT NULL,
    title TEXT NOT NULL,
    template_name TEXT,
    selection_json TEXT NOT NULL,
    options_json TEXT,
    status TEXT NOT NULL DEFAULT 'queued',
    output_path TEXT,
    generated_at TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
, selection_fingerprint TEXT, diagnostics_json TEXT);

-- table: fixed_commitments
CREATE TABLE fixed_commitments (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description_markdown TEXT,
    type TEXT NOT NULL,
    start_at TEXT,
    end_at TEXT,
    duration_minutes INTEGER,
    recurrence_rule TEXT,
    movable INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1,
    privacy_scope TEXT NOT NULL DEFAULT 'private',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- table: goals
CREATE TABLE goals (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    year_theme_id TEXT REFERENCES year_themes(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    description_markdown TEXT,
    why_it_matters_markdown TEXT,
    success_definition_markdown TEXT,
    status TEXT NOT NULL DEFAULT 'inbox',
    priority TEXT NOT NULL DEFAULT 'none',
    target_date TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);

-- table: import_batches
CREATE TABLE import_batches (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    source_root TEXT NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    summary_json TEXT,
    created_at TEXT NOT NULL
);

-- table: imported_records
CREATE TABLE imported_records (
    id TEXT PRIMARY KEY,
    import_batch_id TEXT NOT NULL REFERENCES import_batches(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    source_path TEXT NOT NULL,
    source_date TEXT,
    confidence TEXT NOT NULL DEFAULT 'review',
    warnings_json TEXT,
    imported_at TEXT NOT NULL
, source_key TEXT, content_hash TEXT);

-- table: journal_entries
CREATE TABLE journal_entries (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    occurred_on TEXT NOT NULL,
    title TEXT NOT NULL,
    content_markdown TEXT NOT NULL,
    template_id TEXT,
    mood_label TEXT,
    energy_label TEXT,
    privacy_scope TEXT NOT NULL DEFAULT 'private',
    source TEXT NOT NULL DEFAULT 'user',
    retroactive INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
, emotions_json TEXT NOT NULL DEFAULT '[]', template_version TEXT, template_values_json TEXT NOT NULL DEFAULT '{}', export_eligible INTEGER NOT NULL DEFAULT 0
    CHECK (export_eligible IN (0, 1)));

-- table: journal_entry_legacy_template_values
CREATE TABLE journal_entry_legacy_template_values (
    entry_id TEXT NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
    field_id TEXT NOT NULL,
    value_json TEXT NOT NULL,
    PRIMARY KEY (entry_id, field_id)
);

-- table: journal_task_references
CREATE TABLE journal_task_references (
    id TEXT PRIMARY KEY,
    journal_entry_id TEXT NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    relation TEXT NOT NULL DEFAULT 'mentions',
    created_by TEXT NOT NULL DEFAULT 'user',
    confidence REAL,
    created_at TEXT NOT NULL,
    removed_at TEXT
);

-- table: notification_deliveries
CREATE TABLE "notification_deliveries" (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    delivery_kind TEXT NOT NULL CHECK(delivery_kind IN ('reminder_rule','backup','block')),
    reminder_rule_id TEXT,
    target_entity_type TEXT,
    target_entity_id TEXT,
    scheduled_at TEXT NOT NULL,
    delivered_at TEXT,
    status TEXT NOT NULL DEFAULT 'scheduled',
    channel TEXT NOT NULL,
    safe_message TEXT NOT NULL,
    error_message TEXT,
    created_at TEXT NOT NULL,
    cycle_key TEXT,
    CHECK((delivery_kind = 'reminder_rule' AND reminder_rule_id IS NOT NULL)
       OR (delivery_kind = 'backup' AND reminder_rule_id IS NULL
           AND target_entity_type IS NULL AND target_entity_id IS NULL)
       OR (delivery_kind = 'block' AND reminder_rule_id IS NULL
           AND target_entity_type = 'block' AND target_entity_id IS NOT NULL)),
    FOREIGN KEY(reminder_rule_id, workspace_id)
        REFERENCES reminder_rules(id, workspace_id) ON DELETE CASCADE
);

-- table: notification_delivery_dedupe
CREATE TABLE notification_delivery_dedupe (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    event_key TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (workspace_id, event_key)
);

-- table: projects
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    goal_id TEXT REFERENCES goals(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    description_markdown TEXT,
    why_it_matters_markdown TEXT,
    definition_of_done_markdown TEXT,
    status TEXT NOT NULL DEFAULT 'inbox',
    priority TEXT NOT NULL DEFAULT 'none',
    target_date TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);

-- table: quarter_focus
CREATE TABLE quarter_focus (
    id TEXT PRIMARY KEY,
    quarter_id TEXT NOT NULL REFERENCES quarters(id) ON DELETE CASCADE,
    goal_id TEXT REFERENCES goals(id) ON DELETE SET NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    priority TEXT NOT NULL DEFAULT 'normal',
    intention_markdown TEXT,
    success_signal_markdown TEXT,
    status TEXT NOT NULL DEFAULT 'proposed',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK((goal_id IS NULL) != (project_id IS NULL))
);

-- table: quarters
CREATE TABLE quarters (
    id TEXT PRIMARY KEY,
    year_theme_id TEXT NOT NULL REFERENCES year_themes(id) ON DELETE CASCADE,
    code TEXT NOT NULL CHECK(code IN ('Q1','Q2','Q3','Q4')),
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'upcoming',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(year_theme_id, code)
);

-- table: reminder_rules
CREATE TABLE reminder_rules (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    type TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    recurrence_rule TEXT,
    preferred_time TEXT,
    time_window_start TEXT,
    time_window_end TEXT,
    quiet_hours_start TEXT,
    quiet_hours_end TEXT,
    max_reminders_per_day INTEGER NOT NULL DEFAULT 5,
    snooze_minutes_json TEXT NOT NULL DEFAULT '[10,30,60]',
    channel TEXT NOT NULL DEFAULT 'desktop',
    escalation TEXT NOT NULL DEFAULT 'none',
    privacy_safe_message TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
, deadline_notification_window_days INTEGER NOT NULL DEFAULT 3
    CHECK (deadline_notification_window_days BETWEEN 1 AND 7));

-- table: task_dependencies
CREATE TABLE task_dependencies (
    id TEXT PRIMARY KEY,
    blocked_task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    prerequisite_task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    type TEXT NOT NULL DEFAULT 'blocks',
    note_markdown TEXT,
    created_at TEXT NOT NULL,
    removed_at TEXT,
    UNIQUE(blocked_task_id, prerequisite_task_id)
);

-- table: task_links
CREATE TABLE task_links (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    linked_entity_type TEXT NOT NULL,
    linked_entity_id TEXT NOT NULL,
    relation TEXT NOT NULL DEFAULT 'related',
    created_by TEXT NOT NULL DEFAULT 'user',
    created_at TEXT NOT NULL,
    removed_at TEXT
);

-- table: task_sessions
CREATE TABLE task_sessions (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    block_id TEXT REFERENCES blocks(id) ON DELETE SET NULL,
    daily_plan_task_id TEXT REFERENCES daily_plan_tasks(id) ON DELETE SET NULL,
    started_at TEXT,
    ended_at TEXT,
    status TEXT NOT NULL DEFAULT 'planned',
    outcome_markdown TEXT,
    next_step_markdown TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- table: task_status_history
CREATE TABLE task_status_history (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    from_status TEXT,
    to_status TEXT NOT NULL,
    reason_markdown TEXT,
    source TEXT NOT NULL DEFAULT 'user',
    metadata_json TEXT,
    occurred_at TEXT NOT NULL
);

-- table: tasks
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    goal_id TEXT REFERENCES goals(id) ON DELETE SET NULL,
    parent_task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description_markdown TEXT,
    notes_markdown TEXT,
    status TEXT NOT NULL DEFAULT 'inbox',
    priority TEXT NOT NULL DEFAULT 'none',
    effort_size TEXT,
    effort_points INTEGER GENERATED ALWAYS AS (
        CASE effort_size
            WHEN 'trivial' THEN 1
            WHEN 'small' THEN 2
            WHEN 'medium' THEN 3
            WHEN 'large' THEN 5
            WHEN 'very_large' THEN 8
            ELSE 1
        END
    ) STORED,
    target_date TEXT,
    scheduled_date TEXT,
    completed_at TEXT,
    cancelled_at TEXT,
    closure_mode TEXT,
    closure_reason_markdown TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
, blocking_note_markdown TEXT);

-- table: therapy_collections
CREATE TABLE therapy_collections (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    privacy_scope TEXT NOT NULL DEFAULT 'therapy_private',
    encryption_status TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);

-- table: therapy_records
CREATE TABLE therapy_records (
    id TEXT PRIMARY KEY,
    therapy_collection_id TEXT NOT NULL REFERENCES therapy_collections(id) ON DELETE CASCADE,
    type TEXT NOT NULL,
    title TEXT NOT NULL,
    content_markdown TEXT NOT NULL,
    occurred_on TEXT,
    status TEXT NOT NULL DEFAULT 'open',
    privacy_scope TEXT NOT NULL DEFAULT 'therapy_private',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);

-- table: therapy_task_references
CREATE TABLE therapy_task_references (
    id TEXT PRIMARY KEY,
    therapy_record_id TEXT NOT NULL REFERENCES therapy_records(id) ON DELETE CASCADE,
    task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    relation TEXT NOT NULL DEFAULT 'related_to',
    note_markdown TEXT,
    created_by TEXT NOT NULL DEFAULT 'user',
    created_at TEXT NOT NULL,
    removed_at TEXT
);

-- table: workspaces
CREATE TABLE workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root_path TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    locale TEXT NOT NULL DEFAULT 'en-US',
    default_privacy_scope TEXT NOT NULL DEFAULT 'private',
    current_year_theme_id TEXT,
    settings_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- table: year_themes
CREATE TABLE year_themes (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    year INTEGER NOT NULL,
    name TEXT NOT NULL,
    description_markdown TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT,
    UNIQUE(workspace_id, year)
);

-- index: idx_block_recurrence_lifecycle_intervals_lookup
CREATE INDEX idx_block_recurrence_lifecycle_intervals_lookup
    ON block_recurrence_lifecycle_intervals(workspace_id, recurrence_id, active_from_local_date)
    WHERE active_until_local_date IS NOT NULL;

-- index: idx_block_recurrence_lifecycle_intervals_open
CREATE UNIQUE INDEX idx_block_recurrence_lifecycle_intervals_open
    ON block_recurrence_lifecycle_intervals(workspace_id, recurrence_id)
    WHERE active_until_local_date IS NULL;

-- index: idx_block_recurrence_versions_current
CREATE INDEX idx_block_recurrence_versions_current
    ON block_recurrence_versions(recurrence_id, created_at);

-- index: idx_block_recurrences_workspace
CREATE INDEX idx_block_recurrences_workspace
    ON block_recurrences(workspace_id);

-- index: idx_block_recurrences_workspace_lifecycle
CREATE INDEX idx_block_recurrences_workspace_lifecycle
    ON block_recurrences(workspace_id, lifecycle_state);

-- index: idx_blocks_date
CREATE INDEX idx_blocks_date ON blocks(date);

-- index: idx_blocks_recurrence_local_date
CREATE UNIQUE INDEX idx_blocks_recurrence_local_date
    ON blocks(workspace_id, recurrence_id, local_date)
    WHERE recurrence_id IS NOT NULL;

-- index: idx_blocks_resolved_start
CREATE INDEX idx_blocks_resolved_start
    ON blocks(workspace_id, resolved_start_at)
    WHERE recurrence_id IS NOT NULL;

-- index: idx_blocks_task_id
CREATE INDEX idx_blocks_task_id
    ON blocks(task_id)
    WHERE task_id IS NOT NULL;

-- index: idx_blocks_workspace_lifecycle_date
CREATE INDEX idx_blocks_workspace_lifecycle_date
    ON blocks(workspace_id, lifecycle_state, date, planned_start_at);

-- index: idx_daily_plan_tasks_daily_plan_id
CREATE INDEX idx_daily_plan_tasks_daily_plan_id ON daily_plan_tasks(daily_plan_id);

-- index: idx_daily_plans_date
CREATE INDEX idx_daily_plans_date ON daily_plans(date);

-- index: idx_export_artifacts_job_id
CREATE INDEX idx_export_artifacts_job_id ON export_artifacts(export_job_id);

-- index: idx_export_jobs_workspace_id
CREATE INDEX idx_export_jobs_workspace_id ON export_jobs(workspace_id);

-- index: idx_fixed_commitments_active
CREATE INDEX idx_fixed_commitments_active ON fixed_commitments(active);

-- index: idx_import_batches_workspace_id
CREATE INDEX idx_import_batches_workspace_id ON import_batches(workspace_id);

-- index: idx_imported_records_batch_id
CREATE INDEX idx_imported_records_batch_id ON imported_records(import_batch_id);

-- index: idx_imported_records_source_entity
CREATE INDEX idx_imported_records_source_entity ON imported_records(source_path, entity_type);

-- index: idx_imported_records_source_key
CREATE INDEX idx_imported_records_source_key ON imported_records(source_key);

-- index: idx_imported_records_source_path
CREATE INDEX idx_imported_records_source_path ON imported_records(source_path);

-- index: idx_journal_entries_occurred_on
CREATE INDEX idx_journal_entries_occurred_on ON journal_entries(occurred_on);

-- index: idx_journal_task_references_entry_id
CREATE INDEX idx_journal_task_references_entry_id ON journal_task_references(journal_entry_id);

-- index: idx_notification_deliveries_backup_cycle
CREATE UNIQUE INDEX idx_notification_deliveries_backup_cycle
    ON notification_deliveries(workspace_id, delivery_kind, cycle_key)
    WHERE delivery_kind = 'backup';

-- index: idx_notification_deliveries_due
CREATE INDEX idx_notification_deliveries_due
    ON notification_deliveries(status, channel, scheduled_at)
    WHERE status = 'scheduled';

-- index: idx_notification_deliveries_rule_id
CREATE INDEX idx_notification_deliveries_rule_id
    ON notification_deliveries(reminder_rule_id);

-- index: idx_notification_deliveries_scheduled_at
CREATE INDEX idx_notification_deliveries_scheduled_at
    ON notification_deliveries(scheduled_at);

-- index: idx_notification_deliveries_workspace_kind
CREATE INDEX idx_notification_deliveries_workspace_kind
    ON notification_deliveries(workspace_id, delivery_kind, scheduled_at);

-- index: idx_reminder_rules_id_workspace
CREATE UNIQUE INDEX idx_reminder_rules_id_workspace
    ON reminder_rules(id, workspace_id);

-- index: idx_reminder_rules_workspace_type
CREATE UNIQUE INDEX idx_reminder_rules_workspace_type
    ON reminder_rules(workspace_id, type);

-- index: idx_task_dependencies_blocked
CREATE INDEX idx_task_dependencies_blocked ON task_dependencies(blocked_task_id);

-- index: idx_task_dependencies_prerequisite
CREATE INDEX idx_task_dependencies_prerequisite ON task_dependencies(prerequisite_task_id);

-- index: idx_task_sessions_task_id
CREATE INDEX idx_task_sessions_task_id ON task_sessions(task_id);

-- index: idx_task_status_history_task_id
CREATE INDEX idx_task_status_history_task_id ON task_status_history(task_id);

-- index: idx_tasks_goal_id
CREATE INDEX idx_tasks_goal_id ON tasks(goal_id);

-- index: idx_tasks_parent_task_id
CREATE INDEX idx_tasks_parent_task_id ON tasks(parent_task_id);

-- index: idx_tasks_project_id
CREATE INDEX idx_tasks_project_id ON tasks(project_id);

-- index: idx_therapy_records_collection_id
CREATE INDEX idx_therapy_records_collection_id ON therapy_records(therapy_collection_id);

-- trigger: block_recurrence_current_version_insert
CREATE TRIGGER block_recurrence_current_version_insert
BEFORE INSERT ON block_recurrences
WHEN NEW.current_version_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM block_recurrence_versions
    WHERE id = NEW.current_version_id AND recurrence_id = NEW.id
)
BEGIN
    SELECT RAISE(ABORT, 'current version must belong to recurrence');
END;

-- trigger: block_recurrence_current_version_update
CREATE TRIGGER block_recurrence_current_version_update
BEFORE UPDATE OF current_version_id ON block_recurrences
WHEN NEW.current_version_id IS NULL OR NOT EXISTS (
    SELECT 1 FROM block_recurrence_versions
    WHERE id = NEW.current_version_id AND recurrence_id = NEW.id
)
BEGIN
    SELECT RAISE(ABORT, 'current version must belong to recurrence');
END;

-- trigger: block_recurrence_lifecycle_intervals_order_insert
CREATE TRIGGER block_recurrence_lifecycle_intervals_order_insert
BEFORE INSERT ON block_recurrence_lifecycle_intervals
FOR EACH ROW
WHEN EXISTS (
    SELECT 1 FROM block_recurrence_lifecycle_intervals i
    WHERE i.recurrence_id = NEW.recurrence_id
      AND i.id <> NEW.id
      AND (
          i.active_from_local_date > NEW.active_from_local_date
          OR (i.active_from_local_date < NEW.active_from_local_date
              AND (i.active_until_local_date IS NULL
                   OR i.active_until_local_date > NEW.active_from_local_date))
      )
)
BEGIN
    SELECT RAISE(ABORT, 'block recurrence lifecycle intervals must be non-overlapping and ordered');
END;

-- trigger: block_recurrence_lifecycle_intervals_order_update
CREATE TRIGGER block_recurrence_lifecycle_intervals_order_update
BEFORE UPDATE OF active_from_local_date, active_until_local_date, recurrence_id
ON block_recurrence_lifecycle_intervals
FOR EACH ROW
WHEN EXISTS (
    SELECT 1 FROM block_recurrence_lifecycle_intervals i
    WHERE i.recurrence_id = NEW.recurrence_id
      AND i.id <> NEW.id
      AND (
          i.active_from_local_date > NEW.active_from_local_date
          OR (i.active_from_local_date < NEW.active_from_local_date
              AND (i.active_until_local_date IS NULL
                   OR i.active_until_local_date > NEW.active_from_local_date))
      )
)
BEGIN
    SELECT RAISE(ABORT, 'block recurrence lifecycle intervals must be non-overlapping and ordered');
END;

-- trigger: block_recurrence_versions_immutable_delete
CREATE TRIGGER block_recurrence_versions_immutable_delete
BEFORE DELETE ON block_recurrence_versions
WHEN EXISTS (
    SELECT 1 FROM block_recurrences WHERE id = OLD.recurrence_id
)
BEGIN
    SELECT RAISE(ABORT, 'block recurrence versions are immutable');
END;

-- trigger: block_recurrence_versions_immutable_update
CREATE TRIGGER block_recurrence_versions_immutable_update
BEFORE UPDATE ON block_recurrence_versions
BEGIN
    SELECT RAISE(ABORT, 'block recurrence versions are immutable');
END;

-- trigger: block_recurrence_weekdays_immutable_delete
CREATE TRIGGER block_recurrence_weekdays_immutable_delete
BEFORE DELETE ON block_recurrence_version_weekdays
WHEN EXISTS (
    SELECT 1 FROM block_recurrence_versions WHERE id = OLD.version_id
)
BEGIN
    SELECT RAISE(ABORT, 'block recurrence version weekdays are immutable');
END;

-- trigger: block_recurrence_weekdays_immutable_update
CREATE TRIGGER block_recurrence_weekdays_immutable_update
BEFORE UPDATE ON block_recurrence_version_weekdays
BEGIN
    SELECT RAISE(ABORT, 'block recurrence version weekdays are immutable');
END;

-- trigger: one_off_block_snapshot_insert
CREATE TRIGGER one_off_block_snapshot_insert
BEFORE INSERT ON blocks
WHEN NEW.recurrence_id IS NULL AND (
    NEW.recurrence_version_id IS NOT NULL OR NEW.local_date IS NOT NULL OR
    NEW.local_start_time IS NOT NULL OR NEW.timezone_id IS NOT NULL OR
    NEW.resolved_start_at IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'one-off blocks cannot have recurrence snapshots');
END;

-- trigger: one_off_block_snapshot_update
CREATE TRIGGER one_off_block_snapshot_update
BEFORE UPDATE OF recurrence_id, recurrence_version_id, local_date,
    local_start_time, timezone_id, resolved_start_at ON blocks
WHEN NEW.recurrence_id IS NULL AND (
    NEW.recurrence_version_id IS NOT NULL OR NEW.local_date IS NOT NULL OR
    NEW.local_start_time IS NOT NULL OR NEW.timezone_id IS NOT NULL OR
    NEW.resolved_start_at IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'one-off blocks cannot have recurrence snapshots');
END;

-- trigger: recurring_block_insert_consistency
CREATE TRIGGER recurring_block_insert_consistency
BEFORE INSERT ON blocks
WHEN NEW.recurrence_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM block_recurrences r
    JOIN block_recurrence_versions v ON v.recurrence_id = r.id
    WHERE r.id = NEW.recurrence_id AND r.workspace_id = NEW.workspace_id
      AND v.id = NEW.recurrence_version_id
      AND NEW.date = NEW.local_date
      AND NEW.planned_start_at = NEW.local_start_time
      AND NEW.timezone_id IS NOT NULL AND NEW.resolved_start_at IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'recurring block snapshot is inconsistent');
END;

-- trigger: recurring_block_update_consistency
CREATE TRIGGER recurring_block_update_consistency
BEFORE UPDATE OF workspace_id, date, planned_start_at, recurrence_id,
    recurrence_version_id, local_date, local_start_time, timezone_id,
    resolved_start_at ON blocks
WHEN NEW.recurrence_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM block_recurrences r
    JOIN block_recurrence_versions v ON v.recurrence_id = r.id
    WHERE r.id = NEW.recurrence_id AND r.workspace_id = NEW.workspace_id
      AND v.id = NEW.recurrence_version_id
      AND NEW.date = NEW.local_date
      AND NEW.planned_start_at = NEW.local_start_time
      AND NEW.timezone_id IS NOT NULL AND NEW.resolved_start_at IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'recurring block snapshot is inconsistent');
END;

-- trigger: trg_tasks_no_self_parent
CREATE TRIGGER trg_tasks_no_self_parent
BEFORE INSERT ON tasks
FOR EACH ROW
WHEN NEW.parent_task_id IS NOT NULL AND NEW.parent_task_id = NEW.id
BEGIN
    SELECT RAISE(ABORT, 'task cannot be its own parent');
END;

-- trigger: trg_tasks_no_self_parent_update
CREATE TRIGGER trg_tasks_no_self_parent_update
BEFORE UPDATE OF parent_task_id ON tasks
FOR EACH ROW
WHEN NEW.parent_task_id IS NOT NULL AND NEW.parent_task_id = NEW.id
BEGIN
    SELECT RAISE(ABORT, 'task cannot be its own parent');
END;
