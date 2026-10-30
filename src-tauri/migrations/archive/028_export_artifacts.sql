-- 028_export_artifacts.sql
-- Files produced by an export job. A single export can produce both .tex and .pdf.
CREATE TABLE export_artifacts (
    id TEXT PRIMARY KEY,
    export_job_id TEXT NOT NULL REFERENCES export_jobs(id) ON DELETE CASCADE,
    format TEXT NOT NULL,
    path TEXT NOT NULL,
    checksum TEXT,
    size_bytes INTEGER,
    created_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_export_artifacts_job_id ON export_artifacts(export_job_id);
