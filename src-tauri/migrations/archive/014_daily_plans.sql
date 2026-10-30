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