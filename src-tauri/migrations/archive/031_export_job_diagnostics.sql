-- 031_export_job_diagnostics.sql
-- Export-job diagnostic and confirmation metadata (Phase 7.10 export integrity
-- slice). Numbers 029 and 030 are reserved for Phase 8 (reminder_rules and
-- notification_deliveries), so this forward migration uses 031 and must not be
-- edited once applied.
--
-- `selection_fingerprint` records the deterministic fingerprint over the
-- resolved selection snapshot at job creation, binding the job to the preview
-- the user confirmed. `diagnostics_json` stores a JSON array of safe,
-- structured warnings (code, severity, message); it never contains source text,
-- therapy content, absolute paths, or secrets.
ALTER TABLE export_jobs ADD COLUMN selection_fingerprint TEXT;
ALTER TABLE export_jobs ADD COLUMN diagnostics_json TEXT;
