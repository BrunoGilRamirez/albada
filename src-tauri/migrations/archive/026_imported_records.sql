-- 026_imported_records.sql
-- Provenance for each record created (or matched) by an import batch.
-- entity_id references the created domain entity (journal entry, therapy
-- record, ...). Idempotency matches on (source_path, entity_type) per workspace.
CREATE TABLE imported_records (
    id TEXT PRIMARY KEY,
    import_batch_id TEXT NOT NULL REFERENCES import_batches(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    source_path TEXT NOT NULL,
    source_date TEXT,
    confidence TEXT NOT NULL DEFAULT 'review',
    warnings_json TEXT,
    imported_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_imported_records_batch_id ON imported_records(import_batch_id);
CREATE INDEX IF NOT EXISTS idx_imported_records_source_path ON imported_records(source_path);
CREATE INDEX IF NOT EXISTS idx_imported_records_source_entity ON imported_records(source_path, entity_type);
