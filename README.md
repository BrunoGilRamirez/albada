# Albada

Albada is a local-first personal planning and journaling desktop application for
Windows. It turns meaningful goals into manageable actions without becoming a
Jira clone or a rigid hourly calendar. All data stays on your machine: no
accounts, no synchronization, no telemetry, and no network services.

> **Status: early alpha.** Albada 0.1.0 is a single-user desktop MVP. The
> application is usable day to day, but alpha builds are distributed on a
> best-effort basis with no support guarantee. See [SECURITY.md](./SECURITY.md)
> for privacy, data-ownership, and alpha-expectation details.

## What It Does

- **Execution.** Goals, projects, and tasks with parent-child trees. Tasks can
  also be standalone — `Go grocery shopping` does not need a parent project.
- **Dependencies.** Blocked tasks carry an explicit reason, and the app rejects
  self-dependencies and dependency cycles.
- **Explainable progress.** Progress is derived from eligible leaf tasks using
  effort points (1, 2, 3, 5, 8). Every percentage can be inspected down to its
  contributing tasks.
- **Deadline intelligence.** Deadlines produce warnings, not shame: `normal`,
  `approaching`, `overdue`, and `blocked_risk` are derived states with a
  configurable warning window and a Today-dashboard widget.
- **Daily planning.** A per-day plan with task selection, reordering, and
  explicit replan actions (move to next day, return to planned, remove date,
  defer, divide). Missed work is never silently rescheduled.
- **Flexible blocks.** Work blocks that do not require exact times, alongside
  fixed commitments, availability profiles, and a four-step planning ritual.
  Rest is a valid part of the plan and never counts as progress.
- **Sessions.** Track working time on a task. A session never completes a task
  by itself.
- **Journaling.** Independent journal entries with structured metadata and
  Markdown content, explicitly referenceable to tasks.
- **Therapy boundary.** A separate, private collection with its own records,
  references, privacy guards, and export scope. Therapy content never appears in
  general views, notifications, or generic exports.
- **Local notifications.** Windows notifications with consent, quiet hours,
  snooze, and lock-screen-safe messages that never leak task titles or personal
  text.
- **Import and export.** Read-only import from legacy Markdown folders with a
  reviewable report and provenance, plus selective Markdown export with explicit
  item selection and preview. A Typst/PDF export path renders locally with an
  embedded compiler (release verification for this path is ongoing during the
  alpha).
- **Backup and restore.** Explicit local backup and restore flows; you own and
  control your database file.

## How It Is Built

Albada is a Tauri 2 desktop application. The data path is:

```text
React UI -> typed Tauri command boundary -> Rust domain and repositories -> SQLite
```

- React 19, TypeScript, and Vite 6 provide the UI.
- Rust owns SQLite, the versioned migrations, and all domain rules.
- The frontend calls only typed command wrappers; no raw SQL or filesystem
  paths live in React components.
- UI strings ship in English, Spanish, German, and French catalogs with parity
  tests.

See [`docs/architecture.md`](./docs/architecture.md) for the data-boundary
overview and [`docs/setup.md`](./docs/setup.md) for build instructions.

## Repository Layout

| Path          | Contents                                                            |
| ------------- | ------------------------------------------------------------------- |
| `src/`        | React UI: routes, components, typed API wrappers, i18n, frontend tests |
| `src-tauri/`  | Rust/Tauri shell, commands, domain logic, repositories, migrations, export pipeline, tests, synthetic fixtures |
| `e2e/`        | Playwright end-to-end journey against a synthetic IPC mock          |
| `docs/`       | Public product documentation (architecture, setup)                  |
| `public/`     | Static frontend assets                                              |

## Quick Start

```sh
npm install
npm run tauri dev
```

Build the Windows installer with:

```sh
npm run tauri build
```

Full prerequisites and quality-check commands are in [`docs/setup.md`](./docs/setup.md).

## Contributing and Support

Issues are enabled for reproducible bug reports, and pull requests are welcome
as a voluntary contribution path. GitHub Discussions is disabled during the
alpha. Contributions are reviewed best-effort with no SLA; please read
[`CONTRIBUTING.md`](./CONTRIBUTING.md) before opening a pull request, and never
attach databases, backups, journal entries, therapy content, or other personal
records to an issue.

## Design Principles

- Show the next useful action instead of every item.
- Treat rest and flexibility as valid parts of planning.
- Keep progress explainable and derived, never double-counted.
- Avoid automatic rescheduling; every replan is an explicit decision.
- Keep journaling independent from task completion.
- Keep therapy data private and separately scoped.
- Preserve user-authored text and its original language.

## License

Albada source code is licensed under the GNU Affero General Public License,
version 3.0 or later (SPDX: `AGPL-3.0-or-later`). Copyright © 2026 Bruno Gil
Ramírez. The full license text is in [`LICENSE`](./LICENSE). The project
attribution record is in [`NOTICE`](./NOTICE), and the third-party component
inventory is in
[`THIRD-PARTY-NOTICES.md`](./THIRD-PARTY-NOTICES.md).

## Trademark

The AGPL does not grant trademark rights in the Albada name or logo. If you
fork or redistribute a modified version, you must use a different name and
logo.

## Disclaimer

Albada is an early-alpha personal planning and journaling tool, provided "as
is" with no warranty, to the extent permitted by law (see the warranty and
liability sections of `LICENSE`). It is not a medical, psychological, or
clinical records system, and it must not be used as one. Do not store
third-party patient data in Albada; keep it to your own personal records.
Optional integrations with external platforms are not part of this alpha.
