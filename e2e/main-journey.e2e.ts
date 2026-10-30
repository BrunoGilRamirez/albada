import { expect, test } from "@playwright/test";

/**
 * Phase 9.2 P1 — main user journey E2E (Playwright).
 *
 * Journey: open app -> plan day -> work block (create + start a synthetic
 * focus block through the Today "+ Add block" form) -> complete task ->
 * view progress (parent task detail subtree progress bar) -> write journal
 * entry.
 *
 * Runs against the Vite dev server (port 1420) with an in-page synthetic
 * Tauri IPC mock, so no Rust build or `tauri-driver` is needed. All data is
 * synthetic; nothing touches a real database or personal content.
 *
 * Setup the principal must run once:
 *   npm install                       # installs @playwright/test (devDependency)
 *   npx playwright install chromium   # downloads the browser binary
 *   npx playwright test               # or: npm run test:e2e
 *
 * To exercise the real Tauri webview instead of the mock, point the spec at a
 * `tauri dev` instance behind a WebDriver (tauri-driver); that path is not
 * covered here and remains the follow-up for native-shell E2E.
 */

/**
 * Installed before any app module runs. Mirrors the typed command names the
 * frontend API layer invokes and returns synthetic DTOs shaped like the Rust
 * responses. Unknown commands resolve to `[]` with a console warning so a
 * drifted command name fails visibly in the trace instead of silently.
 */
