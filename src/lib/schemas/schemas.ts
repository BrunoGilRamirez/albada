import { z } from "zod";

const optionalMarkdown = z.string().trim().max(200_000).nullish();
const optionalDate = z
  .string()
  .regex(/^\d{4}-\d{2}-\d{2}$/)
  .nullish();

export const workspaceSchema = z.object({
  name: z.string().trim().min(1, "A name is required.").max(200),
});

export const yearThemeSchema = z.object({
  workspace_id: z.string().min(1),
  year: z.number().int().min(1900).max(9999),
  name: z.string().trim().min(1, "A name is required.").max(200),
  description_markdown: optionalMarkdown,
});

export const quarterSchema = z.object({
  year_theme_id: z.string().min(1),
  code: z.enum(["Q1", "Q2", "Q3", "Q4"]),
  start_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  end_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
});

export const goalSchema = z.object({
  workspace_id: z.string().min(1),
  year_theme_id: z.string().nullish(),
  title: z.string().trim().min(1, "A title is required.").max(500),
  description_markdown: optionalMarkdown,
  why_it_matters_markdown: optionalMarkdown,
  success_definition_markdown: optionalMarkdown,
});

export const projectSchema = z.object({
  workspace_id: z.string().min(1),
  goal_id: z.string().nullish(),
  title: z.string().trim().min(1, "A title is required.").max(500),
  description_markdown: optionalMarkdown,
  why_it_matters_markdown: optionalMarkdown,
  definition_of_done_markdown: optionalMarkdown,
});

export const taskSchema = z.object({
  workspace_id: z.string().min(1),
  project_id: z.string().nullish(),
  goal_id: z.string().nullish(),
  parent_task_id: z.string().nullish(),
  title: z.string().trim().min(1, "A title is required.").max(500),
  description_markdown: optionalMarkdown,
  notes_markdown: optionalMarkdown,
  priority: z.enum(["none", "low", "normal", "high", "critical"]),
  effort_size: z.enum(["trivial", "small", "medium", "large", "very_large"]).nullish(),
  target_date: optionalDate,
  scheduled_date: optionalDate,
});

export const childTaskSchema = taskSchema.omit({
  workspace_id: true,
  project_id: true,
  goal_id: true,
  parent_task_id: true,
});

export const updateTaskSchema = taskSchema.omit({
  workspace_id: true,
  project_id: true,
  goal_id: true,
  parent_task_id: true,
});

export const moveTaskSchema = z.object({
  project_id: z.string().nullish(),
  goal_id: z.string().nullish(),
  parent_task_id: z.string().nullish(),
});

export const transitionTaskStatusSchema = z.object({
  to_status: z.enum([
    "inbox",
    "planned",
    "in_progress",
    "blocked",
    "completed",
    "cancelled",
    "deferred",
  ]),
  reason_markdown: optionalMarkdown,
});

export const closeParentManuallySchema = z.object({
  closure_mode: z.enum([
    "manual_no_longer_applicable",
    "manual_solved_elsewhere",
    "manual_accepted_partial",
    "manual_other",
  ]),
  reason_markdown: z.string().trim().min(1, "A closure reason is required."),
});

export const addDependencySchema = z
  .object({
    blocked_task_id: z.string().min(1),
    prerequisite_task_id: z.string().min(1),
    note_markdown: optionalMarkdown,
  })
  .refine((value) => value.blocked_task_id !== value.prerequisite_task_id, {
    message: "A task cannot depend on itself.",
    path: ["prerequisite_task_id"],
  });

const blockStartTime = z.string().regex(/^([01]\d|2[0-3]):[0-5]\d$/);
const optionalBlockDuration = z.number().int().positive().nullish();
const blockFieldsSchema = z.object({
  date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  block_type: z.enum(["planning", "focus", "review", "journaling", "rest", "personal"]),
  label: z.string().nullish(),
  planned_start_at: blockStartTime.nullish(),
  estimated_duration_seconds: optionalBlockDuration,
  time_window_label: z.enum(["morning", "afternoon", "evening", "after_meal"]).nullish(),
  flexibility: z.enum(["fixed", "flexible", "approximate"]),
  note_markdown: optionalMarkdown,
  // Optional task reference (8.7.8). The native command rejects a reference to
  // a task outside the workspace; the UI only offers eligible tasks.
  task_id: z.string().nullish(),
});

export const createBlockSchema = blockFieldsSchema.extend({
  workspace_id: z.string().min(1),
  daily_plan_id: z.string().nullish(),
});

