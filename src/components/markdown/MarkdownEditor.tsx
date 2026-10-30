/**
 * Markdown editor textarea with auto-resize, format buttons, and heading dropdown.
 *
 * Props:
 *   value: string - current textarea value
 *   onChange: (value: string) => void - change handler
 *   label?: string - optional label above the editor
 *   rows?: number - initial visible rows (default: 8)
 *   minHeight?: number - minimum height in pixels when auto-resizing (default: rows height)
 *   maxHeight?: number - maximum height in pixels before the textarea scrolls (default: 560)
 *   autoResize?: boolean - whether auto-resize is enabled (default: true)
 *   id?: string - optional textarea id attribute
 *   textareaRef?: React.Ref<HTMLTextAreaElement> - optional ref to the textarea element
 *   onScroll?: React.UIEventHandler<HTMLTextAreaElement> - scroll event handler for the textarea
 *   chrome?: MarkdownEditorChrome - localized UI chrome for the format toolbar,
 *     heading selector, and heading help button. Omitted callers keep the
 *     built-in `DEFAULT_CHROME` strings below byte-identically.
 */
import { useLayoutEffect, useRef } from "react";
import type { KeyboardEvent, Ref, UIEventHandler } from "react";
import { useState } from "react";
import { addWordToNativeDictionary } from "@/lib/api/markdownApi";

/**
 * Visible/accessible chrome strings for the editor toolbar and heading
 * controls. This is application-owned UI copy, never user-authored Markdown.
 * Localized screens build a complete object (see `useMarkdownEditorChrome` in
 * `MarkdownField`) and pass it as `chrome`; screens whose feature area is not
 * translated yet omit it and keep the exact strings the editor hardcoded
 * before this opt-in existed.
 */
export interface MarkdownEditorChrome {
  /** Accessible name of the format toolbar. */
  toolbarLabel: string;
  /** Visible format-button labels. */
  bold: string;
  italic: string;
  link: string;
  list: string;
  /** Accessible name of the heading-level selector. */
  headingSelect: string;
  /** Visible label of the disabled placeholder option in the selector. */
  headingPlaceholder: string;
  /** Visible option labels for heading levels 2-6, in `HEADING_LEVELS` order. */
  headingOptions: string[];
  /** Accessible name and visible tooltip of the heading help button. */
  headingHelp: string;
  headingHelpTitle: string;
  viewModeLabel?: string;
  viewSplit?: string;
  viewEditor?: string;
  viewPreview?: string;
  dictionaryAdd?: string;
  dictionaryAdded?: string;
  dictionaryError?: string;
}

/** Heading levels the selector offers, in option order. */
export const HEADING_LEVELS = [2, 3, 4, 5, 6];

/**
 * Built-in chrome for surfaces that have not opted in to localization yet.
 * Every value is byte-identical to the string the editor hardcoded before the
 * `chrome` prop existed, including the original Spanish-only heading-help
 * tooltip, so non-opted-in callers (e.g. the Custom export screens until
 * 8.6.11) render exactly as before; localized areas — Goals/Projects/Tasks
 * (8.6.9) and Journal/Therapy (8.6.10) — pass complete catalog chrome instead.
 */
const DEFAULT_CHROME: MarkdownEditorChrome = {
  toolbarLabel: "Markdown formatting",
  bold: "Bold",
  italic: "Italic",
  link: "Link",
  list: "List",
  headingSelect: "Insert heading",
  headingPlaceholder: "Heading…",
  headingOptions: [
    "Heading 2 (##)",
    "Heading 3 (###)",
    "Heading 4 (####)",
    "Heading 5 (#####)",
    "Heading 6 (######)",
  ],
  headingHelp: "Heading help",
  headingHelpTitle:
    "No necesitas usar # para el título: el campo Title ya lo añade. Usa ## o niveles más profundos solo para subtítulos dentro del contenido.",
  dictionaryAdd: "Add to Windows dictionary",
  dictionaryAdded: "Added to Windows dictionary",
  dictionaryError: "Could not update Windows dictionary",
};

