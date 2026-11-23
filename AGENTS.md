# Albada Agent and Contributor Contract

This document is the public, tool-agnostic contract for working on Albada. It
applies to human contributors and to any coding agent, including Codex,
Claude, OpenCode, Copilot, local models, or custom tools.

The creator's optional AI-assisted development environment is documented at
the [Albada Agent Environment](https://github.com/BrunoGilRamirez/albada-agent-environment)
repository. Its models, agents, routing, and skills are not required for
contribution.

## Project Boundary

- Albada is a Windows desktop, local-first application using Tauri 2, React,
  TypeScript, Vite, Rust, and SQLite.
- The data path is `React UI -> typed Tauri command boundary -> SQLite`.
- Tauri/Rust owns SQLite. Python may consume explicit Markdown exports but must
  not independently write SQLite.
- Mobile, FastAPI, remote access, synchronization, WebSockets, external
  calendars, WhatsApp, LLM integrations, multi-user concepts, and Jira-style
  workflows are outside the MVP unless the project scope explicitly changes.
- Keep raw SQL, filesystem paths, and domain rules out of React components.

## Domain Invariants

- Keep execution, time, journaling, therapy, and exports as separate concepts.
- Tasks may be standalone, have at most one primary project owner and one
  parent, and must reject self-dependencies and parent/dependency cycles.
- Progress is derived from eligible leaf tasks using effort points `1, 2, 3,
  5, 8`; parents are never double-counted.
- `scheduled_date` is the intended work day. `deadline_at` is an optional
  limit. Neither implies an exact time, and overdue state is derived.
- Sessions do not complete tasks. Missed work is not silently copied or
  rescheduled. Closing a parent with open children requires an explicit reason.
- Flexible blocks, fixed commitments, availability, and rest remain distinct;
  rest does not affect progress.

## Privacy and Content

- Keep data local by default. Do not add telemetry or external transmission
  without explicit approval.
- Therapy is a separate private collection and must not appear in general
  views, notifications, or generic project exports.
- Journal and therapy exports require explicit selection and preview.
- Preserve Markdown source and user-authored language; do not silently rewrite
  content.
- Never commit personal records, therapy content, imported legacy content,
  databases, backups, credentials, logs, or machine-specific secrets.

## Legacy Data

- Treat Proyecto Vida source files as read-only during import.
- Preserve original paths, language, provenance, and reviewable warnings.
- Do not interpret every Markdown checkbox as an executable task.

## Development Rules

- Read `README.md`, `docs/architecture.md`, `docs/setup.md`, and the relevant
  domain documentation before changing scope or persistence behavior.
- Before a data-model change, update the relevant model documentation and use a
  versioned migration under `src-tauri/migrations/`.
- Put validation and transactions below the native command boundary.
- Add domain/repository tests for relationship integrity, leaf progress,
  deadlines versus status, independent tasks, journal/therapy isolation, and
  import/export selection when those systems are touched.
- Run the applicable typecheck, lint, frontend tests, formatting checks, Rust
  checks, and build before reporting completion. Report unavailable commands
  honestly.
- Do not modify personal records or commit changes unless explicitly requested.

## Recommended Agent Skills

The creator's current environment uses skills for React best practices,
composition patterns, Tailwind CSS, advanced TypeScript, Vite, Vitest, Tauri 2,
Zod, and minimal-diff review. These are recommendations, not requirements.
Equivalent tools or no AI tooling are valid.

## Language

Code, project documentation, issue descriptions, architecture decisions, test
names, and agent communication are written in English. User-authored personal,
journal, therapy, and imported legacy content remains in its original language.
