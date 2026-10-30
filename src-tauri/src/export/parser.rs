//! CommonMark/GFM parser adapter.
//!
//! Wraps [`pulldown-cmark`](https://crates.io/crates/pulldown-cmark) (pinned,
//! MIT licensed) and reduces its event stream to the constrained
//! [`crate::export::ast`] tree. The enabled GFM subset is limited to the
//! constructs the exporter maps to Typst: tables, task-list checkboxes, and
//! strikethrough. Other extensions (math, footnotes, definition lists,
//! superscript/subscript, metadata blocks, wikilinks, heading attributes,
//! smart punctuation) are intentionally left disabled so their source falls
//! through as plain text or a warning instead of being reinterpreted.
//!
//! Unsupported constructs are preserved rather than discarded: raw HTML is
//! carried through [`ast::Block::Html`] / [`ast::Inline::Html`] and annotated
//! with a warning, and any unexpected container is skipped with an
//! [`ast::WarningCode::UnsupportedConstruct`] warning.

use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::export::ast::{
    Alignment as AstAlignment, Block, BlockQuote, CodeBlock, ConversionWarning, Document, Heading,
    HtmlBlock, Image, Inline, Link, List, ListItem, Paragraph, Table, TableRow, WarningCode,
    MAX_NESTING_DEPTH,
};

/// A parsed document together with the warnings produced while reducing the
/// event stream to the AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMarkdown {
    pub document: Document,
    pub warnings: Vec<ConversionWarning>,
}

/// Parse a Markdown string into the constrained AST.
pub fn parse_markdown(source: &str) -> ParsedMarkdown {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(source, options);
    let events: Vec<Event<'static>> = parser.map(Event::into_static).collect();

    let mut p = ParserState {
        events,
        pos: 0,
        depth: 0,
        prev_heading: None,
        warnings: Vec::new(),
    };

    let blocks = parse_blocks(&mut p, &[]);

    ParsedMarkdown {
        document: Document::new(blocks),
        warnings: p.dedup_warnings(),
    }
}

struct ParserState {
    events: Vec<Event<'static>>,
    pos: usize,
    depth: usize,
    prev_heading: Option<u8>,
    warnings: Vec<ConversionWarning>,
}

impl ParserState {
    fn warn(&mut self, code: WarningCode, message: impl Into<String>) {
        self.warnings.push(ConversionWarning::new(code, message));
    }

    fn dedup_warnings(mut self) -> Vec<ConversionWarning> {
        let mut seen = std::collections::HashSet::new();
        self.warnings
            .retain(|w| seen.insert((w.code, w.message.clone())));
        self.warnings
    }
}

