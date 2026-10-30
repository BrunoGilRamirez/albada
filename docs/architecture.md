# Architecture and Data Boundaries

This is a concise overview of how Albada is layered and which boundaries the
code preserves. It describes the implemented MVP, not planned features.

## Stack

- **Shell:** Tauri 2 (Windows desktop MVP).
- **Frontend:** React 19 + TypeScript + Vite 6 + React Router, Tailwind CSS
  with a shared design-system layer.
- **Backend:** Rust commands, domain modules, and repositories.
- **Storage:** SQLite via `rusqlite` (bundled), owned entirely by the native
  layer.

## The One Data Path

```text
React UI -> typed Tauri command boundary -> Rust domain / repositories -> SQLite
```

- Route pages live in `src/app/`, reusable UI in `src/components/`.
- `src/lib/api/` contains one typed wrapper per command group; every call crosses
  into Rust through Tauri's invoke boundary.
- `src-tauri/src/commands/` is the native boundary: commands stay thin and
  delegate to `domain/` (rules) and `repos/` (persistence).
- Raw SQL, filesystem paths, and domain rules never live in React components.
  Validation and transactions sit below the command boundary.
- There is exactly one database owner per deployment: the Rust/Tauri layer.
  External tools may read explicit Markdown exports but must never write the
  SQLite database.

## Storage and Migrations

- The database is `albada.sqlite` inside the Tauri app-data directory (WAL
  mode, foreign keys enabled).
- The public schema lineage starts from one canonical baseline,
  `src-tauri/migrations/001_initial_schema.sql`, which creates the complete
  final 0.1.0 schema in a fresh, empty database. The pre-release migration
  history (originals `001–042`) is kept under `src-tauri/migrations/archive/`
  for reference only: archived files are never embedded, applied, or rewritten
  at runtime.
- The runner applies the versioned SQL files directly under
  `src-tauri/migrations/` in strictly increasing, contiguous order and records
  each applied version in `_migrations`. The SQL is embedded into the compiled
  binary at build time. Follow-on public migrations therefore begin at
  `002_*.sql`.
- Policy: after public release the sequence is forward-only. A migration that
  has been applied to a public database is never rewritten; schema changes are
  new numbered files only.
- There is no upgrade bridge from private development or pre-alpha databases
  into this lineage; a public installation starts from a clean database (see
  `SECURITY.md`). Historical backfills, migration hooks, and repair passes
  from the pre-release history are intentionally not runtime upgrade support
  here.
- Seeding stays application behavior, not migration backfill: the default
  workspace and lock-screen-safe reminder rules are created lazily through
  normal app paths, and block-recurrence lifecycle intervals are written
  explicitly through app operations.
- On first start the app creates the database, applies the baseline schema,
  and leaves safe defaults to be seeded on first use.

## Domain Boundaries

Execution, time, experience, therapy, and export remain separate concepts:

| Area       | Entities                                                              |
| ---------- | --------------------------------------------------------------------- |
| Execution  | Goals, projects, tasks (trees, dependencies, statuses, progress)      |
| Time       | Daily plans, flexible blocks, work sessions, fixed commitments, availability |
| Experience | Journal entries with structured metadata and task references          |
| Therapy    | Separate collections and records with their own references and export scope |
| Export     | Export jobs and artifacts: Markdown, Typst/PDF (embedded compiler)    |

Key invariants the layering enforces:

- A task is standalone or has at most one primary project owner and one parent.
  Parent-child membership is distinct from dependencies. Self-dependencies and
  cycles are rejected.
- Progress is derived from eligible leaf tasks using effort points
  (`1, 2, 3, 5, 8`); parents are never double-counted, and priority or deadline
  urgency is never a progress weight. Contributors to any percentage are
  inspectable in the UI.
- `scheduled_date` is the intended work day; `deadline_at` is an optional limit.
  Neither implies an exact time, and `overdue` is derived, never a stored
  status.
- A session does not complete a task. Missed work is never silently copied or
  rescheduled; every replan decision is recorded on the plan task. Closing a
  parent with open children requires an explicit reason.
- Flexible blocks, fixed commitments, availability, and rest stay
  distinguishable; rest does not affect progress.

## Privacy Boundaries

- Journaling and therapy are separate collections. Therapy text never appears
  in general views, notifications, or generic project exports.
- Journal and therapy exports require an explicit item selection and a preview;
  the export worker keeps the generated Markdown source and never reports a
  partial PDF as successful.
- Notification messages are lock-screen safe: generic, configurable texts that
  never embed task titles, journal excerpts, or therapy content.
- Everything is local. There is no telemetry, no remote transmission, and no
  background network service.

## Import and Export

- Legacy import is read-only: it discovers a Markdown folder, classifies
  candidates, produces a reviewable report with provenance and warnings, and
  commits records only after explicit user confirmation. Uncertain
  classifications stay flagged for review; the app never treats every Markdown
  checkbox as an executable task.
- The embedded Typst renderer produces PDFs without Python, LaTeX, or a `PATH`
  lookup. This path ships in alpha while its release verification (packaging,
  performance, third-party notices) continues.
- Long-running import/export work runs off the UI thread with progress and
  cooperative cancellation.

## Frontend Conventions

- All UI strings live in the four bundled locale catalogs
  (`src/lib/i18n/locales/`); English, Spanish, German, and French parity is
  enforced by tests.
- State and side effects follow the established hooks and query patterns in
  `src/lib/`; API responses are validated with Zod schemas in `src/lib/schemas/`
  at the wrapper boundary.

## Testing Strategy

- Frontend unit/component tests: Vitest + jsdom + React Testing Library
  (`src/**/*.test.tsx`).
- Rust unit tests live beside their modules; integration tests under
  `src-tauri/tests/` exercise the same repository paths the commands use.
- All Rust tests run against disposable in-memory SQLite databases; the legacy
  import fixtures under `src-tauri/fixtures/legacy/` are entirely synthetic.
- The Playwright E2E journey runs the real frontend against an in-page
  synthetic Tauri IPC mock, so it touches no database.
