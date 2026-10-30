CREATE TABLE journal_entry_legacy_template_values (
    entry_id TEXT NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
    field_id TEXT NOT NULL,
    value_json TEXT NOT NULL,
    PRIMARY KEY (entry_id, field_id)
);
