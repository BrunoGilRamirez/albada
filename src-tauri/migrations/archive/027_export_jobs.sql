-- 027_export_jobs.sql
-- A concrete request to generate Markdown, LaTeX, or PDF from an explicit
-- selection of entity IDs. Generic exports never include journal/therapy content.
CREATE TABLE export_jobs (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    type TEXT NOT NULL,
    scope_type TEXT NOT NULL,
    title TEXT NOT NULL,
    template_name TEXT,
    selection_json TEXT NOT NULL,
    options_json TEXT,
    status TEXT NOT NULL DEFAULT 'queued',
    output_path TEXT,
    generated_at TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_export_jobs_workspace_id ON export_jobs(workspace_id);
