ALTER TABLE blocks ADD COLUMN estimated_duration_seconds INTEGER
    CHECK (estimated_duration_seconds IS NULL OR estimated_duration_seconds > 0);
ALTER TABLE blocks ADD COLUMN recurrence_id TEXT REFERENCES block_recurrences(id) ON DELETE CASCADE;
ALTER TABLE blocks ADD COLUMN recurrence_version_id TEXT REFERENCES block_recurrence_versions(id);
ALTER TABLE blocks ADD COLUMN local_date TEXT;
ALTER TABLE blocks ADD COLUMN local_start_time TEXT;
ALTER TABLE blocks ADD COLUMN timezone_id TEXT;
ALTER TABLE blocks ADD COLUMN resolved_start_at TEXT;
ALTER TABLE blocks ADD COLUMN lifecycle_state TEXT NOT NULL DEFAULT 'active'
    CHECK (lifecycle_state IN ('active', 'inactive', 'deleted'));
ALTER TABLE blocks ADD COLUMN task_id TEXT REFERENCES tasks(id) ON DELETE RESTRICT;
ALTER TABLE blocks ADD COLUMN reactivated_from_block_id TEXT
    REFERENCES blocks(id) ON DELETE RESTRICT;

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

CREATE TABLE block_recurrence_version_weekdays (
    version_id TEXT NOT NULL REFERENCES block_recurrence_versions(id) ON DELETE CASCADE,
    weekday INTEGER NOT NULL CHECK (weekday BETWEEN 1 AND 7),
    PRIMARY KEY (version_id, weekday)
);

CREATE TRIGGER block_recurrence_versions_immutable_update
BEFORE UPDATE ON block_recurrence_versions
BEGIN
    SELECT RAISE(ABORT, 'block recurrence versions are immutable');
END;

CREATE TRIGGER block_recurrence_versions_immutable_delete
BEFORE DELETE ON block_recurrence_versions
WHEN EXISTS (
    SELECT 1 FROM block_recurrences WHERE id = OLD.recurrence_id
)
BEGIN
    SELECT RAISE(ABORT, 'block recurrence versions are immutable');
END;

CREATE TRIGGER block_recurrence_weekdays_immutable_update
BEFORE UPDATE ON block_recurrence_version_weekdays
BEGIN
    SELECT RAISE(ABORT, 'block recurrence version weekdays are immutable');
END;

CREATE TRIGGER block_recurrence_weekdays_immutable_delete
BEFORE DELETE ON block_recurrence_version_weekdays
WHEN EXISTS (
    SELECT 1 FROM block_recurrence_versions WHERE id = OLD.version_id
)
BEGIN
    SELECT RAISE(ABORT, 'block recurrence version weekdays are immutable');
END;

CREATE TRIGGER block_recurrence_current_version_insert
BEFORE INSERT ON block_recurrences
WHEN NEW.current_version_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM block_recurrence_versions
    WHERE id = NEW.current_version_id AND recurrence_id = NEW.id
)
BEGIN
    SELECT RAISE(ABORT, 'current version must belong to recurrence');
END;

CREATE TRIGGER block_recurrence_current_version_update
BEFORE UPDATE OF current_version_id ON block_recurrences
WHEN NEW.current_version_id IS NULL OR NOT EXISTS (
    SELECT 1 FROM block_recurrence_versions
    WHERE id = NEW.current_version_id AND recurrence_id = NEW.id
)
BEGIN
    SELECT RAISE(ABORT, 'current version must belong to recurrence');
END;

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

CREATE UNIQUE INDEX idx_blocks_recurrence_local_date
    ON blocks(workspace_id, recurrence_id, local_date)
    WHERE recurrence_id IS NOT NULL;
CREATE INDEX idx_block_recurrences_workspace
    ON block_recurrences(workspace_id);
CREATE INDEX idx_block_recurrence_versions_current
    ON block_recurrence_versions(recurrence_id, created_at);
CREATE INDEX idx_blocks_resolved_start
    ON blocks(workspace_id, resolved_start_at)
    WHERE recurrence_id IS NOT NULL;
CREATE INDEX idx_blocks_workspace_lifecycle_date
    ON blocks(workspace_id, lifecycle_state, date, planned_start_at);
CREATE INDEX idx_blocks_task_id
    ON blocks(task_id)
    WHERE task_id IS NOT NULL;
CREATE INDEX idx_block_recurrences_workspace_lifecycle
    ON block_recurrences(workspace_id, lifecycle_state);
