import { existsSync, statSync } from "node:fs";
import { log } from "node:console";
import { join } from "node:path";
import { URL, fileURLToPath } from "node:url";

const fixtureRoot = fileURLToPath(new URL("../src-tauri/fixtures/legacy/", import.meta.url));

const requiredFixtures = [
  "templates/daily-template.md",
  "records/2026-Foundation/Q2/diary/2026-06-25.md",
  "records/2026-Foundation/Q2/diary/personal-notes-raw/2000-00-00-raw.md",
  "records/2026-Foundation/Q2/diary/personal-notes-raw/2026-06-25-raw.md",
  "records/2026-Foundation/Q2/more/project-assets/synthetic-project.md",
  "salud-mental/Notas/nota-sintetica.md",
  "journal/entries/export-one.md",
  "journal/entries/export-two.md",
  "journal/entries/not-selected.md",
  "expected-journal-export.md",
];

const ignoredFixtures = [
  "records/generated/archive.tex",
  "records/generated/archive.pdf",
  "records/generated/diagram.png",
  ".opencode/agent.md",
  ".venv/ignored.md",
  "records/2026-Foundation/Q2/target/debug/ignored.md",
];

for (const relativePath of [...requiredFixtures, ...ignoredFixtures]) {
  const path = join(fixtureRoot, ...relativePath.split("/"));
  if (!existsSync(path) || !statSync(path).isFile()) {
    throw new Error(`Missing legacy fixture: ${relativePath}`);
  }
}

log(
  `Legacy fixture inventory is valid (${requiredFixtures.length} required, ${ignoredFixtures.length} ignored-path fixtures).`,
);
