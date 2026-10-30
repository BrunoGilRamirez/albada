-- 030_notification_deliveries.sql
-- Technical notification history for scheduled and delivered reminders
-- (Phase 8.0). This uses the version number reserved for
-- `notification_deliveries` ahead of migrations 031/032.
--
-- A delivery row records what was scheduled and what happened to it. A missed,
-- dismissed, or failed delivery never changes any task status (domain
-- invariant); scheduling and delivery behavior live in the Rust domain
-- (Phase 8.1+), not in the schema.
-- `status`: scheduled | delivered | snoozed | dismissed | failed (validated in
-- Rust at the command boundary).
-- `target_entity_type`/`target_entity_id` are optional, untyped pointers to the
-- entity a rule evaluated (task, project, ...). They are intentionally not
-- foreign keys so delivery history survives the deletion of its target.
-- `safe_message` copies the rule's privacy-safe text at schedule time; it must
-- never contain journal or therapy content.
CREATE TABLE notification_deliveries (
    id TEXT PRIMARY KEY,
    reminder_rule_id TEXT NOT NULL REFERENCES reminder_rules(id) ON DELETE CASCADE,
    target_entity_type TEXT,
    target_entity_id TEXT,
    scheduled_at TEXT NOT NULL,
    delivered_at TEXT,
    status TEXT NOT NULL DEFAULT 'scheduled',
    channel TEXT NOT NULL,
    safe_message TEXT NOT NULL,
    error_message TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_notification_deliveries_rule_id
    ON notification_deliveries(reminder_rule_id);
CREATE INDEX IF NOT EXISTS idx_notification_deliveries_scheduled_at
    ON notification_deliveries(scheduled_at);
