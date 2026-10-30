-- 042_block_recurrence_lifecycle_intervals.sql
-- Phase 8.7.11 recurrence lifecycle interval persistence.
--
-- The current `block_recurrences.lifecycle_state` keeps only the present state,
-- so historical deactivate/reactivate windows of a series are unrecoverable.
-- This additive table records the active schedule periods that the Phase 8.7.10
-- habit-analytics contract (model document sections 8.9/20.10) reads; analytics
-- only ever reads them and never writes.
--
-- `source = 'seed'` is reserved for the migration backfill below, which attests
-- "active since creation" for series that exist when 042 applies. Every runtime
-- write (series creation, deactivation, reactivation, soft deletion) records
-- `'explicit'`.
--
-- Migrations 001-039 are untouched and the checked-in 040/041 are not altered.
--
-- The 042 runtime hook first performs a PRAGMA-guarded ensure of the 040
-- lifecycle/task columns (`ensure_block_recurrence_lifecycle_columns`) before
-- seeding, because a database may have recorded a pre-amendment revision of
-- staged migration 040 that lacks them. No schema statement is added here: the
-- repair lives in the migration runner only.

CREATE TABLE block_recurrence_lifecycle_intervals (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    recurrence_id TEXT NOT NULL REFERENCES block_recurrences(id) ON DELETE CASCADE,
    active_from_local_date TEXT NOT NULL,
    active_until_local_date TEXT,
    source TEXT NOT NULL CHECK (source IN ('seed', 'explicit')),
    created_at TEXT NOT NULL,
    CHECK (active_until_local_date IS NULL OR active_until_local_date >= active_from_local_date)
);

-- At most one open interval per series: `active_until_local_date IS NULL` means
-- the series is still active from that local date on.
CREATE UNIQUE INDEX idx_block_recurrence_lifecycle_intervals_open
    ON block_recurrence_lifecycle_intervals(workspace_id, recurrence_id)
    WHERE active_until_local_date IS NULL;

-- Closed intervals of a series by local date.
CREATE INDEX idx_block_recurrence_lifecycle_intervals_lookup
    ON block_recurrence_lifecycle_intervals(workspace_id, recurrence_id, active_from_local_date)
    WHERE active_until_local_date IS NOT NULL;

-- Intervals of one series are non-overlapping and non-decreasing: a new interval
-- must not start before an existing one and must not overlap a closed interval,
-- and no interval may be added while the series still has an open one.
-- Touching endpoints stay valid, because deactivating and reactivating a series
-- on the same workspace-local date closes one interval and opens the next on
-- that same date.
CREATE TRIGGER block_recurrence_lifecycle_intervals_order_insert
BEFORE INSERT ON block_recurrence_lifecycle_intervals
FOR EACH ROW
WHEN EXISTS (
    SELECT 1 FROM block_recurrence_lifecycle_intervals i
    WHERE i.recurrence_id = NEW.recurrence_id
      AND i.id <> NEW.id
      AND (
          i.active_from_local_date > NEW.active_from_local_date
          OR (i.active_from_local_date < NEW.active_from_local_date
              AND (i.active_until_local_date IS NULL
                   OR i.active_until_local_date > NEW.active_from_local_date))
      )
)
BEGIN
    SELECT RAISE(ABORT, 'block recurrence lifecycle intervals must be non-overlapping and ordered');
END;

CREATE TRIGGER block_recurrence_lifecycle_intervals_order_update
BEFORE UPDATE OF active_from_local_date, active_until_local_date, recurrence_id
ON block_recurrence_lifecycle_intervals
FOR EACH ROW
WHEN EXISTS (
    SELECT 1 FROM block_recurrence_lifecycle_intervals i
    WHERE i.recurrence_id = NEW.recurrence_id
      AND i.id <> NEW.id
      AND (
          i.active_from_local_date > NEW.active_from_local_date
          OR (i.active_from_local_date < NEW.active_from_local_date
              AND (i.active_until_local_date IS NULL
                   OR i.active_until_local_date > NEW.active_from_local_date))
      )
)
BEGIN
    SELECT RAISE(ABORT, 'block recurrence lifecycle intervals must be non-overlapping and ordered');
END;
