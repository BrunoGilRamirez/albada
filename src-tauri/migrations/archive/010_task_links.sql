-- 010_task_links.sql
-- Secondary relationships without altering membership or progress.
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