interface MarkdownEditorProps {
  value: string;
  onChange: (value: string) => void;
  label?: string;
  rows?: number;
  minHeight?: number;
  maxHeight?: number;
  autoResize?: boolean;
  id?: string;
  textareaRef?: Ref<HTMLTextAreaElement>;
  onScroll?: UIEventHandler<HTMLTextAreaElement>;
  chrome?: MarkdownEditorChrome;
}

/**
 * Propagates the next value through onChange, then restores focus and
 * selects [selStart, selEnd) so the user can immediately type over the
 * inserted placeholder (or wrapped selection).
 */
function commitNext(
  textarea: HTMLTextAreaElement,
  next: string,
  selStart: number,
  selEnd: number,
  onChange: (value: string) => void,
): void {
  onChange(next);
  requestAnimationFrame(() => {
    textarea.focus();
    textarea.setSelectionRange(selStart, selEnd);
  });
}

/**
 * Wraps the current selection with prefix/suffix. With no selection,
 * inserts the placeholder between prefix/suffix. Either way the affected
 * text ends up selected for convenient overwriting.
 */
function applyInlineFormat(
  textarea: HTMLTextAreaElement,
  prefix: string,
  suffix: string,
  placeholder: string,
  onChange: (value: string) => void,
): void {
  const { selectionStart, selectionEnd, value } = textarea;
  const hasSelection = selectionEnd > selectionStart;
  const selected = hasSelection ? value.slice(selectionStart, selectionEnd) : placeholder;
  const next =
    value.slice(0, selectionStart) + prefix + selected + suffix + value.slice(selectionEnd);
  const selStart = selectionStart + prefix.length;
  commitNext(textarea, next, selStart, selStart + selected.length, onChange);
}

const PAIRS: Record<string, string> = {
  "(": ")",
  "[": "]",
  "{": "}",
  "'": "'",
  '"': '"',
  "`": "`",
};

function currentLine(value: string, start: number, end: number) {
  const lineStart = value.lastIndexOf("\n", start - 1) + 1;
  const lineEndIndex = value.indexOf("\n", end);
  return { lineStart, lineEnd: lineEndIndex === -1 ? value.length : lineEndIndex };
}

function indentBlock(
  textarea: HTMLTextAreaElement,
  amount: 2 | -2,
  onChange: (value: string) => void,
) {
  const { selectionStart, selectionEnd, value } = textarea;
  const { lineStart, lineEnd } = currentLine(value, selectionStart, selectionEnd);
  const block = value.slice(lineStart, lineEnd);
  const lines = block.split("\n");
  const changed = lines
    .map((line) => {
      if (!line.trim()) return line;
      if (amount > 0) return `  ${line}`;
      return line.replace(/^ {1,2}/, "");
    })
    .join("\n");
  const delta = changed.length - block.length;
  commitNext(
    textarea,
    value.slice(0, lineStart) + changed + value.slice(lineEnd),
    Math.max(lineStart, selectionStart + (amount > 0 ? 2 : Math.min(0, delta))),
    Math.max(lineStart, selectionEnd + delta),
    onChange,
  );
}

