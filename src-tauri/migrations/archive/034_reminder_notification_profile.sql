-- Backfill the default notification profile for enabled reminder rules only.
-- COALESCE preserves any time already configured by the user.
UPDATE reminder_rules
SET preferred_time = COALESCE(preferred_time, '09:00'),
    quiet_hours_start = COALESCE(quiet_hours_start, '22:00'),
    quiet_hours_end = COALESCE(quiet_hours_end, '07:00')
WHERE enabled = 1;
