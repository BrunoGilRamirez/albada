//! Constrained Markdown AST for the export pipeline.
//!
//! This module owns the intermediate representation produced by the parser
//! adapter ([`crate::export::parser`]) and consumed by the deterministic Typst
//! serializer ([`crate::export::typst`]). It is intentionally explicit: every
//! node that the exporter supports maps to a distinct type, and unsupported
//! source is carried verbatim through warning-bearing nodes so that it is
//! never silently discarded.
//!
//! The AST is data-only. It performs no filesystem access, no network access,
//! and never interprets HTML, scripts, or Typst markup supplied by an exported
//! record.

/// Maximum nesting depth for lists and block quotes. Content nested deeper is
/// still preserved but is reported through an [`ConversionWarning`] with
/// [`WarningCode::InvalidNesting`]. This mirrors the initial resource limit in
/// the Typst export architecture patch.
pub const MAX_NESTING_DEPTH: usize = 16;

/// A parsed Markdown document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
}

impl Document {
    pub fn new(blocks: Vec<Block>) -> Self {
        Self { blocks }
    }
}

/// A block-level element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading(Heading),
    Paragraph(Paragraph),
    List(List),
    Table(Table),
    BlockQuote(BlockQuote),
    CodeBlock(CodeBlock),
    Separator,
    /// Raw HTML block. Preserved verbatim and reported as a warning; never
    /// rendered as live HTML or executed.
    Html(HtmlBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    /// Heading level, `1..=6`.
    pub level: u8,
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paragraph {
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct List {
    /// `true` for ordered (numbered) lists, `false` for bullet lists.
    pub ordered: bool,
    /// The start number of an ordered list. `None` for unordered lists.
    pub start: Option<u64>,
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    /// Task-list checkbox state. `None` when the item is not a task.
    pub checked: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// Per-column alignment, aligned to the rendered column count.
    pub alignments: Vec<Option<Alignment>>,
    pub header: TableRow,
    pub rows: Vec<TableRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRow {
    pub cells: Vec<Vec<Inline>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockQuote {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    /// First word of the fenced-code info string, when present.
    pub language: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlBlock {
    /// Raw HTML source, preserved verbatim for the source-visible warning path.
    pub raw: String,
}

/// An inline-level element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Code(String),
    Link(Link),
    Image(Image),
    SoftBreak,
    HardBreak,
    /// Raw inline HTML. Preserved verbatim and reported as a warning.
    Html(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub url: String,
    pub title: Option<String>,
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub url: String,
    pub alt: String,
}

/// A stable, deterministic warning code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WarningCode {
    /// Raw HTML that cannot be represented safely in Typst.
    UnsupportedHtml,
    /// Raw script/style/iframe/event-handler source that must never execute.
    UnsupportedScript,
    /// A link or image target with a disallowed URL scheme.
    UnsafeUrl,
    /// A table with inconsistent cell counts or unsupported structure.
    MalformedTable,
    /// Content nested deeper than [`MAX_NESTING_DEPTH`], or nesting that the
    /// serializer cannot represent faithfully.
    InvalidNesting,
    /// An image that the pure layer cannot resolve (remote, empty, or unsafe).
    UnavailableImage,
    /// A heading whose level skips more than one step.
    SkippedHeading,
    /// An ordered list whose start number is not preserved.
    UnsupportedListStart,
    /// Any other construct outside the supported export subset.
    UnsupportedConstruct,
}

impl WarningCode {
    pub fn as_str(self) -> &'static str {
        match self {
            WarningCode::UnsupportedHtml => "unsupported_html",
            WarningCode::UnsupportedScript => "unsupported_script",
            WarningCode::UnsafeUrl => "unsafe_url",
            WarningCode::MalformedTable => "malformed_table",
            WarningCode::InvalidNesting => "invalid_nesting",
            WarningCode::UnavailableImage => "unavailable_image",
            WarningCode::SkippedHeading => "skipped_heading",
            WarningCode::UnsupportedListStart => "unsupported_list_start",
            WarningCode::UnsupportedConstruct => "unsupported_construct",
        }
    }
}

/// A structured, deterministic warning produced during parse or serialization.
///
/// Warnings never contain user-authored content: messages are fixed templates,
/// so they are safe to display in the export preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionWarning {
    pub code: WarningCode,
    pub message: String,
}

impl ConversionWarning {
    pub fn new(code: WarningCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
