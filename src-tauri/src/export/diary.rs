//! Export-only compatibility normalization for legacy diary Markdown.
//!
//! The source snapshot is never changed.  This deliberately conservative
//! line-oriented pass removes known template scaffolding while leaving fenced
//! code, tables, prose, and real blockquotes untouched.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiaryNormalizationStats {
    pub removed_comments: usize,
    pub removed_prompts: usize,
    pub removed_placeholders: usize,
    pub removed_images: usize,
    pub removed_empty_sections: usize,
    pub removed_empty_day_markers: usize,
    pub removed_empty_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiaryNormalization {
    pub markdown: String,
    pub stats: DiaryNormalizationStats,
}

pub fn normalize(source: &str) -> DiaryNormalization {
    normalize_with_rules(source, &default_cleanup_rules())
}

pub fn normalize_with_rules(
    source: &str,
    rules: &crate::export::manifest::CleanupRules,
) -> DiaryNormalization {
    let mut stats = DiaryNormalizationStats::default();
    let mut output = Vec::new();
    let mut in_fence = false;
    let mut comment = false;
    let mut primary_heading_seen = false;
    let mut active_template_section: Option<usize> = None;
    let mut previous_non_empty_line = None::<(String, usize)>;

    for line in source.lines() {
        let normalized_line =
            normalize_accidental_list_indentation(line, previous_non_empty_line.as_ref());
        let trimmed = normalized_line.trim().to_string();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            output.push(normalized_line);
            continue;
        }
        if in_fence {
            output.push(line.to_string());
            continue;
        }

        // Remove legacy empty-day marker lines when the cleanup rule is enabled.
        // These markers appear as standalone lines under arbitrary headings
        // (not only in known template sections). We match both with and without
        // the trailing period, case-insensitive, and never strip inside fences.
        if rules.remove_empty_day_markers {
            let lower = trimmed.to_ascii_lowercase();
            if lower == "sin novedades registradas este día."
                || lower == "sin novedades registradas este día"
            {
                stats.removed_empty_day_markers += 1;
                continue;
            }
        }

        if let Some((level, heading)) = parse_heading(&trimmed) {
            if active_template_section.is_some_and(|section_level| level <= section_level) {
                active_template_section = None;
            }
            if is_template_section_heading(&heading) {
                active_template_section = Some(level);
            }
        }

        // The export document already owns its single H1. Imported entries
        // can contain another legacy H1 copied from the source template;
        // suppress subsequent primary headings while preserving fenced code.
        if trimmed.starts_with("# ") {
            if primary_heading_seen {
                continue;
            }
            primary_heading_seen = true;
        }

        if comment && rules.remove_comments {
            stats.removed_comments += 1;
            if trimmed.contains("-->") {
                comment = false;
            }
            continue;
        }
        if rules.remove_comments && trimmed.starts_with("<!--") {
            stats.removed_comments += 1;
            if !trimmed.contains("-->") {
                comment = true;
            }
            continue;
        }

        let in_template_section = active_template_section.is_some();
        if in_template_section && rules.remove_guide_blockquotes && is_guide_blockquote(&trimmed) {
            stats.removed_prompts += 1;
            continue;
        }
        if in_template_section && rules.remove_response_markers && is_response_marker(&trimmed) {
            stats.removed_prompts += 1;
            continue;
        }
        if in_template_section && rules.remove_placeholders && is_placeholder_item(&trimmed) {
            stats.removed_placeholders += 1;
            continue;
        }

        let (without_images, removed_images) = if rules.remove_images {
            remove_images(&normalized_line)
        } else {
            (normalized_line, 0)
        };
        stats.removed_images += removed_images;
        output.push(without_images);
        if !trimmed.is_empty() {
            let original_indent = line.len() - line.trim_start_matches([' ', '\t']).len();
            previous_non_empty_line = Some((trimmed, original_indent));
        }
    }

    // Remove whitespace-only gaps and empty H2/H3 sections after structure
    // removal. Headings at other levels are retained as document structure.
    let compacted = remove_empty_sections(&output, &mut stats);

    // Remove journal entry blocks that contain no alphanumeric content after
    // the *Date:* line. An entry block starts at a `## ` heading whose next
    // non-blank line matches `*Date: ...*`. The block runs until the next
    // entry start or EOF; a trailing `---` separator belongs to the block.
    let compacted = remove_empty_entries(&compacted, &mut stats);

    let mut markdown = compacted.join("\n");
    markdown = markdown.trim().to_string();
    if !markdown.is_empty() {
        markdown.push('\n');
    }
    DiaryNormalization { markdown, stats }
}

