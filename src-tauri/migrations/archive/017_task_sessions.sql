CREATE TABLE task_sessions (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    block_id TEXT REFERENCES blocks(id) ON DELETE SET NULL,
    daily_plan_task_id TEXT REFERENCES daily_plan_tasks(id) ON DELETE SET NULL,
    started_at TEXT,
    ended_at TEXT,
    status TEXT NOT NULL DEFAULT 'planned',
    outcome_markdown TEXT,
    next_step_markdown TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);