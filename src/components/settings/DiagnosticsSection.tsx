import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { getEnvironmentDiagnostics } from "@/lib/api/diagnosticsApi";
import { exportTemplateCapability } from "@/lib/api/exportApi";
import { formatNumber } from "@/lib/format";
import type { EnvironmentDiagnostics, TemplateCapability } from "@/lib/types";

/**
 * 9.7 settings diagnostics section. Shows the embedded Typst compiler
 * version, bundled capability status, template and font availability, and
 * local environment versions. Native values (versions, template names)
 * render verbatim in every locale; only the surrounding chrome is
 * translated. A failed load degrades to a muted note rather than an alert
 * so this section can never steal the page-level error channel.
 */
function DiagnosticsSection() {
  const { t } = useTranslation();
  const [capability, setCapability] = useState<TemplateCapability | null>(null);
  const [diagnostics, setDiagnostics] = useState<EnvironmentDiagnostics | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    Promise.all([exportTemplateCapability(), getEnvironmentDiagnostics()])
      .then(([loadedCapability, loadedDiagnostics]) => {
        if (cancelled) return;
        setCapability(loadedCapability);
        setDiagnostics(loadedDiagnostics);
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="xl:col-span-2">
      <h2 className="joi-text text-sm font-medium">{t("settings.diagnostics.title")}</h2>
      <p className="joi-text--muted mt-1 text-sm">{t("settings.diagnostics.help")}</p>
      {capability && diagnostics ? (
        <ul className="joi-text--muted mt-3 flex flex-col gap-1 text-sm">
          <li>
            {t("settings.diagnostics.compilerVersion")}: {capability.compiler_version}
          </li>
          <li>
            {t("settings.diagnostics.capability")}:{" "}
            {capability.valid
              ? t("settings.diagnostics.statusValid")
              : t("settings.diagnostics.statusInvalid")}
            {capability.unavailable_resources.length > 0
              ? ` · ${t("settings.diagnostics.unavailableResources", {
                  resources: capability.unavailable_resources.join(", "),
                })}`
              : ""}
          </li>
          <li>
            {t("settings.diagnostics.template")}: {diagnostics.template_name} v
            {diagnostics.template_version}
          </li>
          <li>
            {t("settings.diagnostics.fonts")}:{" "}
            {t("settings.diagnostics.fontsAvailable", {
              count: formatNumber(diagnostics.bundled_font_count),
            })}
          </li>
          <li>
            {t("settings.diagnostics.environment")}: {t("settings.diagnostics.sqliteVersion")}{" "}
            {diagnostics.sqlite_version} · {t("settings.diagnostics.tauriVersion")}{" "}
            {diagnostics.tauri_version} · {t("settings.diagnostics.osVersion")}{" "}
            {diagnostics.os_platform} {diagnostics.os_version}
          </li>
        </ul>
      ) : failed ? (
        <p className="joi-text--muted mt-3 text-sm">{t("settings.diagnostics.unavailable")}</p>
      ) : (
        <p className="joi-text--muted mt-3 text-sm">{t("common.loading")}</p>
      )}
    </div>
  );
}

export default DiagnosticsSection;
