-- 012_dependency_indexes.sql
-- Indexes for dependency traversal and status-history lookups.
CREATE INDEX IF NOT EXISTS idx_task_dependencies_blocked ON task_dependencies(blocked_task_id);
CREATE INDEX IF NOT EXISTS idx_task_dependencies_prerequisite ON task_dependencies(prerequisite_task_id);
CREATE INDEX IF NOT EXISTS idx_task_status_history_task_id ON task_status_history(task_id);