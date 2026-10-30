import { useTranslation } from "react-i18next";
import type { ImportWarningDto, ImportWarningSeverity } from "../../lib/types";

// Presentation-only map. The severity label now comes from the shared enum
// catalog (`common.labels.severity`), so a warning is never color-only.
const SEVERITY_STYLES: Record<ImportWarningSeverity, string> = {
  info: "joi-badge--info",
  warning: "joi-badge--warning",
  error: "joi-badge--danger",
};

interface ImportWarningRowProps {
  warning: ImportWarningDto;
}

function ImportWarningRow({ warning }: ImportWarningRowProps) {
  const { t } = useTranslation();
  const location =
    warning.line === null ? warning.source_path : `${warning.source_path}:${warning.line}`;

  return (
    <li className="joi-surface--base joi-border--default flex flex-col gap-1 rounded-md border px-3 py-2">
      <div className="flex items-center justify-between gap-3">
        <span className="joi-text--muted min-w-0 truncate font-mono text-xs" title={location}>
          {location}
        </span>
        <span className={`joi-badge shrink-0 ${SEVERITY_STYLES[warning.severity]}`}>
          {t(`common.labels.severity.${warning.severity}`)}
        </span>
      </div>
      {/* The message is native import diagnostics, not application copy. */}
      <p className="joi-text text-sm">{warning.message}</p>
    </li>
  );
}

export default ImportWarningRow;
