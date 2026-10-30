use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppError;
use sha2::{Digest, Sha256};

/// File kind used to route classification and warnings.
///
/// Only Markdown and plain-text files are discovered. All other extensions
/// (`.tex`, `.pdf`, images, JSON, Python, binaries, etc.) are silently ignored
/// before any content is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Markdown,
    Text,
}

/// Directory names that are pruned from discovery at any depth. These are
/// version-control, dependency, build, virtualenv, cache, and editor
/// directories that are not treated as user content. Content-like names such
/// as `docs`, `templates`, `scripts`, and `agent` are deliberately not listed:
/// they may be valid directories in an arbitrary source repository. Matching
/// is case-insensitive.
const PRUNED_DIRS: &[&str] = &[
    ".git",
    ".github",
    ".gitlab",
    ".hg",
    ".svn",
    ".agents",
    ".opencode",
    "node_modules",
    "bower_components",
    "target",
    ".venv",
    "venv",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".cache",
    ".coverage",
    ".idea",
    ".vscode",
    ".obsidian",
    ".trash",
    ".next",
    ".turbo",
    ".gradle",
    ".cargo",
    ".tox",
    ".eggs",
];

/// One Markdown heading (`#`..`######`) with its 1-based line number.
#[derive(Debug, Clone)]
pub struct Heading {
    pub level: usize,
    pub text: String,
    pub line: usize,
}

/// One Markdown checkbox (`- [ ]` / `- [x]`) with its 1-based line number.
#[derive(Debug, Clone)]
pub struct Checkbox {
    pub checked: bool,
    pub text: String,
    pub line: usize,
}

/// A read-only snapshot of one legacy file discovered during import.
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    /// Path relative to the import root, preserving the original layout.
    pub relative_path: String,
    pub absolute_path: PathBuf,
    pub kind: FileKind,
    pub content: String,
    pub content_hash: String,
    pub headings: Vec<Heading>,
    pub checkboxes: Vec<Checkbox>,
    pub links: Vec<String>,
    pub filename_date: Option<String>,
    pub title_date: Option<String>,
    pub is_therapy_path: bool,
    pub potential_therapy_keywords: bool,
}

/// Keywords that, when found in a non-therapy path, flag the file for review.
/// These are never used to silently classify content as therapy.
pub const THERAPY_KEYWORDS: &[&str] = &[
    "terapia",
    "therapy",
    "salud mental",
    "accepts",
    "emociones",
    "emoción",
    "depresión",
    "depresion",
    "ansiedad",
    "suicidio",
    "terapeuta",
    "ideación",
    "ideacion",
];

/// Recursively walk `root` without modifying it, returning a read-only snapshot
/// of every supported file. Directories are traversed deterministically (sorted
/// by name) so results are stable across runs.
pub fn discover(root: &Path) -> Result<Vec<DiscoveredFile>, AppError> {
    if !root.exists() {
        return Err(AppError::validation(
            "The import source path does not exist.",
        ));
    }
    if !root.is_dir() {
        return Err(AppError::validation(
            "The import source path is not a directory.",
        ));
    }

    let mut files = Vec::new();
    walk(root, root, &mut files)?;
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(files)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<DiscoveredFile>) -> Result<(), AppError> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<_, _>>()?;
    entries.sort();

    for path in entries {
        let metadata = fs::metadata(&path)?;
        if metadata.is_dir() {
            // Prune development/cache directories before descending into them,
            // so their contents are never read or surfaced.
            if !is_pruned_dir(&path) {
                walk(root, &path, out)?;
            }
        } else if metadata.is_file() {
            if let Some(file) = read_file(root, &path) {
                out.push(file);
            }
        }
    }
    Ok(())
}

/// True when `path` is a directory whose name matches a pruned development or
/// cache directory (case-insensitive).
fn is_pruned_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|name| {
            let lowered = name.to_ascii_lowercase();
            PRUNED_DIRS.iter().any(|d| lowered == *d)
        })
        .unwrap_or(false)
}