/// Parse block-level content until one of `stops` closes or the stream ends.
///
/// The matching closing [`Event::End`] is consumed.
fn parse_blocks(p: &mut ParserState, stops: &[TagEnd]) -> Vec<Block> {
    let mut blocks = Vec::new();
    while p.pos < p.events.len() {
        let event = p.events[p.pos].clone();
        match event {
            Event::End(end) => {
                if end_matches(end, stops) {
                    p.pos += 1;
                    break;
                }
                // Unexpected closing tag at this level; skip defensively.
                p.pos += 1;
            }
            Event::Rule => {
                p.pos += 1;
                blocks.push(Block::Separator);
            }
            Event::Start(Tag::Paragraph) => {
                p.pos += 1;
                let content = parse_inlines(p, &[TagEnd::Paragraph]);
                blocks.push(Block::Paragraph(Paragraph { content }));
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let level_number = heading_level_number(level);
                if let Some(prev) = p.prev_heading {
                    if level_number > prev + 1 {
                        p.warn(
                            WarningCode::SkippedHeading,
                            format!("Heading level skipped from {prev} to {level_number}."),
                        );
                    }
                }
                p.prev_heading = Some(level_number);
                p.pos += 1;
                let content = parse_inlines(p, &[TagEnd::Heading(level)]);
                blocks.push(Block::Heading(Heading {
                    level: level_number,
                    content,
                }));
            }
            Event::Start(Tag::List(start)) => {
                blocks.push(parse_list(p, start));
            }
            Event::Start(Tag::Table(_)) => {
                blocks.push(parse_table(p));
            }
            Event::Start(Tag::BlockQuote(_)) => {
                blocks.push(parse_blockquote(p));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                blocks.push(parse_code_block(p, kind));
            }
            Event::Start(Tag::HtmlBlock) => {
                blocks.push(parse_html_block(p));
            }
            Event::Start(
                Tag::Emphasis
                | Tag::Strong
                | Tag::Strikethrough
                | Tag::Link { .. }
                | Tag::Image { .. }
                | Tag::Superscript
                | Tag::Subscript,
            ) => {
                // Tight list items may omit an explicit paragraph event when
                // their first token is inline markup (for example
                // `- **Label:** text`). Treat that inline container as the
                // beginning of a paragraph instead of dropping the item as an
                // unsupported block.
                let content = parse_inlines(p, &[TagEnd::Item, TagEnd::List(true)]);
                blocks.push(Block::Paragraph(Paragraph { content }));
                // The inline parser consumes the current item's closing tag.
                // Return before the next sibling item is mistaken for an
                // unsupported nested block; `parse_list` owns sibling items.
                if matches!(
                    p.events.get(p.pos),
                    Some(Event::Start(Tag::Item)) | Some(Event::End(TagEnd::List(_)))
                ) {
                    break;
                }
            }
            Event::Start(_other) => {
                // Unsupported block containers are not interpreted. Keep the
                // warning message fixed so user-authored tag metadata never
                // crosses the diagnostic boundary.
                p.warn(
                    WarningCode::UnsupportedConstruct,
                    "An unsupported block construct was not rendered.",
                );
                skip_container(p);
            }
            // Stray inline-level tokens at block scope are collected into a
            // paragraph defensively. CommonMark normally wraps these, so this
            // only guards malformed or unexpected streams.
            Event::Text(_)
            | Event::Code(_)
            | Event::InlineHtml(_)
            | Event::Html(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::FootnoteReference(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::TaskListMarker(_) => {
                let content = parse_stray_inlines(p);
                blocks.push(Block::Paragraph(Paragraph { content }));
            }
        }
    }
    blocks
}

/// Parse a list (and its items). Consumes through the list's closing event.
fn parse_list(p: &mut ParserState, start: Option<u64>) -> Block {
    p.pos += 1; // consume Start(List)
    let ordered = start.is_some();
    if let Some(n) = start {
        if n != 1 {
            p.warn(
                WarningCode::UnsupportedListStart,
                format!("Ordered list start {n} is not preserved; rendering starts at 1."),
            );
        }
    }

    enter_depth(p);

    let mut items = Vec::new();
    while p.pos < p.events.len() {
        match &p.events[p.pos] {
            Event::Start(Tag::Item) => {
                p.pos += 1; // consume Start(Item)
                let mut checked = None;
                if let Some(Event::TaskListMarker(value)) = p.events.get(p.pos) {
                    checked = Some(*value);
                    p.pos += 1;
                }
                let blocks = parse_blocks(p, &[TagEnd::Item, TagEnd::List(true)]);
                if let Some(Event::End(TagEnd::Item)) = p.events.get(p.pos) {
                    p.pos += 1;
                }
                if matches!(blocks.first(), Some(Block::List(_))) {
                    p.warn(
                        WarningCode::InvalidNesting,
                        "List item begins with a nested list and cannot be represented faithfully.",
                    );
                }
                items.push(ListItem { checked, blocks });
            }
            _ => break,
        }
    }
    if let Some(Event::End(TagEnd::List(_))) = p.events.get(p.pos) {
        p.pos += 1;
    }

    exit_depth(p);
    Block::List(List {
        ordered,
        start,
        items,
    })
}

fn parse_blockquote(p: &mut ParserState) -> Block {
    p.pos += 1; // consume Start(BlockQuote)
    enter_depth(p);
    let blocks = parse_blocks(p, &[TagEnd::BlockQuote(None)]);
    if let Some(Event::End(TagEnd::BlockQuote(_))) = p.events.get(p.pos) {
        p.pos += 1;
    }
    exit_depth(p);
    Block::BlockQuote(BlockQuote { blocks })
}

fn parse_code_block(p: &mut ParserState, kind: CodeBlockKind<'static>) -> Block {
    p.pos += 1; // consume Start(CodeBlock)
    let language = match kind {
        CodeBlockKind::Fenced(info) => sanitize_language(info.as_ref()),
        CodeBlockKind::Indented => None,
    };
    let mut content = String::new();
    while p.pos < p.events.len() {
        match &p.events[p.pos] {
            Event::Text(t) => {
                content.push_str(t.as_ref());
                p.pos += 1;
            }
            _ => break,
        }
    }
    if let Some(Event::End(TagEnd::CodeBlock)) = p.events.get(p.pos) {
        p.pos += 1;
    }
    Block::CodeBlock(CodeBlock { language, content })
}

fn parse_html_block(p: &mut ParserState) -> Block {
    p.pos += 1; // consume Start(HtmlBlock)
    let mut raw = String::new();
    while p.pos < p.events.len() {
        match &p.events[p.pos] {
            Event::Html(h) | Event::InlineHtml(h) => {
                raw.push_str(h.as_ref());
                p.pos += 1;
            }
            _ => break,
        }
    }
    if let Some(Event::End(TagEnd::HtmlBlock)) = p.events.get(p.pos) {
        p.pos += 1;
    }
    Block::Html(HtmlBlock { raw })
}

fn parse_table(p: &mut ParserState) -> Block {
    let alignments: Vec<Option<AstAlignment>> = match &p.events[p.pos] {
        Event::Start(Tag::Table(alignments)) => alignments
            .iter()
            .map(|a| match a {
                Alignment::Left => Some(AstAlignment::Left),
                Alignment::Center => Some(AstAlignment::Center),
                Alignment::Right => Some(AstAlignment::Right),
                Alignment::None => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    p.pos += 1; // consume Start(Table)

    let mut header = None;
    let mut rows = Vec::new();
    let mut current_row: Vec<Vec<Inline>> = Vec::new();

    while p.pos < p.events.len() {
        let event = p.events[p.pos].clone();
        match event {
            Event::Start(Tag::TableHead) => {
                current_row = Vec::new();
                p.pos += 1;
            }
            Event::End(TagEnd::TableHead) => {
                header = Some(TableRow {
                    cells: std::mem::take(&mut current_row),
                });
                p.pos += 1;
            }
            Event::Start(Tag::TableRow) => {
                current_row = Vec::new();
                p.pos += 1;
            }
            Event::End(TagEnd::TableRow) => {
                rows.push(TableRow {
                    cells: std::mem::take(&mut current_row),
                });
                p.pos += 1;
            }
            Event::Start(Tag::TableCell) => {
                p.pos += 1;
                let content = parse_inlines(p, &[TagEnd::TableCell]);
                current_row.push(content);
            }
            Event::End(TagEnd::Table) => {
                p.pos += 1;
                break;
            }
            _ => {
                p.pos += 1;
            }
        }
    }

    let header = header.unwrap_or_else(|| TableRow { cells: Vec::new() });
    // Cell-count normalization and the `malformed_table` warning are owned by
    // the serializer, which is the single place that knows the final column
    // count. The parser only carries the raw column structure forward.
    Block::Table(Table {
        alignments,
        header,
        rows,
    })
}

/// Parse inline-level content until one of `stops` closes it.
///
/// The matching closing [`Event::End`] is consumed.
fn parse_inlines(p: &mut ParserState, stops: &[TagEnd]) -> Vec<Inline> {
    let mut out = Vec::new();
    while p.pos < p.events.len() {
        let event = p.events[p.pos].clone();
        match event {
            Event::Text(t) => {
                p.pos += 1;
                out.push(Inline::Text(t.into_string()));
            }
            Event::Code(c) => {
                p.pos += 1;
                out.push(Inline::Code(c.into_string()));
            }
            Event::SoftBreak => {
                p.pos += 1;
                out.push(Inline::SoftBreak);
            }
            Event::HardBreak => {
                p.pos += 1;
                out.push(Inline::HardBreak);
            }
            Event::InlineHtml(h) | Event::Html(h) => {
                p.pos += 1;
                out.push(Inline::Html(h.into_string()));
            }
            Event::InlineMath(m) | Event::DisplayMath(m) => {
                p.pos += 1;
                p.warn(WarningCode::UnsupportedConstruct, "Math is not supported.");
                out.push(Inline::Text(m.into_string()));
            }
            Event::FootnoteReference(f) => {
                p.pos += 1;
                p.warn(
                    WarningCode::UnsupportedConstruct,
                    "Footnotes are not supported.",
                );
                out.push(Inline::Text(f.into_string()));
            }
            Event::TaskListMarker(_) | Event::Rule => {
                p.pos += 1;
            }
            Event::Start(tag) => match tag {
                Tag::Emphasis => {
                    p.pos += 1;
                    let content = parse_inlines(p, &[TagEnd::Emphasis]);
                    out.push(Inline::Emphasis(content));
                }
                Tag::Strong => {
                    p.pos += 1;
                    let content = parse_inlines(p, &[TagEnd::Strong]);
                    out.push(Inline::Strong(content));
                }
                Tag::Strikethrough => {
                    p.pos += 1;
                    let content = parse_inlines(p, &[TagEnd::Strikethrough]);
                    out.push(Inline::Strikethrough(content));
                }
                Tag::Link {
                    dest_url, title, ..
                } => {
                    p.pos += 1;
                    let content = parse_inlines(p, &[TagEnd::Link]);
                    out.push(Inline::Link(Link {
                        url: dest_url.into_string(),
                        title: if title.is_empty() {
                            None
                        } else {
                            Some(title.into_string())
                        },
                        content,
                    }));
                }
                Tag::Image {
                    dest_url, title, ..
                } => {
                    p.pos += 1;
                    let alt = parse_inlines(p, &[TagEnd::Image]);
                    out.push(Inline::Image(Image {
                        url: dest_url.into_string(),
                        alt: plain_text(&alt).unwrap_or_else(|| {
                            if title.is_empty() {
                                "image".to_string()
                            } else {
                                title.into_string()
                            }
                        }),
                    }));
                }
                Tag::Superscript | Tag::Subscript => {
                    p.pos += 1;
                    p.warn(
                        WarningCode::UnsupportedConstruct,
                        "Superscript and subscript are not supported.",
                    );
                    let content = parse_inlines(p, &[TagEnd::Superscript, TagEnd::Subscript]);
                    out.extend(content);
                }
                _other => {
                    p.warn(
                        WarningCode::UnsupportedConstruct,
                        "An unsupported inline construct was not rendered.",
                    );
                    p.pos += 1;
                    skip_container(p);
                }
            },
            Event::End(end) => {
                if end_matches(end, stops) {
                    p.pos += 1;
                    break;
                }
                p.pos += 1;
            }
        }
    }
    out
}

/// Collect stray inline-level tokens at block scope into inline content.
/// This is a defensive path for malformed streams; the parser stops at the
/// first block boundary.
fn parse_stray_inlines(p: &mut ParserState) -> Vec<Inline> {
    let mut out = Vec::new();
    while p.pos < p.events.len() {
        let event = p.events[p.pos].clone();
        match event {
            Event::Text(t) => {
                p.pos += 1;
                out.push(Inline::Text(t.into_string()));
            }
            Event::Code(c) => {
                p.pos += 1;
                out.push(Inline::Code(c.into_string()));
            }
            Event::SoftBreak => {
                p.pos += 1;
                out.push(Inline::SoftBreak);
            }
            Event::HardBreak => {
                p.pos += 1;
                out.push(Inline::HardBreak);
            }
            _ => break,
        }
    }
    out
}

/// Skip a balanced container subtree. The current position must be just after
/// the opening `Start` event; the matching `End` event is also consumed.
fn skip_container(p: &mut ParserState) {
    let mut depth = 0usize;
    while p.pos < p.events.len() {
        match &p.events[p.pos] {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                if depth == 0 {
                    p.pos += 1;
                    return;
                }
                depth -= 1;
            }
            _ => {}
        }
        p.pos += 1;
    }
}

/// Match an end event against a set of stop tags, ignoring variant payloads
/// (heading level, list ordering, blockquote kind).
fn end_matches(end: TagEnd, stops: &[TagEnd]) -> bool {
    stops
        .iter()
        .any(|s| std::mem::discriminant(s) == std::mem::discriminant(&end))
}

/// Map a heading level to its numeric value `1..=6`.
fn heading_level_number(level: pulldown_cmark::HeadingLevel) -> u8 {
    match level {
        pulldown_cmark::HeadingLevel::H1 => 1,
        pulldown_cmark::HeadingLevel::H2 => 2,
        pulldown_cmark::HeadingLevel::H3 => 3,
        pulldown_cmark::HeadingLevel::H4 => 4,
        pulldown_cmark::HeadingLevel::H5 => 5,
        pulldown_cmark::HeadingLevel::H6 => 6,
    }
}

/// Advance into a nestable block container and warn once when the supported
/// depth limit is exceeded.
fn enter_depth(p: &mut ParserState) {
    p.depth += 1;
    if p.depth > MAX_NESTING_DEPTH {
        p.warn(
            WarningCode::InvalidNesting,
            format!("Nesting depth exceeds the supported limit of {MAX_NESTING_DEPTH}."),
        );
    }
}

fn exit_depth(p: &mut ParserState) {
    if p.depth > 0 {
        p.depth -= 1;
    }
}

/// Reduce a fenced-code info string to a Typst-compatible language tag.
///
/// Only the first whitespace-delimited token is kept, and only when it is a
/// valid Typst identifier (letters/digits/`-`/`_`, starting with a letter or
/// underscore). Otherwise the language tag is dropped so the code is still
/// preserved literally as untagged raw text.
fn sanitize_language(info: &str) -> Option<String> {
    let token = info.split_whitespace().next().unwrap_or("");
    if token.is_empty() {
        return None;
    }
    let mut chars = token.chars();
    let first = chars.next().unwrap();
    if !(first.is_alphabetic() || first == '_') {
        return None;
    }
    if !chars.all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return None;
    }
    Some(token.to_string())
}

/// Flatten inline content to its visible text, used for image alt fallback.
fn plain_text(inlines: &[Inline]) -> Option<String> {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(t) => text.push_str(t),
            Inline::Code(c) => text.push_str(c),
            Inline::Emphasis(c) | Inline::Strong(c) | Inline::Strikethrough(c) => {
                if let Some(inner) = plain_text(c) {
                    text.push_str(&inner);
                }
            }
            Inline::Link(link) => {
                if let Some(inner) = plain_text(&link.content) {
                    text.push_str(&inner);
                }
            }
            Inline::Image(img) => text.push_str(&img.alt),
            Inline::Html(h) => text.push_str(h),
            Inline::SoftBreak | Inline::HardBreak => text.push(' '),
        }
    }
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_heading_levels() {
        let parsed = parse_markdown("# One\n\n## Two");
        assert_eq!(
            parsed.document.blocks,
            vec![
                Block::Heading(Heading {
                    level: 1,
                    content: vec![Inline::Text("One".to_string())],
                }),
                Block::Heading(Heading {
                    level: 2,
                    content: vec![Inline::Text("Two".to_string())],
                }),
            ]
        );
        assert!(parsed.warnings.is_empty());
    }

    #[test]
    fn parses_task_checkboxes() {
        let parsed = parse_markdown("- [ ] open\n- [x] done");
        let Block::List(list) = &parsed.document.blocks[0] else {
            panic!("expected list");
        };
        assert!(!list.ordered);
        assert_eq!(list.items[0].checked, Some(false));
        assert_eq!(list.items[1].checked, Some(true));
    }

    #[test]
    fn parses_table() {
        let parsed = parse_markdown("| a | b |\n| - | - |\n| 1 | 2 |");
        let Block::Table(table) = &parsed.document.blocks[0] else {
            panic!("expected table");
        };
        assert_eq!(table.header.cells.len(), 2);
        assert_eq!(table.rows.len(), 1);
    }

    #[test]
    fn parses_fenced_code_language() {
        let parsed = parse_markdown("```rust\nfn main() {}\n```");
        let Block::CodeBlock(code) = &parsed.document.blocks[0] else {
            panic!("expected code block");
        };
        assert_eq!(code.language.as_deref(), Some("rust"));
        assert_eq!(code.content, "fn main() {}\n");
    }

    #[test]
    fn raw_html_is_preserved_as_html_block() {
        // HTML warnings are owned by the serializer; the parser only preserves
        // the raw source in a warning-bearing node.
        let parsed = parse_markdown("<div>hi</div>");
        assert!(matches!(
            &parsed.document.blocks[0],
            Block::Html(HtmlBlock { .. })
        ));
    }

    #[test]
    fn script_html_is_preserved_as_html_block() {
        let parsed = parse_markdown("<script>alert(1)</script>");
        assert!(matches!(
            &parsed.document.blocks[0],
            Block::Html(HtmlBlock { .. })
        ));
    }

    #[test]
    fn math_is_literal_text_without_extension() {
        // Math extension is disabled, so `$x$` remains plain text.
        let parsed = parse_markdown("cost $5 and $x$");
        let Block::Paragraph(p) = &parsed.document.blocks[0] else {
            panic!("expected paragraph");
        };
        let text = p
            .content
            .iter()
            .map(|i| match i {
                Inline::Text(t) => t.clone(),
                _ => String::new(),
            })
            .collect::<String>();
        assert_eq!(text, "cost $5 and $x$");
    }
}
