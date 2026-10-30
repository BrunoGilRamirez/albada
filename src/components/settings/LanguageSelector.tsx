import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ensureDefaultWorkspace, updateWorkspaceLocale } from "@/lib/api/workspaceApi";
import { toDisplayMessage } from "@/lib/errors";
import {
  SUPPORTED_LOCALES,
  applyWorkspacePreference,
  parseWorkspaceLocalePreference,
  type WorkspaceLocalePreference,
} from "@/lib/i18n";

/**
 * 8.6.7 language selector. Persists the explicit tag or `system` for the
 * current workspace through the typed `update_workspace_locale` boundary,
 * then applies the resulting locale immediately. `System` keeps the Tauri OS
 * plugin authoritative; a failed OS read resolves to English at runtime while
 * the stored preference stays `system` (resolution never rewrites it).
 */
function LanguageSelector() {
  const { t, i18n: instance } = useTranslation();
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [preference, setPreference] = useState<WorkspaceLocalePreference>("system");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    void ensureDefaultWorkspace()
      .then((workspace) => {
        setWorkspaceId(workspace.id);
        setPreference(parseWorkspaceLocalePreference(workspace.locale));
      })
      .catch((err) => setError(toDisplayMessage(err)));
  }, []);

  const handleChange = async (value: WorkspaceLocalePreference) => {
    if (busy || value === preference) return;
    setError(null);
    setNotice(null);
    setBusy(true);
    const previous = preference;
    setPreference(value);
    try {
      let id = workspaceId;
      if (!id) {
        const workspace = await ensureDefaultWorkspace();
        id = workspace.id;
        setWorkspaceId(id);
      }
      // Persist first; only a successful native write may change the UI.
      const updated = await updateWorkspaceLocale(id, value);
      const stored = parseWorkspaceLocalePreference(updated.locale);
      setPreference(stored);
      const resolved = await applyWorkspacePreference(stored);
      // The hook's `t` is bound to the pre-switch render, so the notice must
      // be computed from the newly active locale explicitly; otherwise the
      // frozen string would stay in the previous language.
      const appliedT = instance.getFixedT(resolved);
      const languageName = appliedT(`settings.language.${resolved}`);
      setNotice(appliedT("settings.language.saved", { language: languageName }));
    } catch (err) {
      setPreference(previous);
      setError(toDisplayMessage(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div>
      <label htmlFor="workspace-locale" className="joi-field__label block">
        {t("settings.language.label")}
      </label>
      <p className="joi-text--muted mt-1 text-sm">{t("settings.language.description")}</p>
      <select
        id="workspace-locale"
        className="joi-field__select mt-2"
        value={preference}
        disabled={!workspaceId || busy}
        onChange={(event) => void handleChange(event.target.value as WorkspaceLocalePreference)}
      >
        <option value="system">{t("settings.language.system")}</option>
        {SUPPORTED_LOCALES.map((locale) => (
          <option key={locale} value={locale}>
            {t(`settings.language.${locale}`)}
          </option>
        ))}
      </select>
      {error ? (
        <p role="alert" className="joi-text--danger mt-2 text-sm">
          {error}
        </p>
      ) : null}
      {notice ? (
        <p role="status" className="joi-text--success mt-2 text-sm">
          {notice}
        </p>
      ) : null}
    </div>
  );
}

export default LanguageSelector;
