import { getImportBatchStatus } from "./importApi";
import { cancelExportJob, runExportJob } from "./exportApi";
import type { ExportJob } from "../types";
import { toDisplayMessage } from "../errors";

/** Polling interval in milliseconds when waiting for an import batch. */
const POLL_INTERVAL_MS = 1000;

export type JobStatus = "queued" | "running" | "completed" | "failed" | "cancelled";

/** Terminal statuses after which polling stops. */
const TERMINAL_STATUSES: ReadonlySet<string> = new Set([
  "completed",
  "completed_with_warnings",
  "failed",
  "cancelled",
]);

/**
 * Kind-agnostic job progress snapshot for the shared progress modal.
 * Contains IDs and safe display strings only — never source text,
 * journal/therapy content, absolute paths, or secrets.
 */
export interface JobProgressDto {
  /** Human-readable current phase/step (fixed template string) */
  phase: string | null;
  /** Progress percentage 0–100, or null when indeterminate */
  percent: number | null;
  /** Overall job status */
  status: JobStatus | null;
  /** True when the job has reached a terminal state */
  terminal: boolean;
  /** Safe error message when status is "failed" */
  error: string | null;
  /** Recovery action label, e.g. "View report" */
  recoveryLabel: string | null;
}

function failedProgress(message: string): JobProgressDto {
  return {
    phase: "Failed",
    percent: null,
    status: "failed",
    terminal: true,
    error: message,
    recoveryLabel: "View report",
  };
}

/**
 * Starts polling an import batch by ID. Uses the real
 * `get_import_batch_status` command. Stops at terminal states.
 *
 * @param batchId - The import batch ID.
 * @param updater - Called on each poll cycle with the latest snapshot.
 * @returns Cleanup function that stops polling. Call on component unmount.
 */
export function startImportPoll(
  batchId: string,
  updater: (state: JobProgressDto) => void,
): () => void {
  let cancelled = false;

  const poll = async () => {
    try {
      const result = await getImportBatchStatus(batchId);
      if (cancelled) return;
      const terminal = TERMINAL_STATUSES.has(result.status);
      const status: JobStatus =
        result.status === "failed"
          ? "failed"
          : result.status === "cancelled"
            ? "cancelled"
            : terminal
              ? "completed"
              : "running";
      updater({
        phase: terminal ? "Finished" : "Importing…",
        percent: null,
        status,
        terminal,
        error: null,
        recoveryLabel: terminal ? "View report" : null,
      });
      if (terminal) stop();
    } catch {
      if (!cancelled) {
        updater(failedProgress("Could not read the import status."));
        stop();
      }
    }
  };

  const stop = () => {
    cancelled = true;
    clearInterval(interval);
  };

  void poll();
  const interval = setInterval(poll, POLL_INTERVAL_MS);
  return stop;
}

/**
 * Runs an export job to its terminal state. `run_export_job` executes the
 * full pipeline (snapshot → convert → compile → persist) inside a controlled
 * worker and returns the terminal job, so progress is indeterminate while
 * the call is in flight. Cancellation is cooperative via
 * `cancel_export_job`; the worker observes it at phase boundaries.
 */
export async function runExportWithProgress(
  jobId: string,
  updater: (state: JobProgressDto) => void,
): Promise<ExportJob> {
  updater({
    phase: "Running…",
    percent: null,
    status: "running",
    terminal: false,
    error: null,
    recoveryLabel: null,
  });
  try {
    const job = await runExportJob(jobId);
    const terminal = TERMINAL_STATUSES.has(job.status);
    updater({
      phase: "Finished",
      percent: terminal ? 100 : null,
      status:
        job.status === "failed"
          ? "failed"
          : job.status === "cancelled"
            ? "cancelled"
            : terminal
              ? "completed"
              : "running",
      terminal,
      error: job.error_message ?? null,
      recoveryLabel: terminal ? "View report" : null,
    });
    return job;
  } catch (error) {
    // Keep the native command error intact. The previous implementation
    // replaced every failure with a generic message, which hid useful Rust
    // errors such as a missing template, stale preview, or persistence
    // failure from both the progress modal and the export page.
    const message = toDisplayMessage(error);
    const lost: JobProgressDto = failedProgress(message);
    updater(lost);
    throw error;
  }
}

/** Requests cooperative cancellation of a running export job. */
export function requestExportCancel(jobId: string): Promise<ExportJob> {
  return cancelExportJob(jobId);
}

/**
 * Returns true if the given status is terminal (polling may stop).
 */
export function isTerminalStatus(status: string | null): boolean {
  return TERMINAL_STATUSES.has(status ?? "");
}

export type { ExportJob };
