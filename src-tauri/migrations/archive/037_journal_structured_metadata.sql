ALTER TABLE journal_entries ADD COLUMN emotions_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE journal_entries ADD COLUMN template_version TEXT;
ALTER TABLE journal_entries ADD COLUMN template_values_json TEXT NOT NULL DEFAULT '{}';
ALTER TABLE journal_entries ADD COLUMN export_eligible INTEGER NOT NULL DEFAULT 0
    CHECK (export_eligible IN (0, 1));

UPDATE journal_entries
SET export_eligible = CASE WHEN privacy_scope = 'export_eligible' THEN 1 ELSE 0 END;

UPDATE journal_entries
SET privacy_scope = CASE WHEN export_eligible = 1 THEN 'export_eligible' ELSE 'private' END;
