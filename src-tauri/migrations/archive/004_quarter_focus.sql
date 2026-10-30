-- 004_quarter_focus.sql
-- Contextual selection of goals and projects for a quarter.
CREATE TABLE quarter_focus (
    id TEXT PRIMARY KEY,
    quarter_id TEXT NOT NULL REFERENCES quarters(id) ON DELETE CASCADE,
    goal_id TEXT REFERENCES goals(id) ON DELETE SET NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    priority TEXT NOT NULL DEFAULT 'normal',
    intention_markdown TEXT,
    success_signal_markdown TEXT,
    status TEXT NOT NULL DEFAULT 'proposed',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK((goal_id IS NULL) != (project_id IS NULL))
);