fn parse_heading(line: &str) -> Option<(usize, String)> {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if !(1..=6).contains(&hashes) || !line[hashes..].starts_with(' ') {
        return None;
    }
    let heading = line[hashes..]
        .trim()
        .trim_end_matches('#')
        .trim()
        .to_ascii_lowercase();
    Some((hashes, heading))
}

fn is_template_section_heading(heading: &str) -> bool {
    matches!(
        heading,
        "emociones"
            | "momentos importantes"
            | "reflexión personal"
            | "reflexion personal"
            | "gratitud"
            | "mirada hacia adelante"
            | "registro de conducta problema"
    )
}

/// Legacy entries sometimes indent a top-level list item by four spaces. That
/// changes its CommonMark meaning to an indented code block, which makes
/// inline Markdown such as `**bold**` survive literally into Typst. Remove
/// only this accidental indentation; keep indentation after another list item
/// because it may represent a deliberate nested list.
fn normalize_accidental_list_indentation(
    line: &str,
    previous_non_empty: Option<&(String, usize)>,
) -> String {
    let leading = line.len() - line.trim_start_matches([' ', '\t']).len();
    if leading < 4 || !is_list_item_start(line.trim_start()) {
        return line.to_string();
    }
    // A run of legacy top-level items may have the same accidental indent on
    // every line. Compare against the original indent, not the already
    // normalized previous line, so equal columns are flattened together while
    // a genuinely deeper child remains nested.
    if previous_non_empty.is_some_and(|(previous, previous_indent)| {
        is_list_item_start(previous) && leading > *previous_indent
    }) {
        return line.to_string();
    }
    line.trim_start().to_string()
}

fn is_list_item_start(line: &str) -> bool {
    let bytes = line.as_bytes();
    matches!(bytes.first(), Some(b'-' | b'*' | b'+'))
        && bytes.get(1).is_some_and(|byte| byte.is_ascii_whitespace())
        || bytes
            .iter()
            .position(|byte| !byte.is_ascii_digit())
            .is_some_and(|position| {
                position > 0
                    && bytes.get(position) == Some(&b'.')
                    && bytes
                        .get(position + 1)
                        .is_some_and(|byte| byte.is_ascii_whitespace())
            })
}

fn default_cleanup_rules() -> crate::export::manifest::CleanupRules {
    crate::export::manifest::CleanupRules {
        remove_comments: true,
        remove_guide_blockquotes: true,
        remove_response_markers: true,
        remove_placeholders: true,
        remove_images: true,
        remove_empty_day_markers: true,
    }
}

fn is_guide_blockquote(line: &str) -> bool {
    let text = line.trim_start_matches('>').trim().to_ascii_lowercase();
    let prompt_text = text.trim_start_matches(['-', '*']).trim();
    line.starts_with('>')
        && (text.ends_with('?')
            || prompt_text.ends_with("...")
            || text.starts_with("pregunta")
            || text.starts_with("guía")
            || text.starts_with("guide")
            || text.starts_with("question"))
}

fn is_response_marker(line: &str) -> bool {
    let normalized = line.trim().trim_matches('*').trim().to_ascii_lowercase();
    normalized == "respuesta:" || normalized == "response:"
}

fn is_placeholder_item(line: &str) -> bool {
    let value = line.trim_start_matches(['-', '*', '+']).trim();
    let value = value
        .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.')
        .trim();
    let lower = value.to_ascii_lowercase();
    (lower.starts_with('[') && lower.ends_with(']'))
        || lower.contains("[evento ")
        || lower.contains("[acción ")
        || lower.contains("[accion ")
        || lower.contains("[action ")
}

fn remove_images(line: &str) -> (String, usize) {
    let mut result = String::with_capacity(line.len());
    let mut rest = line;
    let mut removed = 0;
    while let Some(start) = rest.find("![") {
        let Some(end_rel) = rest[start..].find(')') else {
            break;
        };
        let end = start + end_rel + 1;
        result.push_str(&rest[..start]);
        rest = &rest[end..];
        removed += 1;
    }
    result.push_str(rest);
    (result, removed)
}

