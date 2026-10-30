use std::collections::BTreeMap;

use serde_json::Value;

/// A safe compatibility diagnostic. `field_id` is either a bundled manifest
/// field ID or one of the fixed tags `unknown` / `malformed`; it never contains
/// source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalMetadataWarning {
    pub field_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyJournalMetadata {
    pub template_values: BTreeMap<String, Value>,
    pub emotions: Vec<String>,
    pub mood_label: Option<String>,
    pub energy_label: Option<String>,
    pub warnings: Vec<JournalMetadataWarning>,
}

const KNOWN_FIELDS: [&str; 4] = ["entry.free", "entry.emotions", "entry.mood", "entry.energy"];
const END_MARKER: &str = "<!-- joi:end-field -->";
const FIELD_PREFIX: &str = "<!-- joi:field";

/// Read the old form's unambiguous, known field blocks without changing the
/// Markdown source. The caller persists returned metadata separately.
pub fn extract_legacy_journal_metadata(
    source: &str,
    existing_values: &BTreeMap<String, Value>,
    existing_emotions: &[String],
    existing_mood: Option<&str>,
    existing_energy: Option<&str>,
) -> LegacyJournalMetadata {
    let mut result = LegacyJournalMetadata {
        template_values: existing_values.clone(),
        emotions: existing_emotions.to_vec(),
        mood_label: existing_mood.map(str::to_owned),
        energy_label: existing_energy.map(str::to_owned),
        warnings: Vec::new(),
    };

    for marker in field_markers(source) {
        if !matches!(marker.field_id, Some(id) if KNOWN_FIELDS.contains(&id)) {
            result.warnings.push(JournalMetadataWarning {
                field_id: if marker.field_id.is_some() {
                    "unknown".to_owned()
                } else {
                    "malformed".to_owned()
                },
            });
        }
    }
    if source.matches(FIELD_PREFIX).count() != source.matches(END_MARKER).count() {
        result.warnings.push(JournalMetadataWarning {
            field_id: "malformed".to_owned(),
        });
    }
    let overlapping_fields = overlapping_field_ids(source);

    for field_id in KNOWN_FIELDS {
        let extracted = match extract_field(source, field_id, &overlapping_fields) {
            Ok(value) => value,
            Err(()) => {
                if contains_field_candidate(source, field_id) {
                    result.warnings.push(JournalMetadataWarning {
                        field_id: field_id.to_owned(),
                    });
                }
                continue;
            }
        };
        let Some(raw_value) = extracted else {
            continue;
        };
        let Some(ParsedFieldValue {
            template_value,
            emotions,
            scalar,
        }) = parse_field_value(field_id, &raw_value)
        else {
            result.warnings.push(JournalMetadataWarning {
                field_id: field_id.to_owned(),
            });
            continue;
        };

        let map_conflict = template_value.as_ref().is_some_and(|value| {
            result
                .template_values
                .get(field_id)
                .is_some_and(|existing| existing != value)
        });
        let emotions_conflict = emotions.as_ref().is_some_and(|new| {
            !result.emotions.is_empty() && result.emotions.as_slice() != new.as_slice()
        });
        let scalar_conflict = match (field_id, scalar.as_deref()) {
            ("entry.mood", Some(new)) => result.mood_label.as_deref().is_some_and(|old| old != new),
            ("entry.energy", Some(new)) => {
                result.energy_label.as_deref().is_some_and(|old| old != new)
            }
            _ => false,
        };
        if map_conflict || emotions_conflict || scalar_conflict {
            result.warnings.push(JournalMetadataWarning {
                field_id: field_id.to_owned(),
            });
            continue;
        }

        if let Some(value) = template_value {
            result
                .template_values
                .entry(field_id.to_owned())
                .or_insert(value);
        }
        if let Some(new_emotions) = emotions {
            if result.emotions.is_empty() {
                result.emotions = new_emotions;
            }
        }
        match (field_id, scalar) {
            ("entry.mood", Some(value)) if result.mood_label.is_none() => {
                result.mood_label = Some(value)
            }
            ("entry.energy", Some(value)) if result.energy_label.is_none() => {
                result.energy_label = Some(value)
            }
            _ => {}
        }
    }

    result
}

#[derive(Debug)]
struct FieldMarker<'a> {
    position: usize,
    field_id: Option<&'a str>,
}

