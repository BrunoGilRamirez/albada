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