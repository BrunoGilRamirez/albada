-- 011_task_tree_constraints.sql
-- Task tree integrity: reject self-parenting at the database level and
-- index the membership lookups used by tree validation and queries.
--
-- SQLite enforces a single "NOT NULL" on rowid for triggers, so the
-- self-parent check uses the id comparison directly (id is NOT NULL).

-- Indexes for tree navigation and validation.
CREATE INDEX IF NOT EXISTS idx_tasks_parent_task_id ON tasks(parent_task_id);
CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks(project_id);
CREATE INDEX IF NOT EXISTS idx_tasks_goal_id ON tasks(goal_id);

-- Reject parent_task_id = id (a task cannot be its own parent).
CREATE TRIGGER IF NOT EXISTS trg_tasks_no_self_parent
BEFORE INSERT ON tasks
FOR EACH ROW
WHEN NEW.parent_task_id IS NOT NULL AND NEW.parent_task_id = NEW.id
BEGIN
    SELECT RAISE(ABORT, 'task cannot be its own parent');
END;

CREATE TRIGGER IF NOT EXISTS trg_tasks_no_self_parent_update
BEFORE UPDATE OF parent_task_id ON tasks
FOR EACH ROW
WHEN NEW.parent_task_id IS NOT NULL AND NEW.parent_task_id = NEW.id
BEGIN
    SELECT RAISE(ABORT, 'task cannot be its own parent');
END;