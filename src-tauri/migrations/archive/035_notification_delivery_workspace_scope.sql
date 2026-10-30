CREATE UNIQUE INDEX idx_reminder_rules_id_workspace
    ON reminder_rules(id, workspace_id);

CREATE TABLE notification_deliveries_new (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    delivery_kind TEXT NOT NULL CHECK(delivery_kind IN ('reminder_rule','backup')),
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
    CHECK((delivery_kind = 'reminder_rule' AND reminder_rule_id IS NOT NULL)
       OR (delivery_kind = 'backup' AND reminder_rule_id IS NULL
           AND target_entity_type IS NULL AND target_entity_id IS NULL)),
    FOREIGN KEY(reminder_rule_id, workspace_id)
        REFERENCES reminder_rules(id, workspace_id) ON DELETE CASCADE
);

INSERT INTO notification_deliveries_new (
    id, workspace_id, delivery_kind, reminder_rule_id, target_entity_type,
    target_entity_id, scheduled_at, delivered_at, status, channel, safe_message,
    error_message, created_at
)
SELECT d.id, r.workspace_id, 'reminder_rule', d.reminder_rule_id,
       d.target_entity_type, d.target_entity_id, d.scheduled_at, d.delivered_at,
       d.status, d.channel, d.safe_message, d.error_message, d.created_at
FROM notification_deliveries d
LEFT JOIN reminder_rules r ON r.id = d.reminder_rule_id;

DROP TABLE notification_deliveries;
ALTER TABLE notification_deliveries_new RENAME TO notification_deliveries;

CREATE INDEX idx_notification_deliveries_rule_id
    ON notification_deliveries(reminder_rule_id);
CREATE INDEX idx_notification_deliveries_scheduled_at
    ON notification_deliveries(scheduled_at);
CREATE INDEX idx_notification_deliveries_workspace_kind
    ON notification_deliveries(workspace_id, delivery_kind, scheduled_at);
