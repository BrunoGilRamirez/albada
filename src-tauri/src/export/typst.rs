//! Deterministic Markdown-AST -> Typst serializer.
//!
//! This module turns the constrained [`crate::export::ast`] tree into a single
//! Typst source string. It is the pure conversion layer: it never reads or
//! writes files, never executes HTML/scripts/macros, and performs no network
//! access. Unsupported content is rendered as inert, escaped text and reported
//! through a structured warning.
//!
//! # Escaping model
//!
//! Text is escaped against the Typst markup metacharacters (`\ # $ * _ ` < >
//! @ [ ] { } ~ = - + / ' "`) so user-authored content can never start a code
//! expression, a heading, a list, raw text, a label, a reference, or a symbol
//! shorthand. URLs, filenames, link titles, image alt text, and code are
//! placed inside Typst string literals and escaped with
//! [`escape_string`]. Code is always emitted through `#raw` (or a single
//! backtick span) so it is never interpreted as Typst markup.
//!
//! # Known limitations
//!
//! - Typst symbol shorthand for `...` (ellipsis) is intentionally not escaped;
//!   it renders as the `…` glyph. All dash and arrow shorthands are neutralized
//!   because their metacharacters are escaped.
//! - Ordered-list start numbers other than 1 are reported but rendered from 1.
//! - Local image availability and filesystem path validation belong to the
//!   later compiler adapter (7.10.4); this layer only classifies schemes.

use crate::export::ast::{
    Alignment, Block, CodeBlock, ConversionWarning, Document, Heading, Image, Inline, Link, List,
    Table, WarningCode,
};
use crate::export::parser::parse_markdown;

/// The result of converting a Markdown source into Typst.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionOutput {
    /// The deterministic Typst source.
    pub typst: String,
    /// Structured, content-safe warnings.
    pub warnings: Vec<ConversionWarning>,
}

/// Parse Markdown and serialize it to Typst in one step.
///
/// This is the primary entry point for the later compiler adapter.
pub fn convert_markdown(source: &str) -> ConversionOutput {
    let parsed = parse_markdown(source);
    let mut rendered = render_typst(&parsed.document);

    let mut warnings = parsed.warnings;
    warnings.append(&mut rendered.warnings);
    rendered.warnings = dedup_warnings(warnings);
    rendered
}

/// Convert a selected journal snapshot, placing each generated journal entry
/// after the first on a new page. Generic and therapy exports use
/// [`convert_markdown`] and therefore keep their existing flow.
pub fn convert_diary_markdown(source: &str) -> ConversionOutput {
    let parsed = parse_markdown(source);
    let mut warnings = parsed.warnings;
    let mut parts = Vec::with_capacity(parsed.document.blocks.len());
    let mut found_entry = false;

    for (index, block) in parsed.document.blocks.iter().enumerate() {
        if is_journal_entry_start(&parsed.document.blocks, index) {
            if found_entry {
                parts.push("#pagebreak()".to_string());
            }
            found_entry = true;
        }
        parts.push(render_block_to_string(block, &mut warnings));
    }

    let joined = parts.join("\n\n");
    let typst = if joined.is_empty() {
        String::new()
    } else {
        format!("{joined}\n")
    };

    ConversionOutput {
        typst,
        warnings: dedup_warnings(warnings),
    }
}

/// Journal snapshots wrap each selected entry in an H2 followed immediately
/// by an italic `Date: YYYY-MM-DD` paragraph. Requiring the generated date
/// wrapper prevents ordinary H2 sections from becoming page boundaries.
fn is_journal_entry_start(blocks: &[Block], index: usize) -> bool {
    let is_wrapper_heading =
        matches!(blocks.get(index), Some(Block::Heading(heading)) if heading.level == 2);
    if !is_wrapper_heading {
        return false;
    }

    let has_generated_date = matches!(
        blocks.get(index + 1),
        Some(Block::Paragraph(paragraph)) if is_generated_date_line(&paragraph.content)
    );
    has_generated_date && (index == 0 || matches!(blocks.get(index - 1), Some(Block::Separator)))
}

