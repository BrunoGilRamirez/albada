-- 013_task_blocking_note.sql
-- Supports explicit external blockers: a task can be marked blocked when it
-- has an active dependency OR a non-empty blocking note.
ALTER TABLE tasks ADD COLUMN blocking_note_markdown TEXT;