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
);

CREATE INDEX IF NOT EXISTS idx_journal_entries_occurred_on ON journal_entries(occurred_on);
