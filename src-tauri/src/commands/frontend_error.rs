use crate::commands::dto::FrontendErrorReportDto;

const MAX_SOURCE_BYTES: usize = 128;
const MAX_MESSAGE_BYTES: usize = 2_048;
const MAX_NAME_BYTES: usize = 256;
const MAX_STACK_BYTES: usize = 8_192;
const MAX_COMPONENT_STACK_BYTES: usize = 4_096;
const TRUNCATION_MARKER: &str = "…";

/// Record bounded frontend diagnostics in the existing local logger.
#[tauri::command]
pub fn report_frontend_error(report: FrontendErrorReportDto) {
    log::error!("{}", format_report(&report));
}

fn format_report(report: &FrontendErrorReportDto) -> String {
    let source = log_safe_field(&report.source, MAX_SOURCE_BYTES);
    let message = log_safe_field(&report.message, MAX_MESSAGE_BYTES);
    let name = optional_log_safe_field(report.name.as_deref(), MAX_NAME_BYTES);
    let stack = optional_log_safe_field(report.stack.as_deref(), MAX_STACK_BYTES);
    let component_stack =
        optional_log_safe_field(report.component_stack.as_deref(), MAX_COMPONENT_STACK_BYTES);

    format!(
        "frontend error report: source=\"{source}\" message=\"{message}\" name=\"{name}\" stack=\"{stack}\" componentStack=\"{component_stack}\""
    )
}

fn optional_log_safe_field(value: Option<&str>, max_bytes: usize) -> String {
    value
        .map(|value| log_safe_field(value, max_bytes))
        .unwrap_or_else(|| "<omitted>".to_string())
}

/// Escape record-breaking characters and cap the encoded field size before it
/// reaches the logger. The cap applies after escaping, so control characters
/// cannot expand an unbounded input into an unbounded log entry.
fn log_safe_field(value: &str, max_bytes: usize) -> String {
    let mut result = String::new();
    let mut truncated = false;
    let content_limit = max_bytes.saturating_sub(TRUNCATION_MARKER.len());

    for character in value.chars() {
        let escaped = match character {
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\\' => "\\\\".to_string(),
            '"' => "\\\"".to_string(),
            '\u{2028}' => "\\u{2028}".to_string(),
            '\u{2029}' => "\\u{2029}".to_string(),
            character if character.is_control() => character.escape_default().collect(),
            character => character.to_string(),
        };

        if result.len() + escaped.len() > content_limit {
            truncated = true;
            break;
        }
        result.push_str(&escaped);
    }

    if truncated {
        result.push_str(TRUNCATION_MARKER);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_dto_accepts_frontend_camel_case_payload() {
        let report: FrontendErrorReportDto = serde_json::from_str(
            r#"{
                "source": "react",
                "message": "render failed",
                "componentStack": "at Page"
            }"#,
        )
        .expect("deserialize frontend report");

        assert_eq!(report.source, "react");
        assert!(report.name.is_none());
        assert!(report.stack.is_none());
        assert_eq!(report.component_stack.as_deref(), Some("at Page"));
    }

    #[test]
    fn report_format_escapes_line_breaks_and_bounds_fields() {
        let report = FrontendErrorReportDto {
            source: "react\nlevel=INFO".to_string(),
            message: format!("failure\r\n{}", "x".repeat(MAX_MESSAGE_BYTES * 2)),
            name: Some("Bad\"Name".to_string()),
            stack: Some("line\u{2028}break".to_string()),
            component_stack: None,
        };

        let formatted = format_report(&report);

        assert!(!formatted.contains('\n'));
        assert!(!formatted.contains('\r'));
        assert!(!formatted.contains('\u{2028}'));
        assert!(!formatted.contains('\u{2029}'));
        assert!(formatted.contains("react\\nlevel=INFO"));
        assert!(formatted.contains("Bad\\\"Name"));
        assert!(formatted.contains("line\\u{2028}break"));
        assert!(formatted.contains("<omitted>"));
        assert!(formatted.contains(TRUNCATION_MARKER));
        assert!(formatted.len() <= 16_000, "bounded log entry");
    }
}
