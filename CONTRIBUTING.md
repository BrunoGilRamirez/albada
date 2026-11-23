# Contributing to Albada

Thanks for your interest. Albada is in an **early alpha** stage, so this
is a deliberately minimal contribution path:

- **Issues are enabled** for reproducible bug reports and small, concrete
  improvement proposals.
- **Pull requests are welcome** for clearly scoped fixes. A signed
  Contributor License Agreement is required before any pull request can be
  merged (see [Contributor License Agreement](#contributor-license-agreement)).
  Please open an issue first for anything substantial; a PR is not a shortcut
  around a design discussion.
- **GitHub Discussions is disabled** during the alpha.
- Review is best-effort with **no service-level guarantee**. The maintainer may
  close or decline contributions that conflict with the product direction
  (local-first, single-user, no cloud features).

**Never attach private data to an issue or pull request.** This includes real
databases, backups, journal entries, therapy content, exported personal
records, or screenshots containing them. Reproduce problems with synthetic
data. See [SECURITY.md](./SECURITY.md).

## AI-Assisted Contributions

Albada does not require OpenCode, Codex, Claude, Copilot, a specific model, or
any AI tool. Contributors may work manually or use their own workflow.

If you use an agent, follow the public [`AGENTS.md`](./AGENTS.md) contract.
The creator's optional and auditable development environment is documented in
the [Albada Agent Environment](https://github.com/BrunoGilRamirez/albada-agent-environment)
repository.

## Ground Rules

- Project code, documentation, commit messages, issue/PR text, and test names
  are written in **English**. User-authored personal content keeps its original
  language wherever it is stored or exported.
- Keep the app local-first. Do not propose telemetry, remote services,
  synchronization, accounts, LLM integrations, or mobile features; these are
  outside the MVP and are not scheduled.
- Respect the domain invariants (see `docs/architecture.md`). In short: tasks
  have at most one parent and one primary project; dependencies are distinct
  from hierarchy; self-dependencies and cycles are rejected; progress derives
  from eligible leaf tasks using effort points; `overdue` is derived, not
  stored; a session never completes a task; missed work is never silently
  rescheduled; rest does not affect progress.

## Contributor License Agreement

- A **CLA will be required before any pull request can be merged**. You retain the
  copyright in your contributions; the provisional [`CLA.md`](./CLA.md) grants
  the individual maintainer the rights needed to distribute and maintain the
  project.
- **Provisional status.** The agreement is operational through CLA Assistant,
  but it is intentionally subject to later professional legal review. A future
  version may require contributors to accept updated terms.
- **Base templates.** The project CLA will be based on the official Project
  Harmony templates: the **Harmony Individual Contributor License Agreement
  v1.0** and the **Harmony Entity Contributor License Agreement v1.0** (the
  corporate/entity form). Official template index:
  <https://www.harmonyagreements.org/agreements>. Individual template (PDF):
  <https://www.harmonyagreements.org/docs/ha-cla-i-v1.pdf>. Entity/corporate
  template (PDF): <https://www.harmonyagreements.org/docs/ha-cla-e-v1.pdf>.
  The maintainer and rights recipient named in the project CLA is
  **Bruno Gil Ramírez**, an individual. Albada is the project name, not a legal
  entity.
- **Status.** The former planning draft is retained in
  [`CLA-DRAFT.md`](./CLA-DRAFT.md); the provisional agreement is effective only
  when accepted through the signing workflow.
- **Tooling.** CLA Assistant has been selected to manage CLA signing. It is
  **not active yet**; once activated, it will prompt authors on their first
  pull request and record the signatures.
- **Sign-off.** A commit sign-off (`Signed-off-by`) is optional good practice
  but is **not** a substitute for the CLA.

Albada currently accepts individual contributions only. If an employer or
other entity owns the proposed contribution, do not submit it under the
individual agreement; contact the maintainer first. An entity agreement may be
added after the project has a suitable legal structure.

## Governance

The maintainer (Bruno Gil Ramírez, for the Albada project) reviews and merges all
contributions. Contribution handling is voluntary and best-effort, with **no
service-level agreement**, and contributions that conflict with the product
direction may be declined. Sensitive attachments are not accepted: never
attach real databases, backups, journal or therapy content, or other
personal material to issues or pull requests (see
[SECURITY.md](./SECURITY.md)).

## Before You Open a PR

Run the quality gates that CI mirrors:

```sh
npm run typecheck
npm run lint
npm run format:check
npm run test
npm run rust:fmt:check
npm run rust:clippy
cd src-tauri && cargo test
```

## Code Conventions

- **The data path stays `React UI -> typed Tauri command boundary -> SQLite`.**
  Keep raw SQL, filesystem paths, and domain rules out of React components.
  Keep native commands thin; put validation and transaction behavior below the
  command boundary in `src-tauri/src/domain/` and `src-tauri/src/repos/`.
- Add frontend API calls through the typed wrappers in `src/lib/api/` and
  validate shapes with the Zod schemas in `src/lib/schemas/`.
- **Schema changes are new forward-only migrations** under
  `src-tauri/migrations/` with the next version number. The public lineage
  starts from the canonical baseline `001_initial_schema.sql` (it creates the
  complete current schema in a fresh database), so your follow-on migration is
  `002_*.sql` or later, and the runtime sequence stays contiguous. Never edit
  a migration that has already shipped; the runner records applied versions in
  `_migrations`. `src-tauri/migrations/archive/` holds the pre-release history
  for reference only — never apply, edit, or extend archived files.
- Keep the therapy and journal collections separate from general views,
  notifications, and generic exports. Journal and therapy exports require an
  explicit item selection and a preview.
- Store and work with Markdown source, not rendered HTML; never silently
  rewrite user-authored Markdown.
- All UI strings belong in the four locale catalogs under
  `src/lib/i18n/locales/`. Keep English, Spanish, German, and French in parity;
  the locale and parity tests fail otherwise — update them with your change.
- Match the existing patterns: repository module layout, DTO/command naming,
  component structure, and styling through the shared design-system tokens
  rather than ad-hoc styles.

## Tests

- New behavior needs tests: frontend tests with Vitest + React Testing Library
  colocated as `*.test.ts(x)`; Rust tests in the owning module or under
  `src-tauri/tests/`. Domain changes should cover relationship integrity,
  progress derivation, deadline-vs-status behavior, and journal/therapy
  isolation.
- Rust tests must keep using disposable in-memory SQLite databases and the
  synthetic fixtures under `src-tauri/fixtures/legacy/`. Never add real personal
  data as a test fixture.
- The Playwright E2E journey (`e2e/`) runs the frontend against a synthetic
  in-page IPC mock; extend it only if the main journey changes.

## Formatting

Prettier and ESLint own frontend style, `rustfmt` and `clippy` own Rust style;
configuration is committed. Markdown documentation is hand-formatted and
excluded from Prettier. Do not reformat unrelated files in a contribution.