fn is_day_heading(heading: &str) -> bool {
    let lower = heading.to_ascii_lowercase();
    lower.starts_with("día:") || lower.starts_with("dia:") || lower.starts_with("day:")
}

fn remove_empty_sections(lines: &[String], stats: &mut DiaryNormalizationStats) -> Vec<String> {
    let mut result = lines.to_vec();
    loop {
        let mut removed = false;
        for index in 0..result.len() {
            let Some((section_level, heading)) = parse_heading(result[index].trim()) else {
                continue;
            };
            if section_level != 2 {
                continue;
            }
            let is_template = is_template_section_heading(&heading);
            let is_day = is_day_heading(&heading);
            if !is_template && !is_day {
                continue;
            }
            let end = result[index + 1..]
                .iter()
                .position(|line| {
                    line.trim_start().starts_with("## ") || line.trim_start().starts_with("# ")
                })
                .map(|offset| index + 1 + offset)
                .unwrap_or(result.len());
            let has_content = result[index + 1..end].iter().any(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty()
                    && !trimmed.starts_with("---")
                    && !trimmed.starts_with("# ")
                    && !trimmed.starts_with("## ")
                    && !trimmed.starts_with("### ")
            });
            if !has_content {
                result.drain(index..end);
                stats.removed_empty_sections += 1;
                removed = true;
                break;
            }
        }
        if !removed {
            break;
        }
    }
    result
}

