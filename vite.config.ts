import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";
import type { RollupLog } from "rollup";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],

  build: {
    // The app's shared entry is intentionally above Vite's generic 500 kB
    // recommendation because it contains the local four-locale catalog and
    // shell. Route chunks remain split and the boot route is preloaded in App.
    chunkSizeWarningLimit: 600,
    rollupOptions: {
      onwarn(warning: RollupLog, warn: (warning: RollupLog | string) => void) {
        // Zod ships valid esbuild annotations that Rollup 4 reports only
        // because the comments are not positioned where Rollup expects them.
        if (warning.code === "INVALID_ANNOTATION" && warning.id?.includes("node_modules/zod/")) {
          return;
        }
        warn(warning);
      },
    },
  },

  // Tauri serves the built frontend from embedded assets behind a custom
  // protocol (http://tauri.localhost on Windows), not from a guaranteed web
  // root. Vite's documented relative base ("./", "for embedded deployment")
  // makes every generated asset URL resolve relative to the referencing file,
  // which is the established fix for blank-screen asset loading in
  // `tauri build` output. Dev is unaffected: `tauri dev` loads the app from
  // the dev server root (http://localhost:1420/), where relative URLs resolve
  // identically to absolute ones.
  base: "./",

  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },

  // Dev cold-start tuning. The Tauri webview sits idle until Vite's first
  // transform chain (dep optimization + entry graph) completes, so every
  // runtime dependency is force pre-bundled here (`include`) to keep a
  // mid-session discovery from triggering a re-optimization full reload, and
  // the boot-critical modules below are pre-transformed as soon as the server
  // listens. Build/test behavior is unaffected: optimizeDeps and server.warmup
  // are dev-server options only.
  optimizeDeps: {
    include: [
      "react",
      "react-dom/client",
      "react-router",
      "react-markdown",
      "remark-gfm",
      "zod",
      "i18next",
      "react-i18next",
      "@tauri-apps/api/core",
      "@tauri-apps/plugin-notification",
      "@tauri-apps/plugin-opener",
      "@tauri-apps/plugin-os",
    ],
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching non-frontend paths so docs, changelog,
      //    agent config, and Rust sources do not trigger hot reload.
      ignored: ["**/src-tauri/**", "**/docs/**", "**/.opencode/**", "**/.agents/**", "**/*.md"],
    },
    // 4. warm the boot path before the webview's first request arrives:
    //    the entry module, the full CSS pipeline (`/src/index.css` pulls in
    //    design-system.css through Tailwind), the eager shell, and the initial
    //    lazy Today route chunk that the splash waits on.
    warmup: {
      clientFiles: [
        "./src/main.tsx",
        "./src/index.css",
        "./src/app/layout.tsx",
        "./src/app/today/page.tsx",
      ],
    },
  },
}));