function installTauriIpcMock() {
  const workspace = {
    id: "ws-e2e",
    name: "E2E workspace",
    root_path: null,
    timezone: "UTC",
    locale: "en",
    default_privacy_scope: "private",
    current_year_theme_id: null,
    settings_json: null,
    created_at: "2026-10-01T08:00:00Z",
    updated_at: "2026-10-01T08:00:00Z",
  };

  const store = {
    plan: {
      id: "plan-e2e-1",
      workspace_id: "ws-e2e",
      date: "2026-10-07",
      status: "inbox",
      intention_markdown: null,
      capacity_note_markdown: null,
      review_markdown: null,
      planning_block_id: null,
      created_at: "2026-10-07T08:00:00Z",
      updated_at: "2026-10-07T08:00:00Z",
    } as Record<string, unknown>,
    tasks: [
      {
        id: "task-e2e-0",
        workspace_id: "ws-e2e",
        project_id: null,
        goal_id: null,
        parent_task_id: null,
        title: "Quarterly review umbrella",
        description_markdown: null,
        notes_markdown: null,
        blocking_note_markdown: null,
        status: "in_progress",
        priority: "normal",
        effort_size: null,
        effort_points: 0,
        target_date: null,
        scheduled_date: null,
        completed_at: null,
        cancelled_at: null,
        closure_mode: null,
        closure_reason_markdown: null,
        created_at: "2026-10-01T09:00:00Z",
        updated_at: "2026-10-01T09:00:00Z",
        archived_at: null,
      } as Record<string, unknown>,
      {
        id: "task-e2e-1",
        workspace_id: "ws-e2e",
        project_id: null,
        goal_id: null,
        parent_task_id: "task-e2e-0",
        title: "Prepare quarterly review",
        description_markdown: null,
        notes_markdown: null,
        blocking_note_markdown: null,
        status: "in_progress",
        priority: "high",
        effort_size: "medium",
        effort_points: 3,
        target_date: null,
        scheduled_date: "2026-10-07",
        completed_at: null,
        cancelled_at: null,
        closure_mode: null,
        closure_reason_markdown: null,
        created_at: "2026-10-01T09:00:00Z",
        updated_at: "2026-10-01T09:00:00Z",
        archived_at: null,
      } as Record<string, unknown>,
    ],
    planTasks: [] as Record<string, unknown>[],
    blocks: [] as Record<string, unknown>[],
    entries: [] as Record<string, unknown>[],
    counters: { planTask: 1, block: 1, entry: 1 },
  };

  const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;

  const handlers: Record<string, (args: Record<string, unknown>) => unknown> = {
    ensure_default_workspace: () => clone(workspace),
    get_or_create_daily_plan: () => clone(store.plan),
    list_day_tasks: () => clone(store.planTasks),
    list_tasks: () => clone(store.tasks),
    list_blocks_for_date: () => clone(store.blocks),
    list_at_risk_tasks: () => [],
    list_fixed_commitments: () => [],
    get_capacity_suggestion: () => null,
    why_is_blocked: () => [],
    list_all_prerequisites: () => [],
    get_task_deadline_status: () => ({
      status: "normal",
      at_risk_tasks: [],
      warning_window_days: 3,
    }),
    get_task_with_tree: (args) => {
      const task = store.tasks.find((item) => item.id === args.id) ?? store.tasks[0];
      const children = store.tasks
        .filter((item) => item.parent_task_id === task.id)
        .map((child) => ({ task: clone(child), children: [], blocked_by: [] }));
      return { task: clone(task), children, blocked_by: [] };
    },
    get_task_progress: () => ({
      percentage: 100,
      label: "complete",
      completed_points: 3,
      total_points: 3,
      contributors: [],
    }),
    update_daily_plan_intention: (args) => {
      const dto = (args.dto ?? {}) as Record<string, unknown>;
      store.plan.intention_markdown = dto.intention_markdown ?? null;
      store.plan.capacity_note_markdown = dto.capacity_note_markdown ?? null;
      return clone(store.plan);
    },
    select_task_for_day: (args) => {
      const dto = (args.dto ?? {}) as Record<string, unknown>;
      const row = {
        id: `dpt-e2e-${store.counters.planTask++}`,
        daily_plan_id: dto.daily_plan_id,
        task_id: dto.task_id,
        order_index: store.planTasks.length,
        selection_reason: dto.reason ?? "manual",
        planned_status: "selected",
        carryover_from_daily_plan_id: null,
        decision_note_markdown: null,
        created_at: "2026-10-07T08:10:00Z",
        updated_at: "2026-10-07T08:10:00Z",
      };
      store.planTasks.push(row);
      return clone(row);
    },
    create_block: (args) => {
      const dto = (args.dto ?? {}) as Record<string, unknown>;
      const block = {
        id: `block-e2e-${store.counters.block++}`,
        workspace_id: dto.workspace_id,
        daily_plan_id: dto.daily_plan_id ?? null,
        date: dto.date,
        block_type: dto.block_type,
        status: "planned",
        label: dto.label ?? null,
        planned_start_at: dto.planned_start_at ?? null,
        planned_end_at: dto.planned_end_at ?? null,
        time_window_label: dto.time_window_label ?? null,
        actual_start_at: null,
        actual_end_at: null,
        flexibility: dto.flexibility ?? "flexible",
        note_markdown: dto.note_markdown ?? null,
        created_at: "2026-10-07T08:10:00Z",
        updated_at: "2026-10-07T08:10:00Z",
      };
      store.blocks.push(block);
      return clone(block);
    },
    set_daily_plan_planning_block: (args) => {
      store.plan.planning_block_id = args.blockId ?? null;
      return clone(store.plan);
    },
    update_daily_plan_status: (args) => {
      store.plan.status = args.status ?? store.plan.status;
      return clone(store.plan);
    },
    start_block_cmd: (args) => {
      const block = store.blocks.find((item) => item.id === args.id);
      if (block) {
        block.status = "active";
        block.actual_start_at = "2026-10-07T09:00:00Z";
      }
      return clone(block ?? null);
    },
    complete_task_cmd: (args) => {
      const task = store.tasks.find((item) => item.id === args.id);
      if (task) {
        task.status = "completed";
        task.completed_at = "2026-10-07T09:30:00Z";
      }
      return clone(task ?? null);
    },
    export_template_catalog: () => [],
    // Mirrors the Phase 8.6 JournalEntry DTO (src/lib/types.ts) and the Rust
    // `create_journal_entry` defaults (src-tauri/src/commands/journal.rs):
    // no `privacy_scope`/`retroactive`; template pair + structured fields are
    // separate from `content_markdown`; new entries default to export-eligible.
    create_journal_entry: (args) => {
      const dto = (args.dto ?? {}) as Record<string, unknown>;
      const entry = {
        id: `entry-e2e-${store.counters.entry++}`,
        workspace_id: dto.workspace_id,
        occurred_on: dto.occurred_on,
        title: dto.title,
        content_markdown: dto.content_markdown,
        template_id: dto.template_id ?? null,
        template_version: dto.template_version ?? null,
        template_values: dto.template_values ?? {},
        mood_label: dto.mood_label ?? null,
        energy_label: dto.energy_label ?? null,
        emotions: dto.emotions ?? [],
        export_eligible: dto.export_eligible ?? true,
        source: dto.source ?? "user",
        created_at: "2026-10-07T20:00:00Z",
        updated_at: "2026-10-07T20:00:00Z",
        archived_at: null,
      };
      store.entries.push(entry);
      return clone(entry);
    },
    get_journal_entry: (args) => {
      const entry = store.entries.find((item) => item.id === args.id);
      return clone(entry ?? null);
    },
    list_journal_entries: () => clone(store.entries),
    list_references_for_entry: () => [],
    get_due_notifications: () => [],
    record_notification_delivery: () => null,
    report_frontend_startup: () => null,
    report_frontend_error: () => null,
  };

  const internals = {
    invoke: (cmd: string, args?: Record<string, unknown>) => {
      const handler = handlers[cmd];
      if (!handler) {
        console.warn(`[e2e-tauri-mock] unhandled command: ${cmd}`);
        return Promise.resolve([]);
      }
      return Promise.resolve(handler(args ?? {}));
    },
  };

  Object.defineProperty(window, "__TAURI_INTERNALS__", { value: internals });
}

