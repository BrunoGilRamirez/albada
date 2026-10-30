use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::AppError;

use super::discovery::DiscoveredFile;

/// The class of a legacy-import warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningKind {
    DateMismatch,
    Template,
    AmbiguousCheckbox,
    BrokenLink,
    MissingTemplate,
    TherapyInGeneralDiary,
}

/// A reviewable warning attached to one legacy source path. `severity` uses the
/// frontend's `info | warning | error` vocabulary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportWarning {
    pub kind: WarningKind,
    pub severity: String,
    pub source_path: String,
    pub line: Option<i64>,
    pub message: String,
}

impl ImportWarning {
    pub fn new(
        kind: WarningKind,
        severity: &str,
        source_path: &str,
        line: Option<i64>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            severity: severity.to_string(),
            source_path: source_path.to_string(),
            line,
            message: message.into(),
        }
    }
}

const PLACEHOLDER_MARKERS: &[&str] = &[
    "insert your text here",
    "insert day name here",
    "link_if_applicable",
];

/// Focus, energy, and mental-state options from the legacy templates. These are
/// selection/state checkboxes, not executable tasks.
const OPTION_CHECKBOXES: &[&str] = &[
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

/// Run every warning detector against one discovered file.
pub fn detect_warnings(file: &DiscoveredFile) -> Result<Vec<ImportWarning>, AppError> {
    let mut warnings = Vec::new();

    if let Some(filename_date) = &file.filename_date {
        if let Some(title_date) = &file.title_date {
            if filename_date != title_date {
                let line = file.headings.first().map(|h| h.line as i64);
                warnings.push(ImportWarning::new(
                    WarningKind::DateMismatch,
                    "warning",
                    &file.relative_path,
                    line,
                    format!(
                        "Filename date '{filename_date}' does not match title date '{title_date}'."
                    ),
                ));
            }
        }
    }

    if let Some(line) = detect_template(file) {
        warnings.push(ImportWarning::new(
            WarningKind::Template,
            "info",
            &file.relative_path,
            Some(line),
            "The file looks like a template or contains unfilled placeholders.",
        ));
    }

    if has_ambiguous_checkboxes(&file.checkboxes) {
        warnings.push(ImportWarning::new(
            WarningKind::AmbiguousCheckbox,
            "warning",
            &file.relative_path,
            None,
            "The file mixes option/state checkboxes with task-like checkboxes.",
        ));
    }

    for target in &file.links {
        if is_relative_link(target) {
            let resolved = resolve_relative(&file.absolute_path, target);
            let exists = resolved.as_deref().is_some_and(|p| Path::new(p).exists());
            if !exists {
                if target.to_lowercase().contains("template") {
                    warnings.push(ImportWarning::new(
                        WarningKind::MissingTemplate,
                        "warning",
                        &file.relative_path,
                        None,
                        format!("Referenced template '{target}' was not found."),
                    ));
                } else {
                    warnings.push(ImportWarning::new(
                        WarningKind::BrokenLink,
                        "warning",
                        &file.relative_path,
                        None,
                        format!("Relative link '{target}' does not resolve to an existing file."),
                    ));
                }
            }
        }
    }

    if !file.is_therapy_path && file.potential_therapy_keywords {
        warnings.push(ImportWarning::new(
            WarningKind::TherapyInGeneralDiary,
            "warning",
            &file.relative_path,
            None,
            "Possible therapy-related content found in a non-therapy path; review before importing.",
        ));
    }

    Ok(warnings)
}

fn detect_template(file: &DiscoveredFile) -> Option<i64> {
    let name = file
        .absolute_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name == "2000-00-00-raw.md" || name.contains("template") {
        return file.headings.first().map(|h| h.line as i64).or(Some(1));
    }
    for (i, line) in file.content.lines().enumerate() {
        let lowered = line.to_lowercase();
        if PLACEHOLDER_MARKERS.iter().any(|m| lowered.contains(m)) {
            return Some((i + 1) as i64);
        }
        if line.trim() == "..." {
            return Some((i + 1) as i64);
        }
    }
    None
}

fn has_ambiguous_checkboxes(checkboxes: &[super::discovery::Checkbox]) -> bool {
    let mut has_option = false;
    let mut has_task_like = false;
    for checkbox in checkboxes {
        let normalized = checkbox.text.trim().to_lowercase();
        if normalized.is_empty() || normalized == "..." {
            continue;
        }
        if OPTION_CHECKBOXES
            .iter()
            .any(|opt| normalized == *opt || normalized.contains(opt))
        {
            has_option = true;
        } else {
            has_task_like = true;
        }
    }
    has_option && has_task_like
}

fn is_relative_link(target: &str) -> bool {
    let lowered = target.to_lowercase();
    !(lowered.starts_with("http://")
        || lowered.starts_with("https://")
        || lowered.starts_with("mailto:")
        || target.starts_with('/')
        || target.starts_with('#'))
}

fn resolve_relative(file_path: &Path, target: &str) -> Option<String> {
    let dir = file_path.parent()?;
    Some(dir.join(target).to_string_lossy().into_owned())
}
