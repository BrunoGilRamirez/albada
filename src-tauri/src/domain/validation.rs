use crate::error::AppError;
use serde_json::Value;
use std::collections::BTreeMap;

/// A task must have at most one primary owner context: a project or a goal,
/// never both.
pub fn validate_owner_context(
    project_id: &Option<String>,
    goal_id: &Option<String>,
) -> Result<(), AppError> {
    if project_id.is_some() && goal_id.is_some() {
        return Err(AppError::validation(
            "A task may belong to a project or a goal, not both.",
        ));
    }
    Ok(())
}

/// A parent task and its child must have the exact same owner context,
/// including when both are standalone. `parent_ctx` and `child_ctx` are the
/// `(project_id, goal_id)` pairs.
pub fn validate_context_consistency(
    parent_ctx: (&Option<String>, &Option<String>),
    child_ctx: (&Option<String>, &Option<String>),
) -> Result<(), AppError> {
    let (parent_project, parent_goal) = parent_ctx;
    let (child_project, child_goal) = child_ctx;
    let standalone_parent = parent_project.is_none() && parent_goal.is_none();

    if parent_project != child_project {
        let message = if parent_project.is_some() {
            "A child task must share the same project as its parent."
        } else if standalone_parent {
            "A standalone parent cannot have a project-scoped child."
        } else {
            "A child task must share the same owner context as its parent."
        };
        return Err(AppError::validation(message));
    }

    if parent_goal != child_goal {
        let message = if parent_goal.is_some() {
            "A child task must share the same goal as its parent."
        } else if standalone_parent {
            "A standalone parent cannot have a goal-scoped child."
        } else {
            "A child task must share the same owner context as its parent."
        };
        return Err(AppError::validation(message));
    }

    Ok(())
}

/// A task title must be non-empty and reasonably short for list rendering.
pub fn validate_title(title: &str) -> Result<(), AppError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("A title is required."));
    }
    if trimmed.chars().count() > 500 {
        return Err(AppError::validation(
            "The title is too long (maximum 500 characters).",
        ));
    }
    Ok(())
}

/// Workspace locale preferences are limited to system resolution or one of
/// the four bundled catalogs.
pub fn validate_workspace_locale(locale: &str) -> Result<(), AppError> {
    if matches!(locale, "system" | "en-US" | "es-ES" | "de-DE" | "fr-FR") {
        Ok(())
    } else {
        Err(AppError::validation("The workspace locale is invalid."))
    }
}

/// Template identity and version are meaningful only as a complete pair.
pub fn validate_journal_template_pair(
    template_id: &Option<String>,
    template_version: &Option<String>,
) -> Result<(), AppError> {
    match (template_id, template_version) {
        (None, None) => Ok(()),
        (Some(id), Some(version)) if !id.trim().is_empty() && !version.trim().is_empty() => Ok(()),
        (Some(_), Some(_)) => Err(AppError::validation(
            "Journal template ID and version must not be empty.",
        )),
        _ => Err(AppError::validation(
            "Journal template ID and version must be supplied together.",
        )),
    }
}

/// Emotion labels remain free-form and ordered, but blank values are not useful metadata.
pub fn validate_journal_emotions(emotions: &[String]) -> Result<(), AppError> {
    if emotions.iter().any(|emotion| emotion.trim().is_empty()) {
        return Err(AppError::validation(
            "Journal emotion values must not be empty.",
        ));
    }
    Ok(())
}

/// Template values are reserved for manifest-declared non-body fields.
pub fn validate_journal_template_values(
    values: &BTreeMap<String, Value>,
    template_id: Option<&str>,
    template_version: Option<&str>,
) -> Result<(), AppError> {
    if values.is_empty() {
        return Ok(());
    }
    if values.contains_key("entry.free") {
        return Err(AppError::validation(
            "Journal free writing must be stored in content_markdown.",
        ));
    }
    let manifest = crate::export::manifest::foundation_manifest();
    if template_id != Some(manifest.id.as_str())
        || template_version != Some(manifest.version.as_str())
    {
        return Err(AppError::validation(
            "Journal template values require the approved bundled template version.",
        ));
    }
    let dedicated_fields = ["entry.emotions", "entry.mood", "entry.energy"];
    if values.keys().any(|key| {
        dedicated_fields.contains(&key.as_str())
            || !manifest.fields.iter().any(|field| field.id == *key)
    }) {
        return Err(AppError::validation(
            "Journal template values contain an unapproved field ID.",
        ));
    }
    Ok(())
}

/// `joi:*` comments are reserved for generated compatibility/export structure,
/// not newly authored journal storage.
pub fn validate_journal_content_markdown(
    content: &str,
    allow_legacy_markers: bool,
) -> Result<(), AppError> {
    if !allow_legacy_markers && content.contains("<!-- joi:") {
        return Err(AppError::validation(
            "Journal content cannot contain generated metadata markers.",
        ));
    }
    Ok(())
}

/// A planned time window must have valid `HH:MM` boundaries with `start` before
/// `end`. Exact times are optional for blocks; when both are present they must
/// be coherent.
pub fn validate_time_window(start: &str, end: &str) -> Result<(), AppError> {
    let start_minutes = crate::time::parse_time_of_day(start)?;
    let end_minutes = crate::time::parse_time_of_day(end)?;
    if start_minutes >= end_minutes {
        return Err(AppError::validation(
            "The planned start time must be before the end time.",
        ));
    }
    Ok(())
}

pub fn validate_optional_block_start(start: &Option<String>) -> Result<(), AppError> {
    if let Some(start) = start {
        crate::time::parse_time_of_day(start)?;
    }
    Ok(())
}