fn is_generated_date_line(content: &[Inline]) -> bool {
    let [Inline::Emphasis(emphasis)] = content else {
        return false;
    };
    let [Inline::Text(text)] = emphasis.as_slice() else {
        return false;
    };
    let Some(date) = text.strip_prefix("Date: ") else {
        return false;
    };
    let bytes = date.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

/// Serialize an already-parsed AST to Typst.
pub fn render_typst(document: &Document) -> ConversionOutput {
    let mut warnings = Vec::new();
    let mut out = String::new();
    render_document(&mut out, &mut warnings, document);
    let warnings = dedup_warnings(warnings);
    ConversionOutput {
        typst: out,
        warnings,
    }
}

fn render_document(out: &mut String, warnings: &mut Vec<ConversionWarning>, document: &Document) {
    let mut parts = Vec::with_capacity(document.blocks.len());
    for block in &document.blocks {
        parts.push(render_block_to_string(block, warnings));
    }
    let joined = parts.join("\n\n");
    out.push_str(&joined);
    if !joined.is_empty() {
        out.push('\n');
    }
}

/// Render a block to a string without a trailing newline and without leading
/// indentation. Nested constructs produce their own internal indentation
/// relative to their immediate parent.
fn render_block_to_string(block: &Block, warnings: &mut Vec<ConversionWarning>) -> String {
    match block {
        Block::Heading(heading) => render_heading(heading, warnings),
        Block::Paragraph(paragraph) => render_inlines(&paragraph.content, warnings),
        Block::List(list) => render_list(list, warnings),
        Block::Table(table) => render_table(table, warnings),
        Block::BlockQuote(quote) => render_blockquote(quote, warnings),
        Block::CodeBlock(code) => render_code_block(code),
        Block::Separator => "#line(length: 100%)".to_string(),
        Block::Html(html) => {
            warn_html(warnings, &html.raw);
            escape_text(html.raw.trim_end())
        }
    }
}

fn render_heading(heading: &Heading, warnings: &mut Vec<ConversionWarning>) -> String {
    let mut out = "=".repeat(heading.level as usize);
    out.push(' ');
    out.push_str(&render_inlines(&heading.content, warnings));
    out
}

fn render_inlines(inlines: &[Inline], warnings: &mut Vec<ConversionWarning>) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) => out.push_str(&escape_text(text)),
            Inline::Emphasis(content) => {
                out.push('_');
                out.push_str(&render_inlines(content, warnings));
                out.push('_');
            }
            Inline::Strong(content) => {
                out.push('*');
                out.push_str(&render_inlines(content, warnings));
                out.push('*');
            }
            Inline::Strikethrough(content) => {
                out.push_str("#strike[");
                out.push_str(&render_inlines(content, warnings));
                out.push(']');
            }
            Inline::Code(code) => render_inline_code(&mut out, code),
            Inline::Link(link) => render_link(&mut out, warnings, link),
            Inline::Image(image) => render_image(&mut out, warnings, image),
            Inline::SoftBreak => out.push('\n'),
            Inline::HardBreak => {
                out.push('\\');
                out.push('\n');
            }
            Inline::Html(html) => {
                warn_html(warnings, html);
                out.push_str(&escape_text(html.trim_end()));
            }
        }
    }
    out
}

fn render_list(list: &List, warnings: &mut Vec<ConversionWarning>) -> String {
    let marker = if list.ordered { "+" } else { "-" };
    let mut lines: Vec<String> = Vec::new();

    for item in &list.items {
        let prefix = match item.checked {
            Some(true) => format!("{marker} ☑ "),
            Some(false) => format!("{marker} ☐ "),
            None => format!("{marker} "),
        };

        let mut item_lines: Vec<String> = Vec::new();
        for (i, block) in item.blocks.iter().enumerate() {
            if i > 0 {
                // A nested list or blockquote directly following a paragraph
                // is a tight-list continuation and must not be separated by a
                // blank line; other block sequences are loose and are.
                let prev_is_paragraph = matches!(item.blocks[i - 1], Block::Paragraph(_));
                let cur_is_nested = matches!(block, Block::List(_) | Block::BlockQuote(_));
                if !(prev_is_paragraph && cur_is_nested) {
                    item_lines.push(String::new());
                }
            }
            let block_str = render_block_to_string(block, warnings);
            item_lines.extend(block_str.split('\n').map(str::to_string));
        }

        for (j, line) in item_lines.iter().enumerate() {
            if line.is_empty() {
                lines.push(String::new());
            } else if j == 0 {
                lines.push(format!("{prefix}{line}"));
            } else {
                lines.push(format!("  {line}"));
            }
        }
    }

    lines.join("\n")
}

