-- 032_import_provenance.sql
-- Import provenance and idempotency fields (Phase 7.10 import idempotency).
-- Migration 026 is immutable; this forward migration adds deterministic source_key
-- and content_hash. A unique index on source_key alone is NOT created because
-- multiple import batches may reference the same source_key (idempotent re-import).
-- Idempotency is enforced at the application layer via find_existing (per workspace).
-- Numbers 029 and 030 are reserved for Phase 8 (reminder_rules and
-- notification_deliveries).

ALTER TABLE imported_records ADD COLUMN source_key TEXT;
ALTER TABLE imported_records ADD COLUMN content_hash TEXT;

CREATE INDEX IF NOT EXISTS idx_imported_records_source_key ON imported_records(source_key);