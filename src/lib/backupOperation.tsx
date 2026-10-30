import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { createBackup } from "./api/backupApi";
import type { BackupDto } from "./types";

type BackupStatus = "idle" | "backup" | "restore";

interface BackupOperationContextValue {
  status: BackupStatus;
  startBackup: () => Promise<BackupDto>;
  runRestore: <T>(operation: () => Promise<T>) => Promise<T>;
}

const defaultRunRestore = <T,>(operation: () => Promise<T>) => operation();

const defaultContext: BackupOperationContextValue = {
  status: "idle",
  startBackup: createBackup,
  runRestore: defaultRunRestore,
};

const BackupOperationContext = createContext(defaultContext);

export function BackupOperationProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<BackupStatus>("idle");
  const startBackup = useCallback(async () => {
    setStatus("backup");
    try {
      return await createBackup();
    } finally {
      setStatus("idle");
    }
  }, []);
  const runRestore = useCallback(async <T,>(operation: () => Promise<T>) => {
    setStatus("restore");
    try {
      return await operation();
    } finally {
      setStatus("idle");
    }
  }, []);
  const value = useMemo(
    () => ({ status, startBackup, runRestore }),
    [runRestore, startBackup, status],
  );

  return (
    <BackupOperationContext.Provider value={value}>
      <div inert={status === "restore"}>{children}</div>
      <BackupProgress status={status} />
    </BackupOperationContext.Provider>
  );
}

function BackupProgress({ status }: { status: BackupStatus }) {
  const { t } = useTranslation();
  if (status === "idle") return null;

  if (status === "restore") {
    return (
      <div
        role="status"
        aria-busy="true"
        aria-live="polite"
        className="fixed inset-0 z-50 flex items-center justify-center p-6"
        style={{ backgroundColor: "var(--joi-scrim)" }}
      >
        <div className="joi-alert--info flex items-center gap-3 rounded-lg border p-4 shadow-lg">
          <span
            aria-hidden="true"
            className="h-5 w-5 shrink-0 animate-spin rounded-full border-2 border-current border-t-transparent"
          />
          <span className="font-medium">{t("settings.backups.restoring")}</span>
        </div>
      </div>
    );
  }

  return (
    <div
      role="status"
      aria-live="polite"
      className="joi-alert--info fixed bottom-4 right-4 z-50 flex max-w-sm items-start gap-3 rounded-lg border p-4 shadow-lg"
    >
      <span
        aria-hidden="true"
        className="mt-0.5 h-4 w-4 shrink-0 animate-spin rounded-full border-2 border-current border-t-transparent"
      />
      <div>
        <p className="font-medium">{t("settings.backups.creating")}</p>
        <p className="mt-1 text-sm">{t("settings.backups.creatingHelp")}</p>
      </div>
    </div>
  );
}

export function useBackupOperation() {
  return useContext(BackupOperationContext);
}