fn render_blockquote(
    quote: &crate::export::ast::BlockQuote,
    warnings: &mut Vec<ConversionWarning>,
) -> String {
    let mut inner = Vec::with_capacity(quote.blocks.len());
    for block in &quote.blocks {
        inner.push(render_block_to_string(block, warnings));
    }
    format!("#quote[{}]", inner.join("\n\n"))
}

fn render_code_block(code: &CodeBlock) -> String {
    let mut out = String::from("#raw(\"");
    out.push_str(&escape_string(&code.content));
    out.push_str("\", block: true");
    if let Some(lang) = &code.language {
        out.push_str(", lang: \"");
        out.push_str(&escape_string(lang));
        out.push('"');
    }
    out.push(')');
    out
}

fn render_inline_code(out: &mut String, code: &str) {
    if !code.contains('`') {
        out.push('`');
        out.push_str(code);
        out.push('`');
    } else {
        out.push_str("#raw(\"");
        out.push_str(&escape_string(code));
        out.push_str("\")");
    }
}

fn render_link(out: &mut String, warnings: &mut Vec<ConversionWarning>, link: &Link) {
    if link.url.is_empty() {
        render_inlines_into(out, warnings, &link.content);
        return;
    }
    match url_scheme(&link.url).as_deref() {
        Some("http") | Some("https") | None if is_local_target(&link.url) => {
            out.push_str("#link(\"");
            out.push_str(&escape_string(&link.url));
            out.push_str("\")[");
            render_inlines_into(out, warnings, &link.content);
            out.push(']');
        }
        Some(_) | None => {
            warnings.push(ConversionWarning::new(
                WarningCode::UnsafeUrl,
                "Link uses a disallowed URL scheme and was rendered as text.",
            ));
            render_inlines_into(out, warnings, &link.content);
        }
    }
}

fn render_image(out: &mut String, warnings: &mut Vec<ConversionWarning>, image: &Image) {
    if image.url.is_empty() {
        warnings.push(ConversionWarning::new(
            WarningCode::UnavailableImage,
            "Image has no source and was rendered as its alt text.",
        ));
        out.push_str(&escape_text(&image.alt));
        return;
    }
    match url_scheme(&image.url).as_deref() {
        Some("http") | Some("https") => {
            warnings.push(ConversionWarning::new(
                WarningCode::UnavailableImage,
                "Remote image requires explicit local asset selection and was rendered as its alt text.",
            ));
            out.push_str(&escape_text(&image.alt));
        }
        Some(_) => {
            warnings.push(ConversionWarning::new(
                WarningCode::UnsafeUrl,
                "Image uses a disallowed URL scheme and was rendered as its alt text.",
            ));
            out.push_str(&escape_text(&image.alt));
        }
        None if is_local_target(&image.url) => {
            out.push_str("#image(\"");
            out.push_str(&escape_string(&image.url));
            out.push_str("\", alt: \"");
            out.push_str(&escape_string(&image.alt));
            out.push_str("\")");
        }
        None => {
            warnings.push(ConversionWarning::new(
                WarningCode::UnsafeUrl,
                "Image target is not a local relative path and was rendered as its alt text.",
            ));
            out.push_str(&escape_text(&image.alt));
        }
    }
}

fn render_inlines_into(
    out: &mut String,
    warnings: &mut Vec<ConversionWarning>,
    inlines: &[Inline],
) {
    out.push_str(&render_inlines(inlines, warnings));
}

fn render_table(table: &Table, warnings: &mut Vec<ConversionWarning>) -> String {
    let column_count = table
        .alignments
        .len()
        .max(table.header.cells.len())
        .max(table.rows.iter().map(|r| r.cells.len()).max().unwrap_or(0));

    if table.header.cells.len() != column_count
        || table.rows.iter().any(|r| r.cells.len() != column_count)
        || (!table.alignments.is_empty() && table.alignments.len() != column_count)
    {
        warnings.push(ConversionWarning::new(
            WarningCode::MalformedTable,
            "Table has inconsistent cell counts and was normalized.",
        ));
    }

    let mut alignments = table.alignments.clone();
    alignments.resize(column_count, None);

    let mut entries = Vec::with_capacity(1 + table.rows.len());
    entries.push(format!(
        "table.header({})",
        render_cells(&table.header.cells, column_count, warnings)
    ));
    for row in &table.rows {
        entries.push(render_cells(&row.cells, column_count, warnings));
    }
    let body = entries.join(",\n  ");

    let align = alignments
        .iter()
        .map(|a| align_word(*a))
        .collect::<Vec<_>>()
        .join(", ");

    format!("#table(\n  columns: {column_count},\n  align: ({align}),\n  {body}\n)")
}

