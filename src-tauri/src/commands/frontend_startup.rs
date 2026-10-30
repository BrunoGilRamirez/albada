use crate::commands::dto::{FrontendStartupPhase, FrontendStartupReportDto};
use crate::error::AppError;

const MAX_ELAPSED_MS: u64 = 86_400_000;
const MAX_COMMAND_BYTES: usize = 128;

/// Record a frontend startup milestone in the existing local logger.
#[tauri::command]
pub fn report_frontend_startup(report: FrontendStartupReportDto) -> Result<(), AppError> {
    let command = validate_report(&report)?;

    log::info!(
        "frontend startup: phase={} elapsedMs={} command={}",
        report.phase.label(),
        report.elapsed_ms,
        command
    );
    Ok(())
}

fn validate_report(report: &FrontendStartupReportDto) -> Result<String, AppError> {
    if report.elapsed_ms > MAX_ELAPSED_MS {
        return Err(AppError::validation(
            "The startup elapsed time exceeds the supported range.",
        ));
    }

    match report.command.as_deref() {
        Some(value) if value.len() > MAX_COMMAND_BYTES => Err(AppError::validation(
            "The startup command label is too long.",
        )),
        Some(value) => Ok(format!("\"{}\"", log_safe_command(value))),
        None => Ok("<omitted>".to_string()),
    }
}

impl FrontendStartupPhase {
    fn label(&self) -> &'static str {
        match self {
            Self::ModuleEval => "module-eval",
            Self::FirstCommit => "first-commit",
            Self::FirstInvoke => "first-invoke",
        }
    }
}

fn log_safe_command(value: &str) -> String {
    let mut safe = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\n' => safe.push_str("\\n"),
            '\r' => safe.push_str("\\r"),
            '\\' => safe.push_str("\\\\"),
            '"' => safe.push_str("\\\""),
            '\u{2028}' => safe.push_str("\\u{2028}"),
            '\u{2029}' => safe.push_str("\\u{2029}"),
            character if character.is_control() => {
                safe.extend(character.escape_default());
            }
            character => safe.push(character),
        }
    }
    safe
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_report_accepts_phase_labels_and_camel_case_fields() {
        let report: FrontendStartupReportDto = serde_json::from_str(
            r#"{"phase":"first-invoke","elapsedMs":123,"command":"get_workspace"}"#,
        )
        .expect("deserialize startup report");

        assert!(matches!(report.phase, FrontendStartupPhase::FirstInvoke));
        assert_eq!(report.elapsed_ms, 123);
        assert_eq!(report.command.as_deref(), Some("get_workspace"));
    }

    #[test]
    fn startup_report_rejects_unknown_phase_and_negative_elapsed_time() {
        assert!(serde_json::from_str::<FrontendStartupReportDto>(
            r#"{"phase":"ready","elapsedMs":1}"#
        )
        .is_err());
        assert!(serde_json::from_str::<FrontendStartupReportDto>(
            r#"{"phase":"module-eval","elapsedMs":-1}"#
        )
        .is_err());
    }

    #[test]
    fn startup_command_label_is_bounded_and_log_safe() {
        let safe = log_safe_command("lookup\nlevel=ERROR\"\u{2028}end");
        assert_eq!(safe, "lookup\\nlevel=ERROR\\\"\\u{2028}end");
        assert!(!safe.contains('\n'));
        assert!(!safe.contains('\u{2028}'));

        let oversized_elapsed = FrontendStartupReportDto {
            phase: FrontendStartupPhase::ModuleEval,
            elapsed_ms: MAX_ELAPSED_MS + 1,
            command: None,
        };
        assert!(validate_report(&oversized_elapsed).is_err());

        let oversized_command = FrontendStartupReportDto {
            phase: FrontendStartupPhase::ModuleEval,
            elapsed_ms: 0,
            command: Some("x".repeat(MAX_COMMAND_BYTES + 1)),
        };
        assert!(validate_report(&oversized_command).is_err());
    }
}
