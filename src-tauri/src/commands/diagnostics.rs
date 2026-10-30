//! Local environment diagnostics command (Phase 9.7).
//!
//! Returns a read-only snapshot of runtime versions and bundled-template
//! status for the settings diagnostics section. The command reads no
//! database and exposes no content, filesystem paths, or secrets: version
//! strings and status flags only.

use crate::commands::dto::DiagnosticsDto;
use crate::error::AppError;

/// Local environment diagnostics snapshot.
///
/// Versions come from the linked libraries themselves (`rusqlite`,
/// the `tauri` runtime crate, and `tauri-plugin-os` backed by `os_info`);
/// no shell, registry, or external process is involved. Template status
/// reuses the same bundled-package checks as
/// [`crate::commands::export::export_template_capability`].
#[tauri::command]
pub fn get_environment_diagnostics() -> Result<DiagnosticsDto, AppError> {
    let template_valid = crate::export::template::bundled_package_is_valid()
        && crate::export::template::resolve_template(&Some(
            crate::export::compiler::APPROVED_TEMPLATE_NAME.to_string(),
        ))
        .is_ok();
    Ok(DiagnosticsDto {
        sqlite_version: rusqlite::version().to_string(),
        tauri_version: tauri::VERSION.to_string(),
        os_platform: tauri_plugin_os::platform().to_string(),
        os_version: tauri_plugin_os::version().to_string(),
        typst_compiler_version: crate::export::compiler::EMBEDDED_COMPILER_VERSION.to_string(),
        template_name: crate::export::compiler::APPROVED_TEMPLATE_NAME.to_string(),
        template_version: crate::export::template::DEFAULT_TEMPLATE_VERSION.to_string(),
        template_valid,
        bundled_font_count: crate::export::compiler::bundled_font_count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_diagnostics_reports_versions_and_template_status() {
        let diagnostics = get_environment_diagnostics().expect("diagnostics snapshot should build");
        assert!(
            !diagnostics.sqlite_version.is_empty(),
            "SQLite version must be reported"
        );
        assert!(
            !diagnostics.tauri_version.is_empty(),
            "Tauri version must be reported"
        );
        assert!(
            !diagnostics.os_platform.is_empty(),
            "OS platform must be reported"
        );
        assert!(
            !diagnostics.os_version.is_empty(),
            "OS version must be reported"
        );
        assert_eq!(
            diagnostics.typst_compiler_version,
            crate::export::compiler::EMBEDDED_COMPILER_VERSION
        );
        assert_eq!(
            diagnostics.template_name,
            crate::export::compiler::APPROVED_TEMPLATE_NAME
        );
        assert_eq!(
            diagnostics.template_version,
            crate::export::template::DEFAULT_TEMPLATE_VERSION
        );
        assert!(
            diagnostics.template_valid,
            "bundled template must validate in a normal build"
        );
        assert_eq!(
            diagnostics.bundled_font_count,
            crate::export::compiler::bundled_font_count()
        );
        assert_eq!(diagnostics.bundled_font_count, 6);
    }
}