pub fn validate_block_duration(duration: Option<i64>) -> Result<(), AppError> {
    if duration.is_some_and(|seconds| seconds <= 0) {
        return Err(AppError::validation(
            "Estimated block duration must be a positive number of seconds.",
        ));
    }
    Ok(())
}

pub fn validate_block_recurrence(
    weekdays: &[i64],
    local_start_time: &str,
    duration_seconds: i64,
    effective_from: &str,
    effective_until: Option<&str>,
) -> Result<(), AppError> {
    if weekdays.is_empty()
        || weekdays.iter().any(|weekday| !(1..=7).contains(weekday))
        || weekdays
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != weekdays.len()
    {
        return Err(AppError::validation(
            "Select one or more unique ISO weekdays from 1 through 7.",
        ));
    }
    crate::time::parse_time_of_day(local_start_time)?;
    if duration_seconds <= 0 {
        return Err(AppError::validation(
            "Recurring block duration must be a positive number of seconds.",
        ));
    }
    let start = crate::time::parse_date(effective_from)?;
    if let Some(end) = effective_until {
        if crate::time::parse_date(end)? < start {
            return Err(AppError::validation(
                "Recurrence end date cannot be before its effective start date.",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_context_rejects_both() {
        assert!(validate_owner_context(&Some("p".into()), &Some("g".into())).is_err());
        assert!(validate_owner_context(&Some("p".into()), &None).is_ok());
        assert!(validate_owner_context(&None, &Some("g".into())).is_ok());
        assert!(validate_owner_context(&None, &None).is_ok());
    }

    #[test]
    fn context_consistency_rejects_mismatch() {
        let parent = (&Some("p1".to_string()), &None);
        let child = (&Some("p2".to_string()), &None);
        assert!(validate_context_consistency(parent, child).is_err());
    }

    #[test]
    fn context_consistency_requires_exact_project_goal_or_standalone_match() {
        let standalone = (None, None);
        let project = (Some("project-a".to_string()), None);
        let goal = (None, Some("goal-a".to_string()));

        let project_child =
            validate_context_consistency((&standalone.0, &standalone.1), (&project.0, &project.1))
                .expect_err("standalone parent must reject project child");
        assert_eq!(
            project_child.message,
            "A standalone parent cannot have a project-scoped child."
        );
        let goal_child =
            validate_context_consistency((&standalone.0, &standalone.1), (&goal.0, &goal.1))
                .expect_err("standalone parent must reject goal child");
        assert_eq!(
            goal_child.message,
            "A standalone parent cannot have a goal-scoped child."
        );
        assert!(validate_context_consistency(
            (&standalone.0, &standalone.1),
            (&standalone.0, &standalone.1),
        )
        .is_ok());
        assert!(
            validate_context_consistency((&project.0, &project.1), (&project.0, &project.1),)
                .is_ok()
        );
        assert!(validate_context_consistency((&goal.0, &goal.1), (&goal.0, &goal.1),).is_ok());
    }

    #[test]
    fn recurrence_validation_requires_unique_iso_weekdays_valid_time_and_positive_duration() {
        assert!(validate_block_recurrence(&[1, 7], "09:05", 60, "2026-10-19", None).is_ok());
        assert!(validate_block_recurrence(&[], "09:05", 60, "2026-10-19", None).is_err());
        assert!(validate_block_recurrence(&[0], "09:05", 60, "2026-10-19", None).is_err());
        assert!(validate_block_recurrence(&[8], "09:05", 60, "2026-10-19", None).is_err());
        assert!(validate_block_recurrence(&[1, 1], "09:05", 60, "2026-10-19", None).is_err());
        assert!(validate_block_recurrence(&[1], "9:05", 60, "2026-10-19", None).is_err());
        assert!(validate_block_recurrence(&[1], "09:05", 0, "2026-10-19", None).is_err());
        assert!(
            validate_block_recurrence(&[1], "09:05", 60, "2026-10-20", Some("2026-10-19")).is_err()
        );
    }

    #[test]
    fn optional_one_off_block_fields_keep_legacy_flexibility_but_validate_values() {
        assert!(validate_optional_block_start(&None).is_ok());
        assert!(validate_optional_block_start(&Some("09:05".into())).is_ok());
        assert!(validate_optional_block_start(&Some("9:05".into())).is_err());
        assert!(validate_block_duration(None).is_ok());
        assert!(validate_block_duration(Some(1)).is_ok());
        assert!(validate_block_duration(Some(0)).is_err());
    }

    #[test]
    fn title_is_required() {
        assert!(validate_title("").is_err());
        assert!(validate_title("   ").is_err());
        assert!(validate_title("Go shopping").is_ok());
    }

    #[test]
    fn workspace_locale_accepts_only_supported_values() {
        for locale in ["system", "en-US", "es-ES", "de-DE", "fr-FR"] {
            assert!(validate_workspace_locale(locale).is_ok(), "{locale}");
        }
        for locale in ["", "en", "es-ES-u-co-trad", "pt-BR", "SYSTEM"] {
            let error = validate_workspace_locale(locale).expect_err(locale);
            assert_eq!(error.code, crate::error::ErrorCode::Validation);
        }
    }

    #[test]
    fn time_window_requires_start_before_end() {
        assert!(validate_time_window("09:00", "10:00").is_ok());
        assert!(validate_time_window("10:00", "09:00").is_err());
        assert!(validate_time_window("09:00", "09:00").is_err());
        assert!(validate_time_window("9:00", "10:00").is_err());
    }
}
