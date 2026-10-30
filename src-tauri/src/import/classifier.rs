use super::discovery::DiscoveredFile;

pub const CONFIDENCE_HIGH: &str = "high";
pub const CONFIDENCE_MEDIUM: &str = "medium";
pub const CONFIDENCE_LOW: &str = "low";
pub const CONFIDENCE_REVIEW: &str = "review";

/// Target entity for a legacy content item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    JournalEntry,
    TherapyRecord,
    Task,
    Goal,
    Project,
    YearTheme,
    Quarter,
    QuarterFocus,
    Note,
}

impl EntityKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityKind::JournalEntry => "journal_entry",
            EntityKind::TherapyRecord => "therapy_record",
            EntityKind::Task => "task",
            EntityKind::Goal => "goal",
            EntityKind::Project => "project",
            EntityKind::YearTheme => "year_theme",
            EntityKind::Quarter => "quarter",
            EntityKind::QuarterFocus => "quarter_focus",
            EntityKind::Note => "note",
        }
    }
}

/// A classified legacy content item. Items with `confidence == review` require a
/// human decision and are never inserted as entities.
#[derive(Debug, Clone)]
pub struct Classification {
    pub source_path: String,
    pub entity_kind: EntityKind,
    pub confidence: &'static str,
    pub title: Option<String>,
    pub snippet: Option<String>,
    pub source_date: Option<String>,
    pub therapy_record_type: Option<String>,
    /// SHA-256 over the exact source file content, hex-encoded.
    pub source_hash: String,
}

impl Classification {
    fn review(
        source_path: &str,
        entity_kind: EntityKind,
        title: Option<String>,
        snippet: Option<String>,
        source_date: Option<String>,
        source_hash: String,
    ) -> Self {
        Self {
            source_path: source_path.to_string(),
            entity_kind,
            confidence: CONFIDENCE_REVIEW,
            title,
            snippet,
            source_date,
            therapy_record_type: None,
            source_hash,
        }
    }
}

/// Classify one file. A single file may produce several items: the file-level
/// entity plus a task-candidate review item for each task-like checkbox.
///
/// Only Markdown and plain-text files reach this point; unsupported extensions
/// are filtered out during discovery and never classified.
pub fn classify(file: &DiscoveredFile) -> Vec<Classification> {
    let mut items = vec![classify_file(file)];

    for checkbox in &file.checkboxes {
        if is_task_like_checkbox(&checkbox.text) {
            items.push(Classification::review(
                &file.relative_path,
                EntityKind::Task,
                Some(checkbox.text.clone()),
                None,
                file.filename_date.clone(),
                file.content_hash.clone(),
            ));
        }
    }

    items
}

fn classify_file(file: &DiscoveredFile) -> Classification {
    let title = derive_title(file);
    let snippet = derive_snippet(file);
    let date = file
        .filename_date
        .clone()
        .or_else(|| file.title_date.clone());

    classify_text(file, title, snippet, date)
}

/// Classify a supported text file (Markdown or plain text).
///
/// Path names are compatibility hints only. They never grant enough confidence
/// to create a journal or therapy record automatically. Every path-derived
/// classification is surfaced for review so a repository with a different
/// layout cannot be imported into the wrong private collection.
fn classify_text(
    file: &DiscoveredFile,
    title: Option<String>,
    snippet: Option<String>,
    date: Option<String>,
) -> Classification {
    let path = &file.relative_path;

    if is_template_path(path) {
        return Classification::review(
            path,
            template_kind(path),
            title,
            snippet,
            date,
            file.content_hash.clone(),
        );
    }

    // Folder names are hints only. Even the known private therapy subtree must
    // be explicitly reviewed before it becomes a TherapyRecord.
    if file.is_therapy_path {
        return Classification::review(
            path,
            EntityKind::TherapyRecord,
            title,
            snippet,
            date,
            file.content_hash.clone(),
        );
    }

    if path_contains(path, "/personal-notes-raw/") || path.ends_with("/personal-notes-raw") {
        return Classification::review(
            path,
            EntityKind::JournalEntry,
            title,
            snippet,
            date,
            file.content_hash.clone(),
        );
    }

    if path_contains(path, "/diary/")
        || path.ends_with("/diary")
        || path_contains(path, "journal/entries/")
    {
        return Classification::review(
            path,
            EntityKind::JournalEntry,
            title,
            snippet,
            date,
            file.content_hash.clone(),
        );
    }

    if path_contains(path, "/planning/") {
        return Classification::review(
            path,
            EntityKind::Quarter,
            title,
            snippet,
            date,
            file.content_hash.clone(),
        );
    }
    if path_contains(path, "project-assets") {
        return Classification::review(
            path,
            EntityKind::Project,
            title,
            snippet,
            date,
            file.content_hash.clone(),
        );
    }
    if path_contains(path, "/notes/") {
        return Classification::review(
            path,
            EntityKind::Note,
            title,
            snippet,
            date,
            file.content_hash.clone(),
        );
    }
    if path_contains(path, "/more/") {
        return Classification::review(
            path,
            EntityKind::Project,
            title,
            snippet,
            date,
            file.content_hash.clone(),
        );
    }

    Classification::review(
        path,
        EntityKind::Note,
        title,
        snippet,
        date,
        file.content_hash.clone(),
    )
}

