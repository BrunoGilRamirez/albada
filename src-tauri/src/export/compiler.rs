//! Embedded Typst compiler adapter (Phase 7.10.4).
//!
//! This module compiles generated `.typ` source into a PDF byte stream using
//! the pinned, embedded Typst 0.15.1 compiler. It is deliberately isolated from
//! SQLite, Tauri commands, and the filesystem: every input is passed in as an
//! owned value, and the compiler is driven through a controlled [`World`] that
//! serves only the sources and attachments supplied by the caller.
//!
//! # Sandbox boundary
//!
//! The adapter never invokes `PATH`, Python, LaTeX, a shell, or a subprocess,
//! and it performs no network access. It does not read the real filesystem at
//! all: the [`SandboxWorld`] resolves every `FileId` against an in-memory map
//! keyed by a normalized virtual path. A template or attachment that is not
//! explicitly supplied is reported as a safe "not found" diagnostic rather than
//! being opened from disk.
//!
//! # Template contract
//!
//! The compiler accepts at most one template, modeled as an owned name plus
//! source ([`Template`]). The name must equal [`APPROVED_TEMPLATE_NAME`];
//! empty, traversal, absolute, separator-containing, and unknown names are
//! rejected before compilation. The approved template is always served at the
//! fixed virtual path [`TEMPLATE_PATH`], so a template name can never influence
//! where its source resolves and template imports cannot escape the sandbox
//! root.
//!
//! # Limits
//!
//! The provisional limits from the Typst patch are enforced where the compiler
//! API makes them enforceable in this slice:
//!
//! - **Source and template size** (5 MiB each) and **attachment sizes** (10 MiB
//!   each, 50 MiB total) are enforced before compilation and are hard limits.
//! - **Nesting depth** (16) is enforced by the parser/serializer layer
//!   ([`crate::export::ast::MAX_NESTING_DEPTH`]); this adapter does not
//!   re-derive it.
//! - **Page count** (100) is enforced after layout by inspecting the produced
//!   [`PagedDocument`]; exceeding it is a hard failure and no PDF is returned.
//! - **Compile timeout** (30 s) is *not* enforceable through the Typst 0.15.1
//!   library API, which has no cancellation or deadline hook. It is therefore
//!   documented as adapter policy: callers must run compilation on a controlled
//!   worker boundary and apply their own cooperative timeout/cancellation. This
//!   module does not silently claim a timeout it cannot enforce.
//!
//! # Diagnostics
//!
//! Diagnostics are reduced to a structured, content-safe form. They never
//! contain source text, absolute filesystem paths, environment variables, or
//! compiler internals. Only the severity, a fixed message, and a 1-based
//! line/column (when the span points into a known source) are exposed.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use typst::diag::{FileError, FileResult, SourceDiagnostic, Warned};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::{Library, LibraryExt, World, WorldExt};
use typst_layout::PagedDocument;
use typst_pdf::{PdfOptions, PdfStandards};
use typst_utils::LazyHash;

/// Provisional source-size limit (bytes). Hard limit enforced before compile.
pub const MAX_SOURCE_BYTES: usize = 5 * 1024 * 1024;

/// Provisional per-attachment size limit (bytes). Hard limit.
pub const MAX_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;

/// Provisional total-attachment size limit (bytes). Hard limit.
pub const MAX_TOTAL_ATTACHMENT_BYTES: usize = 50 * 1024 * 1024;

/// Provisional page-count limit. Hard limit enforced after layout.
pub const MAX_PAGES: usize = 100;

/// The single approved template name for the MVP.
///
/// User-authored templates are out of scope until the sandbox spike proves they
/// can be constrained (Phase 7.10.7); only this reviewed template name is
/// accepted by [`Template`]. See [`TEMPLATE_PATH`] for the virtual path the
/// template is served at.
pub const APPROVED_TEMPLATE_NAME: &str = "foundation-journal";
pub const LEGACY_TEMPLATE_NAME: &str = "default";

