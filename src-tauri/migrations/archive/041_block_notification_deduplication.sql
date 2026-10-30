-- 041_block_notification_deduplication.sql
-- Phase 8.7.3 block notification delivery support.
--
-- Block start/boundary/overrun events are workspace-scoped deliveries with no
-- reminder rule. They use a new terminal-origin kind so they never consume or
-- inherit reminder-rule daily-cap accounting (blocker B4 stays open).
--
-- Durable event deduplication lives outside visible history so keys survive
-- poll repeats, app restarts, history clearing, and pruning. Existing backup
-- cycle deduplication keeps its partial unique index unchanged and is also
-- seeded here before any future history cleanup.
--
-- Migration 040 and all earlier migrations are unchanged.

CREATE TABLE notification_delivery_dedupe (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    event_key TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (workspace_id, event_key)
);

INSERT OR IGNORE INTO notification_delivery_dedupe (workspace_id, event_key, created_at)
SELECT workspace_id, 'backup:' || cycle_key, created_at
FROM notification_deliveries
WHERE delivery_kind = 'backup' AND cycle_key IS NOT NULL;

CREATE TABLE notification_deliveries_new (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    delivery_kind TEXT NOT NULL CHECK(delivery_kind IN ('reminder_rule','backup','block')),
    reminder_rule_id TEXT,
    target_entity_type TEXT,
    target_entity_id TEXT,
    scheduled_at TEXT NOT NULL,
    delivered_at TEXT,
    status TEXT NOT NULL DEFAULT 'scheduled',
    channel TEXT NOT NULL,
    safe_message TEXT NOT NULL,
    error_message TEXT,
    created_at TEXT NOT NULL,
    cycle_key TEXT,
    CHECK((delivery_kind = 'reminder_rule' AND reminder_rule_id IS NOT NULL)
       OR (delivery_kind = 'backup' AND reminder_rule_id IS NULL
           AND target_entity_type IS NULL AND target_entity_id IS NULL)
       OR (delivery_kind = 'block' AND reminder_rule_id IS NULL
           AND target_entity_type = 'block' AND target_entity_id IS NOT NULL)),
    FOREIGN KEY(reminder_rule_id, workspace_id)
        REFERENCES reminder_rules(id, workspace_id) ON DELETE CASCADE
);

INSERT INTO notification_deliveries_new (
    id, workspace_id, delivery_kind, reminder_rule_id, target_entity_type,
    target_entity_id, scheduled_at, delivered_at, status, channel, safe_message,
    error_message, created_at, cycle_key
)
SELECT id, workspace_id, delivery_kind, reminder_rule_id, target_entity_type,
       target_entity_id, scheduled_at, delivered_at, status, channel, safe_message,
       error_message, created_at, cycle_key
FROM notification_deliveries;

DROP TABLE notification_deliveries;
ALTER TABLE notification_deliveries_new RENAME TO notification_deliveries;

CREATE INDEX idx_notification_deliveries_rule_id
    ON notification_deliveries(reminder_rule_id);
CREATE INDEX idx_notification_deliveries_scheduled_at
    ON notification_deliveries(scheduled_at);
CREATE INDEX idx_notification_deliveries_workspace_kind
    ON notification_deliveries(workspace_id, delivery_kind, scheduled_at);
CREATE UNIQUE INDEX idx_notification_deliveries_backup_cycle
    ON notification_deliveries(workspace_id, delivery_kind, cycle_key)
    WHERE delivery_kind = 'backup';
CREATE INDEX idx_notification_deliveries_due
    ON notification_deliveries(status, channel, scheduled_at)
    WHERE status = 'scheduled';
