/**
 * Normalizes Markdown ATX headings by demoting each level by one
 * (`#` -> `##`, …, `#####` -> `######`, `######` stays). The entry Title
 * field already provides the H1, so body content must not introduce one;
 * demoting preserves the relative hierarchy of every section.
 *
 * Lines inside fenced code blocks (```) and lines without a space/tab
 * after the hashes (e.g. `#tag`) are left untouched.
 *
 * @param markdown - The markdown source to normalize.
 * @returns The markdown with headings demoted by one level.
 */
export function normalizeMarkdownHeadings(markdown: string): string {
  const lines = markdown.split("\n");
  let inFence = false;

  return lines
    .map((line) => {
      if (line.trimStart().startsWith("```")) {
        inFence = !inFence;
        return line;
      }
      if (inFence) return line;

      const match = line.match(/^([ ]{0,3})(#{1,6})([ \t])(.*)$/);
      if (!match) return line;

      const [, indent, hashes, separator, rest] = match;
      const demoted = "#".repeat(Math.min(6, hashes.length + 1));
      return `${indent}${demoted}${separator}${rest}`;
    })
    .join("\n");
}
