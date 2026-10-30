-- 029_reminder_rules.sql
-- Reminder rules for the daily-loop rituals (Phase 8.0). This uses the version
-- number reserved for `reminder_rules` ahead of migrations 031/032.
--
-- One rule per (workspace_id, type): the unique index below is the idempotency
-- key for seeding the five default rules. Seeding runs in the established
-- initialization path (`SqliteWorkspaceRepo::ensure_default`, see
-- `src-tauri/src/db/seed.rs`) because the default workspace is created lazily
-- and does not exist at migration time, while `workspace_id` is a NOT NULL
-- foreign key. No second DB owner is introduced.
--
-- Time-of-day columns (`preferred_time`, `time_window_start`/`time_window_end`,
-- `quiet_hours_start`/`quiet_hours_end`) are local `HH:MM` strings; their
-- interpretation belongs to the scheduling domain (Phase 8.1), not the schema.
-- `type`: planning | focus | journaling | review | deadline (validated in Rust).
-- `channel`: desktop | mobile | both; MVP uses `desktop` only.
-- `escalation`: none | gentle.
-- `privacy_safe_message` is the only text a notification may ever show:
-- lock-screen safe, never task titles, journal excerpts, or therapy content.
CREATE TABLE reminder_rules (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    type TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    recurrence_rule TEXT,
    preferred_time TEXT,
    time_window_start TEXT,
    time_window_end TEXT,
    quiet_hours_start TEXT,
    quiet_hours_end TEXT,
    max_reminders_per_day INTEGER NOT NULL DEFAULT 5,
    snooze_minutes_json TEXT NOT NULL DEFAULT '[10,30,60]',
    channel TEXT NOT NULL DEFAULT 'desktop',
    escalation TEXT NOT NULL DEFAULT 'none',
    privacy_safe_message TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_reminder_rules_workspace_type
    ON reminder_rules(workspace_id, type);