/// Remove journal entry blocks that contain no alphanumeric content.
///
/// An entry block starts at a `## ` heading line whose next non-blank line
/// matches `*Date: ...*` (this discriminator separates entry titles from
/// in-body `## ` headings). The block runs until the next entry start or EOF;
/// a trailing `---` separator line belongs to the block. If the block body
/// (lines after the *Date:* line, ignoring blank lines, `---` lines, and
/// heading lines `^#{1,6} `) contains no alphanumeric character, the whole
/// block is dropped.
fn remove_empty_entries(lines: &[String], stats: &mut DiaryNormalizationStats) -> Vec<String> {
    let mut result = lines.to_vec();
    let mut i = 0;
    while i < result.len() {
        // Look for an entry start: a `## ` heading followed by a `*Date: ...*` line.
        let is_entry_heading = result[i].trim_start().starts_with("## ");
        if !is_entry_heading {
            i += 1;
            continue;
        }
        // Find the next non-blank line after the heading.
        let date_line_idx = result
            .iter()
            .enumerate()
            .skip(i + 1)
            .find(|(_, line)| !line.trim().is_empty())
            .map(|(idx, _)| idx);
        let Some(date_idx) = date_line_idx else {
            i += 1;
            continue;
        };
        let date_line = result[date_idx].trim();
        // Check if it matches `*Date: ...*`
        if !date_line.starts_with("*Date:") || !date_line.ends_with('*') {
            i += 1;
            continue;
        }
        // Found an entry block. Determine its end (next entry start or EOF).
        let mut block_end = result.len();
        for (j, line) in result.iter().enumerate().skip(date_idx + 1) {
            let trimmed = line.trim_start();
            if trimmed.starts_with("## ") {
                // Check if this is another entry start.
                let next_date_idx = result
                    .iter()
                    .enumerate()
                    .skip(j + 1)
                    .find(|(_, l)| !l.trim().is_empty())
                    .map(|(idx, _)| idx);
                if let Some(next_date) = next_date_idx {
                    let next_date_line = result[next_date].trim();
                    if next_date_line.starts_with("*Date:") && next_date_line.ends_with('*') {
                        block_end = j;
                        break;
                    }
                }
            }
        }
        // Include trailing `---` separator if present at block_end - 1.
        if block_end > date_idx + 1 {
            let prev = result[block_end - 1].trim();
            if prev == "---" {
                // Keep the separator as part of the block (it will be dropped with the block).
            }
        }
        // Check if the block body has any alphanumeric content.
        // Body lines are those after the *Date:* line, up to block_end.
        // Ignore blank lines, `---` lines, and heading lines.
        let has_alphanumeric = result
            .iter()
            .take(block_end)
            .skip(date_idx + 1)
            .any(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed == "---" {
                    return false;
                }
                if trimmed.starts_with('#') {
                    return false;
                }
                trimmed.chars().any(|c| c.is_ascii_alphanumeric())
            });
        if !has_alphanumeric {
            // Drop the entire block from the entry heading to block_end.
            result.drain(i..block_end);
            stats.removed_empty_entries += 1;
            // Restart scanning from the same index since we removed lines.
            continue;
        }
        // Block has content, continue after it.
        i = block_end;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_template_structure_but_preserves_real_content() {
        let result = normalize("# Entry\n\n## Emociones\n\n<!-- guide -->\n> What happened?\n\n**Respuesta:**\n- [evento 1]\n\n## Custom\n\nReal prose.\n\n![image](x.png)\n\n> Real quote.\n");
        assert!(result.markdown.contains("Real prose."));
        assert!(result.markdown.contains("> Real quote."));
        assert!(!result.markdown.contains("evento 1"));
        assert!(!result.markdown.contains("![image]"));
        assert_eq!(result.stats.removed_comments, 1);
    }

    #[test]
    fn does_not_clean_fenced_code() {
        let result = normalize("```md\n<!-- keep -->\n> keep?\n```\n");
        assert!(result.markdown.contains("<!-- keep -->"));
        assert!(result.markdown.contains("> keep?"));
    }

    #[test]
    fn removes_multiline_comments_and_empty_sections() {
        let result = normalize(
            "# Entry\n\n<!-- start\nignored\nend -->\n\n## Emociones\n\n<!-- only structure -->\n\n## Kept\n\nActual text.\n",
        );
        assert!(!result.markdown.contains("ignored"));
        assert!(!result.markdown.contains("## Emociones"));
        assert!(result.markdown.contains("## Kept"));
        assert!(result.markdown.contains("Actual text."));
    }

    #[test]
    fn preserves_indented_prose_links_and_real_structure() {
        let result = normalize(
            "# Entry\n\n    pasted conversation\n\n[reference](https://example.com)\n\n> Real blockquote\n\n- [ ] A real task\n",
        );
        assert!(result.markdown.contains("pasted conversation"));
        assert!(result.markdown.contains("https://example.com"));
        assert!(result.markdown.contains("> Real blockquote"));
        assert!(result.markdown.contains("A real task"));
    }

    #[test]
    fn removes_numbered_and_unordered_placeholders() {
        let result = normalize(
            "# Entry\n\n## Emociones\n\n- [acción 1]\n1. [evento 2]\n\n## Custom\n\nReal prose.\n",
        );
        assert!(!result.markdown.contains("acción 1"));
        assert!(!result.markdown.contains("evento 2"));
        assert!(result.markdown.contains("Real prose."));
    }

    #[test]
    fn removes_prompt_only_sections_including_nested_empty_sections() {
        let result = normalize(
            "## Gratitud\n\n> - Hoy agradezco...\n\n## Mirada hacia adelante\n\n> - Mañana quiero enfocarme en...\n\n## Registro de conducta problema\n\n### 1. Describe la conducta problema\n\n**Respuesta:**\n\n1. [evento 1...]\n",
        );
        assert!(!result.markdown.contains("## Gratitud"));
        assert!(!result.markdown.contains("## Mirada hacia adelante"));
        assert!(!result.markdown.contains("## Registro de conducta problema"));
    }

    #[test]
    fn keeps_export_heading_and_removes_entry_primary_headings() {
        let result = normalize(
            "# Journal export\n\n## Diario personal — 2026-05-27\n\n# Diario personal — 2026-05-27\n\n## Entrada Libre\n\nContenido relevante.\n",
        );
        assert_eq!(result.markdown.matches("# Journal export").count(), 1);
        assert!(!result
            .markdown
            .lines()
            .any(|line| line == "# Diario personal — 2026-05-27"));
        assert!(result.markdown.contains("Contenido relevante."));
    }

    #[test]
    fn cleans_response_markers_only_inside_known_template_sections() {
        let result = normalize(
            "#### Reflexiones personalizadas\n\n- **Respuesta:** Esta respuesta pertenece al usuario.\n\n## Emociones\n\n> - ¿Cómo te sientes?\n\n**Respuesta:**\n\n",
        );
        assert!(result
            .markdown
            .contains("- **Respuesta:** Esta respuesta pertenece al usuario."));
        assert!(!result.markdown.contains("> - ¿Cómo te sientes?"));
        assert!(!result.markdown.contains("## Emociones"));
    }

    #[test]
    fn preserves_empty_custom_sections() {
        let result = normalize("## Custom section\n\n## Emociones\n\n**Respuesta:**\n");
        assert!(result.markdown.contains("## Custom section"));
        assert!(!result.markdown.contains("## Emociones"));
    }

    #[test]
    fn repairs_accidentally_indented_top_level_lists_without_flattening_nested_lists() {
        let result = normalize(
            "Paragraph.\n\n    - **Bold item:** preserved as Markdown.\n    - Second item stays flat.\n    - Third item stays flat.\n\n- Parent\n    - Child\n",
        );
        assert!(result
            .markdown
            .contains("- **Bold item:** preserved as Markdown."));
        assert!(result
            .markdown
            .contains("- **Bold item:** preserved as Markdown.\n- Second item stays flat.\n- Third item stays flat."));
        assert!(result.markdown.contains("- Parent\n    - Child"));
    }

    #[test]
    fn removes_empty_day_markers_both_variants_outside_template_sections() {
        let result = normalize(
            "# Entry\n\n## Custom Section\n\nSin novedades registradas este día.\n\n## Another Section\n\nSin novedades registradas este día\n\nReal content.\n",
        );
        assert!(!result
            .markdown
            .contains("Sin novedades registradas este día."));
        assert!(!result
            .markdown
            .contains("Sin novedades registradas este día"));
        assert!(result.markdown.contains("Real content."));
        assert_eq!(result.stats.removed_empty_day_markers, 2);
    }

    #[test]
    fn preserves_empty_day_marker_inside_fenced_code() {
        let result = normalize(
            "```md\nSin novedades registradas este día.\n```\n\n## Section\n\nSin novedades registradas este día.\n",
        );
        assert!(result
            .markdown
            .contains("Sin novedades registradas este día."));
        assert_eq!(result.stats.removed_empty_day_markers, 1);
    }

    #[test]
    fn drops_entry_containing_only_empty_day_marker() {
        let result = normalize(
            "# Journal\n\n## Día: 2026-01-01\n\n*Date: 2026-01-01*\n\nSin novedades registradas este día.\n\n---\n\n## Día: 2026-01-02\n\n*Date: 2026-01-02*\n\nReal content here.\n",
        );
        assert!(!result.markdown.contains("Día: 2026-01-01"));
        assert!(!result
            .markdown
            .contains("Sin novedades registradas este día"));
        assert!(result.markdown.contains("Día: 2026-01-02"));
        assert!(result.markdown.contains("Real content here."));
        assert_eq!(result.stats.removed_empty_entries, 1);
    }

    #[test]
    fn keeps_entry_with_prose_and_removes_marker() {
        let result = normalize(
            "# Journal\n\n## Día: 2026-01-01\n\n*Date: 2026-01-01*\n\nSome real prose.\n\nSin novedades registradas este día.\n\n---\n",
        );
        assert!(result.markdown.contains("Día: 2026-01-01"));
        assert!(result.markdown.contains("Some real prose."));
        assert!(!result
            .markdown
            .contains("Sin novedades registradas este día"));
        assert_eq!(result.stats.removed_empty_day_markers, 1);
        assert_eq!(result.stats.removed_empty_entries, 0);
    }

    #[test]
    fn stats_count_empty_day_markers_and_empty_entries() {
        let result = normalize(
            "# Journal\n\n## Día: 2026-01-01\n\n*Date: 2026-01-01*\n\nSin novedades registradas este día.\n\n---\n\n## Día: 2026-01-02\n\n*Date: 2026-01-02*\n\nSin novedades registradas este día\n\n---\n\n## Día: 2026-01-03\n\n*Date: 2026-01-03*\n\nReal content.\n\nSin novedades registradas este día.\n\n---\n",
        );
        assert_eq!(result.stats.removed_empty_day_markers, 3);
        assert_eq!(result.stats.removed_empty_entries, 2);
        assert!(result.markdown.contains("Día: 2026-01-03"));
        assert!(result.markdown.contains("Real content."));
        assert!(!result.markdown.contains("Día: 2026-01-01"));
        assert!(!result.markdown.contains("Día: 2026-01-02"));
    }

    #[test]
    fn removes_empty_day_heading_block_with_only_marker() {
        let result = normalize(
            "# Journal\n\n## Día: Sábado 18 de abril\n\nSin novedades registradas este día.\n\n---\n\n## Día: Domingo 19 de abril\n\n*Date: 2026-04-19*\n\nReal event happened.\n\n---\n",
        );
        assert!(!result.markdown.contains("Día: Sábado 18 de abril"));
        assert!(!result
            .markdown
            .contains("Sin novedades registradas este día"));
        assert!(result.markdown.contains("Día: Domingo 19 de abril"));
        assert!(result.markdown.contains("Real event happened."));
        assert_eq!(result.stats.removed_empty_sections, 1);
    }

    #[test]
    fn keeps_day_heading_block_with_real_content() {
        let result = normalize(
            "# Journal\n\n## Día: Lunes 20 de abril\n\n*Date: 2026-04-20*\n\nHad a meeting.\n\n---\n",
        );
        assert!(result.markdown.contains("Día: Lunes 20 de abril"));
        assert!(result.markdown.contains("Had a meeting."));
        assert_eq!(result.stats.removed_empty_sections, 0);
    }

    #[test]
    fn mixed_doc_keeps_only_content_day_blocks() {
        let result = normalize(
            "# Journal\n\n## Día: 2026-01-01\n\nSin novedades registradas este día.\n\n---\n\n## Día: 2026-01-02\n\n*Date: 2026-01-02*\n\nReal content here.\n\n---\n\n## Día: 2026-01-03\n\nSin novedades registradas este día\n\n---\n",
        );
        assert!(!result.markdown.contains("Día: 2026-01-01"));
        assert!(!result.markdown.contains("Día: 2026-01-03"));
        assert!(result.markdown.contains("Día: 2026-01-02"));
        assert!(result.markdown.contains("Real content here."));
        assert_eq!(result.stats.removed_empty_sections, 2);
    }

    #[test]
    fn day_heading_variants_dia_dia_day() {
        let result = normalize(
            "# Journal\n\n## Día: 2026-01-01\n\nSin novedades registradas este día.\n\n---\n\n## Dia: 2026-01-02\n\nSin novedades registradas este día.\n\n---\n\n## Day: 2026-01-03\n\nSin novedades registradas este día.\n\n---\n\n## Day: 2026-01-04\n\n*Date: 2026-01-04*\n\nReal content.\n\n---\n",
        );
        assert!(!result.markdown.contains("Día: 2026-01-01"));
        assert!(!result.markdown.contains("Dia: 2026-01-02"));
        assert!(!result.markdown.contains("Day: 2026-01-03"));
        assert!(result.markdown.contains("Day: 2026-01-04"));
        assert!(result.markdown.contains("Real content."));
        assert_eq!(result.stats.removed_empty_sections, 3);
    }

    #[test]
    fn ignores_blank_lines_and_separators_when_judging_day_block_emptiness() {
        let result = normalize(
            "# Journal\n\n## Día: 2026-01-01\n\n\n---\n\n\n---\n\n## Día: 2026-01-02\n\n*Date: 2026-01-02*\n\n\n---\n\nReal content.\n\n---\n",
        );
        assert!(!result.markdown.contains("Día: 2026-01-01"));
        assert!(result.markdown.contains("Día: 2026-01-02"));
        assert!(result.markdown.contains("Real content."));
        assert_eq!(result.stats.removed_empty_sections, 1);
    }

    #[test]
    fn ignores_nested_headings_when_judging_day_block_emptiness() {
        let result = normalize(
            "# Journal\n\n## Día: 2026-01-01\n\n### Subheading\n\n---\n\n## Día: 2026-01-02\n\n*Date: 2026-01-02*\n\n### Real subheading\n\nContent.\n\n---\n",
        );
        assert!(!result.markdown.contains("Día: 2026-01-01"));
        assert!(result.markdown.contains("Día: 2026-01-02"));
        assert!(result.markdown.contains("Content."));
        assert_eq!(result.stats.removed_empty_sections, 1);
    }

    #[test]
    fn does_not_remove_day_heading_inside_fenced_code() {
        let result = normalize(
            "```md\n## Día: 2026-01-01\n\nSin novedades registradas este día.\n```\n\n## Día: 2026-01-02\n\nSin novedades registradas este día.\n\n---\n",
        );
        assert!(result.markdown.contains("Día: 2026-01-01"));
        assert!(!result.markdown.contains("Día: 2026-01-02"));
        assert_eq!(result.stats.removed_empty_sections, 1);
    }
}
