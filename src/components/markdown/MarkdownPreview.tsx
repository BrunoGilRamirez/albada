import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

interface MarkdownPreviewProps {
  content: string | null;
  className?: string;
  /**
   * Optional visible text shown when there is no content yet. Callers pass a
   * translated value (e.g. `common.markdown.empty`) when their surface is
   * localized; the default keeps the existing English placeholder for
   * surfaces that have not been translated yet.
   */
  emptyText?: string;
}

const DEFAULT_EMPTY_TEXT = "No content yet.";

/**
 * Safe Markdown rendering.
 *
 * Raw HTML is disabled by default (`skipHtml`), so user-authored text cannot
 * inject markup into the DOM. GFM tables and strikethrough are enabled.
 */
function MarkdownPreview({
  content,
  className,
  emptyText = DEFAULT_EMPTY_TEXT,
}: MarkdownPreviewProps) {
  if (!content || content.trim().length === 0) {
    return <p className="joi-empty__state">{emptyText}</p>;
  }
  return (
    <div className={`joi-markdown prose prose-sm max-w-none dark:prose-invert ${className ?? ""}`}>
      <ReactMarkdown remarkPlugins={[remarkGfm]} skipHtml>
        {content}
      </ReactMarkdown>
    </div>
  );
}

export default MarkdownPreview;