fn read_file(root: &Path, path: &Path) -> Option<DiscoveredFile> {
    let relative_path = path
        .strip_prefix(root)
        .ok()
        .and_then(|p| p.to_str())
        .map(|s| s.replace('\\', "/"))?;

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    // Only `.md`, `.txt`, and `.text` are discovered (case-insensitive). Every
    // other extension — `.tex`, `.pdf`, images, JSON, Python, binaries — is
    // silently ignored before any content is read.
    let kind = if name.ends_with(".md") {
        FileKind::Markdown
    } else if name.ends_with(".txt") || name.ends_with(".text") {
        FileKind::Text
    } else {
        return None;
    };

    let content = fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();

    let content_hash = compute_content_hash_str(&content);

    let headings = parse_headings(&content);
    let checkboxes = parse_checkboxes(&content);
    let links = parse_links(&content);
    let filename_date = extract_date(&relative_path);
    let title_date = extract_title_date(&headings);
    let is_therapy_path = path_is_therapy(&relative_path);
    let potential_therapy_keywords = detect_therapy_keywords(&content);

    Some(DiscoveredFile {
        relative_path,
        absolute_path: path.to_path_buf(),
        kind,
        content,
        content_hash,
        headings,
        checkboxes,
        links,
        filename_date,
        title_date,
        is_therapy_path,
        potential_therapy_keywords,
    })
}

pub(crate) fn parse_headings(content: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        let level = trimmed.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&level) && trimmed.as_bytes().get(level) == Some(&b' ') {
            let text = trimmed[level..].trim().to_string();
            if !text.is_empty() {
                headings.push(Heading {
                    level,
                    text,
                    line: i + 1,
                });
            }
        }
    }
    headings
}

pub(crate) fn parse_checkboxes(content: &str) -> Vec<Checkbox> {
    let mut checkboxes = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        let rest = trimmed
            .strip_prefix("- [ ]")
            .or_else(|| trimmed.strip_prefix("* [ ]"))
            .or_else(|| trimmed.strip_prefix("+ [ ]"))
            .map(|r| (false, r));
        let rest = rest.or_else(|| {
            trimmed
                .strip_prefix("- [x]")
                .or_else(|| trimmed.strip_prefix("* [x]"))
                .or_else(|| trimmed.strip_prefix("+ [x]"))
                .map(|r| (true, r))
        });
        if let Some((checked, rest)) = rest {
            checkboxes.push(Checkbox {
                checked,
                text: rest.trim().to_string(),
                line: i + 1,
            });
        }
    }
    checkboxes
}

fn parse_links(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    for line in content.lines() {
        // Collect `[text](target)` and `](target)` targets.
        let bytes = line.as_bytes();
        let mut i = 0;
        while i + 1 < bytes.len() {
            if bytes[i] == b']' && bytes[i + 1] == b'(' {
                let start = i + 2;
                let mut end = start;
                while end < bytes.len() && bytes[end] != b')' {
                    end += 1;
                }
                if end > start {
                    let target = &line[start..end];
                    let target = target.split_whitespace().next().unwrap_or("");
                    if !target.is_empty() {
                        links.push(target.to_string());
                    }
                }
                i = end + 1;
            } else {
                i += 1;
            }
        }
    }
    links
}

/// Return the first `YYYY-MM-DD` date found in `text`, if any.
pub fn extract_date(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    for i in 0..bytes.len().saturating_sub(9) {
        if is_date_at(bytes, i) {
            return Some(text[i..i + 10].to_string());
        }
    }
    None
}

fn is_date_at(bytes: &[u8], i: usize) -> bool {
    bytes[i].is_ascii_digit()
        && bytes[i + 1].is_ascii_digit()
        && bytes[i + 2].is_ascii_digit()
        && bytes[i + 3].is_ascii_digit()
        && bytes[i + 4] == b'-'
        && bytes[i + 5].is_ascii_digit()
        && bytes[i + 6].is_ascii_digit()
        && bytes[i + 7] == b'-'
        && bytes[i + 8].is_ascii_digit()
        && bytes[i + 9].is_ascii_digit()
}

fn extract_title_date(headings: &[Heading]) -> Option<String> {
    headings.iter().find_map(|h| extract_date(&h.text))
}

/// True when the path lives under the private `salud-mental/` tree.
pub fn path_is_therapy(relative_path: &str) -> bool {
    let first = relative_path.split('/').next().unwrap_or("");
    first.eq_ignore_ascii_case("salud-mental")
}

fn detect_therapy_keywords(content: &str) -> bool {
    let lowered = content.to_lowercase();
    THERAPY_KEYWORDS.iter().any(|k| lowered.contains(k))
}

