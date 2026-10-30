import { defineConfig, devices } from "@playwright/test";

/**
 * Phase 9.2 P1: Playwright E2E for the main user journey.
 *
 * The spec (`e2e/main-journey.e2e.ts`) runs against the Vite dev server with
 * an in-page synthetic Tauri IPC mock, so no Rust build or `tauri-driver`
 * is required for this smoke journey. See the spec header for how to point
 * it at the real `tauri dev` webview instead.
 */
const baseURL = process.env.E2E_BASE_URL ?? "http://localhost:1420";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "**/*.e2e.ts",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [["list"]],
  use: {
    baseURL,
    trace: "on-first-retry",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "npm run dev",
    url: baseURL,
    reuseExistingServer: true,
    timeout: 60_000,
  },
});