test("main journey: plan day -> work block -> complete task -> view progress -> journal", async ({
  page,
}) => {
  await page.addInitScript(installTauriIpcMock);

  // 1. Open app on the executive dashboard.
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Today" })).toBeVisible();

  // 2. Plan the day through the planning ritual.
  await page.getByRole("button", { name: "Planning ritual" }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog.getByRole("heading", { name: /Planning ritual/ })).toBeVisible();
  await dialog.getByRole("checkbox", { name: /Prepare quarterly review/ }).check();
  await dialog.getByRole("button", { name: "Continue" }).click();
  await expect(dialog.getByText("Nothing is blocked right now.")).toBeVisible();
  await dialog.getByRole("button", { name: "Continue" }).click();
  await expect(dialog.getByText("Manual")).toBeVisible();
  await dialog.getByRole("button", { name: "Continue" }).click();
  await dialog.getByLabel("Intention note").fill("Finish the review prep today.");
  await dialog.getByRole("button", { name: "Finish planning" }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByLabel("Plan status")).toHaveValue("planned");

  // 3. Work block: create a synthetic focus block through the Today UI's
  // "+ Add block" form, then start it. Creating a block does not refetch the
  // block list (current product behavior), so a sidebar round-trip back to
  // Today remounts the page and surfaces the new card before starting it.
  await page.getByRole("button", { name: "+ Add block" }).click();
  await page.getByLabel("Type").selectOption("Focus");
  await page.getByLabel("Label").fill("Deep work");
  await page.getByRole("button", { name: "Add block" }).click();

  await page
    .getByRole("navigation", { name: "Primary navigation" })
    .getByRole("link", { name: "Journal" })
    .click();
  await expect(page.getByRole("heading", { name: "Journal" })).toBeVisible();
  await page
    .getByRole("navigation", { name: "Primary navigation" })
    .getByRole("link", { name: "Today" })
    .click();
  await expect(page.getByRole("heading", { name: "Today" })).toBeVisible();

  const blockCard = page.locator("article", { hasText: "Deep work" });
  await expect(blockCard).toBeVisible();
  await blockCard.getByRole("button", { name: "Start" }).click();
  await expect(blockCard.getByText("Active")).toBeVisible();

  // 4. Complete the planned task from the Today list.
  const taskRow = page.locator('[data-notification-target="task-e2e-1"]');
  await taskRow.getByRole("button", { name: "Complete" }).click();
  // Exact match: the row also contains a Complete/Defer/Deselect/Replan/Session
  // action-controls div, so substring "Completed" would resolve ambiguously.
  await expect(taskRow.getByText("Completed", { exact: true })).toBeVisible();

  // 5. View progress: open the completed task, then navigate up the
  // breadcrumb to its parent task, whose detail page renders the derived
  // subtree progress bar fed by `get_task_progress`.
  await taskRow.getByRole("link", { name: "Prepare quarterly review" }).click();
  await expect(page.getByRole("heading", { name: "Prepare quarterly review" })).toBeVisible();
  await page
    .getByRole("navigation", { name: "Breadcrumb" })
    .getByRole("link", { name: "Quarterly review umbrella" })
    .click();
  await expect(page.getByRole("heading", { name: "Quarterly review umbrella" })).toBeVisible();
  const progress = page.getByRole("progressbar");
  await expect(progress).toBeVisible();
  await expect(progress).toHaveAttribute("aria-valuenow", "100");
  await expect(progress).toContainText("100.0%");

  // 6. Write a journal entry (SPA navigation through the sidebar; the dev
  // server keeps `base: "./"`, so deep links are avoided on purpose).
  await page
    .getByRole("navigation", { name: "Primary navigation" })
    .getByRole("link", { name: "Journal" })
    .click();
  await expect(page.getByRole("heading", { name: "Journal" })).toBeVisible();
  await page.getByRole("link", { name: "New entry" }).click();
  await page.getByLabel("Title").fill("A quiet day");
  await page.getByLabel("Date").fill("2026-10-07");
  // Role filter avoids the getByLabel ambiguity: the Markdown preview region
  // is named "Content (Markdown) preview", so substring label matching would
  // match both the editor textbox and the preview region (strict-mode fail).
  await page.getByRole("textbox", { name: "Content (Markdown)" }).fill("Finished the review prep.");
  await page.getByRole("button", { name: "Create entry" }).click();
  await expect(page.getByRole("heading", { name: "A quiet day" })).toBeVisible();

  // Phase 8.6 response-shape check: the mock returns the current JournalEntry
  // DTO (no `privacy_scope`/`retroactive`) and new entries default to
  // `export_eligible: true`, which the detail page renders as the
  // "Exportable" badge next to the title.
  await expect(page.getByText("Exportable", { exact: true })).toBeVisible();
});
