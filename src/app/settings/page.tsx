import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import PageContainer from "@/components/navigation/PageContainer";
import BackupReminderForm from "@/components/settings/BackupReminderForm";
import DiagnosticsSection from "@/components/settings/DiagnosticsSection";
import LanguageSelector from "@/components/settings/LanguageSelector";
import { listArchivedTasks, restoreArchivedTask } from "../../lib/api/taskApi";
import { deleteBackup, listBackups, restoreBackup, validateBackup } from "../../lib/api/backupApi";
import {
  ensureDefaultWorkspace,
  getBackupReminderPreference,
  getWarningWindow,
  updateWarningWindow,
} from "../../lib/api/workspaceApi";
import { toDisplayMessage } from "../../lib/errors";
import { formatDateTime, formatNumber } from "../../lib/format";
import { useBackupOperation } from "../../lib/backupOperation";
import type { BackupDto, BackupReminderPreference, Task } from "../../lib/types";

function SettingsPage() {
  const { t } = useTranslation();
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [archived, setArchived] = useState<Task[]>([]);
  const [backups, setBackups] = useState<BackupDto[]>([]);
  const [warningWindow, setWarningWindow] = useState<number>(3);
  const [warningDraft, setWarningDraft] = useState<string>("3");
  const [backupReminder, setBackupReminder] = useState<BackupReminderPreference | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [restoring, setRestoring] = useState(false);
  const [deletingBackupPath, setDeletingBackupPath] = useState<string | null>(null);
  const { status: backupStatus, runRestore, startBackup } = useBackupOperation();
  const creatingBackup = backupStatus === "backup";

  const loadArchived = useCallback(async (wsId: string) => {
    try {
      setArchived(await listArchivedTasks(wsId));
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  }, []);

  const loadBackups = useCallback(async () => {
    try {
      setBackups(await listBackups());
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  }, []);

  const loadWarningWindow = useCallback(async (wsId: string) => {
    try {
      const days = await getWarningWindow(wsId);
      setWarningWindow(days);
      setWarningDraft(String(days));
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  }, []);

  const loadBackupReminder = useCallback(async (wsId: string) => {
    try {
      setBackupReminder(await getBackupReminderPreference(wsId));
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  }, []);

  useEffect(() => {
    void ensureDefaultWorkspace()
      .then((workspace) => {
        setWorkspaceId(workspace.id);
        return workspace.id;
      })
      .then((wsId) => {
        void loadArchived(wsId);
        void loadBackups();
        void loadWarningWindow(wsId);
        void loadBackupReminder(wsId);
      })
      .catch((err) => setError(toDisplayMessage(err)));
  }, [loadArchived, loadBackups, loadWarningWindow, loadBackupReminder]);

  const handleSaveWarningWindow = async () => {
    setError(null);
    setNotice(null);
    if (!workspaceId) return;
    const days = Number.parseInt(warningDraft, 10);
    if (Number.isNaN(days) || days < 0 || days > 90) {
      setError(t("settings.warningWindow.invalid"));
      return;
    }
    try {
      await updateWarningWindow(workspaceId, days);
      setWarningWindow(days);
      setNotice(t("settings.warningWindow.saved", { days: formatNumber(days) }));
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleRestore = async (id: string) => {
    setError(null);
    setNotice(null);
    if (!workspaceId) return;
    try {
      await restoreArchivedTask(id);
      setNotice(t("settings.archived.restored"));
      await loadArchived(workspaceId);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleBackup = async () => {
    if (creatingBackup || restoring) return;
    setError(null);
    setNotice(null);
    try {
      await startBackup();
      setNotice(t("settings.backups.created"));
      await loadBackups();
      // A verified backup advances the reminder's success timestamp natively.
      if (workspaceId) await loadBackupReminder(workspaceId);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleRestoreBackup = async (backup: BackupDto) => {
    if (restoring || creatingBackup || deletingBackupPath) return;
    setError(null);
    setNotice(null);
    setRestoring(true);
    try {
      // Pre-validate before asking for confirmation. A rejected or invalid
      // validation must never reach window.confirm or restoreBackup.
      const validation = await validateBackup(backup.path);
      if (validation.status !== "valid") {
        setError(validation.message || t("settings.backups.restoreFailed"));
        return;
      }
      if (!validation.checksum) {
        setError(t("settings.backups.restoreMissingChecksum"));
        return;
      }
      const checksum = validation.checksum;
      const kindLabel = t(
        backup.kind === "recovery"
          ? "settings.backups.restoreKind.recovery"
          : "settings.backups.restoreKind.user",
      );
      const sizeKb = (validation.size_bytes ?? backup.size_bytes) / 1024;
      const schemaVersion = validation.schema_version ?? backup.schema_version;
      const schemaLabel =
        schemaVersion === null || schemaVersion === undefined
          ? t("settings.backups.schemaUnknown")
          : formatNumber(schemaVersion);
      const confirmed = window.confirm(
        [
          t("settings.backups.restoreConfirmTitle", {
            kind: kindLabel,
            createdAt: formatDateTime(backup.created_at),
          }),
          t("settings.backups.restoreConfirmDetail", {
            size: formatNumber(sizeKb, { maximumFractionDigits: 0 }),
            schema: schemaLabel,
            checksum,
          }),
          "",
          t("settings.backups.restoreConfirmWarning"),
          t("settings.backups.restoreConfirmPrivacy"),
        ].join("\n"),
      );
      if (!confirmed) return;
      await runRestore(async () => {
        const workspace = await restoreBackup(backup.path, checksum);
        setWorkspaceId(workspace.id);
        setNotice(t("settings.backups.restored"));
        await Promise.all([
          loadArchived(workspace.id),
          loadBackups(),
          loadWarningWindow(workspace.id),
          loadBackupReminder(workspace.id),
        ]);
      });
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setRestoring(false);
    }
  };

  const handleDeleteBackup = async (backup: BackupDto) => {
    if (restoring || creatingBackup || deletingBackupPath) return;
    if (!window.confirm(t("settings.backups.deleteConfirm"))) return;
    setError(null);
    setNotice(null);
    setDeletingBackupPath(backup.path);
    try {
      await deleteBackup(backup.path);
      setNotice(t("settings.backups.deleted"));
      await loadBackups();
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setDeletingBackupPath(null);
    }
  };

  return (
    <PageContainer>
      <h1 className="text-2xl font-semibold">{t("settings.title")}</h1>
      <p className="joi-text--muted mt-2">{t("settings.subtitle")}</p>

      <nav className="mt-4 flex flex-wrap gap-2" aria-label={t("settings.sections.aria")}>
        <Link
          to="/settings/commitments"
          className="joi-btn joi-btn--secondary rounded-md border px-3 py-1.5 text-sm"
        >
          {t("settings.sections.commitments")}
        </Link>
        <Link
          to="/settings/availability"
          className="joi-btn joi-btn--secondary rounded-md border px-3 py-1.5 text-sm"
        >
          {t("settings.sections.availability")}
        </Link>
        <Link
          to="/settings/import"
          className="joi-btn joi-btn--secondary rounded-md border px-3 py-1.5 text-sm"
        >
          {t("settings.sections.import")}
        </Link>
        <Link
          to="/settings/notifications"
          className="joi-btn joi-btn--secondary rounded-md border px-3 py-1.5 text-sm"
        >
          {t("settings.sections.notifications")}
        </Link>
      </nav>

      {error ? (
        <p role="alert" className="joi-text--danger mt-4 text-sm">
          {error}
        </p>
      ) : null}
      {notice ? (
        <p role="status" className="joi-text--success mt-4 text-sm">
          {notice}
        </p>
      ) : null}

      <div className="mt-6 grid gap-x-10 gap-y-8 xl:grid-cols-2">
        <div>
          <label htmlFor="workspace-path" className="joi-field__label block">
            {t("settings.workspace.label")}
          </label>
          <input
            id="workspace-path"
            type="text"
            value={
              workspaceId
                ? t("settings.workspace.value", { id: workspaceId.slice(0, 8) })
                : t("common.loading")
            }
            disabled
            className="joi-field__input mt-1 w-full max-w-md"
          />
        </div>

        <LanguageSelector />

        <div>
          <label htmlFor="warning-window" className="joi-field__label block">
            {t("settings.warningWindow.label")}
          </label>
          <p className="joi-text--muted mt-1 text-sm">
            {t("settings.warningWindow.help", { days: formatNumber(warningWindow) })}
          </p>
          <div className="mt-2 flex items-center gap-2">
            <input
              id="warning-window"
              type="number"
              min={0}
              max={90}
              value={warningDraft}
              onChange={(e) => setWarningDraft(e.target.value)}
              className="joi-field__input w-32"
            />
            <button
              type="button"
              onClick={handleSaveWarningWindow}
              className="joi-btn joi-btn--primary rounded-md px-3 py-2 text-sm font-medium"
            >
              {t("common.actions.save")}
            </button>
          </div>
        </div>

        <div>
          <h2 className="joi-text text-sm font-medium">{t("settings.archived.title")}</h2>
          <p className="joi-text--muted mt-1 text-sm">{t("settings.archived.help")}</p>
          {archived.length === 0 ? (
            <p className="joi-empty__state mt-3">{t("settings.archived.empty")}</p>
          ) : (
            <ul className="mt-3 flex flex-col gap-2">
              {archived.map((task) => (
                <li
                  key={task.id}
                  className="joi-surface--base joi-border--default flex items-center justify-between gap-3 rounded-lg border px-3 py-2"
                >
                  <div className="flex min-w-0 items-center gap-2">
                    <Link
                      to={`/tasks/${task.id}`}
                      className="joi-text truncate text-sm hover:underline"
                    >
                      {task.title}
                    </Link>
                    <span className="joi-badge joi-badge--neutral shrink-0">
                      {t(`common.labels.taskStatus.${task.status}`)}
                    </span>
                  </div>
                  <button
                    type="button"
                    onClick={() => handleRestore(task.id)}
                    className="joi-btn joi-btn--secondary shrink-0 rounded-md border px-2 py-1 text-xs font-medium"
                  >
                    {t("common.actions.restore")}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="xl:col-span-2">
          <h2 className="joi-text text-sm font-medium">{t("settings.backups.title")}</h2>
          <p className="joi-text--muted mt-1 text-sm">{t("settings.backups.help")}</p>
          <button
            type="button"
            onClick={handleBackup}
            disabled={creatingBackup || restoring}
            className="joi-btn joi-btn--primary mt-3 rounded-md px-3 py-2 text-sm font-medium disabled:opacity-50"
          >
            {creatingBackup ? t("settings.backups.creating") : t("settings.backups.create")}
          </button>
          {backups.length > 0 ? (
            <ul className="mt-3 flex flex-col gap-2">
              {backups.map((backup) => (
                <li
                  key={backup.path}
                  className="joi-surface--base joi-border--default flex items-center justify-between gap-3 rounded-lg border px-3 py-2"
                >
                  <div className="flex min-w-0 items-center gap-2">
                    <span className="joi-text--muted truncate text-sm">
                      {formatDateTime(backup.created_at)} —{" "}
                      {t("common.exportJob.kilobytes", {
                        size: formatNumber(backup.size_bytes / 1024, {
                          maximumFractionDigits: 0,
                        }),
                      })}
                    </span>
                    <span className="joi-badge joi-badge--neutral shrink-0">
                      {t(
                        backup.kind === "recovery"
                          ? "settings.backups.kinds.recovery"
                          : "settings.backups.kinds.user",
                      )}
                    </span>
                  </div>
                  <div className="flex shrink-0 items-center gap-2">
                    <button
                      type="button"
                      onClick={() => handleRestoreBackup(backup)}
                      disabled={restoring || creatingBackup || Boolean(deletingBackupPath)}
                      aria-label={t("settings.backups.restoreAria")}
                      className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
                    >
                      {t("common.actions.restore")}
                    </button>
                    <button
                      type="button"
                      onClick={() => void handleDeleteBackup(backup)}
                      disabled={restoring || creatingBackup || Boolean(deletingBackupPath)}
                      aria-label={t("settings.backups.deleteAria")}
                      className="joi-btn joi-btn--danger rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
                    >
                      {t("settings.backups.delete")}
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          ) : null}
        </div>

        {workspaceId && backupReminder ? (
          <div className="xl:col-span-2">
            <BackupReminderForm
              workspaceId={workspaceId}
              preference={backupReminder}
              onSaved={setBackupReminder}
            />
          </div>
        ) : null}

        <DiagnosticsSection />
      </div>
    </PageContainer>
  );
}

export default SettingsPage;
