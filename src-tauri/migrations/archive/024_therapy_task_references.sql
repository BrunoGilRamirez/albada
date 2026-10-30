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
