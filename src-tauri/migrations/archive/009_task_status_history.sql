-- 009_task_status_history.sql
-- Preserves every status transition for traceability.
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