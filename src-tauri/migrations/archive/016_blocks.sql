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
);