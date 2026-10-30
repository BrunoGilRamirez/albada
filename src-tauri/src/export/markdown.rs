use rusqlite::Connection;

use crate::commands::dto::ExportWarningDto;
use crate::domain::models::{Goal, JournalEntry, Project, Quarter, Task, TherapyRecord};
use crate::domain::progress::{calculate_progress, ProgressResult, ProgressScope};
use crate::error::AppError;
use crate::export::selection::ResolvedSelection;

/// Input describing what to render into Markdown.
pub struct MarkdownInput<'a> {
    pub scope_type: &'a str,
    pub title: &'a str,
}

/// Generate a Markdown document from a resolved selection.
///
/// User-authored Markdown source is preserved unchanged. Execution scopes
/// include a derived progress summary so the export remains explainable.
pub fn generate(
    conn: &Connection,
    input: &MarkdownInput,
    selection: &ResolvedSelection,
) -> Result<String, AppError> {
    let mut md = String::new();
    md.push_str("# ");
    md.push_str(input.title);
    md.push('\n');
    md.push('\n');

    match input.scope_type {
        "journal" => render_journal_entries(&mut md, &selection.journal_entries)?,
        "therapy" => render_therapy_records(&mut md, &selection.therapy_records),
        "task" => render_tasks(conn, &mut md, &selection.tasks)?,
        "project" => render_projects(conn, &mut md, &selection.projects)?,
        "goal" => render_goals(conn, &mut md, &selection.goals)?,
        "quarter" => render_quarters(&mut md, &selection.quarters),
        "custom" => render_custom(conn, &mut md, selection)?,
        other => {
            return Err(AppError::validation(format!(
                "Unsupported export scope '{other}'."
            )))
        }
    }

    Ok(md)
}

/// Safe compatibility warnings for legacy-unversioned journal entries. The
/// warning contains no title, prose, metadata, or other personal content.
pub fn journal_compatibility_warnings(
    scope_type: &str,
    selection: &ResolvedSelection,
) -> Vec<ExportWarningDto> {
    if scope_type == "journal"
        && selection
            .journal_entries
            .iter()
            .any(|entry| entry.template_id.is_none() || entry.template_version.is_none())
    {
        vec![ExportWarningDto {
            code: "journal_template_unverified".to_string(),
            severity: "warning".to_string(),
            message: "Exact journal template compatibility could not be verified; the source-preserving legacy export path was used.".to_string(),
            source: Some("input.md".to_string()),
            phase: Some("compatibility".to_string()),
            category: Some("diary_compatibility".to_string()),
            line: None,
            column: None,
        }]
    } else {
        Vec::new()
    }
}

fn render_journal_entries(md: &mut String, entries: &[JournalEntry]) -> Result<(), AppError> {
    // Days without entries are never emitted — no filler day sections are generated.
    for entry in entries {
        md.push_str("## ");
        md.push_str(&entry.title);
        md.push('\n');
        md.push_str("*Date: ");
        md.push_str(&entry.occurred_on);
        md.push_str("*\n\n");
        md.push_str(&journal_export_body(entry)?);
        md.push_str("\n\n---\n\n");
    }
    Ok(())
}

/// Custom exports retain their original source-only journal rendering. Diary
/// reconstruction and compatibility diagnostics belong only to the journal
/// diary profile.
fn render_custom_journal_entries(md: &mut String, entries: &[JournalEntry]) {
    for entry in entries {
        md.push_str("## ");
        md.push_str(&entry.title);
        md.push('\n');
        md.push_str("*Date: ");
        md.push_str(&entry.occurred_on);
        md.push_str("*\n\n");
        md.push_str(&nest_body_headings(&entry.content_markdown));
        md.push_str("\n\n---\n\n");
    }
}

