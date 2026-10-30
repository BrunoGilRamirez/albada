import { describe, expect, it } from "vitest";
import { normalizeMarkdownHeadings } from "@/lib/markdownHeadings";

describe("normalizeMarkdownHeadings", () => {
  it("demotes a lone H1 to H2", () => {
    expect(normalizeMarkdownHeadings("# Title\nbody")).toBe("## Title\nbody");
  });

  it("demotes every level by one and caps H6", () => {
    const input = ["# a", "## b", "### c", "#### d", "##### e", "###### f"].join("\n");
    const expected = ["## a", "### b", "#### c", "##### d", "###### e", "###### f"].join("\n");
    expect(normalizeMarkdownHeadings(input)).toBe(expected);
  });

  it("ignores headings inside fenced code blocks", () => {
    const input = ["```", "# not a heading", "```", "# real"].join("\n");
    const expected = ["```", "# not a heading", "```", "## real"].join("\n");
    expect(normalizeMarkdownHeadings(input)).toBe(expected);
  });

  it("leaves non-heading hash usages untouched", () => {
    const input = ["#tag without space", "    # indented code", "plain # inline"].join("\n");
    expect(normalizeMarkdownHeadings(input)).toBe(input);
  });

  it("preserves indentation and trailing text", () => {
    expect(normalizeMarkdownHeadings("  ## Sub  ")).toBe("  ### Sub  ");
  });
});