fn field_markers(source: &str) -> Vec<FieldMarker<'_>> {
    let mut markers = Vec::new();
    let mut from = 0;
    while let Some(relative) = source[from..].find(FIELD_PREFIX) {
        let start = from + relative;
        let tail = &source[start + FIELD_PREFIX.len()..];
        let parsed = tail
            .strip_prefix(" id=\"")
            .and_then(|tail| tail.find('"').map(|end| &tail[..end]));
        markers.push(FieldMarker {
            position: start,
            field_id: parsed,
        });
        from = start + FIELD_PREFIX.len();
    }
    markers
}

fn overlapping_field_ids(source: &str) -> std::collections::HashSet<String> {
    let markers = field_markers(source);
    let ends: Vec<_> = source
        .match_indices(END_MARKER)
        .map(|(position, _)| position)
        .collect();
    let mut events: Vec<(usize, bool, Option<&str>)> = markers
        .iter()
        .map(|marker| (marker.position, true, marker.field_id))
        .collect();
    events.extend(ends.iter().map(|position| (*position, false, None)));
    events.sort_by_key(|(position, _, _)| *position);

    let mut active: Vec<Option<&str>> = Vec::new();
    let mut overlaps = std::collections::HashSet::new();
    for (_, is_open, field_id) in events {
        if is_open {
            if !active.is_empty() {
                overlaps.extend(active.iter().flatten().map(|id| (*id).to_owned()));
                if let Some(field_id) = field_id {
                    overlaps.insert(field_id.to_owned());
                }
            }
            active.push(field_id);
        } else if active.pop().is_none() {
            continue;
        }
    }
    if markers.len() != ends.len() {
        overlaps.extend(
            markers
                .iter()
                .filter_map(|marker| marker.field_id)
                .filter(|field_id| KNOWN_FIELDS.contains(field_id))
                .map(str::to_owned),
        );
    }
    overlaps
}

fn extract_field(
    source: &str,
    field_id: &str,
    overlapping_fields: &std::collections::HashSet<String>,
) -> Result<Option<String>, ()> {
    if overlapping_fields.contains(field_id) {
        return Err(());
    }
    let open_marker = format!("{FIELD_PREFIX} id=\"{field_id}\" -->");
    let occurrences: Vec<usize> = source.match_indices(&open_marker).map(|(i, _)| i).collect();
    let has_malformed_candidate =
        contains_field_candidate(source, field_id) && occurrences.is_empty();
    if has_malformed_candidate || occurrences.len() > 1 {
        return Err(());
    }
    let Some(&open_start) = occurrences.first() else {
        return Ok(None);
    };
    if field_markers(source)
        .iter()
        .filter(|m| m.field_id == Some(field_id))
        .count()
        != 1
    {
        return Err(());
    }
    let content_start = open_start + open_marker.len();
    if !standalone_marker_line(source, open_start, content_start) {
        return Err(());
    }
    let Some(end_relative) = source[content_start..].find(END_MARKER) else {
        return Err(());
    };
    let end_start = content_start + end_relative;
    if !standalone_marker_line(source, end_start, end_start + END_MARKER.len())
        || source[content_start..end_start].contains(FIELD_PREFIX)
    {
        return Err(());
    }
    let raw = source[content_start..end_start].trim();
    if raw.is_empty() || raw.contains("<!-- joi:") {
        return Err(());
    }
    Ok(Some(raw.to_owned()))
}

fn standalone_marker_line(source: &str, start: usize, end: usize) -> bool {
    let before = source[..start].rsplit('\n').next().unwrap_or_default();
    let after = source[end..].split('\n').next().unwrap_or_default();
    before.trim().is_empty() && after.trim().is_empty()
}

fn contains_field_candidate(source: &str, field_id: &str) -> bool {
    source.match_indices(FIELD_PREFIX).any(|(start, _)| {
        let tail = &source[start + FIELD_PREFIX.len()..];
        tail.contains(field_id)
    })
}

struct ParsedFieldValue {
    template_value: Option<Value>,
    emotions: Option<Vec<String>>,
    scalar: Option<String>,
}