fn journal_export_body(entry: &JournalEntry) -> Result<String, AppError> {
    match (
        entry.template_id.as_deref(),
        entry.template_version.as_deref(),
    ) {
        (Some(template_id), Some(template_version)) => {
            let manifest = crate::export::manifest::foundation_manifest();
            if template_id != manifest.id || template_version != manifest.version {
                return Err(AppError::validation(
                    "This journal entry uses a template version that is unavailable. Install or restore that exact template version before exporting.",
                ));
            }
            if !crate::export::template::bundled_package_is_valid() {
                return Err(AppError::validation(
                    "The recorded journal template is unavailable or invalid; restore the exact template package before exporting.",
                ));
            }
            let field_values = manifest
                .fields
                .iter()
                .map(|field| {
                    let value = match field.id.as_str() {
                        "entry.emotions" if !entry.emotions.is_empty() => {
                            Some(entry.emotions.join(", "))
                        }
                        "entry.mood" => entry.mood_label.clone(),
                        "entry.energy" => entry.energy_label.clone(),
                        _ => entry
                            .template_values
                            .get(&field.id)
                            .map(|value| match value {
                                serde_json::Value::String(value) => value.clone(),
                                value => value.to_string(),
                            }),
                    };
                    (field.id.as_str(), value)
                })
                .collect::<Vec<_>>();
            Ok(project_template_fields(
                &nest_body_headings(&entry.content_markdown),
                &manifest,
                &field_values,
            ))
        }
        // Incomplete pairs are legacy-unversioned. Preserve the established
        // source projection; never infer a methodology from Markdown.
        _ => Ok(nest_body_headings(&entry.content_markdown)),
    }
}

fn project_template_fields(
    source: &str,
    manifest: &crate::export::manifest::TemplateManifest,
    values: &[(&str, Option<String>)],
) -> String {
    let mut output = source.to_owned();
    let mut replacements = Vec::new();
    let mut found_ids = std::collections::HashSet::new();

    for field in &manifest.fields {
        let open = format!("<!-- joi:field id=\"{}\" -->", field.id);
        let starts = output
            .match_indices(&open)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if starts.len() != 1 {
            if !starts.is_empty() {
                found_ids.insert(field.id.as_str());
            }
            continue;
        }
        let start = starts[0];
        let value_start = start + open.len();
        let Some(end_relative) = output[value_start..].find("<!-- joi:end-field -->") else {
            found_ids.insert(field.id.as_str());
            continue;
        };
        let end = value_start + end_relative;
        let end_marker = "<!-- joi:end-field -->";
        let range_start = output[..start].rfind('\n').map_or(0, |newline| newline + 1);
        let range_end = output[end + end_marker.len()..]
            .find('\n')
            .map_or(output.len(), |newline| end + end_marker.len() + newline + 1);
        let start_line = output[range_start..start].trim();
        let end_line_start = output[..end].rfind('\n').map_or(0, |newline| newline + 1);
        let end_line = output[end_line_start..end + end_marker.len()].trim();
        if !start_line.is_empty()
            || end_line != end_marker
            || output[value_start..end].contains("<!-- joi:field")
        {
            found_ids.insert(field.id.as_str());
            continue;
        }
        let value = values
            .iter()
            .find(|(id, _)| *id == field.id)
            .and_then(|(_, value)| value.as_deref())
            .unwrap_or("");
        let replacement = if value.is_empty() {
            format!("{open}\n{end_marker}\n")
        } else {
            format!("{open}\n{value}\n{end_marker}\n")
        };
        replacements.push((range_start, range_end, replacement));
        found_ids.insert(field.id.as_str());
    }

    for (start, end, replacement) in replacements.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }

    let mut missing_fields = Vec::new();
    for section in &manifest.sections {
        let mut section_fields = Vec::new();
        for field_id in &section.fields {
            if found_ids.contains(field_id.as_str()) {
                continue;
            }
            let Some(value) = values
                .iter()
                .find(|(id, _)| *id == field_id)
                .and_then(|(_, value)| value.as_deref())
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            section_fields.push(format!(
                "<!-- joi:field id=\"{field_id}\" -->\n{value}\n<!-- joi:end-field -->"
            ));
            found_ids.insert(field_id.as_str());
        }
        if !section_fields.is_empty() {
            missing_fields.push(format!(
                "<!-- joi:section id=\"{}\" -->\n{}\n<!-- joi:end-section -->",
                section.id,
                section_fields.join("\n\n")
            ));
        }
    }
    if !missing_fields.is_empty() {
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        output.push_str(&missing_fields.join("\n\n"));
    }

    // Add the manifest's visible section heading to the generated projection
    // when the authored/legacy body does not already carry one. Existing
    // section markers remain the structural boundary, and an existing exact
    // heading is never duplicated.
    for section in &manifest.sections {
        let heading = format!("## {}", section.heading);
        if has_markdown_heading(source, &heading) {
            continue;
        }
        if let Some(first_field) = section
            .fields
            .iter()
            .filter_map(|field_id| {
                let marker = format!("<!-- joi:field id=\"{field_id}\" -->");
                output.find(&marker)
            })
            .min()
        {
            let line_start = output[..first_field]
                .rfind('\n')
                .map_or(0, |newline| newline + 1);
            output.insert_str(line_start, &format!("{heading}\n\n"));
        }
    }
    output
}

