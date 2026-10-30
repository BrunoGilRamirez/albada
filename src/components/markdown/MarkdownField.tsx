/**
 * Reusable Markdown form field: pairs the existing `MarkdownEditor` with a
 * live `MarkdownPreview` side by side, with synchronized scrolling.
 *
 * The value is fully controlled and never transformed here: the editor and
 * preview always show the same verbatim Markdown source the user typed.
 * Raw HTML stays disabled because rendering goes through `MarkdownPreview`.
 *
 * Layout: one column when narrow (the preview stacks below the editor) and
 * two columns from the `xl` breakpoint up. The preview pane scrolls inside a
 * bounded region so long documents do not stretch the form.
 *
 * Accessibility: the textarea is labelled by the field `label` (via
 * `MarkdownEditor`), and the preview pane is a named region labelled
 * "<label> preview" by default, so each paired field in a multi-field form
 * exposes a unique accessible name. Translated screens pass localized preview
 * strings instead (see `previewRegionLabel` below).
 *
 * Props:
 *   id: string - textarea id; also anchors the label association
 *   label: string - visible label, e.g. "Description (Markdown)"
 *   value: string - current Markdown source
 *   onChange: (value: string) => void - change handler
 *   rows?: number - initial visible rows for the textarea
 *   minHeight?: number - minimum height in pixels for editor and preview
 *   maxHeight?: number - maximum height before scrolling (default: 560)
 *   previewLabel?: string - localized visible "Preview" pane label
 *   previewRegionLabel?: string - localized preview region accessible name
 *   previewEmptyText?: string - localized empty-preview placeholder
 *   editorChrome?: MarkdownEditorChrome - localized editor toolbar/heading
 *     chrome, built from the catalog by `useMarkdownEditorChrome` below
 *
 * The preview props and `editorChrome` are opt-in so already-translated
 * screens can pass catalog copy (e.g. `common.markdown.*` from the
 * Goals/Projects/Tasks forms) while surfaces whose feature area is not
 * localized yet keep the original English defaults unchanged.
 */
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import MarkdownEditor, { HEADING_LEVELS, type MarkdownEditorChrome } from "./MarkdownEditor";
import MarkdownPreview from "./MarkdownPreview";

interface MarkdownFieldProps {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  rows?: number;
  minHeight?: number;
  maxHeight?: number;
  previewLabel?: string;
  previewRegionLabel?: string;
  previewEmptyText?: string;
  editorChrome?: MarkdownEditorChrome;
}

const DEFAULT_MIN_HEIGHT = 360;
const DEFAULT_MAX_HEIGHT = 720;
const DEFAULT_PREVIEW_LABEL = "Preview";

/**
 * Builds the localized `MarkdownEditor` chrome from the shared
 * `common.markdown` catalog keys for screens whose feature area is already
 * translated (the 8.6.9 Goal/Project/Task forms and the 8.6.10 Journal/Therapy
 * forms). Localized callers pass the result into `MarkdownField` as
 * `editorChrome`. Surfaces in areas that are not translated yet must not call
 * this hook: the editor then keeps its built-in default strings byte-for-byte.
 * The heading option labels interpolate the level number and its literal
 * Markdown markers (`#`.repeat(level)) — the inserted source text itself is
 * application-generated Markdown content and is never localized here.
 */
export function useMarkdownEditorChrome(): MarkdownEditorChrome {
  const { t } = useTranslation();
  return {
    toolbarLabel: t("common.markdown.toolbarLabel"),
    bold: t("common.markdown.bold"),
    italic: t("common.markdown.italic"),
    link: t("common.markdown.link"),
    list: t("common.markdown.list"),
    headingSelect: t("common.markdown.headingSelect"),
    headingPlaceholder: t("common.markdown.headingPlaceholder"),
    headingOptions: HEADING_LEVELS.map((level) =>
      t("common.markdown.headingOption", { level, markers: "#".repeat(level) }),
    ),
    headingHelp: t("common.markdown.headingHelp"),
    headingHelpTitle: t("common.markdown.headingHelpTitle"),
    viewModeLabel: t("common.markdown.viewModeLabel"),
    viewSplit: t("common.markdown.viewSplit"),
    viewEditor: t("common.markdown.viewEditor"),
    viewPreview: t("common.markdown.viewPreview"),
    dictionaryAdd: t("common.markdown.dictionaryAdd"),
    dictionaryAdded: t("common.markdown.dictionaryAdded"),
    dictionaryError: t("common.markdown.dictionaryError"),
  };
}