/// The embedded Typst compiler version reported in diagnostics.
///
/// Single source of truth for the compiler label shown in export capability
/// snapshots, environment diagnostics, and settings. The compiler is embedded
/// (no PATH/executable dependency); only this label changes if the pinned
/// `typst` dependency is ever bumped.
pub const EMBEDDED_COMPILER_VERSION: &str = "typst 0.15.1 (embedded)";

/// The virtual path at which the approved template is served inside the
/// sandbox.
///
/// The generated source must `#import` this path to use the template. The path
/// is fixed (derived from [`APPROVED_TEMPLATE_NAME`]) so a template name can
/// never change where its source resolves, keeping template imports inside the
/// sandbox root.
pub const TEMPLATE_PATH: &str = "/templates/foundation-journal.typ";

/// The only fonts exposed to the embedded compiler. Each file is OFL-1.1
/// licensed and bundled from the reviewed Libertinus Serif family.
const BUNDLED_FONT_FILES: [(&str, &[u8]); 6] = [
    (
        "LibertinusSerif-Regular.otf",
        include_bytes!("../../assets/fonts/LibertinusSerif-Regular.otf"),
    ),
    (
        "LibertinusSerif-Bold.otf",
        include_bytes!("../../assets/fonts/LibertinusSerif-Bold.otf"),
    ),
    (
        "LibertinusSerif-Italic.otf",
        include_bytes!("../../assets/fonts/LibertinusSerif-Italic.otf"),
    ),
    (
        "LibertinusSerif-BoldItalic.otf",
        include_bytes!("../../assets/fonts/LibertinusSerif-BoldItalic.otf"),
    ),
    (
        "LibertinusSerif-Semibold.otf",
        include_bytes!("../../assets/fonts/LibertinusSerif-Semibold.otf"),
    ),
    (
        "LibertinusSerif-SemiboldItalic.otf",
        include_bytes!("../../assets/fonts/LibertinusSerif-SemiboldItalic.otf"),
    ),
];

/// Number of individual font faces loaded from the approved font files.
pub fn bundled_font_count() -> usize {
    BUNDLED_FONT_FILES
        .iter()
        .map(|(_, data)| Font::iter(Bytes::new(*data)).count())
        .sum()
}

/// The virtual path of the main source file inside the sandbox.
const MAIN_PATH: &str = "/main.typ";

/// A single local attachment supplied to the compiler.
///
/// The `path` is a virtual, forward-slash path relative to the sandbox root
/// (for example `assets/pic.png`). It is validated by
/// [`validate_attachment_path`] before use and must not be absolute or contain
/// traversal.
#[derive(Debug, Clone)]
pub struct Attachment {
    /// Virtual path relative to the sandbox root, using `/` separators.
    pub path: String,
    /// Raw bytes of the attachment.
    pub data: Vec<u8>,
}

/// An approved template supplied to the compiler.
///
/// The template is identified by an owned [`name`](Template::name) that must
/// exactly match [`APPROVED_TEMPLATE_NAME`]; any other name, and any name that
/// looks like a path (traversal, absolute, or separator-containing), is
/// rejected before compilation. The [`source`](Template::source) is the
/// already-approved template source, served at the fixed virtual path
/// [`TEMPLATE_PATH`] so template imports cannot escape the sandbox root.
#[derive(Debug, Clone)]
pub struct Template {
    /// The template name; must equal [`APPROVED_TEMPLATE_NAME`].
    pub name: String,
    /// The approved template source.
    pub source: String,
}

/// A complete, owned compilation request.
#[derive(Debug, Clone)]
pub struct CompileRequest {
    /// The generated `.typ` source.
    pub source: String,
    /// The approved template, or `None` to compile the source directly.
    pub template: Option<Template>,
    /// Explicitly supplied local attachments.
    pub attachments: Vec<Attachment>,
}