export const updateBlockSchema = blockFieldsSchema.extend({
  daily_plan_id: z.string().nullish(),
});

export const blockRecurrenceSchema = z
  .object({
    workspace_id: z.string().min(1),
    weekdays: z.array(z.number().int().min(1).max(7)).min(1),
    local_start_time: blockStartTime,
    estimated_duration_seconds: z.number().int().positive(),
    block_type: z.enum(["planning", "focus", "review", "journaling", "rest", "personal"]),
    label: z.string().nullish(),
    flexibility: z.enum(["fixed", "flexible", "approximate"]),
    note_markdown: optionalMarkdown,
    enabled: z.boolean(),
    effective_from: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
    effective_until: z
      .string()
      .regex(/^\d{4}-\d{2}-\d{2}$/)
      .nullish(),
    task_id: z.string().nullish(),
  })
  .refine((value) => new Set(value.weekdays).size === value.weekdays.length, {
    message: "Weekdays must be unique.",
    path: ["weekdays"],
  })
  .refine(
    (value) => value.effective_until == null || value.effective_until >= value.effective_from,
    { message: "The end date cannot be before the start date.", path: ["effective_until"] },
  );

/**
 * Reactivation scheduling input (8.7.9). Mirrors the native
 * `reactivate_block_cmd` DTO: an ISO weekday plus an explicit once/recurring
 * choice. A recurring reactivation additionally needs an exact local start
 * time and a strictly positive duration; the native command stays
 * authoritative, this schema is the pre-submit check.
 */
export const reactivateBlockSchema = z
  .object({
    schedule: z.enum(["once", "recurring"]),
    weekday: z.number().int().min(1).max(7),
    local_start_time: blockStartTime.nullish(),
    estimated_duration_seconds: optionalBlockDuration,
  })
  .superRefine((value, ctx) => {
    if (value.schedule !== "recurring") {
      return;
    }
    if (!value.local_start_time) {
      ctx.addIssue({
        code: "custom",
        path: ["local_start_time"],
        message: "A recurring reactivation requires an exact local start time.",
      });
    }
    if (value.estimated_duration_seconds == null) {
      ctx.addIssue({
        code: "custom",
        path: ["estimated_duration_seconds"],
        message: "A recurring reactivation requires a positive estimated duration.",
      });
    }
  });

const analyticsDate = z.string().regex(/^\d{4}-\d{2}-\d{2}$/);

/**
 * Read-only habit analytics request (8.7.12). The native command clamps the end
 * to the workspace-local today; this schema only rejects an inverted or malformed
 * range before the call.
 */
export const blockRecurrenceAnalyticsInputSchema = z
  .object({
    workspace_id: z.string().min(1),
    from_date: analyticsDate,
    to_date: analyticsDate,
  })
  .refine((value) => value.from_date <= value.to_date, {
    message: "The range start date cannot be after its end date.",
    path: ["to_date"],
  });

const analyticsCountsSchema = z.object({
  scheduled: z.number().int().nonnegative(),
  completed: z.number().int().nonnegative(),
  started: z.number().int().nonnegative(),
  skipped: z.number().int().nonnegative(),
  cancelled: z.number().int().nonnegative(),
  unrecorded: z.number().int().nonnegative(),
});

const analyticsRateSchema = z.object({
  denominator: z.number().int().positive(),
  completed: z.number().int().nonnegative(),
  percent: z.number().int().min(0).max(100),
});

/**
 * Response mirror of `RecurrenceAnalyticsResponse`. Nulls are explicit: a
 * series without a label, a coverage date, or a visible rate stays null.
 */
export const blockRecurrenceAnalyticsResponseSchema = z.object({
  effective_from: analyticsDate,
  effective_to: analyticsDate,
  series: z.array(
    z.object({
      series_id: z.string().min(1),
      display_label: z.string().nullable(),
      display_label_is_current_fallback: z.boolean(),
      coverage: z.object({
        start: analyticsDate.nullable(),
        intervals: z.array(
          z.object({
            from: analyticsDate,
            until: analyticsDate.nullable(),
          }),
        ),
      }),
      counts: analyticsCountsSchema,
      rate: analyticsRateSchema.nullable(),
      weekly: z.array(
        z.object({
          week_start: analyticsDate,
          ...analyticsCountsSchema.shape,
        }),
      ),
    }),
  ),
});

export type TaskFormValues = z.infer<typeof updateTaskSchema> & {
  project_id?: string | null;
  goal_id?: string | null;
};
