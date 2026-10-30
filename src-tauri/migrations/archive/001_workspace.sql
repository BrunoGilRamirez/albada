-- 001_workspace.sql
-- A Workspace represents one installation managed by the application.
CREATE TABLE workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root_path TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    locale TEXT NOT NULL DEFAULT 'en-US',
    default_privacy_scope TEXT NOT NULL DEFAULT 'private',
    current_year_theme_id TEXT,
    settings_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);