/// A safe, structured diagnostic produced by the compiler.
///
/// Messages are fixed templates; no source text, path, or environment data is
/// carried. `line`/`column` are 1-based and only present when the span points
/// into a known source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileDiagnostic {
    pub source: String,
    pub category: String,
    pub phase: String,
    pub severity: Severity,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

/// Severity of a [`CompileDiagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
}

/// The outcome of a compilation attempt.
///
/// A successful PDF and diagnostics are mutually exclusive: a failed
/// compilation can never be represented as a successful PDF, and no partial
/// output is returned.
#[derive(Debug)]
pub enum CompileOutcome {
    /// Compilation succeeded; `pdf` holds the complete PDF byte stream.
    Success {
        pdf: Vec<u8>,
        warnings: Vec<CompileDiagnostic>,
    },
    /// Compilation failed; `diagnostics` holds the safe diagnostics.
    Failure { diagnostics: Vec<CompileDiagnostic> },
}

/// A validation error raised before compilation begins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// The generated source exceeded [`MAX_SOURCE_BYTES`].
    SourceTooLarge,
    /// The template source exceeded [`MAX_SOURCE_BYTES`].
    TemplateTooLarge,
    /// An attachment exceeded [`MAX_ATTACHMENT_BYTES`].
    AttachmentTooLarge,
    /// The attachments exceeded [`MAX_TOTAL_ATTACHMENT_BYTES`] in total.
    AttachmentsTooLarge,
    /// Two attachments resolved to the same virtual path.
    DuplicateAttachmentPath,
    /// An attachment path was absolute, contained traversal, or was otherwise
    /// unsupported.
    InvalidAttachmentPath,
    /// A template name was empty, absolute, or contained traversal/separators.
    InvalidTemplateName,
    /// A template name did not match the approved set.
    UnknownTemplate,
    /// The compilation exceeded the 30-second timeout.
    CompileTimeout,
}

impl CompileError {
    /// A safe, fixed message for this error.
    pub fn message(&self) -> &'static str {
        match self {
            CompileError::SourceTooLarge => "The document source is too large to export.",
            CompileError::TemplateTooLarge => "The template source is too large to export.",
            CompileError::AttachmentTooLarge => "An attachment is too large to export.",
            CompileError::AttachmentsTooLarge => "The attachments are too large to export.",
            CompileError::DuplicateAttachmentPath => "A duplicate attachment path is not allowed.",
            CompileError::InvalidAttachmentPath => "An attachment path is not allowed.",
            CompileError::InvalidTemplateName => "The template name is not allowed.",
            CompileError::UnknownTemplate => "The template name is not recognized.",
            CompileError::CompileTimeout => "The export timed out during compilation.",
        }
    }
}

/// Compile a request into a PDF byte stream.
///
/// This is the primary entry point. It validates limits, builds the sandbox
/// world, runs the embedded compiler, and reduces the result to a safe
/// [`CompileOutcome`]. It performs no filesystem or network access.
pub fn compile(request: &CompileRequest) -> Result<CompileOutcome, CompileError> {
    validate_request(request)?;

    let world = SandboxWorld::new(request)?;
    let Warned { output, warnings } = typst::compile::<PagedDocument>(&world);

    let mut diagnostics = warnings
        .iter()
        .map(|d| reduce_diagnostic(d, &world))
        .collect::<Vec<_>>();

    let document = match output {
        Ok(document) => document,
        Err(errors) => {
            diagnostics.extend(errors.iter().map(|d| reduce_diagnostic(d, &world)));
            diagnostics = dedup_diagnostics(diagnostics);
            return Ok(CompileOutcome::Failure { diagnostics });
        }
    };

    if document.pages().len() > MAX_PAGES {
        diagnostics.push(CompileDiagnostic {
            source: "input.typ".to_string(),
            category: "page_limit".to_string(),
            phase: "compile".to_string(),
            severity: Severity::Error,
            message: "The document exceeds the maximum page count.".to_string(),
            line: None,
            column: None,
        });
        return Ok(CompileOutcome::Failure { diagnostics });
    }

    let pdf = match typst_pdf::pdf(&document, &pdf_options()) {
        Ok(pdf) => pdf,
        Err(errors) => {
            diagnostics.extend(errors.iter().map(|d| reduce_diagnostic(d, &world)));
            diagnostics = dedup_diagnostics(diagnostics);
            return Ok(CompileOutcome::Failure { diagnostics });
        }
    };

    Ok(CompileOutcome::Success {
        pdf,
        warnings: dedup_diagnostics(diagnostics),
    })
}

