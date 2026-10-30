import { createContext, useContext, useEffect, useState } from "react";

// Pure helper: resolve the effective theme string given the mode and OS prefers-dark status.
// Used by tests and the apply-effect so that "system" is resolved at apply time,
// not at init time.
function resolveEffectiveTheme(
  mode: "light" | "dark" | "system",
  prefersDark: boolean,
): "light" | "dark" {
  if (mode === "dark") return "dark";
  if (mode === "light") return "light";
  // mode === "system": use OS preference
  return prefersDark ? "dark" : "light";
}

const ThemeContext = createContext<{
  mode: "light" | "dark" | "system";
  setMode: (mode: "light" | "dark" | "system") => void;
  toggle: () => void;
}>({
  mode: "system",
  setMode: () => {},
  toggle: () => {},
});

export function useTheme() {
  return useContext(ThemeContext);
}

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  // Read stored preference on first render; default to "system" if missing/invalid.
  const [storedMode, setStoredMode] = useState<"light" | "dark" | "system">("system");

  // Initialize storedMode from localStorage once on first render.
  useEffect(() => {
    try {
      const item = localStorage.getItem("albada-theme");
      if (item && item in { light: "", dark: "", system: "" }) {
        setStoredMode(item as "light" | "dark" | "system");
      }
    } catch (e) {
      // localStorage not available (SSR, tests); keep default "system"
      void e;
    }
  }, []);

  const [mode, setModeInternal] = useState<"light" | "dark" | "system">("system");

  // Sync mode from storedMode after it's been initialized.
  useEffect(() => {
    setModeInternal(storedMode);
  }, [storedMode]);

  // Apply the mode to <html> and persist. Runs when mode changes.
  useEffect(() => {
    const html = document.documentElement;
    try {
      const prefersDark =
        typeof window !== "undefined"
          ? window.matchMedia("(prefers-color-scheme: dark)").matches
          : false;
      const effective = resolveEffectiveTheme(mode, prefersDark);

      if (effective === "dark") {
        html.classList.add("dark");
        html.style.colorScheme = "dark";
      } else {
        html.classList.remove("dark");
        html.style.colorScheme = "light";
      }

      // Persist the user's chosen mode (not the resolved effective value)
      try {
        localStorage.setItem("albada-theme", mode);
      } catch (e) {
        // localStorage full/quota; silently skip
        void e;
      }
    } catch (e) {
      // DOM not available (SSR, test environment); skip apply
      void e;
    }
  }, [mode]);

  // Subscribe to OS preference changes when in system mode.
  // Only re-applies the class; does NOT overwrite the stored mode.
  useEffect(() => {
    if (mode !== "system") return;
    try {
      const mql = window.matchMedia("(prefers-color-scheme: dark)");
      const handler = (ev: MediaQueryListEvent) => {
        try {
          const prefersDark = ev.matches;
          const effective = resolveEffectiveTheme("system", prefersDark);
          const html = document.documentElement;
          if (effective === "dark") {
            html.classList.add("dark");
            html.style.colorScheme = "dark";
          } else {
            html.classList.remove("dark");
            html.style.colorScheme = "light";
          }
        } catch (e) {
          // DOM not available
          void e;
        }
      };
      mql.addEventListener?.("change", handler);
      return () => mql.removeEventListener?.("change", handler);
    } catch (e) {
      // matchMedia not available (SSR, test environment)
      void e;
      return undefined;
    }
  }, [mode]);

  const setMode = (newMode: "light" | "dark" | "system") => {
    setModeInternal(newMode);
    // Persistence is written in the apply-effect above, after the state update.
  };

  const toggle = () => {
    if (mode === "system") {
      setMode("dark");
    } else if (mode === "dark") {
      setMode("light");
    } else {
      // mode === "light"
      setMode("dark");
    }
  };

  return (
    <ThemeContext.Provider value={{ mode, setMode, toggle }}>{children}</ThemeContext.Provider>
  );
}

export { ThemeContext, resolveEffectiveTheme };
