CREATE TABLE therapy_records (
    id TEXT PRIMARY KEY,
    therapy_collection_id TEXT NOT NULL REFERENCES therapy_collections(id) ON DELETE CASCADE,
    type TEXT NOT NULL,
    title TEXT NOT NULL,
    content_markdown TEXT NOT NULL,
    occurred_on TEXT,
    status TEXT NOT NULL DEFAULT 'open',
    privacy_scope TEXT NOT NULL DEFAULT 'therapy_private',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_therapy_records_collection_id ON therapy_records(therapy_collection_id);