fn has_markdown_heading(markdown: &str, heading: &str) -> bool {
    let mut in_fence = false;
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
        } else if !in_fence && trimmed.trim() == heading {
            return true;
        }
    }
    false
}

/// The entry title is rendered as H2. Nest body headings below it without
/// changing the user's stored Markdown; body H1s and H2s both become H3s.
fn nest_body_headings(markdown: &str) -> String {
    let mut output: Vec<String> = Vec::new();
    let mut in_fence = false;
    for line in markdown.lines() {
        let indent = line.bytes().take_while(|byte| *byte == b' ').count();
        let content = &line[indent..];
        if content.starts_with("```") || content.starts_with("~~~") {
            in_fence = !in_fence;
            output.push(line.to_owned());
            continue;
        }
        if !in_fence && indent <= 3 {
            let level = content.bytes().take_while(|byte| *byte == b'#').count();
            if (1..=6).contains(&level)
                && matches!(content.as_bytes().get(level), Some(b' ' | b'\t'))
            {
                let nested_level = (level + 1).clamp(3, 6);
                output.push(format!(
                    "{}{}{}",
                    &line[..indent],
                    "#".repeat(nested_level),
                    &content[level..]
                ));
                continue;
            }
        }
        output.push(line.to_owned());
    }
    output.join("\n")
}

fn render_therapy_records(md: &mut String, records: &[TherapyRecord]) {
    for record in records {
        md.push_str("## ");
        md.push_str(&record.title);
        md.push('\n');
        if let Some(date) = &record.occurred_on {
            md.push_str("*Date: ");
            md.push_str(date);
            md.push_str("*\n\n");
        }
        md.push_str("*Privacy: therapy private*\n\n");
        md.push_str(&record.content_markdown);
        md.push_str("\n\n---\n\n");
    }
}

fn render_tasks(conn: &Connection, md: &mut String, tasks: &[Task]) -> Result<(), AppError> {
    for task in tasks {
        md.push_str("## ");
        md.push_str(&task.title);
        md.push('\n');
        md.push_str("*Status: ");
        md.push_str(&task.status.to_string());
        md.push_str("*\n\n");

        let progress = calculate_progress(conn, &ProgressScope::Task(&task.id))?;
        render_progress_summary(md, &progress);

        if let Some(description) = &task.description_markdown {
            md.push('\n');
            md.push_str(description);
            md.push('\n');
        }
        md.push_str("\n\n---\n\n");
    }
    Ok(())
}