fn parse_field_value(field_id: &str, raw: &str) -> Option<ParsedFieldValue> {
    match field_id {
        "entry.free" => Some(ParsedFieldValue {
            template_value: Some(Value::String(raw.to_owned())),
            emotions: None,
            scalar: None,
        }),
        "entry.emotions" => {
            const OPTIONS: [&str; 6] = ["calm", "joy", "sadness", "anger", "anxiety", "gratitude"];
            let values: Vec<String> = raw.split(',').map(str::trim).map(str::to_owned).collect();
            if values.is_empty()
                || values
                    .iter()
                    .any(|value| !OPTIONS.contains(&value.as_str()))
                || values
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != values.len()
            {
                return None;
            }
            Some(ParsedFieldValue {
                template_value: None,
                emotions: Some(values),
                scalar: None,
            })
        }
        "entry.mood" => {
            if !["low", "neutral", "good", "high"].contains(&raw) {
                return None;
            }
            Some(ParsedFieldValue {
                template_value: None,
                emotions: None,
                scalar: Some(raw.to_owned()),
            })
        }
        "entry.energy" => {
            let value = raw
                .parse::<u8>()
                .ok()
                .filter(|value| (1..=10).contains(value))?;
            Some(ParsedFieldValue {
                template_value: None,
                emotions: None,
                scalar: Some(value.to_string()),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(id: &str, value: &str) -> String {
        format!("<!-- joi:field id=\"{id}\" -->\n{value}\n<!-- joi:end-field -->")
    }

    #[test]
    fn extracts_known_fields_and_keeps_ordered_emotions() {
        let source = format!(
            "Authored\n{}\n{}\n{}",
            block("entry.emotions", "joy, anxiety"),
            block("entry.mood", "good"),
            block("entry.energy", "7")
        );
        let extracted = extract_legacy_journal_metadata(&source, &BTreeMap::new(), &[], None, None);
        assert_eq!(
            extracted.emotions,
            vec!["joy".to_owned(), "anxiety".to_owned()]
        );
        assert_eq!(extracted.mood_label.as_deref(), Some("good"));
        assert_eq!(extracted.energy_label.as_deref(), Some("7"));
        assert!(extracted.template_values.is_empty());
        assert!(extracted.warnings.is_empty());
        assert!(source.starts_with("Authored\n"));
    }

    #[test]
    fn ambiguity_and_conflicting_structured_values_are_not_guessed() {
        let duplicate = format!(
            "{}\n{}",
            block("entry.mood", "good"),
            block("entry.mood", "low")
        );
        let extracted =
            extract_legacy_journal_metadata(&duplicate, &BTreeMap::new(), &[], None, None);
        assert!(extracted.mood_label.is_none());
        assert!(extracted
            .warnings
            .iter()
            .any(|warning| warning.field_id == "entry.mood"));

        let conflict = block("entry.mood", "good");
        let extracted =
            extract_legacy_journal_metadata(&conflict, &BTreeMap::new(), &[], Some("low"), None);
        assert!(extracted.template_values.is_empty());
        assert_eq!(extracted.mood_label.as_deref(), Some("low"));
        assert!(extracted
            .warnings
            .iter()
            .any(|warning| warning.field_id == "entry.mood"));
        assert_eq!(conflict, block("entry.mood", "good"));

        let mut existing_values = BTreeMap::new();
        existing_values.insert(
            "entry.free".to_owned(),
            Value::String("existing".to_owned()),
        );
        let free_conflict = block("entry.free", "different");
        let extracted =
            extract_legacy_journal_metadata(&free_conflict, &existing_values, &[], None, None);
        assert_eq!(
            extracted.template_values["entry.free"],
            Value::String("existing".to_owned())
        );
        assert!(extracted
            .warnings
            .iter()
            .any(|warning| warning.field_id == "entry.free"));
    }

    #[test]
    fn unknown_and_overlapping_markers_are_reportable_and_preserved() {
        let overlap = format!(
            "{}\n{}\n<!-- joi:end-field -->",
            "<!-- joi:field id=\"entry.mood\" -->", "<!-- joi:field id=\"other.field\" -->"
        );
        let extracted =
            extract_legacy_journal_metadata(&overlap, &BTreeMap::new(), &[], None, None);
        assert!(extracted.mood_label.is_none());
        assert!(extracted
            .warnings
            .iter()
            .any(|warning| warning.field_id == "entry.mood"));
        assert!(extracted
            .warnings
            .iter()
            .any(|warning| warning.field_id == "unknown"));
        assert!(overlap.contains("other.field"));

        let known_overlap = concat!(
            "<!-- joi:field id=\"entry.mood\" -->\n",
            "<!-- joi:field id=\"entry.energy\" -->\n7\n",
            "<!-- joi:end-field -->\ngood\n<!-- joi:end-field -->"
        );
        let extracted =
            extract_legacy_journal_metadata(known_overlap, &BTreeMap::new(), &[], None, None);
        assert!(extracted.mood_label.is_none());
        assert!(extracted.energy_label.is_none());
        assert!(extracted
            .warnings
            .iter()
            .any(|warning| warning.field_id == "entry.mood"));
        assert!(extracted
            .warnings
            .iter()
            .any(|warning| warning.field_id == "entry.energy"));
    }
}
