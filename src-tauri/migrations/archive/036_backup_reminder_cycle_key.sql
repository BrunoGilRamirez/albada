-- 036_backup_reminder_cycle_key.sql
-- Backup-reminder cycle deduplication (settled L3 contract, 2026-10-07).
--
-- Adds a nullable `cycle_key` to `notification_deliveries`. Rule deliveries
-- keep `cycle_key` NULL and are otherwise unchanged; every existing row is
-- preserved. Backup deliveries always receive a key from the dispatcher, and
-- a partial unique index enforces at most one backup delivery per
-- (workspace, delivery_kind, cycle_key) so a pending, delivered, dismissed,
-- or failed row satisfies that cycle's dedupe. The index is filtered to
-- `delivery_kind = 'backup'` so rule rows (NULL key) remain valid and
-- unconstrained.
ALTER TABLE notification_deliveries ADD COLUMN cycle_key TEXT;

CREATE UNIQUE INDEX idx_notification_deliveries_backup_cycle
    ON notification_deliveries(workspace_id, delivery_kind, cycle_key)
    WHERE delivery_kind = 'backup';