fn render_projects(
    conn: &Connection,
    md: &mut String,
    projects: &[Project],
) -> Result<(), AppError> {
    for project in projects {
        md.push_str("## ");
        md.push_str(&project.title);
        md.push('\n');
        md.push_str("*Status: ");
        md.push_str(&project.status.to_string());
        md.push_str("*\n\n");

        let progress = calculate_progress(conn, &ProgressScope::Project(&project.id))?;
        render_progress_summary(md, &progress);

        if let Some(description) = &project.description_markdown {
            md.push('\n');
            md.push_str(description);
            md.push('\n');
        }
        md.push_str("\n\n---\n\n");
    }
    Ok(())
}

fn render_goals(conn: &Connection, md: &mut String, goals: &[Goal]) -> Result<(), AppError> {
    for goal in goals {
        md.push_str("## ");
        md.push_str(&goal.title);
        md.push('\n');
        md.push_str("*Status: ");
        md.push_str(&goal.status.to_string());
        md.push_str("*\n\n");

        let progress = calculate_progress(conn, &ProgressScope::Goal(&goal.id))?;
        render_progress_summary(md, &progress);

        if let Some(description) = &goal.description_markdown {
            md.push('\n');
            md.push_str(description);
            md.push('\n');
        }
        md.push_str("\n\n---\n\n");
    }
    Ok(())
}

fn render_quarters(md: &mut String, quarters: &[Quarter]) {
    for quarter in quarters {
        md.push_str("## ");
        md.push_str(&quarter.code);
        md.push('\n');
        md.push_str("*Period: ");
        md.push_str(&quarter.start_date);
        md.push_str(" to ");
        md.push_str(&quarter.end_date);
        md.push_str("*\n\n");
        md.push_str("*Status: ");
        md.push_str(&quarter.status);
        md.push_str("*\n\n");
        md.push_str("\n\n---\n\n");
    }
}

fn render_custom(
    conn: &Connection,
    md: &mut String,
    selection: &ResolvedSelection,
) -> Result<(), AppError> {
    if !selection.journal_entries.is_empty() {
        md.push_str("## Journal entries\n\n");
        render_custom_journal_entries(md, &selection.journal_entries);
    }
    if !selection.therapy_records.is_empty() {
        md.push_str("## Therapy records\n\n");
        render_therapy_records(md, &selection.therapy_records);
    }
    if !selection.tasks.is_empty() {
        md.push_str("## Tasks\n\n");
        render_tasks(conn, md, &selection.tasks)?;
    }
    if !selection.projects.is_empty() {
        md.push_str("## Projects\n\n");
        render_projects(conn, md, &selection.projects)?;
    }
    if !selection.goals.is_empty() {
        md.push_str("## Goals\n\n");
        render_goals(conn, md, &selection.goals)?;
    }
    if !selection.quarters.is_empty() {
        md.push_str("## Quarters\n\n");
        render_quarters(md, &selection.quarters);
    }
    Ok(())
}

