# Setup and Development

Albada is a Tauri 2 desktop application with a React 19, TypeScript, and Vite 6
frontend. The MVP uses SQLite (via `rusqlite` in Rust) as its local data store,
owned entirely by the native layer.

## Prerequisites

- Windows 10 or 11.
- Node.js 20 or newer.
- Rust toolchain 1.77.2 or newer, including `cargo`, `rustfmt`, and `clippy`.
  On Windows this means the MSVC toolchain; see the official Tauri 2
  prerequisites guide for the Windows build-dependency setup.

## Install

```sh
npm install
cd src-tauri
cargo fetch
```

## Development

```sh
npm run tauri dev
```

This launches the Vite dev server and the Tauri desktop window. On first start
the application creates the SQLite database in the Tauri app-data directory and
applies the canonical baseline schema
(`src-tauri/migrations/001_initial_schema.sql`) to build the complete current
schema in the fresh database.

## Build

```sh
npm run tauri build
```

This produces the Windows installer bundles (NSIS and MSI) under
`src-tauri/target/release/bundle/`. Alpha installers are not code-signed, so
Windows may show an untrusted-publisher warning during installation.

## Quality Checks

Frontend (ESLint flat config, Prettier, TypeScript, Vitest):

```sh
npm run typecheck
npm run lint
npm run format:check
npm run test
```

Rust formatting and lints (require a Rust toolchain on `PATH`; not run by the
frontend scripts):

```sh
npm run rust:fmt:check
npm run rust:clippy
```

All Rust unit and integration tests:

```sh
cd src-tauri
cargo test
```

Rust tests use disposable in-memory SQLite databases only; they never open or
modify the real application database.

End-to-end (Playwright against the Vite dev server with a synthetic in-page
Tauri IPC mock — no Rust build required):

```sh
npx playwright install chromium   # once, to download the browser binary
npm run test:e2e
```

## Database and Logs

- Database file: `albada.sqlite` inside the Tauri app-data directory (on
  Windows typically `%APPDATA%\io.github.brunogilramirez.albada\`).
- Mode: WAL, foreign keys enabled, `synchronous = NORMAL`.
- Migrations: the runtime sequence is the versioned SQL files directly under
  `src-tauri/migrations/`. The public lineage begins with the canonical
  baseline `001_initial_schema.sql` (full schema in one file); follow-on
  migrations start at `002_*.sql`. The runner in
  `src-tauri/src/db/migrations.rs` applies them in contiguous order and records
  each in `_migrations`. The SQL is embedded into the compiled binary during
  `cargo build`, so an installed application does not depend on the build
  machine's source paths.
- `src-tauri/migrations/archive/` keeps the pre-release history (originals
  `001–042`) for reference only; archived files are never applied. A public
  database is always created fresh from the baseline — there is no supported
  upgrade path from private development or pre-alpha databases or their
  backups (see `SECURITY.md`).
- Log file: `albada.log` in the same app-data directory. It records database
  failures, command-level begin/success/failure lines, and panic backtraces.
  Logs stay on the local machine; there is no telemetry. Log output is also
  mirrored to stderr, so `npm run tauri dev` shows it in the terminal.

## PDF Export Notes

The export path is implemented as
`SQLite selection -> generated Markdown -> Markdown AST -> Typst -> PDF` using
an embedded Rust Typst compiler. No Typst installation, Python runtime, LaTeX
distribution, or `PATH` configuration is required. During the alpha, this path
is still under release verification (packaging, performance, and third-party
notice review); Markdown export is the stable path.

## Test Fixtures

The legacy-import fixtures under `src-tauri/fixtures/legacy/` are synthetic,
fabricated files used by the importer tests — including sentinel directories
(`.opencode/`, `.venv/`, `target/`) that verify ignored paths are never
scanned. `node scripts/validate-legacy-fixtures.mjs` checks that the fixture
inventory is intact. Never replace fixtures with real personal data.
