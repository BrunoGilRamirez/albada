import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  deactivateBlock,
  deactivateBlockRecurrence,
  deleteBlock,
  deleteBlockRecurrence,
  listBlocksForDate,
  listInactiveBlockRecurrences,
  listInactiveBlocks,
  listScheduledBlockRecurrences,
  listUpcomingBlocks,
  reactivateBlockRecurrence,
} from "../../lib/api/blockApi";
import { toDisplayMessage } from "../../lib/errors";
import { formatDate as intlFormatDate } from "../../lib/format";
import BlockCard from "../blocks/BlockCard";
import BlockForm from "../blocks/BlockForm";
import ReactivateScheduleForm from "../blocks/ReactivateScheduleForm";
import type { Block, BlockRecurrence, ScheduledBlockRecurrence } from "../../lib/types";
import { isoWeekdayKey } from "../../lib/weekdays";

const SCHEDULED_PAGE_SIZE = 50;
const SCHEDULED_WINDOW_DAYS = 90;

function addDays(date: string, days: number): string {
  const parsed = new Date(`${date}T00:00:00`);
  parsed.setDate(parsed.getDate() + days);
  const year = parsed.getFullYear();
  const month = String(parsed.getMonth() + 1).padStart(2, "0");
  const day = String(parsed.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function firstWindowEnd(date: string): string {
  return addDays(addDays(date, 1), SCHEDULED_WINDOW_DAYS - 1);
}

function mergeRows(current: Block[], incoming: Block[]): Block[] {
  const seen = new Set(current.map((row) => row.id));
  return [...current, ...incoming.filter((row) => !seen.has(row.id))];
}

function isUpcomingRow(row: Block): boolean {
  return row.lifecycle_state === "active" && (row.status === "planned" || row.status === "active");
}

interface ScheduledGroup {
  date: string;
  rows: Block[];
}

function groupByLocalDate(rows: Block[]): ScheduledGroup[] {
  const groups = new Map<string, Block[]>();
  for (const row of rows) {
    const key = row.local_date ?? row.date;
    const bucket = groups.get(key);
    if (bucket) bucket.push(row);
    else groups.set(key, [row]);
  }
  return [...groups.entries()]
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([date, bucket]) => ({ date, rows: sortBlocks(bucket) }));
}

interface TodayBlockListProps {
  workspaceId: string;
  date: string;
  dailyPlanId?: string | null;
  blocks: Block[];
  onChanged: (blocks: Block[]) => void;
  /**
   * Re-read the owning page after a lifecycle write that can change rows this
   * component does not model locally (a series action or a reactivation).
   */
  onRefresh?: () => void;
}

/**
 * Client-side mirror of the native `list_for_date` ordering
 * (`planned_start_at IS NULL, planned_start_at, created_at, id`): timed
 * blocks first in exact local start order, untimed blocks last, and a
 * deterministic `created_at` → `id` tie order for stable rendering.
 * "HH:MM" strings compare chronologically, and materialized occurrences
 * snapshot their rule's local start into `planned_start_at`. Pure and
 * immutable: the input array is never reordered in place.
 */
function sortBlocks(blocks: Block[]): Block[] {
  return [...blocks].sort((a, b) => {
    const at = a.planned_start_at ?? a.local_start_time;
    const bt = b.planned_start_at ?? b.local_start_time;
    if (at === bt) {
      if (a.created_at !== b.created_at) {
        return a.created_at.localeCompare(b.created_at);
      }
      return a.id.localeCompare(b.id);
    }
    if (at === null) {
      return 1;
    }
    if (bt === null) {
      return -1;
    }
    return at.localeCompare(bt);
  });
}

function replaceOrAppend(blocks: Block[], block: Block): Block[] {
  const without = blocks.filter((candidate) => candidate.id !== block.id);
  return sortBlocks([...without, block]);
}

function TodayBlockList({
  workspaceId,
  date,
  dailyPlanId,
  blocks,
  onChanged,
  onRefresh,
}: TodayBlockListProps) {
  const { t } = useTranslation();
  const [showCreate, setShowCreate] = useState(false);
  const [editingTarget, setEditingTarget] = useState<Block | null>(null);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  // True while a form save is in flight; used to disable card actions so no
  // concurrent write can interleave with the pending row or its rollback.
  const [saveInFlight, setSaveInFlight] = useState(false);

  // --- scheduled blocks section -------------------------------------------
  const [scheduledOpen, setScheduledOpen] = useState(false);
  const [scheduledRows, setScheduledRows] = useState<Block[]>([]);
  const [scheduledSeries, setScheduledSeries] = useState<ScheduledBlockRecurrence[]>([]);
  const [scheduledLoading, setScheduledLoading] = useState(false);
  const [scheduledError, setScheduledError] = useState<string | null>(null);
  // True once a page was read, so collapsing and reopening the section does
  // not refetch the same rows.
  const [scheduledLoaded, setScheduledLoaded] = useState(false);
  const [scheduledOffset, setScheduledOffset] = useState(0);
  const [scheduledWindowEnd, setScheduledWindowEnd] = useState(() => firstWindowEnd(date));
  const [scheduledHasMore, setScheduledHasMore] = useState(false);
  const [scheduledEditingBlock, setScheduledEditingBlock] = useState<Block | null>(null);
  const [scheduledEditing, setScheduledEditing] = useState<ScheduledBlockRecurrence | null>(null);

  // --- inactive blocks section --------------------------------------------
  const [inactiveOpen, setInactiveOpen] = useState(false);
  const [inactiveBlocks, setInactiveBlocks] = useState<Block[]>([]);
  const [inactiveSeries, setInactiveSeries] = useState<BlockRecurrence[]>([]);
  const [inactiveLoading, setInactiveLoading] = useState(false);
  const [inactiveError, setInactiveError] = useState<string | null>(null);
  // True once the inactive lists were read, so collapsing and reopening the
  // section does not refetch them.
  const [inactiveLoaded, setInactiveLoaded] = useState(false);
  const [reactivateTarget, setReactivateTarget] = useState<Block | null>(null);
  const [inactiveNotice, setInactiveNotice] = useState<string | null>(null);
  const [reactivatedSeriesIds, setReactivatedSeriesIds] = useState<string[]>([]);

  // The optimistic handlers below are invoked from async paths that were
  // started in an earlier render, so they read the latest visible rows
  // through refs instead of a captured `blocks` closure.
  const blocksRef = useRef(blocks);
  useEffect(() => {
    blocksRef.current = blocks;
  }, [blocks]);
  // Scoped undo data for the one in-flight form save: the provisional row
  // currently shown and, for edits, the original row to restore. A rollback
  // touches only that row so concurrent status actions or unrelated list
  // updates made while the save was pending are never discarded.
  const pendingRef = useRef<Block | null>(null);
  const originalRef = useRef<Block | null>(null);

  // A planned row that was manually deactivated or soft-deleted no longer
  // belongs to the active day list. Terminal rows (completed/expired) keep
  // rendering, and the lifecycle state stays independent of the workflow
  // status.
  const visible = useMemo(
    () =>
      sortBlocks(
        blocks.filter(
          (block) => !(block.status === "planned" && block.lifecycle_state !== "active"),
        ),
      ),
    [blocks],
  );

  const closeForm = () => {
    setShowCreate(false);
    setEditingTarget(null);
  };

  const replaceBlock = (updated: Block) => {
    onChanged(blocksRef.current.map((block) => (block.id === updated.id ? updated : block)));
  };

  // --- scheduled section loading -------------------------------------------

  const loadScheduledFirstPage = useCallback(async () => {
    const windowEnd = firstWindowEnd(date);
    setScheduledLoading(true);
    setScheduledError(null);
    try {
      const [rows, series] = await Promise.all([
        listUpcomingBlocks(workspaceId, addDays(date, 1), windowEnd, SCHEDULED_PAGE_SIZE, 0),
        listScheduledBlockRecurrences(workspaceId),
      ]);
      setScheduledRows(rows);
      setScheduledSeries(series);
      setScheduledOffset(rows.length);
      setScheduledWindowEnd(windowEnd);
      setScheduledHasMore(rows.length === SCHEDULED_PAGE_SIZE);
      setScheduledLoaded(true);
    } catch (err) {
      setScheduledError(toDisplayMessage(err));
    } finally {
      setScheduledLoading(false);
    }
  }, [workspaceId, date]);

  const loadMoreScheduled = useCallback(async () => {
    setScheduledLoading(true);
    setScheduledError(null);
    try {
      let windowEnd = scheduledWindowEnd;
      let rows = await listUpcomingBlocks(
        workspaceId,
        addDays(date, 1),
        windowEnd,
        SCHEDULED_PAGE_SIZE,
        scheduledOffset,
      );
      if (rows.length === 0) {
        windowEnd = addDays(windowEnd, SCHEDULED_WINDOW_DAYS);
        rows = await listUpcomingBlocks(
          workspaceId,
          addDays(date, 1),
          windowEnd,
          SCHEDULED_PAGE_SIZE,
          scheduledOffset,
        );
      }
      setScheduledRows((prev) => mergeRows(prev, rows));
      setScheduledOffset((prev) => prev + rows.length);
      setScheduledWindowEnd(windowEnd);
      setScheduledHasMore(rows.length === SCHEDULED_PAGE_SIZE);
    } catch (err) {
      setScheduledError(toDisplayMessage(err));
    } finally {
      setScheduledLoading(false);
    }
  }, [workspaceId, date, scheduledOffset, scheduledWindowEnd]);

  // --- inactive section loading --------------------------------------------

  const loadInactive = useCallback(async () => {
    setInactiveLoading(true);
    setInactiveError(null);
    try {
      const [rows, series] = await Promise.all([
        listInactiveBlocks(workspaceId),
        listInactiveBlockRecurrences(workspaceId),
      ]);
      setInactiveBlocks(rows);
      setInactiveSeries(series);
      setInactiveLoaded(true);
    } catch (err) {
      setInactiveError(toDisplayMessage(err));
    } finally {
      setInactiveLoading(false);
    }
  }, [workspaceId]);

  // --- lifecycle actions ----------------------------------------------------

  const handleDeactivateBlock = async (block: Block) => {
    const updated = await deactivateBlock(workspaceId, block.id);
    // The returned row carries the new lifecycle, so it no longer qualifies
    // for the active day list.
    replaceBlock(updated);
    setScheduledRows((prev) => prev.filter((row) => row.id !== updated.id));
    setInactiveBlocks((prev) => prev.map((row) => (row.id === updated.id ? updated : row)));
  };

  const handleDeleteBlock = async (block: Block) => {
    const updated = await deleteBlock(workspaceId, block.id);
    replaceBlock(updated);
    setScheduledRows((prev) => prev.filter((row) => row.id !== updated.id));
    setInactiveBlocks((prev) => prev.filter((row) => row.id !== updated.id));
  };

  const handleDeactivateSeries = async (target: Block | string) => {
    const recurrenceId = typeof target === "string" ? target : target.recurrence_id;
    if (!recurrenceId) return;
    const updated = await deactivateBlockRecurrence(workspaceId, recurrenceId);
    setInactiveSeries((prev) =>
      prev.some((row) => row.id === updated.id) ? prev : [...prev, updated],
    );
    setScheduledSeries((prev) => prev.filter((row) => row.recurrence.id !== recurrenceId));
    if (scheduledOpen) {
      await loadScheduledFirstPage();
    }
    onRefresh?.();
  };

  const handleDeleteSeries = async (target: Block | string) => {
    const recurrenceId = typeof target === "string" ? target : target.recurrence_id;
    if (!recurrenceId) return;
    await deleteBlockRecurrence(workspaceId, recurrenceId);
    setInactiveSeries((prev) => prev.filter((row) => row.id !== recurrenceId));
    setScheduledSeries((prev) => prev.filter((row) => row.recurrence.id !== recurrenceId));
    if (scheduledOpen) {
      await loadScheduledFirstPage();
    }
    onRefresh?.();
  };

  const handleDeleteInactiveBlock = async (block: Block) => {
    const updated = await deleteBlock(workspaceId, block.id);
    replaceBlock(updated);
    setInactiveBlocks((prev) => prev.filter((row) => row.id !== updated.id));
  };

  const handleScheduledBlockChanged = (updated: Block) => {
    setScheduledRows((prev) =>
      prev.map((row) => (row.id === updated.id ? updated : row)).filter(isUpcomingRow),
    );
  };

  const handleDeleteInactiveSeries = async (series: BlockRecurrence) => {
    await deleteBlockRecurrence(workspaceId, series.id);
    setInactiveSeries((prev) => prev.filter((row) => row.id !== series.id));
    if (scheduledOpen) {
      await loadScheduledFirstPage();
    }
    onRefresh?.();
  };

  /**
   * Run an inactive-section action. These rows are rendered by this
   * component (not by a card), so a failure is reported in the section's own
   * error area instead of a card alert.
   */
  const runInactiveAction = (action: () => Promise<void>) => {
    void (async () => {
      setInactiveError(null);
      try {
        await action();
      } catch (err) {
        setInactiveError(toDisplayMessage(err));
      }
    })();
  };

  const runScheduledAction = (action: () => Promise<void>) => {
    void (async () => {
      setScheduledError(null);
      try {
        await action();
      } catch (err) {
        setScheduledError(toDisplayMessage(err));
      }
    })();
  };

  /**
   * Reactivate an inactive weekly series. The native command takes no
   * weekday, so the user is guided to the rule editor instead: the notice
   * points at the Edit action of the series' next occurrence.
   */
  const handleReactivateSeries = async (series: BlockRecurrence) => {
    const updated = await reactivateBlockRecurrence(workspaceId, series.id);
    setInactiveSeries((prev) => prev.map((row) => (row.id === updated.id ? updated : row)));
    setReactivatedSeriesIds((prev) => (prev.includes(updated.id) ? prev : [...prev, updated.id]));
    if (scheduledOpen) {
      await loadScheduledFirstPage();
    }
    onRefresh?.();
  };

  /** A reactivated one-time block: close the form and refresh every list. */
  const handleReactivatedBlock = async () => {
    setInactiveNotice(t("today.blocks.inactive.reactivated"));
    await loadInactive();
    if (scheduledOpen) {
      await loadScheduledFirstPage();
    }
    onRefresh?.();
  };

  const beginPending = (pending: Block) => {
    if (pendingRef.current === null) {
      pendingRef.current = pending;
      originalRef.current = editingTarget;
    }
    setSaveInFlight(true);
    setRefreshError(null);
    onChanged(replaceOrAppend(blocksRef.current, pending));
  };

  const commitBlock = (pending: Block, committed: Block) => {
    // Rebuild from the latest visible rows: drop the provisional row (on
    // create it carries a temp id) and any stale version of the same id.
    const rest = blocksRef.current.filter(
      (block) => block.id !== pending.id && block.id !== committed.id,
    );
    onChanged(sortBlocks([...rest, committed]));
    pendingRef.current = null;
    originalRef.current = null;
    setSaveInFlight(false);
    closeForm();
  };

  // Scoped restore: only the row this save touched reverts; rows changed
  // by other actions while the save was pending stay committed.
  const restoreScoped = (pending: Block, original: Block | null) => {
    const next = original
      ? blocksRef.current.map((block) => (block.id === pending.id ? original : block))
      : blocksRef.current.filter((block) => block.id !== pending.id);
    onChanged(next);
  };

  const rollbackPending = () => {
    setSaveInFlight(false);
    const pending = pendingRef.current;
    if (pending === null) {
      return;
    }
    restoreScoped(pending, originalRef.current);
    pendingRef.current = null;
    originalRef.current = null;
  };

  // The rule save itself already persisted (the form awaited the native
  // command before calling this), so a refresh failure gets its own
  // translated notice instead of implying a write failure. The
  // provisional row must still never remain on screen as a live action
  // target: on refresh failure a new series' pending row is removed (it
  // carries a temp id and no recurrence identity, so Start/Edit would
  // address an id that was never persisted), and an edited occurrence
  // reverts to its original committed row while unrelated rows stay.
  // Card actions remain locked until the re-read settles. On success the
  // authoritative day rows commit sorted.
  const commitRecurrence = () => {
    void (async () => {
      const pending = pendingRef.current;
      const original = originalRef.current;
      pendingRef.current = null;
      originalRef.current = null;
      closeForm();
      try {
        const fresh = await listBlocksForDate(workspaceId, date);
        onChanged(sortBlocks(fresh));
        setRefreshError(null);
      } catch (err) {
        if (pending !== null) {
          restoreScoped(pending, original);
        }
        setRefreshError(toDisplayMessage(err));
      } finally {
        setSaveInFlight(false);
      }
      // A rule change can change the next eligible date, so refresh the series list.
      if (scheduledOpen) {
        await loadScheduledFirstPage();
      }
    })();
  };

  const formOpen = showCreate || editingTarget !== null;

  const toggleScheduled = () => {
    const next = !scheduledOpen;
    setScheduledOpen(next);
    if (next && !scheduledLoaded && !scheduledLoading) {
      void loadScheduledFirstPage();
    }
  };

  const toggleInactive = () => {
    const next = !inactiveOpen;
    setInactiveOpen(next);
    if (next && !inactiveLoaded && !inactiveLoading) {
      void loadInactive();
    }
  };

  const scheduledGroups = useMemo(() => groupByLocalDate(scheduledRows), [scheduledRows]);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-2">
        {visible.length === 0 ? (
          <p className="joi-empty__state">{t("today.block.empty")}</p>
        ) : (
          visible.map((block) => (
            <BlockCard
              key={block.id}
              block={block}
              onChanged={replaceBlock}
              actionDisabled={saveInFlight}
              onEdit={(target) => {
                setRefreshError(null);
                setShowCreate(false);
                setEditingTarget(target);
              }}
              onDeactivate={block.recurrence_id ? handleDeactivateSeries : handleDeactivateBlock}
              onDelete={block.recurrence_id ? handleDeleteSeries : handleDeleteBlock}
            />
          ))
        )}

        {refreshError ? (
          // Truthful feedback: the write persisted; only the re-read failed.
          <p role="alert" className="joi-text--danger text-sm">
            {t("today.block.refreshFailed")} {refreshError}
          </p>
        ) : null}

        {formOpen ? (
          <div className="joi-surface--base joi-border--default rounded-lg border p-3">
            <BlockForm
              key={editingTarget ? `edit-${editingTarget.id}` : "create"}
              workspaceId={workspaceId}
              date={date}
              dailyPlanId={dailyPlanId}
              defaultType={blocks.length === 0 ? "focus" : "rest"}
              editing={editingTarget}
              onPending={beginPending}
              onCommitted={commitBlock}
              onRecurrenceCommitted={commitRecurrence}
              onRollback={rollbackPending}
              onClose={closeForm}
            />
          </div>
        ) : (
          <div className="flex gap-1">
            <button
              type="button"
              onClick={() => {
                setRefreshError(null);
                setShowCreate(true);
              }}
              className="joi-btn joi-btn--secondary rounded-md border border-dashed px-3 py-1.5 text-xs font-medium"
            >
              {t("today.block.add")}
            </button>
          </div>
        )}
      </div>

      {/* One card represents each active recurring schedule. Occurrences are
          created only for the current and elapsed dates. */}
      <section
        aria-labelledby="today-scheduled-blocks-heading"
        className="joi-surface--base joi-border--default rounded-lg border p-3"
      >
        <h3 id="today-scheduled-blocks-heading" className="joi-text text-sm font-medium">
          <button
            type="button"
            aria-expanded={scheduledOpen}
            aria-controls="today-scheduled-blocks-panel"
            onClick={toggleScheduled}
            className="joi-btn joi-btn--secondary w-full rounded-md border px-2 py-1 text-xs font-medium"
          >
            {t("today.blocks.scheduled.title")}
          </button>
        </h3>

        {scheduledOpen ? (
          <div id="today-scheduled-blocks-panel" className="mt-3 flex flex-col gap-2">
            {scheduledError ? (
              <p role="alert" className="joi-text--danger text-sm">
                {t("today.blocks.scheduled.loadFailed")} {scheduledError}
              </p>
            ) : null}
            {scheduledLoading && scheduledRows.length === 0 && scheduledSeries.length === 0 ? (
              <p className="joi-empty__loading">{t("common.loading")}</p>
            ) : null}
            {!scheduledLoading &&
            scheduledError === null &&
            scheduledRows.length === 0 &&
            scheduledSeries.length === 0 ? (
              <p className="joi-empty__state">{t("today.blocks.scheduled.empty")}</p>
            ) : null}
            {scheduledGroups.map((group) => (
              <div key={group.date} data-local-date={group.date} className="flex flex-col gap-1">
                <h4 className="joi-text--muted text-xs font-medium">
                  {intlFormatDate(new Date(`${group.date}T12:00:00`), {
                    weekday: "long",
                    year: "numeric",
                    month: "long",
                    day: "numeric",
                  })}
                </h4>
                {group.rows.map((row) => (
                  <BlockCard
                    key={row.id}
                    block={row}
                    onChanged={handleScheduledBlockChanged}
                    actionDisabled={saveInFlight}
                    onEdit={setScheduledEditingBlock}
                    onDeactivate={handleDeactivateBlock}
                    onDelete={handleDeleteBlock}
                  />
                ))}
              </div>
            ))}
            {scheduledSeries.map(({ recurrence, next_occurrence_date }) => (
              <article
                key={recurrence.id}
                data-recurrence-id={recurrence.id}
                className="joi-surface--base joi-border--default flex min-w-0 flex-col gap-2 rounded-lg border px-3 py-2"
              >
                <div className="flex min-w-0 flex-col gap-2">
                  <div className="min-w-0 flex-1">
                    <div className="flex min-w-0 flex-wrap items-center gap-2">
                      <span className="joi-text min-w-0 break-words text-sm font-medium">
                        {recurrence.label ?? t(`today.block.types.${recurrence.block_type}`)}
                      </span>
                      <span className="joi-badge joi-badge--neutral shrink-0">
                        {t(`today.block.types.${recurrence.block_type}`)}
                      </span>
                      <span className="joi-badge joi-badge--info shrink-0">
                        {t("today.block.recurring")}
                      </span>
                    </div>
                    <p className="joi-text--muted mt-1 break-words text-xs">
                      {recurrence.local_start_time}
                      {` · ${t("today.block.durationMinutes", {
                        count: Math.round(recurrence.estimated_duration_seconds / 60),
                      })}`}
                    </p>
                    <p className="joi-text--muted mt-1 text-xs">
                      {t("today.blocks.scheduled.nextOccurrence", {
                        date: intlFormatDate(new Date(`${next_occurrence_date}T12:00:00`), {
                          weekday: "long",
                          year: "numeric",
                          month: "long",
                          day: "numeric",
                        }),
                      })}
                    </p>
                    <div className="mt-1 flex flex-wrap gap-1">
                      {[...recurrence.weekdays]
                        .sort((a, b) => a - b)
                        .map((iso) => (
                          <span key={iso} className="joi-badge joi-badge--info">
                            {t(`common.labels.weekday.${isoWeekdayKey(iso)}`)}
                          </span>
                        ))}
                    </div>
                    {recurrence.note_markdown ? (
                      <p className="joi-text mt-1 break-words text-xs">
                        {recurrence.note_markdown}
                      </p>
                    ) : null}
                  </div>
                  <div className="flex min-w-0 flex-wrap justify-start gap-1">
                    <button
                      type="button"
                      disabled={saveInFlight}
                      onClick={() => setScheduledEditing({ recurrence, next_occurrence_date })}
                      className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
                    >
                      {t("common.actions.edit")}
                    </button>
                    <button
                      type="button"
                      disabled={saveInFlight}
                      onClick={() =>
                        runScheduledAction(() => handleDeactivateSeries(recurrence.id))
                      }
                      className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
                    >
                      {t("today.block.deactivateRoutine")}
                    </button>
                    <button
                      type="button"
                      disabled={saveInFlight}
                      onClick={() => {
                        if (!window.confirm(t("today.blocks.scheduled.deleteConfirm"))) return;
                        runScheduledAction(() => handleDeleteSeries(recurrence.id));
                      }}
                      className="joi-btn joi-btn--danger rounded-md px-2 py-1 text-xs font-medium disabled:opacity-50"
                    >
                      {t("today.block.deleteRoutine")}
                    </button>
                  </div>
                </div>
              </article>
            ))}
            {scheduledEditingBlock ? (
              <div className="joi-surface--base joi-border--default rounded-lg border p-3">
                <BlockForm
                  key={`scheduled-block-edit-${scheduledEditingBlock.id}`}
                  workspaceId={workspaceId}
                  date={scheduledEditingBlock.date}
                  dailyPlanId={null}
                  defaultType={scheduledEditingBlock.block_type}
                  editing={scheduledEditingBlock}
                  onPending={() => {}}
                  onCommitted={(_pending, committed) => {
                    setScheduledEditingBlock(null);
                    handleScheduledBlockChanged(committed);
                    void loadScheduledFirstPage();
                  }}
                  onRecurrenceCommitted={() => {
                    setScheduledEditingBlock(null);
                    void loadScheduledFirstPage();
                  }}
                  onRollback={() => {}}
                  onClose={() => setScheduledEditingBlock(null)}
                />
              </div>
            ) : null}
            {scheduledEditing ? (
              <div className="joi-surface--base joi-border--default rounded-lg border p-3">
                <BlockForm
                  key={`scheduled-edit-${scheduledEditing.recurrence.id}`}
                  workspaceId={workspaceId}
                  date={scheduledEditing.next_occurrence_date}
                  dailyPlanId={null}
                  defaultType={scheduledEditing.recurrence.block_type}
                  editingRecurrenceId={scheduledEditing.recurrence.id}
                  onPending={() => {}}
                  onCommitted={() => setScheduledEditing(null)}
                  onRecurrenceCommitted={() => {
                    setScheduledEditing(null);
                    void loadScheduledFirstPage();
                    onRefresh?.();
                  }}
                  onRollback={() => {}}
                  onClose={() => setScheduledEditing(null)}
                />
              </div>
            ) : null}
            {scheduledHasMore ? (
              <button
                type="button"
                onClick={() => void loadMoreScheduled()}
                disabled={scheduledLoading}
                className="joi-btn joi-btn--secondary rounded-md border px-3 py-1.5 text-xs font-medium disabled:opacity-50"
              >
                {t("today.blocks.scheduled.loadMore")}
              </button>
            ) : null}
          </div>
        ) : null}
      </section>

      {/* Inactive blocks: the manually deactivated, completed, and deleted
          one-time blocks plus the inactive weekly series. Collapsed by
          default. */}
      <section
        aria-labelledby="today-inactive-blocks-heading"
        className="joi-surface--base joi-border--default rounded-lg border p-3"
      >
        <h3 id="today-inactive-blocks-heading" className="joi-text text-sm font-medium">
          <button
            type="button"
            aria-expanded={inactiveOpen}
            aria-controls="today-inactive-blocks-panel"
            onClick={toggleInactive}
            className="joi-btn joi-btn--secondary w-full rounded-md border px-2 py-1 text-xs font-medium"
          >
            {t("today.blocks.inactive.title")}
          </button>
        </h3>

        {inactiveOpen ? (
          <div id="today-inactive-blocks-panel" className="mt-3 flex flex-col gap-2">
            {inactiveError ? (
              <p role="alert" className="joi-text--danger text-sm">
                {t("today.blocks.inactive.loadFailed")} {inactiveError}
              </p>
            ) : null}
            {inactiveNotice ? (
              <p className="joi-text--success text-xs" role="status">
                {inactiveNotice}
              </p>
            ) : null}
            {inactiveLoading && inactiveBlocks.length === 0 && inactiveSeries.length === 0 ? (
              <p className="joi-empty__loading">{t("common.loading")}</p>
            ) : null}
            {!inactiveLoading &&
            inactiveError === null &&
            inactiveBlocks.length === 0 &&
            inactiveSeries.length === 0 ? (
              <p className="joi-empty__state">{t("today.blocks.inactive.empty")}</p>
            ) : null}

            {inactiveBlocks.map((row) => (
              <article
                key={row.id}
                className="joi-surface--base joi-border--default flex min-w-0 flex-col gap-2 rounded-lg border px-3 py-2"
              >
                <div className="flex min-w-0 flex-col gap-2">
                  <div className="min-w-0">
                    <div className="flex min-w-0 flex-wrap items-center gap-2">
                      <span className="joi-text min-w-0 break-words text-sm font-medium">
                        {row.label ?? t(`today.block.types.${row.block_type}`)}
                      </span>
                      <span className="joi-badge joi-badge--neutral shrink-0">
                        {t("today.blocks.inactive.oneTime")}
                      </span>
                      <span className="joi-badge joi-badge--neutral shrink-0">
                        {t(`today.block.statuses.${row.status}`)}
                      </span>
                    </div>
                    <p className="joi-text--muted mt-1 break-words text-xs">
                      {row.planned_start_at ?? row.local_start_time ?? t("today.block.noTime")}
                      {typeof row.estimated_duration_seconds === "number"
                        ? ` · ${t("today.block.durationMinutes", {
                            count: Math.round(row.estimated_duration_seconds / 60),
                          })}`
                        : ""}
                    </p>
                  </div>
                  <div className="flex min-w-0 flex-wrap justify-start gap-1">
                    <button
                      type="button"
                      onClick={() => setReactivateTarget(row)}
                      className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium"
                    >
                      {t("today.blocks.inactive.reactivate")}
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        // Deletion is soft but explicit: it always confirms
                        // first, matching the destructive-action convention.
                        if (!window.confirm(t("today.blocks.inactive.deleteConfirm"))) {
                          return;
                        }
                        runInactiveAction(() => handleDeleteInactiveBlock(row));
                      }}
                      className="joi-btn joi-btn--danger rounded-md px-2 py-1 text-xs font-medium"
                    >
                      {t("today.blocks.inactive.delete")}
                    </button>
                  </div>
                </div>
              </article>
            ))}

            {inactiveSeries.map((series) => (
              <article
                key={series.id}
                className="joi-surface--base joi-border--default flex min-w-0 flex-col gap-2 rounded-lg border px-3 py-2"
              >
                <div className="flex min-w-0 flex-col gap-2">
                  <div className="min-w-0">
                    <div className="flex min-w-0 flex-wrap items-center gap-2">
                      <span className="joi-text min-w-0 break-words text-sm font-medium">
                        {series.label ?? t(`today.block.types.${series.block_type}`)}
                      </span>
                      <span className="joi-badge joi-badge--neutral shrink-0">
                        {t("today.blocks.inactive.routine")}
                      </span>
                    </div>
                    <p className="joi-text--muted mt-1 break-words text-xs">
                      {series.local_start_time}
                      {` · ${t("today.block.durationMinutes", {
                        count: Math.round(series.estimated_duration_seconds / 60),
                      })}`}
                    </p>
                    <div className="mt-1 flex flex-wrap gap-1">
                      {[...series.weekdays]
                        .sort((a, b) => a - b)
                        .map((iso) => (
                          <span key={iso} className="joi-badge joi-badge--info">
                            {t(`common.labels.weekday.${isoWeekdayKey(iso)}`)}
                          </span>
                        ))}
                    </div>
                    {reactivatedSeriesIds.includes(series.id) ? (
                      <p className="joi-text--muted mt-1 text-xs">
                        {t("today.blocks.inactive.seriesReactivatedNotice")}
                      </p>
                    ) : null}
                  </div>
                  <div className="flex min-w-0 flex-wrap justify-start gap-1">
                    {reactivatedSeriesIds.includes(series.id) ? null : (
                      <button
                        type="button"
                        onClick={() => runInactiveAction(() => handleReactivateSeries(series))}
                        className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium"
                      >
                        {t("today.blocks.inactive.reactivate")}
                      </button>
                    )}
                    <button
                      type="button"
                      onClick={() => {
                        if (!window.confirm(t("today.blocks.inactive.deleteConfirm"))) {
                          return;
                        }
                        runInactiveAction(() => handleDeleteInactiveSeries(series));
                      }}
                      className="joi-btn joi-btn--danger rounded-md px-2 py-1 text-xs font-medium"
                    >
                      {t("today.blocks.inactive.delete")}
                    </button>
                  </div>
                </div>
              </article>
            ))}

            {reactivateTarget ? (
              <ReactivateScheduleForm
                key={reactivateTarget.id}
                workspaceId={workspaceId}
                block={reactivateTarget}
                onClose={() => setReactivateTarget(null)}
                onReactivated={() => void handleReactivatedBlock()}
              />
            ) : null}
          </div>
        ) : null}
      </section>
    </div>
  );
}

export default TodayBlockList;