function continueMarkdownLine(textarea: HTMLTextAreaElement, onChange: (value: string) => void) {
  const { selectionStart, selectionEnd, value } = textarea;
  if (selectionStart !== selectionEnd) return false;
  const { lineStart, lineEnd } = currentLine(value, selectionStart, selectionEnd);
  const line = value.slice(lineStart, lineEnd);
  const unordered = line.match(/^(\s*)([-+*])\s+(.*)$/);
  const ordered = line.match(/^(\s*)(\d+)([.)])\s+(.*)$/);
  const quote = line.match(/^(\s*)>\s?(.*)$/);
  const match = unordered ?? ordered ?? quote;
  if (!match) return false;
  const prefix = unordered
    ? `${unordered[1]}${unordered[2]} `
    : ordered
      ? `${ordered[1]}${Number(ordered[2]) + 1}${ordered[3]} `
      : `${quote?.[1] ?? ""}> `;
  const content = match[match.length - 1];
  if (!content.trim()) {
    if (quote) {
      const next = value.slice(0, lineStart) + value.slice(lineEnd);
      commitNext(textarea, next, lineStart, lineStart, onChange);
      return true;
    }
    const indentation = (unordered ?? ordered)?.[1] ?? "";
    if (indentation.length >= 2) {
      const parentIndent = indentation.slice(0, -2);
      const parentMarker = unordered
        ? `${parentIndent}${unordered[2]} `
        : `${parentIndent}1${ordered?.[3] ?? "."} `;
      const next = value.slice(0, lineStart) + parentMarker + value.slice(lineEnd);
      commitNext(
        textarea,
        next,
        lineStart + parentMarker.length,
        lineStart + parentMarker.length,
        onChange,
      );
    } else {
      const next = value.slice(0, lineStart) + value.slice(lineEnd);
      commitNext(textarea, next, lineStart, lineStart, onChange);
    }
    return true;
  }
  const insertAt = lineEnd;
  const next = value.slice(0, insertAt) + `\n${prefix}` + value.slice(insertAt);
  commitNext(textarea, next, insertAt + 1 + prefix.length, insertAt + 1 + prefix.length, onChange);
  return true;
}

function handlePairKey(
  textarea: HTMLTextAreaElement,
  key: string,
  onChange: (value: string) => void,
) {
  const closing = PAIRS[key];
  if (!closing) return false;
  const { selectionStart, selectionEnd, value } = textarea;
  if (selectionStart !== selectionEnd) {
    const next =
      value.slice(0, selectionStart) +
      key +
      value.slice(selectionStart, selectionEnd) +
      closing +
      value.slice(selectionEnd);
    commitNext(textarea, next, selectionStart + 1, selectionEnd + 1, onChange);
    return true;
  }
  if (value[selectionStart] === closing) {
    commitNext(textarea, value, selectionStart + 1, selectionStart + 1, onChange);
    return true;
  }
  const next = value.slice(0, selectionStart) + key + closing + value.slice(selectionEnd);
  commitNext(textarea, next, selectionStart + 1, selectionStart + 1, onChange);
  return true;
}

/**
 * Toggles list markers. With a multi-line selection, prefixes every
 * non-blank selected line with "- ". With a collapsed cursor, inserts a
 * new "- placeholder" item after the current line (or fills the current
 * blank line). The inserted placeholder is selected.
 */
function applyListFormat(
  textarea: HTMLTextAreaElement,
  placeholder: string,
  onChange: (value: string) => void,
): void {
  const { selectionStart, selectionEnd, value } = textarea;
  const lineStart = value.lastIndexOf("\n", selectionStart - 1) + 1;
  const endIdx = value.indexOf("\n", selectionEnd);
  const lineEnd = endIdx === -1 ? value.length : endIdx;

  if (selectionEnd === selectionStart) {
    const line = value.slice(lineStart, lineEnd);
    if (line.trim() === "") {
      const next = `${value.slice(0, lineStart)}- ${placeholder}${value.slice(lineEnd)}`;
      commitNext(textarea, next, lineStart + 2, lineStart + 2 + placeholder.length, onChange);
    } else {
      const insert = `\n- ${placeholder}`;
      const next = value.slice(0, lineEnd) + insert + value.slice(lineEnd);
      const selStart = lineEnd + 3;
      commitNext(textarea, next, selStart, selStart + placeholder.length, onChange);
    }
    return;
  }

  const block = value.slice(lineStart, lineEnd);
  const prefixed = block
    .split("\n")
    .map((line) => (line.trim() === "" ? line : `- ${line}`))
    .join("\n");
  const next = value.slice(0, lineStart) + prefixed + value.slice(lineEnd);
  commitNext(textarea, next, lineStart, lineStart + prefixed.length, onChange);
}

/**
 * Applies an ATX heading (##-######) to the line holding the cursor:
 * replaces an existing marker, keeps indentation, or fills a blank line
 * with a placeholder. The placeholder (or shifted selection) is selected.
 */
