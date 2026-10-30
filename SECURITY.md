# Privacy, Security, and Alpha Expectations

Albada is a local-first, single-user desktop application. This document states
what the application guarantees about your data, what the alpha does and does
not promise, and how to report problems safely.

## Data Ownership and Privacy Model

- **All data is local.** Albada stores everything in one SQLite file inside
  your Tauri app-data directory (on Windows, under
  `%APPDATA%\io.github.brunogilramirez.albada\`). There are no accounts, no
  servers, and no background network services.
- **No encryption at rest.** In this alpha, the database file and exported
  Markdown are stored unencrypted, like ordinary documents in your Windows
  profile. Anyone with access to your user account or your backup files can
  read them; protect your machine and your backups accordingly.
- **No telemetry.** The app sends nothing to anyone. Diagnostics and logs stay
  on your machine.
- **You own your data.** The app includes explicit backup and restore flows.
  Backups are your responsibility, and making one before updating or reinstalling
  is strongly recommended (see below).
- **Journaling is independent** of tasks and exports. Journal and therapy
  exports require an explicit item selection and a preview before anything is
  written.
- **Therapy content is a separate private collection.** It never appears in
  general views, notifications, or generic project exports, and its export
  scope is separate.
- **Notifications are lock-screen safe.** Reminder and warning messages are
  generic texts with quiet hours and snooze; they never embed task titles,
  journal excerpts, or therapy content.
- **Markdown source is preserved.** Exports store and emit Markdown source,
  never rendered HTML, and never rewrite user-authored text.

## What the Alpha Does Not Promise

Albada 0.1.0 is an alpha distributed on a **best-effort basis**:

- **No support SLA.** Issues and pull requests are reviewed when the maintainer
  can; response time is not guaranteed. GitHub Discussions is disabled during
  the alpha.
- **Windows only.** Other platforms are not built or tested.
- **Independent local installations only.** "Multiple users" of the alpha means
  multiple people each installing the app on their own machine. There are no
  shared accounts, shared data, synchronization, or remote access.
- **Unsigned installers.** Alpha builds are not code-signed, so Windows may
  show an untrusted-publisher/SmartScreen warning. Install only builds obtained
  from this repository's Releases page.
- **Clean-start boundary at 0.1.0 (pre-release line).** The canonical public
  `0.1.0` baseline will not open or upgrade databases or backups created by
  private development / pre-alpha builds of Albada, unless a separately
  approved migration bridge is offered later. Start the public alpha with a
  clean database, and do not restore a private or pre-release backup into the
  public snapshot.
- **No auto-update.** Each version is installed manually.
- **Upgrades between public releases are forward-only.** A fresh public
  installation builds the complete schema from the canonical
  `001_initial_schema.sql` baseline; later public releases apply their new
  versioned, forward-only migrations automatically on first launch after an
  update. Downgrading to an older version is not supported, so make a backup
  before updating. Public releases intend forward-only version upgrades, but
  moving data between machines or Windows profiles has no guarantee during
  the alpha: that path works only if a specific release explicitly documents
  and verifies it.
- **PDF export is still under release verification.** Markdown export is the
  stable path; the embedded Typst/PDF renderer ships in alpha with ongoing
  packaging, performance, and third-party-notice checks.

## Reporting a Problem Safely

- Use GitHub Issues for reproducible bug reports. Include version, OS, and
  steps with **synthetic data only**.
- **Never attach or paste private content into an issue:** databases, backup
  files, journal entries, therapy records, personal exports, screenshots that
  reveal personal text, or log files copied from a real profile. Log lines can
  contain references to your own content; reproduce with a fresh app data
  directory instead. The same rule applies to security reports.
- **Security vulnerabilities:** report them privately through the future
  public repository's GitHub **Security** tab using **Report a
  vulnerability** — GitHub's private vulnerability reporting — once that
  feature is enabled for the repository. Describe the issue with synthetic
  reproduction details only; do not attach or paste databases, backups,
  journal or therapy content, or other personal material, and do not include
  exploit details. While private reporting is not yet enabled, open an issue
  **without** exploit details or any personal data; the maintainer will
  follow up privately.

## Scope Limits (What Will Not Change Without an Explicit Decision)

Adding telemetry, remote transmission, cloud services, accounts,
synchronization, or multi-user features is outside the current product
direction and would require an explicit, documented scope change.