/// Normalize and validate a user-supplied include path.
///
/// Returns the normalized relative prefix (forward slashes, no leading `./`,
/// no trailing `/`). Rejects absolute paths and paths that escape the source
/// root via `..` segments.
pub fn normalize_include_path(input: &str) -> Result<String, AppError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("An include path cannot be empty."));
    }

    // Reject absolute paths: Unix `/...` or `\...`, and Windows drive letters.
    if trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return Err(AppError::validation(
            "Include paths must be relative to the source root.",
        ));
    }
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return Err(AppError::validation(
            "Include paths must be relative to the source root.",
        ));
    }

    // Normalize separators and strip leading `./`.
    let mut normalized = trimmed.replace('\\', "/");
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_string();
    }

    // Reject `..` segments that would escape the root.
    for segment in normalized.split('/') {
        if segment == ".." {
            return Err(AppError::validation("Include paths must not contain '..'."));
        }
    }

    let normalized = normalized.trim_end_matches('/').to_string();
    if normalized.is_empty() {
        return Err(AppError::validation("An include path cannot be empty."));
    }
    Ok(normalized)
}

/// True when `relative_path` equals `prefix` or lives under `prefix/`.
fn path_matches_prefix(relative_path: &str, prefix: &str) -> bool {
    relative_path == prefix || relative_path.starts_with(&format!("{prefix}/"))
}

/// Filter discovered files to those under any of the include prefixes.
///
/// Each include path is normalized and validated first. When `include_paths` is
/// empty, all files are returned unchanged.
pub fn filter_by_include_paths(
    files: Vec<DiscoveredFile>,
    include_paths: &[String],
) -> Result<Vec<DiscoveredFile>, AppError> {
    if include_paths.is_empty() {
        return Ok(files);
    }
    let prefixes: Vec<String> = include_paths
        .iter()
        .map(|p| normalize_include_path(p))
        .collect::<Result<_, _>>()?;
    Ok(files
        .into_iter()
        .filter(|f| {
            prefixes
                .iter()
                .any(|p| path_matches_prefix(&f.relative_path, p))
        })
        .collect())
}