function MarkdownField({
  id,
  label,
  value,
  onChange,
  rows,
  minHeight,
  maxHeight,
  previewLabel = DEFAULT_PREVIEW_LABEL,
  previewRegionLabel = `${label} preview`,
  previewEmptyText,
  editorChrome,
}: MarkdownFieldProps) {
  const editorTextareaRef = useRef<HTMLTextAreaElement | null>(null);
  const previewRef = useRef<HTMLDivElement | null>(null);
  const syncing = useRef<"editor" | "preview" | null>(null);
  const effectiveMinHeight = minHeight ?? DEFAULT_MIN_HEIGHT;
  const effectiveMaxHeight = maxHeight ?? DEFAULT_MAX_HEIGHT;
  const [viewMode, setViewMode] = useState<"split" | "editor" | "preview">("split");
  const [matchedHeight, setMatchedHeight] = useState(effectiveMinHeight);

  useEffect(() => {
    if (viewMode !== "preview") editorTextareaRef.current?.focus();
  }, [viewMode]);

  useLayoutEffect(() => {
    const textarea = editorTextareaRef.current;
    if (!textarea || viewMode === "preview") return;
    const height = Math.min(
      effectiveMaxHeight,
      Math.max(effectiveMinHeight, textarea.getBoundingClientRect().height),
    );
    setMatchedHeight((current) => (current === height ? current : height));
  }, [effectiveMaxHeight, effectiveMinHeight, value, viewMode]);

  const chrome = {
    viewModeLabel: "Markdown view",
    viewSplit: "Split",
    viewEditor: "Editor only",
    viewPreview: "Preview only",
    ...editorChrome,
  };

  const syncScroll = (source: "editor" | "preview") => {
    if (syncing.current) return;
    const editor = editorTextareaRef.current;
    const preview = previewRef.current;
    if (!editor || !preview) return;
    const from = source === "editor" ? editor : preview;
    const to = source === "editor" ? preview : editor;
    const maxFrom = from.scrollHeight - from.clientHeight;
    const maxTo = to.scrollHeight - to.clientHeight;
    if (maxFrom <= 0 || maxTo <= 0) return;
    const ratio = from.scrollTop / maxFrom;
    syncing.current = source;
    to.scrollTop = ratio * maxTo;
    requestAnimationFrame(() => {
      syncing.current = null;
    });
  };

  const handleEditorScroll = () => syncScroll("editor");
  const handlePreviewScroll = () => syncScroll("preview");

  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="joi-field__label">{chrome.viewModeLabel}</span>
        <div className="flex flex-wrap gap-1" role="group" aria-label={chrome.viewModeLabel}>
          {(["split", "editor", "preview"] as const).map((mode) => {
            const labelByMode = {
              split: chrome.viewSplit,
              editor: chrome.viewEditor,
              preview: chrome.viewPreview,
            };
            return (
              <button
                key={mode}
                type="button"
                className="joi-btn joi-btn--secondary joi-btn--sm"
                aria-pressed={viewMode === mode}
                onClick={() => setViewMode(mode)}
              >
                {labelByMode[mode]}
              </button>
            );
          })}
        </div>
      </div>
      <div
        className={
          viewMode === "split"
            ? "grid items-stretch gap-4 xl:grid-cols-2"
            : "grid items-stretch gap-4 grid-cols-1"
        }
      >
        <div className={viewMode === "preview" ? "hidden" : undefined}>
          <MarkdownEditor
            id={id}
            label={label}
            value={value}
            onChange={onChange}
            rows={rows}
            minHeight={effectiveMinHeight}
            maxHeight={effectiveMaxHeight}
            textareaRef={editorTextareaRef}
            onScroll={handleEditorScroll}
            chrome={editorChrome}
          />
        </div>
        <div className={viewMode === "editor" ? "hidden" : "flex flex-col gap-1"}>
          <span className="block joi-field__label">{previewLabel}</span>
          <div aria-hidden="true" className="hidden flex-wrap gap-1 xl:flex invisible select-none">
            <span className="joi-border--default rounded border px-2 py-1 text-xs font-medium">
              &nbsp;
            </span>
          </div>
          <div
            ref={previewRef}
            role="region"
            aria-label={previewRegionLabel}
            tabIndex={0}
            onScroll={handlePreviewScroll}
            className="joi-border--default flex-1 min-h-0 overflow-y-auto rounded-lg border p-3"
            style={{
              height: `${matchedHeight}px`,
              minHeight: `${effectiveMinHeight}px`,
              maxHeight: `${effectiveMaxHeight}px`,
            }}
          >
            <MarkdownPreview content={value} emptyText={previewEmptyText} />
          </div>
        </div>
      </div>
    </div>
  );
}

export default MarkdownField;
