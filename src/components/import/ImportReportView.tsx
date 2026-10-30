import { useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  ImportConfidence,
  ImportReportDto,
  ImportReviewItemDto,
  ImportWarningDto,
} from "../../lib/types";
import { formatNumber } from "../../lib/format";
import ImportWarningRow from "./ImportWarningRow";

// Presentation-only map. Each confidence level keeps its own translated label
// (`import.confidence.*`), so the chip is never color-only.
const CONFIDENCE_STYLES: Record<ImportConfidence, string> = {
  high: "joi-badge--success",
  medium: "joi-badge--info",
  low: "joi-badge--warning",
  review: "joi-badge--warning",
};

const KNOWN_ENTITY_TYPES = [
  "journal_entry",
  "therapy_record",
  "task",
  "goal",
  "project",
  "note",
] as const;
type KnownEntityType = (typeof KNOWN_ENTITY_TYPES)[number];

function isKnownEntityType(value: string): value is KnownEntityType {
  return (KNOWN_ENTITY_TYPES as readonly string[]).includes(value);
}

/**
 * Entity types with a localized label. Anything else is an arbitrary backend
 * value: it keeps the raw stored identifier instead of a mechanical
 * title-cased guess, so an unclassified value is never invented in any locale.
 * The `isKnownEntityType` guard narrows the template literal at each call
 * site to the finite union of known `import.entityTypes.*` keys.
 */

function WarningList({ warnings }: { warnings: ImportWarningDto[] }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  if (warnings.length === 0) {
    return <p className="joi-empty__state">{t("import.report.noWarnings")}</p>;
  }

  return (
    <div>
      <button
        type="button"
        onClick={() => setOpen((prev) => !prev)}
        aria-expanded={open}
        className="joi-btn joi-btn--secondary rounded-md border px-3 py-1.5 text-sm font-medium"
      >
        {t(open ? "import.report.hideWarnings" : "import.report.showWarnings", {
          count: warnings.length,
        })}
      </button>
      {open ? (
        <ul className="mt-3 flex flex-col gap-2">
          {warnings.map((warning, index) => (
            <ImportWarningRow key={`${warning.source_path}-${index}`} warning={warning} />
          ))}
        </ul>
      ) : null}
    </div>
  );
}

type ReviewAction = "accept" | "reject" | "edit";

/** Entity types a reviewer may assign to an item, plus "ignore" to skip it. */
export const REVIEW_DECISION_OPTIONS = [
  "journal_entry",
  "therapy_record",
  "task",
  "goal",
  "project",
  "note",
  "ignore",
] as const;

export type ReviewDecisionValue = (typeof REVIEW_DECISION_OPTIONS)[number];

function ReviewItemRow({
  item,
  onAction,
  decision,
  onDecisionChange,
}: {
  item: ImportReviewItemDto;
  onAction: (item: ImportReviewItemDto, action: ReviewAction) => void;
  decision?: ReviewDecisionValue;
  onDecisionChange?: (item: ImportReviewItemDto, value: ReviewDecisionValue) => void;
}) {
  const { t } = useTranslation();
  const title = item.title ?? t("import.report.untitled");

  return (
    <li className="joi-surface--base joi-border--default rounded-lg border px-3 py-2">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="min-w-0">
          <p className="joi-text truncate text-sm font-medium">{title}</p>
          {/* The source path and the excerpt come from the legacy repository
              and are rendered verbatim; only the chrome translates. */}
          <p className="joi-text--muted truncate font-mono text-xs">{item.source_path}</p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <span className="joi-badge joi-badge--neutral">
            {isKnownEntityType(item.proposed_entity_type)
              ? t(`import.entityTypes.${item.proposed_entity_type}`)
              : item.proposed_entity_type}
          </span>
          <span className={`joi-badge ${CONFIDENCE_STYLES[item.confidence]}`}>
            {t(`import.confidence.${item.confidence}`)}
          </span>
        </div>
      </div>
      {item.snippet ? (
        <p className="joi-text--muted mt-1 text-sm italic">&ldquo;{item.snippet}&rdquo;</p>
      ) : null}
      <div className="mt-2 flex gap-2">
        {onDecisionChange ? (
          <label className="joi-text--muted flex items-center gap-2 text-xs font-medium">
            {t("import.report.decision")}
            <select
              value={decision ?? ""}
              onChange={(e) => onDecisionChange(item, e.target.value as ReviewDecisionValue)}
              aria-label={t("import.report.decisionAria", { title })}
              className="joi-field__select w-auto px-2 py-1 text-xs"
            >
              <option value="">{t("import.report.decisionSelect")}</option>
              {REVIEW_DECISION_OPTIONS.map((option) => (
                <option key={option} value={option}>
                  {option === "ignore"
                    ? t("import.decisions.ignore")
                    : t(`import.entityTypes.${option}`)}
                </option>
              ))}
            </select>
          </label>
        ) : (
          <>
            <button
              type="button"
              onClick={() => onAction(item, "accept")}
              className="joi-btn joi-btn--primary rounded-md px-2 py-1 text-xs font-medium"
            >
              {t("import.report.accept")}
            </button>
            <button
              type="button"
              onClick={() => onAction(item, "reject")}
              className="joi-btn joi-btn--secondary rounded-md px-2 py-1 text-xs font-medium"
            >
              {t("import.report.reject")}
            </button>
            <button
              type="button"
              onClick={() => onAction(item, "edit")}
              className="joi-btn joi-btn--secondary rounded-md px-2 py-1 text-xs font-medium"
            >
              {t("common.actions.edit")}
            </button>
          </>
        )}
      </div>
    </li>
  );
}