#[cfg(test)]
pub(crate) fn compiled_page_count(request: &CompileRequest) -> Option<usize> {
    validate_request(request).ok()?;
    let world = SandboxWorld::new(request).ok()?;
    let Warned { output, .. } = typst::compile::<PagedDocument>(&world);
    output.ok().map(|document| document.pages().len())
}

fn dedup_diagnostics(mut diagnostics: Vec<CompileDiagnostic>) -> Vec<CompileDiagnostic> {
    let mut seen = HashSet::new();
    diagnostics.retain(|diagnostic| {
        seen.insert((
            diagnostic.source.clone(),
            diagnostic.category.clone(),
            diagnostic.phase.clone(),
            diagnostic.severity,
            diagnostic.line,
            diagnostic.column,
            diagnostic.message.clone(),
        ))
    });
    diagnostics
}

/// Validate the request against the provisional hard limits.
fn validate_request(request: &CompileRequest) -> Result<(), CompileError> {
    if request.source.len() > MAX_SOURCE_BYTES {
        return Err(CompileError::SourceTooLarge);
    }

    if let Some(template) = &request.template {
        validate_template(template)?;
    }

    let mut total = 0usize;
    let mut seen = HashSet::new();
    for attachment in &request.attachments {
        validate_attachment_path(&attachment.path)?;
        if !seen.insert(attachment.path.as_str()) {
            return Err(CompileError::DuplicateAttachmentPath);
        }
        if attachment.data.len() > MAX_ATTACHMENT_BYTES {
            return Err(CompileError::AttachmentTooLarge);
        }
        total = total.saturating_add(attachment.data.len());
    }
    if total > MAX_TOTAL_ATTACHMENT_BYTES {
        return Err(CompileError::AttachmentsTooLarge);
    }

    Ok(())
}

/// Validate an attachment virtual path.
///
/// The path must be a relative, forward-slash path with no `..` segments, no
/// leading slash, no drive/UNC prefix, and no backslash. This mirrors the
/// serializer's local-target rules and rejects anything that could escape the
/// sandbox root.
fn validate_attachment_path(path: &str) -> Result<(), CompileError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.contains('\\')
        || path.contains(':')
        || Path::new(path).is_absolute()
    {
        return Err(CompileError::InvalidAttachmentPath);
    }
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(CompileError::InvalidAttachmentPath);
        }
    }
    Ok(())
}

/// Validate an approved template's name and source.
fn validate_template(template: &Template) -> Result<(), CompileError> {
    validate_template_name(&template.name)?;
    if template.source.len() > MAX_SOURCE_BYTES {
        return Err(CompileError::TemplateTooLarge);
    }
    Ok(())
}

/// Validate a template name against the approved, fixed-name contract.
///
/// The name must be a single, safe segment that exactly equals
/// [`APPROVED_TEMPLATE_NAME`]. Empty, absolute, traversal (`.`/`..`), and
/// separator-containing names are rejected with
/// [`CompileError::InvalidTemplateName`]; any other well-formed name is rejected
/// with [`CompileError::UnknownTemplate`]. Because only the approved name is
/// accepted and it always maps to the fixed virtual path [`TEMPLATE_PATH`], a
/// template cannot influence its own virtual path and therefore cannot escape
/// the sandbox root through imports.
///
/// This function is public so the command boundary can reject unknown or
/// path-like template names before a job is persisted, rather than silently
/// ignoring them.
pub fn validate_template_name(name: &str) -> Result<(), CompileError> {
    if name.is_empty()
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.contains('/')
        || name.contains('\\')
        || name.contains(':')
        || name == "."
        || name == ".."
        || Path::new(name).is_absolute()
    {
        return Err(CompileError::InvalidTemplateName);
    }
    if name != APPROVED_TEMPLATE_NAME && name != LEGACY_TEMPLATE_NAME {
        return Err(CompileError::UnknownTemplate);
    }
    Ok(())
}

