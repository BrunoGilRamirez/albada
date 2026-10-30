-- 007_tasks.sql
-- The single Task entity for standalone, parent, and child tasks.
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    goal_id TEXT REFERENCES goals(id) ON DELETE SET NULL,
    parent_task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description_markdown TEXT,
    notes_markdown TEXT,
    status TEXT NOT NULL DEFAULT 'inbox',
    priority TEXT NOT NULL DEFAULT 'none',
    effort_size TEXT,
    effort_points INTEGER GENERATED ALWAYS AS (
        CASE effort_size
            WHEN 'trivial' THEN 1
            WHEN 'small' THEN 2
            WHEN 'medium' THEN 3
            WHEN 'large' THEN 5
            WHEN 'very_large' THEN 8
            ELSE 1
        END
    ) STORED,
    target_date TEXT,
    scheduled_date TEXT,
    completed_at TEXT,
    cancelled_at TEXT,
    closure_mode TEXT,
    closure_reason_markdown TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);