/// Compute SHA-256 hash of a file's content, returned as hex string.
pub fn compute_content_hash(path: &Path) -> Result<String, AppError> {
    let content = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Compute SHA-256 hash of a string content, returned as hex string.
pub fn compute_content_hash_str(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_date_finds_first_iso_date() {
        assert_eq!(extract_date("2026-06-25-raw.md"), Some("2026-06-25".into()));
        assert_eq!(
            extract_date("# Diario 2026-05-29"),
            Some("2026-05-29".into())
        );
        assert_eq!(extract_date("no date here"), None);
    }

    #[test]
    fn headings_and_checkboxes_are_parsed() {
        let content = "# Title\n- [ ] A task\n- [x] Done\n";
        let headings = parse_headings(content);
        let checkboxes = parse_checkboxes(content);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "Title");
        assert_eq!(checkboxes.len(), 2);
        assert!(!checkboxes[0].checked);
        assert!(checkboxes[1].checked);
    }

    #[test]
    fn therapy_path_detection_is_prefix_based() {
        assert!(path_is_therapy("salud-mental/Notas/ACCEPTS.md"));
        assert!(!path_is_therapy(
            "records/2026-Foundation/Q2/diary/2026-01-01.md"
        ));
    }

    #[test]
    fn normalize_include_path_handles_separators_and_dots() {
        assert_eq!(
            normalize_include_path("salud-mental/").expect("normalize"),
            "salud-mental"
        );
        assert_eq!(
            normalize_include_path("./records/2026-Foundation/Q2/diary/personal-notes-raw/")
                .expect("normalize"),
            "records/2026-Foundation/Q2/diary/personal-notes-raw"
        );
        assert_eq!(
            normalize_include_path("records\\2026-Foundation\\Q2").expect("normalize"),
            "records/2026-Foundation/Q2"
        );
    }

    #[test]
    fn normalize_include_path_rejects_absolute_and_parent() {
        assert!(normalize_include_path("/etc/passwd").is_err());
        assert!(normalize_include_path("C:\\Users\\x").is_err());
        assert!(normalize_include_path("../outside").is_err());
        assert!(normalize_include_path("records/../../outside").is_err());
        assert!(normalize_include_path("").is_err());
        assert!(normalize_include_path("   ").is_err());
    }

    #[test]
    fn filter_by_include_paths_selects_only_matching_prefixes() {
        let empty_hash = compute_content_hash_str("");
        let files = vec![
            DiscoveredFile {
                relative_path: "salud-mental/Notas/terapia.md".to_string(),
                absolute_path: PathBuf::from("salud-mental/Notas/terapia.md"),
                kind: FileKind::Markdown,
                content: String::new(),
                content_hash: empty_hash.clone(),
                headings: Vec::new(),
                checkboxes: Vec::new(),
                links: Vec::new(),
                filename_date: None,
                title_date: None,
                is_therapy_path: true,
                potential_therapy_keywords: false,
            },
            DiscoveredFile {
                relative_path:
                    "records/2026-Foundation/Q2/diary/personal-notes-raw/2026-06-25-raw.md"
                        .to_string(),
                absolute_path: PathBuf::from(
                    "records/2026-Foundation/Q2/diary/personal-notes-raw/2026-06-25-raw.md",
                ),
                kind: FileKind::Markdown,
                content: String::new(),
                content_hash: empty_hash.clone(),
                headings: Vec::new(),
                checkboxes: Vec::new(),
                links: Vec::new(),
                filename_date: None,
                title_date: None,
                is_therapy_path: false,
                potential_therapy_keywords: false,
            },
            DiscoveredFile {
                relative_path: "records/2026-Foundation/Q2/planning/plan-Q2.md".to_string(),
                absolute_path: PathBuf::from("records/2026-Foundation/Q2/planning/plan-Q2.md"),
                kind: FileKind::Markdown,
                content: String::new(),
                content_hash: empty_hash,
                headings: Vec::new(),
                checkboxes: Vec::new(),
                links: Vec::new(),
                filename_date: None,
                title_date: None,
                is_therapy_path: false,
                potential_therapy_keywords: false,
            },
        ];

        let filtered = filter_by_include_paths(
            files,
            &[
                "salud-mental".to_string(),
                "records/2026-Foundation/Q2/diary/personal-notes-raw".to_string(),
            ],
        )
        .expect("filter");

        assert_eq!(filtered.len(), 2);
        assert!(filtered
            .iter()
            .any(|f| f.relative_path.starts_with("salud-mental/")));
        assert!(filtered
            .iter()
            .any(|f| f.relative_path.contains("personal-notes-raw")));
        assert!(!filtered
            .iter()
            .any(|f| f.relative_path.contains("planning")));
    }

    #[test]
    fn filter_with_empty_include_paths_returns_all() {
        let empty_hash = compute_content_hash_str("");
        let files = vec![DiscoveredFile {
            relative_path: "a.md".to_string(),
            absolute_path: PathBuf::from("a.md"),
            kind: FileKind::Markdown,
            content: String::new(),
            content_hash: empty_hash,
            headings: Vec::new(),
            checkboxes: Vec::new(),
            links: Vec::new(),
            filename_date: None,
            title_date: None,
            is_therapy_path: false,
            potential_therapy_keywords: false,
        }];
        let filtered = filter_by_include_paths(files, &[]).expect("filter");
        assert_eq!(filtered.len(), 1);
    }

    fn write_fixture(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(path, content).expect("write fixture");
    }

    fn fixture_root() -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "joi-import-discovery-test-{}",
            crate::id::new_uuid()
        ));
        dir
    }

    fn discovered_paths(root: &Path) -> Vec<String> {
        discover(root)
            .expect("discover")
            .into_iter()
            .map(|f| f.relative_path)
            .collect()
    }

    #[test]
    fn discover_only_finds_md_txt_and_text_case_insensitive() {
        let root = fixture_root();
        write_fixture(&root, "a.md", "# A\n");
        write_fixture(&root, "b.MD", "# B\n");
        write_fixture(&root, "c.txt", "C\n");
        write_fixture(&root, "d.TXT", "D\n");
        write_fixture(&root, "e.text", "E\n");
        write_fixture(&root, "f.TeXt", "F\n");
        // Unsupported extensions are silently ignored.
        write_fixture(&root, "g.tex", "\\documentclass{article}\n");
        write_fixture(&root, "h.pdf", "%PDF-1.4\n");
        write_fixture(&root, "i.png", "not an image\n");
        write_fixture(&root, "j.json", "{}\n");
        write_fixture(&root, "k.py", "print('x')\n");
        write_fixture(&root, "l.bin", "\u{0}\u{1}\u{2}");
        write_fixture(&root, "m", "no extension\n");

        let paths = discovered_paths(&root);
        assert_eq!(
            paths,
            vec![
                "a.md".to_string(),
                "b.MD".to_string(),
                "c.txt".to_string(),
                "d.TXT".to_string(),
                "e.text".to_string(),
                "f.TeXt".to_string(),
            ]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn discover_prunes_generated_and_dependency_directories_at_any_depth() {
        let root = fixture_root();
        write_fixture(&root, "node_modules/pkg/index.md", "# Dep\n");
        write_fixture(&root, ".git/config.md", "# Git\n");
        write_fixture(&root, ".github/workflows/ci.md", "# CI\n");
        write_fixture(&root, ".gitlab/ci.md", "# CI\n");
        write_fixture(&root, ".hg/state.md", "# Hg\n");
        write_fixture(&root, ".svn/entries.md", "# Svn\n");
        write_fixture(&root, ".agents/skills.md", "# Skills\n");
        write_fixture(&root, ".opencode/agent.md", "# Agent\n");
        write_fixture(&root, "bower_components/x.md", "# Bower\n");
        write_fixture(&root, "target/debug/build/x.md", "# Build\n");
        write_fixture(&root, "scripts/README.md", "# Scripts\n");
        write_fixture(&root, "docs/README.md", "# Docs\n");
        write_fixture(&root, "prompts/README.md", "# Prompts\n");
        write_fixture(&root, "templates/template.md", "# Template\n");
        write_fixture(&root, ".venv/lib/site.md", "# Venv\n");
        write_fixture(&root, "venv/lib/site.md", "# Venv\n");
        write_fixture(&root, "__pycache__/x.md", "# Cache\n");
        write_fixture(&root, ".pytest_cache/x.md", "# Cache\n");
        write_fixture(&root, ".mypy_cache/x.md", "# Cache\n");
        write_fixture(&root, ".idea/workspace.md", "# Idea\n");
        write_fixture(&root, ".vscode/settings.md", "# Vscode\n");
        write_fixture(&root, ".obsidian/plugins/x.md", "# Obsidian\n");
        write_fixture(&root, ".trash/old.md", "# Trash\n");
        // Nested pruned dirs at any depth are also skipped.
        write_fixture(&root, "journal/entries/.git/notes.md", "# Nested git\n");
        write_fixture(
            &root,
            "journal/entries/node_modules/x.md",
            "# Nested node_modules\n",
        );
        // A real importable file survives alongside the pruned trees.
        write_fixture(
            &root,
            "journal/entries/2026-06-25-raw.md",
            "# Diario 2026-06-25\n\nContenido.\n",
        );

        let paths = discovered_paths(&root);
        assert_eq!(
            paths,
            vec![
                "docs/README.md".to_string(),
                "journal/entries/2026-06-25-raw.md".to_string(),
                "prompts/README.md".to_string(),
                "scripts/README.md".to_string(),
                "templates/template.md".to_string()
            ]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn discover_prunes_directories_case_insensitively() {
        let root = fixture_root();
        write_fixture(&root, "Node_Modules/x.md", "# Dep\n");
        write_fixture(&root, ".GIT/config.md", "# Git\n");
        write_fixture(&root, "VENV/lib/site.md", "# Venv\n");
        write_fixture(&root, "__PYCACHE__/x.md", "# Cache\n");
        write_fixture(&root, "records/keep.md", "# Keep\n");

        let paths = discovered_paths(&root);
        assert_eq!(paths, vec!["records/keep.md".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn discover_excludes_unsupported_files_inside_include_paths() {
        let root = fixture_root();
        write_fixture(&root, "salud-mental/Notas/terapia.md", "# Terapia\n");
        write_fixture(&root, "salud-mental/Notas/terapia.tex", "\\documentclass\n");
        write_fixture(&root, "salud-mental/Notas/terapia.pdf", "%PDF-1.4\n");
        write_fixture(&root, "salud-mental/Notas/terapia.png", "image\n");
        write_fixture(&root, "salud-mental/Notas/terapia.json", "{}\n");
        write_fixture(&root, "salud-mental/.git/notes.md", "# Git\n");
        write_fixture(&root, "salud-mental/node_modules/x.md", "# Dep\n");

        // Exclusion happens during discovery, before include_paths filtering.
        let files = discover(&root).expect("discover");
        let filtered =
            filter_by_include_paths(files, &["salud-mental".to_string()]).expect("filter");

        let paths: Vec<String> = filtered.iter().map(|f| f.relative_path.clone()).collect();
        assert_eq!(paths, vec!["salud-mental/Notas/terapia.md".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }
}