fn render_progress_summary(md: &mut String, progress: &ProgressResult) {
    md.push_str("**Progress:** ");
    if let Some(pct) = progress.percentage {
        md.push_str(&format!(
            "{:.1}% ({}/{} effort points)",
            pct, progress.completed_points, progress.total_points
        ));
    } else {
        md.push_str(&progress.label);
    }
    md.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::dto::ExportSelectionDto;
    use crate::domain::types::ExportScopeType;
    use crate::domain::types::JournalSource;
    use crate::export::selection::resolve_selection;
    use crate::repos::journal_entry_repo::{JournalEntryRepo, SqliteJournalEntryRepo};
    use crate::test_utils::test_conn;

    #[test]
    fn journal_export_nests_body_primary_heading_below_structured_title() {
        let entry = JournalEntry {
            id: "entry-1".to_string(),
            workspace_id: "workspace-1".to_string(),
            occurred_on: "2026-05-27".to_string(),
            title: "Diario personal — 2026-05-27".to_string(),
            content_markdown:
                "# Diario personal — 2026-05-27\n\n## Entrada Libre\n\nContenido relevante."
                    .to_string(),
            template_id: None,
            template_version: None,
            template_values: Default::default(),
            mood_label: None,
            energy_label: None,
            emotions: Vec::new(),
            export_eligible: false,
            source: JournalSource::Import,
            created_at: "2026-05-27T00:00:00Z".to_string(),
            updated_at: "2026-05-27T00:00:00Z".to_string(),
            archived_at: None,
        };
        let mut markdown = String::new();
        render_journal_entries(&mut markdown, &[entry]).expect("render journal entry");
        assert!(markdown.contains("## Diario personal — 2026-05-27"));
        assert!(markdown.contains("### Diario personal — 2026-05-27"));
        assert!(markdown.contains("Contenido relevante."));
    }

    #[test]
    fn journal_export_nests_body_headings_and_preserves_fenced_code() {
        let body = nest_body_headings(
            "# Main\n## Entry free\n### Custom section\n###### Deep section\n\n```md\n# code heading\n```\n#tag\n",
        );
        assert_eq!(
            body,
            "### Main\n### Entry free\n#### Custom section\n###### Deep section\n\n```md\n# code heading\n```\n#tag"
        );
    }

    #[test]
    fn journal_export_no_filler_days_only_existing_entries_rendered() {
        // Regression test: entries on 2026-01-01, 2026-01-05, 2026-02-10 with a wide
        // date range must resolve to exactly 3 entries, and generated markdown must
        // contain the 3 titles and no filler text for missing days.
        let entries = vec![
            JournalEntry {
                id: "entry-1".to_string(),
                workspace_id: "w1".to_string(),
                occurred_on: "2026-01-01".to_string(),
                title: "First Entry".to_string(),
                content_markdown: "Content for Jan 1".to_string(),
                template_id: None,
                template_version: None,
                template_values: Default::default(),
                mood_label: None,
                energy_label: None,
                emotions: Vec::new(),
                export_eligible: false,
                source: JournalSource::User,
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
                archived_at: None,
            },
            JournalEntry {
                id: "entry-2".to_string(),
                workspace_id: "w1".to_string(),
                occurred_on: "2026-01-05".to_string(),
                title: "Second Entry".to_string(),
                content_markdown: "Content for Jan 5".to_string(),
                template_id: None,
                template_version: None,
                template_values: Default::default(),
                mood_label: None,
                energy_label: None,
                emotions: Vec::new(),
                export_eligible: false,
                source: JournalSource::User,
                created_at: "2026-01-05T00:00:00Z".to_string(),
                updated_at: "2026-01-05T00:00:00Z".to_string(),
                archived_at: None,
            },
            JournalEntry {
                id: "entry-3".to_string(),
                workspace_id: "w1".to_string(),
                occurred_on: "2026-02-10".to_string(),
                title: "Third Entry".to_string(),
                content_markdown: "Content for Feb 10".to_string(),
                template_id: None,
                template_version: None,
                template_values: Default::default(),
                mood_label: None,
                energy_label: None,
                emotions: Vec::new(),
                export_eligible: false,
                source: JournalSource::User,
                created_at: "2026-02-10T00:00:00Z".to_string(),
                updated_at: "2026-02-10T00:00:00Z".to_string(),
                archived_at: None,
            },
        ];

        let mut markdown = String::new();
        render_journal_entries(&mut markdown, &entries).expect("render journal entries");

        // Exactly 3 entry titles should appear.
        assert!(markdown.contains("## First Entry"));
        assert!(markdown.contains("## Second Entry"));
        assert!(markdown.contains("## Third Entry"));

        // No filler day sections for missing dates (e.g., 2026-01-02, 2026-01-03, etc.).
        // The renderer only emits sections for entries that exist.
        assert!(!markdown.contains("2026-01-02"));
        assert!(!markdown.contains("2026-01-03"));
        assert!(!markdown.contains("2026-01-04"));
        assert!(!markdown.contains("2026-02-09"));
        assert!(!markdown.contains("2026-02-11"));

        // Content should be present.
        assert!(markdown.contains("Content for Jan 1"));
        assert!(markdown.contains("Content for Jan 5"));
        assert!(markdown.contains("Content for Feb 10"));

        // Each entry should have its date rendered.
        assert!(markdown.contains("*Date: 2026-01-01*"));
        assert!(markdown.contains("*Date: 2026-01-05*"));
        assert!(markdown.contains("*Date: 2026-02-10*"));
    }

    #[test]
    fn explicit_synthetic_fixture_selection_matches_exact_markdown_export() {
        let conn = test_conn();
        let fixtures = [
            (
                "2026-08-01",
                "Diario sintético — 2026-08-01",
                include_str!("../../fixtures/legacy/journal/entries/export-one.md"),
            ),
            (
                "2026-08-02",
                "English synthetic note — 2026-08-02",
                include_str!("../../fixtures/legacy/journal/entries/export-two.md"),
            ),
            (
                "2026-08-03",
                "Not selected — 2026-08-03",
                include_str!("../../fixtures/legacy/journal/entries/not-selected.md"),
            ),
        ];
        let mut ids = Vec::new();
        for (date, title, markdown) in fixtures {
            let entry = SqliteJournalEntryRepo
                .create(
                    &conn,
                    "w1",
                    date,
                    title,
                    markdown,
                    &None,
                    &None,
                    &std::collections::BTreeMap::new(),
                    &None,
                    &None,
                    &[],
                    true,
                    JournalSource::Import,
                )
                .expect("create synthetic fixture entry");
            ids.push(entry.id);
        }

        // The third fixture is deliberately not selected.
        let selection = ExportSelectionDto {
            journal_entry_ids: Some(vec![ids[1].clone(), ids[0].clone()]),
            therapy_record_ids: None,
            task_ids: None,
            project_ids: None,
            goal_ids: None,
            quarter_ids: None,
            date_range: None,
            export_all: None,
        };
        let resolved = resolve_selection(&conn, "w1", ExportScopeType::Journal, &selection)
            .expect("resolve explicit journal selection");
        assert_eq!(resolved.journal_entries.len(), 2);
        assert!(!resolved
            .journal_entries
            .iter()
            .any(|entry| entry.id == ids[2]));

        let markdown = generate(
            &conn,
            &MarkdownInput {
                scope_type: "journal",
                title: "Synthetic selection",
            },
            &resolved,
        )
        .expect("generate exact Markdown export");
        assert_eq!(
            markdown,
            format!(
                "{}\n\n",
                include_str!("../../fixtures/legacy/expected-journal-export.md").trim_end()
            )
        );
        assert!(!markdown.contains("Not selected"));
        assert!(!markdown.contains("therapy"));
    }

    #[test]
    fn template_metadata_is_projected_without_mutating_authored_markdown() {
        let conn = test_conn();
        let values = std::collections::BTreeMap::new();
        let source = "Authored prose stays byte-exact.\r\n";
        let entry = SqliteJournalEntryRepo
            .create(
                &conn,
                "w1",
                "2026-09-04",
                "Synthetic entry",
                source,
                &Some("foundation-journal".to_owned()),
                &Some("1.0.1".to_owned()),
                &values,
                &Some("good".to_owned()),
                &Some("7".to_owned()),
                &["calm".to_owned(), "invented emotion".to_owned()],
                true,
                JournalSource::User,
            )
            .expect("create synthetic entry");
        let selection = crate::export::selection::ResolvedSelection {
            journal_entries: vec![entry.clone()],
            therapy_records: Vec::new(),
            tasks: Vec::new(),
            projects: Vec::new(),
            goals: Vec::new(),
            quarters: Vec::new(),
        };
        let markdown = generate(
            &conn,
            &MarkdownInput {
                scope_type: "journal",
                title: "Synthetic export",
            },
            &selection,
        )
        .expect("generate synthetic export");

        assert!(markdown.contains("Authored prose stays byte-exact."));
        assert!(markdown.contains("<!-- joi:section id=\"entry\" -->"));
        assert!(!markdown.contains("entry.free"));
        assert!(markdown.contains("calm, invented emotion"));
        assert!(markdown.contains("<!-- joi:field id=\"entry.mood\" -->\ngood\n"));
        assert!(markdown.contains("<!-- joi:field id=\"entry.energy\" -->\n7\n"));
        let normalized = crate::export::diary::normalize_with_rules(
            &markdown,
            &crate::export::manifest::foundation_manifest().cleanup,
        )
        .markdown;
        assert!(normalized.contains("## Entry"));
        assert!(normalized.contains("calm, invented emotion"));
        assert!(normalized.contains("good"));
        assert!(normalized.contains("7"));
        assert!(!normalized.contains("<!-- joi:"));
        assert_eq!(
            SqliteJournalEntryRepo
                .get(&conn, &entry.id)
                .expect("source entry remains stored")
                .content_markdown
                .as_bytes(),
            source.as_bytes()
        );
    }

    #[test]
    fn custom_export_keeps_source_only_journal_projection_and_no_diary_warning() {
        let conn = test_conn();
        let values = std::collections::BTreeMap::new();
        let entry = SqliteJournalEntryRepo
            .create(
                &conn,
                "w1",
                "2026-09-04",
                "Synthetic custom entry",
                "Authored source only.",
                &Some("foundation-journal".to_owned()),
                &Some("1.0.1".to_owned()),
                &values,
                &Some("good".to_owned()),
                &None,
                &[],
                true,
                JournalSource::User,
            )
            .expect("create synthetic entry");
        let selection = crate::export::selection::ResolvedSelection {
            journal_entries: vec![entry],
            therapy_records: Vec::new(),
            tasks: Vec::new(),
            projects: Vec::new(),
            goals: Vec::new(),
            quarters: Vec::new(),
        };
        let markdown = generate(
            &conn,
            &MarkdownInput {
                scope_type: "custom",
                title: "Synthetic custom export",
            },
            &selection,
        )
        .expect("generate custom export");

        assert!(markdown.contains("Authored source only."));
        assert!(!markdown.contains("Structured value"));
        assert!(!markdown.contains("<!-- joi:field"));
        assert!(journal_compatibility_warnings("custom", &selection).is_empty());
    }

    #[test]
    fn versioned_projection_replaces_legacy_marker_values_and_rejects_missing_template() {
        let mut legacy_values = std::collections::BTreeMap::new();
        legacy_values.insert(
            "entry.free".to_string(),
            serde_json::json!("Hidden archived prose"),
        );
        let entry = JournalEntry {
            id: "entry-synthetic".to_string(),
            workspace_id: "w1".to_string(),
            occurred_on: "2026-05-27".to_string(),
            title: "Synthetic".to_string(),
            content_markdown: concat!(
                "Authored prose.\n\n## Entry\n\n",
                "<!-- joi:field id=\"entry.mood\" -->\nold value\n<!-- joi:end-field -->\n"
            )
            .to_string(),
            template_id: Some("foundation-journal".to_string()),
            template_version: Some("1.0.1".to_string()),
            template_values: legacy_values,
            mood_label: Some("good".to_string()),
            energy_label: None,
            emotions: Vec::new(),
            export_eligible: true,
            source: JournalSource::Import,
            created_at: "2026-05-27T00:00:00Z".to_string(),
            updated_at: "2026-05-27T00:00:00Z".to_string(),
            archived_at: None,
        };
        let body = journal_export_body(&entry).expect("exact template match");
        assert!(body.contains("Authored prose."));
        assert!(body.contains("<!-- joi:field id=\"entry.mood\" -->\ngood\n"));
        assert!(!body.contains("old value"));
        assert!(!body.contains("Hidden archived prose"));
        assert!(!body.contains("entry.free"));
        assert_eq!(body.matches("### Entry").count(), 1);

        let mut unavailable = entry;
        unavailable.template_version = Some("9.9.9".to_string());
        let error = journal_export_body(&unavailable).expect_err("do not substitute template");
        assert!(error.message.contains("exact template version"));
    }
}