function insertHeading(
  textarea: HTMLTextAreaElement,
  level: number,
  placeholder: string,
  onChange: (value: string) => void,
): void {
  const { selectionStart, selectionEnd, value } = textarea;
  const marker = `${"#".repeat(level)} `;
  const lineStart = value.lastIndexOf("\n", selectionStart - 1) + 1;
  const endIdx = value.indexOf("\n", selectionEnd);
  const lineEnd = endIdx === -1 ? value.length : endIdx;
  const line = value.slice(lineStart, lineEnd);

  if (line.trim() === "") {
    const next = value.slice(0, lineStart) + marker + placeholder + value.slice(lineEnd);
    const selStart = lineStart + marker.length;
    commitNext(textarea, next, selStart, selStart + placeholder.length, onChange);
    return;
  }

  const existing = line.match(/^(\s{0,3})#{1,6}(?=\s|$)/);
  let nextLine: string;
  if (existing) {
    nextLine = existing[1] + marker + line.slice(existing[0].length).replace(/^\s+/, "");
  } else {
    const indent = line.match(/^\s*/)?.[0] ?? "";
    nextLine = `${indent}${marker}${line.slice(indent.length)}`;
  }
  const next = value.slice(0, lineStart) + nextLine + value.slice(lineEnd);
  const delta = nextLine.length - line.length;
  commitNext(textarea, next, selectionStart + delta, selectionEnd + delta, onChange);
}

function MarkdownEditor({
  value,
  onChange,
  label,
  rows = 8,
  minHeight,
  maxHeight = 560,
  autoResize = true,
  id,
  textareaRef,
  onScroll,
  chrome,
}: MarkdownEditorProps) {
  const chromeCopy = chrome ?? DEFAULT_CHROME;
  const internalTextareaRef = useRef<HTMLTextAreaElement | null>(null);
  const [selectedWord, setSelectedWord] = useState("");
  const [dictionaryState, setDictionaryState] = useState<"idle" | "added" | "error">("idle");

  const setTextareaRefs = (el: HTMLTextAreaElement | null) => {
    internalTextareaRef.current = el;
    if (!textareaRef) return;
    if (typeof textareaRef === "function") {
      textareaRef(el);
    } else {
      textareaRef.current = el;
    }
  };

  // Auto-resize: grow with content up to maxHeight, then scroll internally.
  useLayoutEffect(() => {
    const el = internalTextareaRef.current;
    if (!el || !autoResize) return;
    el.style.height = "auto";
    const floor = minHeight ?? el.scrollHeight;
    const target = Math.max(el.scrollHeight, floor);
    if (target > maxHeight) {
      el.style.overflowY = "auto";
      el.style.height = `${maxHeight}px`;
    } else {
      el.style.overflowY = "hidden";
      el.style.height = `${target}px`;
    }
  }, [value, minHeight, maxHeight, autoResize]);
  const applyFormat = (prefix: string, suffix: string, placeholder: string) => {
    const textarea = internalTextareaRef.current;
    if (!textarea) return;
    applyInlineFormat(textarea, prefix, suffix, placeholder, onChange);
  };

  // The placeholder strings these handlers insert ("bold text", "italic
  // text", "cursor here", "list item", "Heading") become part of the
  // user's Markdown source, so they are application-generated content,
  // not UI chrome: they stay byte-identical in every locale.
  const applyList = () => {
    const textarea = internalTextareaRef.current;
    if (!textarea) return;
    applyListFormat(textarea, "list item", onChange);
  };

  const applyHeading = (level: number) => {
    const textarea = internalTextareaRef.current;
    if (!textarea) return;
    insertHeading(textarea, level, "Heading", onChange);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    const textarea = event.currentTarget;
    const modifier = event.ctrlKey || event.metaKey;
    if (modifier && ["b", "i", "k"].includes(event.key.toLowerCase())) {
      event.preventDefault();
      if (event.key.toLowerCase() === "b") applyFormat("**", "**", "bold text");
      else if (event.key.toLowerCase() === "i") applyFormat("*", "*", "italic text");
      else applyFormat("[", "](link_here)", "cursor here");
      return;
    }
    if (event.key === "Enter" && continueMarkdownLine(textarea, onChange)) {
      event.preventDefault();
      return;
    }
    if (event.key === "Tab") {
      event.preventDefault();
      indentBlock(textarea, event.shiftKey ? -2 : 2, onChange);
      return;
    }
    if (event.key === "Backspace" && textarea.selectionStart === textarea.selectionEnd) {
      const { selectionStart } = textarea;
      if (PAIRS[textarea.value[selectionStart - 1]] === textarea.value[selectionStart]) {
        event.preventDefault();
        const next =
          textarea.value.slice(0, selectionStart - 1) + textarea.value.slice(selectionStart + 1);
        commitNext(textarea, next, selectionStart - 1, selectionStart - 1, onChange);
        return;
      }
    }
    if (!modifier && PAIRS[event.key]) {
      event.preventDefault();
      handlePairKey(textarea, event.key, onChange);
    }
  };

  const buttonClass = "joi-btn joi-btn--secondary joi-btn--sm";

  return (
    <div className="flex flex-col gap-1">
      {label ? (
        <label htmlFor={id ?? "markdown-editor"} className="joi-field__label">
          {label}
        </label>
      ) : null}
      <div className="flex flex-wrap items-center gap-1" aria-label={chromeCopy.toolbarLabel}>
        <button
          type="button"
          className={buttonClass}
          onClick={() => applyFormat("**", "**", "bold text")}
        >
          {chromeCopy.bold}
        </button>
        <button
          type="button"
          className={buttonClass}
          onClick={() => applyFormat("*", "*", "italic text")}
        >
          {chromeCopy.italic}
        </button>
        <button
          type="button"
          className={buttonClass}
          onClick={() => applyFormat("[", "](link_here)", "cursor here")}
        >
          {chromeCopy.link}
        </button>
        <button type="button" className={buttonClass} onClick={applyList}>
          {chromeCopy.list}
        </button>
        <select
          aria-label={chromeCopy.headingSelect}
          defaultValue=""
          className={buttonClass}
          onChange={(event) => {
            const level = Number(event.target.value);
            if (level >= 2 && level <= 6) {
              applyHeading(level);
            }
            event.target.value = "";
          }}
        >
          <option value="" disabled>
            {chromeCopy.headingPlaceholder}
          </option>
          {HEADING_LEVELS.map((level, index) => (
            <option key={level} value={String(level)}>
              {chromeCopy.headingOptions[index]}
            </option>
          ))}
        </select>
        <button
          type="button"
          aria-label={chromeCopy.headingHelp}
          title={chromeCopy.headingHelpTitle}
          className={buttonClass}
        >
          i
        </button>
        {selectedWord ? (
          <button
            type="button"
            className={buttonClass}
            disabled={dictionaryState === "added"}
            onClick={() => {
              void addWordToNativeDictionary(selectedWord, "en-US")
                .then(() => setDictionaryState("added"))
                .catch(() => setDictionaryState("error"));
            }}
          >
            {dictionaryState === "added"
              ? chromeCopy.dictionaryAdded
              : dictionaryState === "error"
                ? chromeCopy.dictionaryError
                : chromeCopy.dictionaryAdd}
          </button>
        ) : null}
      </div>
      <textarea
        ref={setTextareaRefs}
        id={id ?? "markdown-editor"}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        rows={rows}
        className="joi-field__textarea w-full min-w-0 resize-y p-3 font-mono"
        style={{ minHeight: minHeight ? `${minHeight}px` : undefined, maxHeight: `${maxHeight}px` }}
        onScroll={onScroll}
        onKeyDown={onKeyDown}
        spellCheck
        lang="en-US"
        onSelect={(event) => {
          const target = event.currentTarget;
          const selected = target.value.slice(target.selectionStart, target.selectionEnd).trim();
          setSelectedWord(/^\S+$/.test(selected) ? selected : "");
          setDictionaryState("idle");
        }}
      />
    </div>
  );
}

export default MarkdownEditor;