interface ImportReportViewProps {
  report: ImportReportDto;
  decisions?: Record<string, ReviewDecisionValue>;
  onDecisionChange?: (item: ImportReviewItemDto, value: ReviewDecisionValue) => void;
}

const KNOWN_BATCH_STATUSES = ["queued", "running", "completed", "failed", "cancelled"] as const;
type KnownBatchStatus = (typeof KNOWN_BATCH_STATUSES)[number];

function isKnownBatchStatus(value: string): value is KnownBatchStatus {
  return (KNOWN_BATCH_STATUSES as readonly string[]).includes(value);
}

function ImportReportView({ report, decisions, onDecisionChange }: ImportReportViewProps) {
  const { t } = useTranslation();
  const [notice, setNotice] = useState<string | null>(null);

  const statusLabel = isKnownBatchStatus(report.status)
    ? t(`common.labels.jobStatus.${report.status}`)
    : report.status;

  const handleReviewAction = (item: ImportReviewItemDto, action: ReviewAction) => {
    console.info("[import] review action placeholder", { itemId: item.id, action });
    setNotice(t("import.report.placeholderNotice"));
  };

  return (
    <section aria-label={t("import.report.aria", { sourceRoot: report.source_root })}>
      <h2 className="text-lg font-medium">{t("import.report.title")}</h2>
      <p className="joi-text--muted mt-1 text-sm">
        {t("import.report.source")} <span className="font-mono">{report.source_root}</span> ·{" "}
        {t("import.report.status")} {statusLabel}
      </p>

      {notice ? (
        <p role="status" className="joi-text--success mt-3 text-sm">
          {notice}
        </p>
      ) : null}

      <div className="mt-4">
        <h3 className="joi-text text-sm font-medium">{t("import.report.summary")}</h3>
        {report.counts.length === 0 ? (
          <p className="joi-empty__state mt-2">{t("import.report.nothingImported")}</p>
        ) : (
          <dl className="mt-2 grid grid-cols-2 gap-3 sm:grid-cols-3">
            {report.counts.map((count) => (
              <div
                key={count.entity_type}
                className="joi-surface--base joi-border--default rounded-lg border px-3 py-2"
              >
                <dt className="joi-text--muted text-xs font-medium">
                  {isKnownEntityType(count.entity_type)
                    ? t(`import.entityTypes.${count.entity_type}`)
                    : count.entity_type}
                </dt>
                <dd className="joi-text mt-1 text-2xl font-semibold tabular-nums">
                  {formatNumber(count.count)}
                </dd>
              </div>
            ))}
          </dl>
        )}
      </div>

      <div className="mt-6">
        <h3 className="joi-text text-sm font-medium">{t("import.report.warnings")}</h3>
        <div className="mt-2">
          <WarningList warnings={report.warnings} />
        </div>
      </div>

      <div className="mt-6">
        <h3 className="joi-text text-sm font-medium">
          {t("import.report.reviewItems", { count: report.review_items.length })}
        </h3>
        {report.review_items.length === 0 ? (
          <p className="joi-empty__state mt-2">{t("import.report.reviewEmpty")}</p>
        ) : (
          <ul className="mt-2 flex flex-col gap-2">
            {report.review_items.map((item) => (
              <ReviewItemRow
                key={item.id}
                item={item}
                onAction={handleReviewAction}
                decision={decisions?.[item.id]}
                onDecisionChange={onDecisionChange}
              />
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}

export default ImportReportView;
