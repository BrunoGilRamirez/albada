-- 005_goals.sql
-- A goal expresses a meaningful direction or result.
CREATE TABLE goals (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    year_theme_id TEXT REFERENCES year_themes(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    description_markdown TEXT,
    why_it_matters_markdown TEXT,
    success_definition_markdown TEXT,
    status TEXT NOT NULL DEFAULT 'inbox',
    priority TEXT NOT NULL DEFAULT 'none',
    target_date TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);