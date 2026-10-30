-- 008_task_dependencies.sql
-- Dependency relationships between tasks (membership is distinct from dependency).
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