/// Build the PDF options used for every export.
fn pdf_options() -> PdfOptions {
    PdfOptions {
        ident: typst::foundations::Smart::Auto,
        creator: typst::foundations::Smart::Auto,
        timestamp: None,
        page_ranges: None,
        standards: PdfStandards::default(),
        tagged: true,
        pretty: false,
    }
}

/// Reduce a Typst [`SourceDiagnostic`] to a safe [`CompileDiagnostic`].
///
/// The message is a fixed template; the original diagnostic text (which may
/// embed source snippets) is intentionally discarded. Line/column are derived
/// from the span when it points into a known source file, using the world's
/// in-memory source text.
fn reduce_diagnostic(diag: &SourceDiagnostic, world: &SandboxWorld) -> CompileDiagnostic {
    let severity = match diag.severity {
        typst::diag::Severity::Error => Severity::Error,
        typst::diag::Severity::Warning => Severity::Warning,
    };

    let message = match severity {
        Severity::Error => "The document could not be compiled.".to_string(),
        Severity::Warning => "The document compiled with a warning.".to_string(),
    };

    let (line, column) = diag
        .span
        .id()
        .and_then(|id| world.source(id).ok())
        .and_then(|source| {
            let range = world.range(diag.span)?;
            let (line, column) = source.lines().byte_to_line_column(range.start)?;
            Some((line + 1, column + 1))
        })
        .map_or((None, None), |(l, c)| (Some(l), Some(c)));

    CompileDiagnostic {
        source: if diag.span.id() == Some(world.main()) {
            "input.typ".to_string()
        } else {
            "foundation-journal.typ".to_string()
        },
        category: match severity {
            Severity::Error => "typst_error",
            Severity::Warning => "typst_warning",
        }
        .to_string(),
        phase: "compile".to_string(),
        severity,
        message,
        line,
        column,
    }
}

/// A controlled [`World`] that serves only in-memory sources and attachments.
///
/// It owns the main source, an optional template, and the supplied attachments,
/// all keyed by normalized virtual paths. It never touches the filesystem or
/// the network. Fonts are the explicit OFL-licensed Libertinus Serif allowlist
/// so compilation is deterministic and does not depend on host-installed fonts.
struct SandboxWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: FileId,
    sources: HashMap<FileId, Source>,
    files: HashMap<FileId, Bytes>,
}

impl SandboxWorld {
    fn new(request: &CompileRequest) -> Result<Self, CompileError> {
        let library = LazyHash::new(Library::default());

        // Build the font book only from the reviewed OFL-licensed assets.
        let mut book = FontBook::new();
        let mut fonts = Vec::new();
        for (_, data) in BUNDLED_FONT_FILES {
            let bytes = Bytes::new(data);
            for font in Font::iter(bytes) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }

        let mut sources = HashMap::new();
        let mut files = HashMap::new();

        let main_id = intern_path(MAIN_PATH);
        sources.insert(main_id, Source::new(main_id, request.source.clone()));

        // The template is served as a source file under a fixed virtual path.
        // The generated source is expected to `#import` it; if no template is
        // supplied, the source is compiled standalone.
        if let Some(template) = &request.template {
            let template_id = intern_path(TEMPLATE_PATH);
            sources.insert(
                template_id,
                Source::new(template_id, template.source.clone()),
            );
        }

        for attachment in &request.attachments {
            let vpath = format!("/{}", attachment.path.trim_start_matches('/'));
            let id = intern_path(&vpath);
            files.insert(id, Bytes::new(attachment.data.clone()));
        }

        Ok(Self {
            library,
            book: LazyHash::new(book),
            fonts,
            main: main_id,
            sources,
            files,
        })
    }
}