fn render_cells(
    cells: &[Vec<Inline>],
    column_count: usize,
    warnings: &mut Vec<ConversionWarning>,
) -> String {
    (0..column_count)
        .map(|i| {
            let content = cells
                .get(i)
                .map(|c| render_inlines(c, warnings))
                .unwrap_or_default();
            format!("[{content}]")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn align_word(alignment: Option<Alignment>) -> &'static str {
    match alignment {
        Some(Alignment::Center) => "center",
        Some(Alignment::Right) => "right",
        Some(Alignment::Left) | None => "left",
    }
}

/// Escape text for Typst markup mode.
fn escape_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\\' | '#' | '$' | '*' | '_' | '`' | '<' | '>' | '@' | '[' | ']' | '{' | '}' | '~'
            | '=' | '-' | '+' | '/' | '\'' | '"' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Escape text for inside a Typst string literal (`"..."`).
fn escape_string(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Extract the URI scheme (lowercased) or `None` for relative/empty targets.
fn url_scheme(url: &str) -> Option<String> {
    let bytes = url.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return None;
    }
    let mut end = 1;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric()
            || bytes[end] == b'+'
            || bytes[end] == b'-'
            || bytes[end] == b'.')
    {
        end += 1;
    }
    if end < bytes.len() && bytes[end] == b':' {
        Some(url[..end].to_ascii_lowercase())
    } else {
        None
    }
}

/// Return whether a target is a relative path or local fragment. Protocol-
/// relative URLs (`//host/path`) are deliberately rejected because they can
/// resolve to a network resource despite having no explicit scheme.
fn is_local_target(url: &str) -> bool {
    !url.starts_with("//") && !url.starts_with('\\') && !std::path::Path::new(url).is_absolute()
}

/// Emit a warning for raw HTML, distinguishing active script/style content.
fn warn_html(warnings: &mut Vec<ConversionWarning>, html: &str) {
    let lowered = html.to_ascii_lowercase();
    let active = lowered.contains("<script")
        || lowered.contains("<style")
        || lowered.contains("<iframe")
        || lowered.contains("javascript:")
        || lowered.contains("onload=")
        || lowered.contains("onerror=")
        || lowered.contains("onclick=");
    if active {
        warnings.push(ConversionWarning::new(
            WarningCode::UnsupportedScript,
            "Raw script or active content was not executed and is shown as plain text.",
        ));
    } else {
        warnings.push(ConversionWarning::new(
            WarningCode::UnsupportedHtml,
            "Raw HTML is not supported and is shown as plain text.",
        ));
    }
}

fn dedup_warnings(mut warnings: Vec<ConversionWarning>) -> Vec<ConversionWarning> {
    let mut seen = std::collections::HashSet::new();
    warnings.retain(|w| seen.insert((w.code, w.message.clone())));
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::ast::{BlockQuote, ListItem, Paragraph, TableRow};
    use crate::export::compiler::{self, CompileRequest};
    use crate::export::template::{apply_template, default_template};

    fn convert(md: &str) -> ConversionOutput {
        convert_markdown(md)
    }

    #[test]
    fn golden_headings() {
        let out = convert("# Title\n\n## Section");
        assert_eq!(out.typst, "= Title\n\n== Section\n");
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn golden_emphasis_and_strong() {
        let out = convert("_em_ and **strong** and ~~strike~~");
        assert_eq!(out.typst, "_em_ and *strong* and #strike[strike]\n");
    }

    #[test]
    fn golden_unordered_list() {
        let out = convert("- one\n- two");
        assert_eq!(out.typst, "- one\n- two\n");
    }

    #[test]
    fn golden_unordered_list_with_strong_prefix() {
        let out = convert(
            "- **La música:** Activa el sistema.\n- **Movimiento cinético:** Acciones físicas.\n- **Soledad y aburrimiento:** Busca estímulos.",
        );
        assert_eq!(
            out.typst,
            "- *La música:* Activa el sistema.\n- *Movimiento cinético:* Acciones físicas.\n- *Soledad y aburrimiento:* Busca estímulos.\n"
        );
    }

    #[test]
    fn golden_ordered_list() {
        let out = convert("1. one\n2. two");
        assert_eq!(out.typst, "+ one\n+ two\n");
    }

    #[test]
    fn golden_task_list() {
        let out = convert("- [ ] open\n- [x] done");
        assert_eq!(out.typst, "- ☐ open\n- ☑ done\n");
    }

    #[test]
    fn golden_nested_list() {
        let out = convert("- parent\n  - child");
        assert_eq!(out.typst, "- parent\n  - child\n");
    }

    #[test]
    fn golden_loose_list_item() {
        let out = convert("- first\n\n  second");
        assert_eq!(out.typst, "- first\n\n  second\n");
    }

    #[test]
    fn golden_table() {
        let out = convert("| a | b |\n| - | - |\n| 1 | 2 |");
        assert_eq!(
            out.typst,
            "#table(\n  columns: 2,\n  align: (left, left),\n  table.header([a], [b]),\n  [1], [2]\n)\n"
        );
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn golden_table_alignment() {
        let out = convert("| a | b | c |\n| :- | :-: | -: |\n| 1 | 2 | 3 |");
        assert_eq!(
            out.typst,
            "#table(\n  columns: 3,\n  align: (left, center, right),\n  table.header([a], [b], [c]),\n  [1], [2], [3]\n)\n"
        );
    }

    #[test]
    fn golden_link() {
        let out = convert("[label](https://example.com)");
        assert_eq!(out.typst, "#link(\"https://example.com\")[label]\n");
    }

    #[test]
    fn golden_unsafe_link_warns() {
        let out = convert("[label](javascript:alert(1))");
        assert_eq!(out.typst, "label\n");
        assert!(out
            .warnings
            .iter()
            .any(|w| w.code == WarningCode::UnsafeUrl));
    }

    #[test]
    fn protocol_relative_link_is_not_treated_as_local() {
        let out = convert("[label](//example.com/path)");
        assert_eq!(out.typst, "label\n");
        assert!(out
            .warnings
            .iter()
            .any(|w| w.code == WarningCode::UnsafeUrl));
    }

    #[test]
    fn golden_local_image() {
        let out = convert("![alt text](assets/pic.png)");
        assert_eq!(out.typst, "#image(\"assets/pic.png\", alt: \"alt text\")\n");
    }

    #[test]
    fn golden_remote_image_warns() {
        let out = convert("![alt text](https://example.com/x.png)");
        assert_eq!(out.typst, "alt text\n");
        assert!(out
            .warnings
            .iter()
            .any(|w| w.code == WarningCode::UnavailableImage));
    }

    #[test]
    fn golden_inline_code() {
        let out = convert("use `crate::export` now");
        assert_eq!(out.typst, "use `crate::export` now\n");
    }

    #[test]
    fn golden_fenced_code() {
        let out = convert("```rust\nfn main() {}\n```");
        assert_eq!(
            out.typst,
            "#raw(\"fn main() {}\\n\", block: true, lang: \"rust\")\n"
        );
    }

    #[test]
    fn golden_blockquote() {
        let out = convert("> quoted text");
        assert_eq!(out.typst, "#quote[quoted text]\n");
    }

    #[test]
    fn golden_separator() {
        let out = convert("---");
        assert_eq!(out.typst, "#line(length: 100%)\n");
    }

    #[test]
    fn golden_text_escaping() {
        let out = convert("hash #tag and $dollar and 100% done");
        assert_eq!(out.typst, "hash \\#tag and \\$dollar and 100% done\n");
    }

    #[test]
    fn golden_html_block_warns() {
        let out = convert("<div>hello</div>");
        assert_eq!(out.typst, "\\<div\\>hello\\<\\/div\\>\n");
        assert!(out
            .warnings
            .iter()
            .any(|w| w.code == WarningCode::UnsupportedHtml));
    }

    #[test]
    fn golden_script_warns() {
        let out = convert("<script>alert(1)</script>");
        assert!(out
            .warnings
            .iter()
            .any(|w| w.code == WarningCode::UnsupportedScript));
    }

    #[test]
    fn golden_skipped_heading_warns() {
        let out = convert("# One\n\n### Three");
        assert!(out
            .warnings
            .iter()
            .any(|w| w.code == WarningCode::SkippedHeading));
    }

    #[test]
    fn golden_ordered_start_warns() {
        let out = convert("3. three\n4. four");
        assert_eq!(out.typst, "+ three\n+ four\n");
        assert!(out
            .warnings
            .iter()
            .any(|w| w.code == WarningCode::UnsupportedListStart));
    }

    #[test]
    fn renders_hand_built_table_with_normalization() {
        // A hand-built AST bypasses the parser, so normalization and the
        // malformed-table warning must come from the renderer.
        let table = Table {
            alignments: vec![None, None],
            header: TableRow {
                cells: vec![
                    vec![Inline::Text("a".into())],
                    vec![Inline::Text("b".into())],
                ],
            },
            rows: vec![TableRow {
                cells: vec![vec![Inline::Text("1".into())]],
            }],
        };
        let doc = Document::new(vec![Block::Table(table)]);
        let out = render_typst(&doc);
        assert_eq!(
            out.typst,
            "#table(\n  columns: 2,\n  align: (left, left),\n  table.header([a], [b]),\n  [1], []\n)\n"
        );
        assert!(out
            .warnings
            .iter()
            .any(|w| w.code == WarningCode::MalformedTable));
    }

    #[test]
    fn empty_document_renders_empty() {
        let doc = Document::new(vec![]);
        let out = render_typst(&doc);
        assert_eq!(out.typst, "");
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn diary_entries_end_with_a_forced_break_before_the_next_entry() {
        let markdown = "## First entry\n\n*Date: 2026-10-15*\n\nFirst synthetic body.\n\n---\n\n## Second entry\n\n*Date: 2026-10-16*\n\nSecond synthetic body.\n\n---\n";

        let diary = convert_diary_markdown(markdown);
        assert_eq!(diary.typst.matches("#pagebreak()").count(), 1);
        assert!(diary.typst.starts_with("== First entry\n"));
        assert!(diary
            .typst
            .contains("#line(length: 100%)\n\n#pagebreak()\n\n== Second entry"));

        // The generic serializer remains unaware of journal page boundaries.
        let generic = convert_markdown(markdown);
        assert!(!generic.typst.contains("#pagebreak"));
    }

    #[test]
    fn diary_subsections_are_not_treated_as_entry_boundaries() {
        let markdown = "## First entry\n\n*Date: 2026-10-15*\n\n## Section\n\n*Date: 2026-10-16*\n\nSection body.\n\n---\n\n## Second entry\n\n*Date: 2026-10-16*\n\nSecond body.\n";

        let diary = convert_diary_markdown(markdown);
        assert_eq!(diary.typst.matches("#pagebreak()").count(), 1);
    }

    #[test]
    fn one_diary_entry_has_no_page_break() {
        let diary =
            convert_diary_markdown("## One entry\n\n*Date: 2026-10-15*\n\nSynthetic body.\n");
        assert!(!diary.typst.contains("#pagebreak"));
    }

    #[test]
    fn diary_page_breaks_compile_to_exactly_one_page_per_entry() {
        let markdown = "## First entry\n\n*Date: 2026-10-15*\n\nFirst synthetic body.\n\n---\n\n## Second entry\n\n*Date: 2026-10-16*\n\nSecond synthetic body.\n\n---\n";
        let template = default_template();

        let diary = convert_diary_markdown(markdown);
        let diary_request = CompileRequest {
            source: apply_template(&diary.typst, &Some(template.clone())),
            template: Some(template.clone()),
            attachments: Vec::new(),
        };
        assert_eq!(compiler::compiled_page_count(&diary_request), Some(2));

        // The same content through the shared generic/therapy serializer keeps
        // its existing one-page flow and receives no journal-specific break.
        let generic = convert_markdown(markdown);
        assert!(!generic.typst.contains("#pagebreak"));
        let generic_request = CompileRequest {
            source: apply_template(&generic.typst, &Some(template.clone())),
            template: Some(template),
            attachments: Vec::new(),
        };
        assert_eq!(compiler::compiled_page_count(&generic_request), Some(1));
    }

    #[test]
    fn blockquote_with_list() {
        let doc = Document::new(vec![Block::BlockQuote(BlockQuote {
            blocks: vec![Block::List(List {
                ordered: false,
                start: None,
                items: vec![ListItem {
                    checked: None,
                    blocks: vec![Block::Paragraph(Paragraph {
                        content: vec![Inline::Text("item".into())],
                    })],
                }],
            })],
        })]);
        let out = render_typst(&doc);
        assert_eq!(out.typst, "#quote[- item]\n");
    }

    #[test]
    fn heading_level_six() {
        let out = convert("###### six");
        assert_eq!(out.typst, "====== six\n");
    }

    #[test]
    fn inline_code_with_backtick_uses_raw() {
        let out = convert("run ``a`b`` now");
        assert_eq!(out.typst, "run #raw(\"a`b\") now\n");
    }
}
