-- 002_year_themes.sql
-- Annual narrative context for a workspace.
CREATE TABLE year_themes (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    year INTEGER NOT NULL,
    name TEXT NOT NULL,
    description_markdown TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT,
    UNIQUE(workspace_id, year)
);