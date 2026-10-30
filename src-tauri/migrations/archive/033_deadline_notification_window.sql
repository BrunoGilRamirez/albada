ALTER TABLE reminder_rules
    ADD COLUMN deadline_notification_window_days INTEGER NOT NULL DEFAULT 3
    CHECK (deadline_notification_window_days BETWEEN 1 AND 7);