fn is_template_path(relative_path: &str) -> bool {
    let lowered = relative_path.to_ascii_lowercase();
    lowered.contains("/templates/")
        || lowered.starts_with("templates/")
        || lowered.contains("2000-00-00")
}

fn template_kind(relative_path: &str) -> EntityKind {
    let lowered = relative_path.to_ascii_lowercase();
    if lowered.contains("plan") {
        EntityKind::Goal
    } else if lowered.contains("raw") || lowered.contains("diary") || lowered.contains("diario") {
        EntityKind::JournalEntry
    } else {
        EntityKind::Note
    }
}

fn path_contains(relative_path: &str, needle: &str) -> bool {
    relative_path.to_ascii_lowercase().contains(needle)
}

fn derive_title(file: &DiscoveredFile) -> Option<String> {
    // Prefer the first H1 heading.
    if let Some(h1) = file.headings.iter().find(|h| h.level == 1) {
        let text = h1.text.trim();
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    // Fall back to the first heading of any level.
    if let Some(heading) = file.headings.first() {
        let text = heading.text.trim();
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    // Generic title derived from the filename date and/or file name.
    derive_generic_title(file)
}

/// Build a generic title when no heading is available: `Journal entry — <date>`
/// when the filename carries a date, otherwise the file stem. This only affects
/// the title field; content is never rewritten.
fn derive_generic_title(file: &DiscoveredFile) -> Option<String> {
    if let Some(date) = &file.filename_date {
        return Some(format!("Journal entry — {date}"));
    }
    file.absolute_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn derive_snippet(file: &DiscoveredFile) -> Option<String> {
    for line in file.content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let text = trimmed
            .trim_start_matches(['-', '*', '+', ' '])
            .trim_start_matches(['[', ']', 'x', 'X', ' ']);
        if text.is_empty() || text == "..." {
            continue;
        }
        let snippet: String = text.chars().take(200).collect();
        return Some(snippet);
    }
    None
}

fn is_task_like_checkbox(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed == "..." {
        return false;
    }
    let normalized = trimmed.to_lowercase();
    !is_option_checkbox(&normalized)
}

fn is_option_checkbox(normalized: &str) -> bool {
    const OPTIONS: &[&str] = &[
        "proyecto joi",
        "algoritmos",
        "especialización",
        "especializacion",
        "análisis de datos",
        "analisis de datos",
        "ia",
        "inglés",
        "ingles",
        "libre",
        "alta",
        "media",
        "baja",
        "saturado",
        "neutral",
        "motivado",
        "cansado",
        "entusiasmado",
        "satisfecho",
        "ansioso",
        "desmotivado",
        "irritado",
    ];
    OPTIONS
        .iter()
        .any(|o| normalized == *o || normalized.contains(o))
}

#[cfg(test)]
mod tests {
    use super::super::discovery::FileKind;
    use super::*;
    use std::path::PathBuf;

    fn file(
        relative_path: &str,
        kind: FileKind,
        content: &str,
        filename_date: Option<&str>,
    ) -> DiscoveredFile {
        let headings = super::super::discovery::parse_headings(content);
        let checkboxes = super::super::discovery::parse_checkboxes(content);
        let content_hash = super::super::discovery::compute_content_hash_str(content);
        DiscoveredFile {
            relative_path: relative_path.to_string(),
            absolute_path: PathBuf::from(relative_path),
            kind,
            content: content.to_string(),
            content_hash,
            headings,
            checkboxes,
            links: Vec::new(),
            filename_date: filename_date.map(str::to_string),
            title_date: None,
            is_therapy_path: super::super::discovery::path_is_therapy(relative_path),
            potential_therapy_keywords: false,
        }
    }

    #[test]
    fn entity_kind_renders_snake_case() {
        assert_eq!(EntityKind::JournalEntry.as_str(), "journal_entry");
        assert_eq!(EntityKind::TherapyRecord.as_str(), "therapy_record");
        assert_eq!(EntityKind::Task.as_str(), "task");
    }

    #[test]
    fn option_checkboxes_are_not_tasks() {
        assert!(!is_task_like_checkbox("Alta"));
        assert!(!is_task_like_checkbox("Proyecto JOI"));
        assert!(!is_task_like_checkbox("..."));
        assert!(!is_task_like_checkbox(""));
        assert!(is_task_like_checkbox("Migrar la API de reportes"));
    }

    #[test]
    fn title_prefers_first_h1() {
        let f = file(
            "notes/example.md",
            FileKind::Markdown,
            "## Subtitle\n# Real Title\nBody\n",
            None,
        );
        assert_eq!(derive_title(&f).as_deref(), Some("Real Title"));
    }

    #[test]
    fn title_falls_back_to_generic_from_date_and_name() {
        let f = file(
            "records/2026-06-25-raw.md",
            FileKind::Markdown,
            "No heading here.\n",
            Some("2026-06-25"),
        );
        assert_eq!(
            derive_title(&f).as_deref(),
            Some("Journal entry — 2026-06-25")
        );
    }

    #[test]
    fn title_falls_back_to_stem_without_date() {
        let f = file(
            "records/notes/reflection.md",
            FileKind::Markdown,
            "No heading here.\n",
            None,
        );
        assert_eq!(derive_title(&f).as_deref(), Some("reflection"));
    }

    #[test]
    fn personal_notes_raw_markdown_is_review_hint() {
        let f = file(
            "records/2026-Foundation/Q2/diary/personal-notes-raw/2026-06-25-raw.md",
            FileKind::Markdown,
            "# Diario\n\nContenido personal.\n",
            Some("2026-06-25"),
        );
        let items = classify(&f);
        assert_eq!(items[0].entity_kind, EntityKind::JournalEntry);
        assert_eq!(items[0].confidence, CONFIDENCE_REVIEW);
    }

    #[test]
    fn personal_notes_raw_txt_is_review_hint() {
        let f = file(
            "records/2026-Foundation/Q2/diary/personal-notes-raw/plain.txt",
            FileKind::Text,
            "Plain text content.\n",
            None,
        );
        let items = classify(&f);
        assert_eq!(items[0].entity_kind, EntityKind::JournalEntry);
        assert_eq!(items[0].confidence, CONFIDENCE_REVIEW);
    }

    #[test]
    fn notes_path_is_review_not_journal_entry() {
        let f = file(
            "records/2026-Foundation/Q2/notes/reflection.md",
            FileKind::Markdown,
            "# Reflection\n\nSome content.\n",
            None,
        );
        let items = classify(&f);
        assert_eq!(items[0].entity_kind, EntityKind::Note);
        assert_eq!(items[0].confidence, CONFIDENCE_REVIEW);
    }

    #[test]
    fn structured_diary_is_review_not_journal_entry() {
        let f = file(
            "records/2026-Foundation/Q2/diary/2026-06-25.md",
            FileKind::Markdown,
            "# Diario\n\nContenido estructurado.\n",
            Some("2026-06-25"),
        );
        let items = classify(&f);
        assert_eq!(items[0].entity_kind, EntityKind::JournalEntry);
        assert_eq!(items[0].confidence, CONFIDENCE_REVIEW);
    }

    #[test]
    fn planning_is_review_quarter() {
        let f = file(
            "records/2026-Foundation/Q2/planning/plan-Q2.md",
            FileKind::Markdown,
            "# Plan Q2\n\nContenido.\n",
            None,
        );
        let items = classify(&f);
        assert_eq!(items[0].entity_kind, EntityKind::Quarter);
        assert_eq!(items[0].confidence, CONFIDENCE_REVIEW);
    }

    #[test]
    fn more_path_is_review_project() {
        let f = file(
            "records/2026-Foundation/Q2/more/project-assets/portafolio.md",
            FileKind::Markdown,
            "# Portafolio\n\nContenido.\n",
            None,
        );
        let items = classify(&f);
        assert_eq!(items[0].entity_kind, EntityKind::Project);
        assert_eq!(items[0].confidence, CONFIDENCE_REVIEW);
    }

    #[test]
    fn unknown_path_is_review_note() {
        let f = file(
            "records/2026-Foundation/Q2/other/unknown.md",
            FileKind::Markdown,
            "# Unknown\n\nContenido.\n",
            None,
        );
        let items = classify(&f);
        assert_eq!(items[0].entity_kind, EntityKind::Note);
        assert_eq!(items[0].confidence, CONFIDENCE_REVIEW);
    }

    #[test]
    fn therapy_path_is_review_hint() {
        let f = file(
            "salud-mental/Notas/terapia.md",
            FileKind::Markdown,
            "# Terapia\n\nContenido.\n",
            None,
        );
        let items = classify(&f);
        assert_eq!(items[0].entity_kind, EntityKind::TherapyRecord);
        assert_eq!(items[0].confidence, CONFIDENCE_REVIEW);
    }

    #[test]
    fn template_is_review_not_entry() {
        let f = file(
            "templates/daily-template.md",
            FileKind::Markdown,
            "# Daily template\n\ninsert your text here\n",
            None,
        );
        let items = classify(&f);
        assert_eq!(items[0].confidence, CONFIDENCE_REVIEW);
    }
}
