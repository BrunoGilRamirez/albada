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

CREATE INDEX IF NOT EXISTS idx_journal_task_references_entry_id ON journal_task_references(journal_entry_id);