/// Intern a virtual path into a [`FileId`] rooted at the sandbox project root.
fn intern_path(path: &str) -> FileId {
    let vpath = VirtualPath::new(path).expect("sandbox path is valid");
    RootedPath::new(VirtualRoot::Project, vpath).intern()
}

impl typst::World for SandboxWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.sources.get(&id).cloned().ok_or_else(|| {
            FileError::NotFound(Path::new(id.vpath().get_without_slash()).to_path_buf())
        })
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.get(&id).cloned().ok_or_else(|| {
            FileError::NotFound(Path::new(id.vpath().get_without_slash()).to_path_buf())
        })
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        // Deterministic local date; the export pipeline supplies explicit
        // dates in the generated source, so this is only a fallback for
        // `datetime.today()`.
        Datetime::from_ymd(2026, 1, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_request(source: &str) -> CompileRequest {
        CompileRequest {
            source: source.to_string(),
            template: None,
            attachments: Vec::new(),
        }
    }

    #[test]
    fn compiles_valid_source_to_pdf() {
        let request = simple_request("= Hello\n\nThis is a test document.\n");
        let outcome = compile(&request).expect("compile");
        match outcome {
            CompileOutcome::Success { pdf, .. } => {
                assert!(!pdf.is_empty());
                // PDF magic bytes.
                assert_eq!(&pdf[..5], b"%PDF-");
            }
            CompileOutcome::Failure { diagnostics } => {
                panic!("expected success, got diagnostics: {diagnostics:?}");
            }
        }
    }

    #[test]
    fn sandbox_exposes_only_the_six_allowlisted_libertinus_fonts() {
        let expected_files = [
            "LibertinusSerif-Regular.otf",
            "LibertinusSerif-Bold.otf",
            "LibertinusSerif-Italic.otf",
            "LibertinusSerif-BoldItalic.otf",
            "LibertinusSerif-Semibold.otf",
            "LibertinusSerif-SemiboldItalic.otf",
        ];
        assert_eq!(
            BUNDLED_FONT_FILES
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>(),
            expected_files
        );

        let world = SandboxWorld::new(&simple_request("= Font check\n")).expect("world");
        assert_eq!(world.fonts.len(), 6);
        assert_eq!(bundled_font_count(), world.fonts.len());
        assert!(world
            .fonts
            .iter()
            .all(|font| font.info().family.as_str() == "Libertinus Serif"));
    }

    #[test]
    fn malformed_source_fails_with_safe_diagnostics() {
        let request = simple_request("#set page(\n");
        let outcome = compile(&request).expect("compile");
        match outcome {
            CompileOutcome::Failure { diagnostics } => {
                assert!(!diagnostics.is_empty());
                for d in &diagnostics {
                    assert_eq!(d.severity, Severity::Error);
                    // Messages are fixed templates, never source content.
                    assert!(!d.message.contains('#'));
                }
            }
            CompileOutcome::Success { .. } => panic!("expected failure"),
        }
    }

    #[test]
    fn source_over_limit_is_rejected() {
        let mut source = String::from("= Big\n");
        source.push_str(&"x".repeat(MAX_SOURCE_BYTES));
        let request = simple_request(&source);
        assert_eq!(compile(&request).unwrap_err(), CompileError::SourceTooLarge);
    }

    #[test]
    fn attachment_over_limit_is_rejected() {
        let request = CompileRequest {
            source: "= Doc\n".to_string(),
            template: None,
            attachments: vec![Attachment {
                path: "a.png".to_string(),
                data: vec![0u8; MAX_ATTACHMENT_BYTES + 1],
            }],
        };
        assert_eq!(
            compile(&request).unwrap_err(),
            CompileError::AttachmentTooLarge
        );
    }

    #[test]
    fn total_attachments_over_limit_is_rejected() {
        let mut attachments = Vec::new();
        for i in 0..6 {
            attachments.push(Attachment {
                path: format!("a{i}.png"),
                data: vec![0u8; MAX_ATTACHMENT_BYTES],
            });
        }
        let request = CompileRequest {
            source: "= Doc\n".to_string(),
            template: None,
            attachments,
        };
        assert_eq!(
            compile(&request).unwrap_err(),
            CompileError::AttachmentsTooLarge
        );
    }

    #[test]
    fn traversal_attachment_path_is_rejected() {
        let request = CompileRequest {
            source: "= Doc\n".to_string(),
            template: None,
            attachments: vec![Attachment {
                path: "../secret.png".to_string(),
                data: vec![0u8; 4],
            }],
        };
        assert_eq!(
            compile(&request).unwrap_err(),
            CompileError::InvalidAttachmentPath
        );
    }

    #[test]
    fn absolute_attachment_path_is_rejected() {
        let request = CompileRequest {
            source: "= Doc\n".to_string(),
            template: None,
            attachments: vec![Attachment {
                path: "C:\\Users\\secret.png".to_string(),
                data: vec![0u8; 4],
            }],
        };
        assert_eq!(
            compile(&request).unwrap_err(),
            CompileError::InvalidAttachmentPath
        );
    }

    #[test]
    fn network_url_attachment_path_is_rejected() {
        let request = CompileRequest {
            source: "= Doc\n".to_string(),
            template: None,
            attachments: vec![Attachment {
                path: "https://example.com/x.png".to_string(),
                data: vec![0u8; 4],
            }],
        };
        assert_eq!(
            compile(&request).unwrap_err(),
            CompileError::InvalidAttachmentPath
        );
    }

    fn approved_template() -> Template {
        Template {
            name: APPROVED_TEMPLATE_NAME.to_string(),
            source: "#set text(size: 12pt)\n".to_string(),
        }
    }

    #[test]
    fn template_is_compiled_with_source() {
        // A template that defines a show rule; the source imports it at the
        // fixed virtual path.
        let request = CompileRequest {
            source: format!("#import \"{TEMPLATE_PATH}\": *\n= Title\n"),
            template: Some(approved_template()),
            attachments: Vec::new(),
        };
        let outcome = compile(&request).expect("compile");
        assert!(matches!(outcome, CompileOutcome::Success { .. }));
    }

    #[test]
    fn template_over_limit_is_rejected() {
        let mut source = String::from("#set text(size: 12pt)\n");
        source.push_str(&"x".repeat(MAX_SOURCE_BYTES));
        let request = CompileRequest {
            source: format!("#import \"{TEMPLATE_PATH}\": *\n= Title\n"),
            template: Some(Template {
                name: APPROVED_TEMPLATE_NAME.to_string(),
                source,
            }),
            attachments: Vec::new(),
        };
        assert_eq!(
            compile(&request).unwrap_err(),
            CompileError::TemplateTooLarge
        );
    }

    #[test]
    fn traversal_template_name_is_rejected() {
        let request = CompileRequest {
            source: "= Doc\n".to_string(),
            template: Some(Template {
                name: "../default".to_string(),
                source: "#set text(size: 12pt)\n".to_string(),
            }),
            attachments: Vec::new(),
        };
        assert_eq!(
            compile(&request).unwrap_err(),
            CompileError::InvalidTemplateName
        );
    }

    #[test]
    fn absolute_template_name_is_rejected() {
        let request = CompileRequest {
            source: "= Doc\n".to_string(),
            template: Some(Template {
                name: "C:\\templates\\default".to_string(),
                source: "#set text(size: 12pt)\n".to_string(),
            }),
            attachments: Vec::new(),
        };
        assert_eq!(
            compile(&request).unwrap_err(),
            CompileError::InvalidTemplateName
        );
    }

    #[test]
    fn empty_template_name_is_rejected() {
        let request = CompileRequest {
            source: "= Doc\n".to_string(),
            template: Some(Template {
                name: String::new(),
                source: "#set text(size: 12pt)\n".to_string(),
            }),
            attachments: Vec::new(),
        };
        assert_eq!(
            compile(&request).unwrap_err(),
            CompileError::InvalidTemplateName
        );
    }

    #[test]
    fn dot_dot_template_name_is_rejected() {
        let request = CompileRequest {
            source: "= Doc\n".to_string(),
            template: Some(Template {
                name: "..".to_string(),
                source: "#set text(size: 12pt)\n".to_string(),
            }),
            attachments: Vec::new(),
        };
        assert_eq!(
            compile(&request).unwrap_err(),
            CompileError::InvalidTemplateName
        );
    }

    #[test]
    fn separator_in_template_name_is_rejected() {
        let request = CompileRequest {
            source: "= Doc\n".to_string(),
            template: Some(Template {
                name: "a/b".to_string(),
                source: "#set text(size: 12pt)\n".to_string(),
            }),
            attachments: Vec::new(),
        };
        assert_eq!(
            compile(&request).unwrap_err(),
            CompileError::InvalidTemplateName
        );
    }

    #[test]
    fn unknown_template_name_is_rejected() {
        let request = CompileRequest {
            source: "= Doc\n".to_string(),
            template: Some(Template {
                name: "professional".to_string(),
                source: "#set text(size: 12pt)\n".to_string(),
            }),
            attachments: Vec::new(),
        };
        assert_eq!(
            compile(&request).unwrap_err(),
            CompileError::UnknownTemplate
        );
    }

    #[test]
    fn duplicate_attachment_path_is_rejected() {
        let request = CompileRequest {
            source: "= Doc\n".to_string(),
            template: None,
            attachments: vec![
                Attachment {
                    path: "a.png".to_string(),
                    data: vec![0u8; 4],
                },
                Attachment {
                    path: "a.png".to_string(),
                    data: vec![0u8; 4],
                },
            ],
        };
        assert_eq!(
            compile(&request).unwrap_err(),
            CompileError::DuplicateAttachmentPath
        );
    }

    #[test]
    fn approved_template_name_matches_template_path() {
        // Keep the public name and its fixed virtual path in sync so the
        // generated source can import the template.
        assert!(TEMPLATE_PATH.ends_with(&format!("/{APPROVED_TEMPLATE_NAME}.typ")));
    }

    #[test]
    fn missing_attachment_fails_safely() {
        // The source references an image that was not supplied.
        let request = simple_request("#image(\"assets/missing.png\")\n");
        let outcome = compile(&request).expect("compile");
        match outcome {
            CompileOutcome::Failure { diagnostics } => {
                assert!(!diagnostics.is_empty());
            }
            CompileOutcome::Success { .. } => panic!("expected failure for missing asset"),
        }
    }

    #[test]
    fn supplied_attachment_resolves() {
        // A valid 1x1 transparent PNG.
        let png: Vec<u8> = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78,
            0xDA, 0x63, 0x64, 0x60, 0xF8, 0x5F, 0x0F, 0x00, 0x02, 0x87, 0x01, 0x80, 0xEB, 0x47,
            0xBA, 0x92, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        let request = CompileRequest {
            source: "#image(\"assets/pic.png\")\n".to_string(),
            template: None,
            attachments: vec![Attachment {
                path: "assets/pic.png".to_string(),
                data: png,
            }],
        };
        let outcome = compile(&request).expect("compile");
        assert!(matches!(outcome, CompileOutcome::Success { .. }));
    }

    #[test]
    fn diagnostics_never_contain_source_text() {
        // A source with a distinctive secret string that must not leak.
        let request = simple_request("#let secret = \"TOP-SECRET-VALUE\"\n#unknown_fn()\n");
        let outcome = compile(&request).expect("compile");
        if let CompileOutcome::Failure { diagnostics } = outcome {
            for d in &diagnostics {
                assert!(!d.message.contains("TOP-SECRET-VALUE"));
                assert!(!d.message.contains("unknown_fn"));
            }
        }
    }
}
