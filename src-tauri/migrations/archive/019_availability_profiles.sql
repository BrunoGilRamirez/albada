CREATE TABLE availability_profiles (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    weekday INTEGER NOT NULL CHECK(weekday BETWEEN 0 AND 6),
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    preferred_block_length_minutes INTEGER,
    energy_expectation TEXT,
    active_from TEXT,
    active_until TEXT,
    note_markdown TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_daily_plans_date ON daily_plans(date);
CREATE INDEX IF NOT EXISTS idx_daily_plan_tasks_daily_plan_id ON daily_plan_tasks(daily_plan_id);
CREATE INDEX IF NOT EXISTS idx_blocks_date ON blocks(date);
CREATE INDEX IF NOT EXISTS idx_task_sessions_task_id ON task_sessions(task_id);
CREATE INDEX IF NOT EXISTS idx_fixed_commitments_active ON fixed_commitments(active);