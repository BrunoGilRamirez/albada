import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { isTauri } from "@tauri-apps/api/core";
import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import PageContainer from "../../../components/navigation/PageContainer";
import ReminderRuleForm from "../../../components/settings/ReminderRuleForm";
import { listReminderRules } from "../../../lib/api/notificationApi";
import { ensureDefaultWorkspace } from "../../../lib/api/workspaceApi";
import { toDisplayMessage } from "../../../lib/errors";
import type { ReminderRule, ReminderType } from "../../../lib/types";
import { REMINDER_TYPES } from "../../../lib/types";

type PermissionStatus = "checking" | "granted" | "not-granted" | "denied" | "unavailable";

// Native permission states with a localized message under
// `notifications.permission.*`. The literal key union keeps `t` type-safe.
const PERMISSION_KEYS = {
  checking: "notifications.permission.checking",
  granted: "notifications.permission.granted",
  "not-granted": "notifications.permission.notGranted",
  denied: "notifications.permission.denied",
  unavailable: "notifications.permission.unavailable",
} as const satisfies Record<PermissionStatus, string>;

const RULE_ORDER = new Map<ReminderType, number>(
  REMINDER_TYPES.map((type, index) => [type, index]),
);

function NotificationsSettingsPage() {
  const { t } = useTranslation();
  const [rules, setRules] = useState<ReminderRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [permission, setPermission] = useState<PermissionStatus>("checking");

  const loadRules = useCallback(async () => {
    try {
      const workspace = await ensureDefaultWorkspace();
      const listed = await listReminderRules(workspace.id);
      listed.sort(
        (a, b) =>
          (RULE_ORDER.get(a.reminder_type) ?? REMINDER_TYPES.length) -
          (RULE_ORDER.get(b.reminder_type) ?? REMINDER_TYPES.length),
      );
      setRules(listed);
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const refreshPermission = useCallback(async () => {
    if (!isTauri()) {
      setPermission("unavailable");
      return;
    }
    try {
      setPermission((await isPermissionGranted()) ? "granted" : "not-granted");
    } catch {
      // Outside a configured desktop shell the plugin call cannot resolve;
      // treat it as unavailable rather than blocking the settings screen.
      setPermission("unavailable");
    }
  }, []);

  useEffect(() => {
    void loadRules();
    void refreshPermission();
  }, [loadRules, refreshPermission]);

  const handleRequestPermission = async () => {
    setError(null);
    try {
      const outcome = await requestPermission();
      setPermission(outcome === "granted" ? "granted" : "denied");
    } catch {
      setPermission("unavailable");
    }
  };

  const handleRuleSaved = (updated: ReminderRule) => {
    setRules((prev) => prev.map((rule) => (rule.id === updated.id ? updated : rule)));
  };

  return (
    <PageContainer>
      <h1 className="text-2xl font-semibold">{t("notifications.title")}</h1>
      <p className="joi-text--muted mt-2">{t("notifications.subtitle")}</p>

      <section
        aria-label={t("notifications.permission.aria")}
        className="joi-surface--base joi-border--default mt-6 rounded-lg border p-4"
      >
        <h2 className="joi-text text-base font-semibold">{t("notifications.permission.title")}</h2>
        <p role="status" className="joi-text--muted mt-1 text-sm" data-testid="permission-status">
          {t(PERMISSION_KEYS[permission])}
        </p>
        {(permission === "not-granted" || permission === "denied") && isTauri() ? (
          <button
            type="button"
            onClick={handleRequestPermission}
            className="joi-btn joi-btn--primary mt-3 rounded-lg px-4 py-2"
          >
            {t("notifications.permission.request")}
          </button>
        ) : null}
      </section>

      {error ? (
        <p role="alert" className="joi-text--danger mt-4 text-sm">
          {error}
        </p>
      ) : null}

      <p className="joi-text--muted mt-6 text-sm">{t("notifications.privacyNote")}</p>

      {loading ? (
        <p className="joi-empty__loading mt-4">{t("notifications.rulesLoading")}</p>
      ) : rules.length === 0 ? (
        <p className="joi-empty__state mt-4">{t("notifications.rulesEmpty")}</p>
      ) : (
        <div className="mt-4 flex flex-col gap-4">
          {rules.map((rule) => (
            <ReminderRuleForm key={rule.id} rule={rule} onSaved={handleRuleSaved} />
          ))}
        </div>
      )}
    </PageContainer>
  );
}

export default NotificationsSettingsPage;
