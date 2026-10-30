CREATE TABLE daily_plan_tasks (
    id TEXT PRIMARY KEY,
    daily_plan_id TEXT NOT NULL REFERENCES daily_plans(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    order_index INTEGER NOT NULL,
    selection_reason TEXT NOT NULL DEFAULT 'manual',
    planned_status TEXT NOT NULL DEFAULT 'selected',
    carryover_from_daily_plan_id TEXT REFERENCES daily_plans(id) ON DELETE SET NULL,
    decision_note_markdown TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(daily_plan_id, task_id)
);