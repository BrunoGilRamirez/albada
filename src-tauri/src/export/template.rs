//! Application-owned reviewed template source (Phase 7.10.7 partial slice).
//!
//! The MVP ships exactly one reviewed template, `default`, embedded at compile
//! time through [`include_str!`]. It is a safe, minimal layout that imports
//! nothing, performs no network or shell access, and relies only on the fonts
//! bundled with the embedded compiler. User-authored template installation is
//! out of scope until the sandbox spike proves it can be constrained.
//!
//! This module ties a stored template name to the compiler's approved
//! [`Template`] value. Only the approved `default` name resolves; an absent
//! template (`None`) means the generated source is compiled directly.

use crate::export::compiler::{self, CompileError, Template};

/// Version of the reviewed default template. Bump when the embedded source
/// changes so historical exports stay reproducible from job metadata.
pub const DEFAULT_TEMPLATE_VERSION: &str = "1.0.1";

/// The reviewed default template source, embedded at compile time.
///
/// It is served to the compiler at the fixed virtual path
/// [`TEMPLATE_PATH`](crate::export::compiler::TEMPLATE_PATH); the generated
/// source imports that path. Because the compiler sandbox serves only sources
/// supplied by the caller, this template can never read the filesystem or the
/// network at runtime.
pub const DEFAULT_TEMPLATE_SOURCE: &str = include_str!("templates/default.typ");

/// Build the approved `default` template value for the compiler.
pub fn default_template() -> Template {
    Template {
        name: compiler::APPROVED_TEMPLATE_NAME.to_string(),
        source: DEFAULT_TEMPLATE_SOURCE.to_string(),
    }
}

/// Resolve a stored template name into the compiler template value.
///
/// Returns `None` for no template (direct compilation) and `Some` only for the
/// approved `default` template. Any other name is rejected here as a last line
/// of defense; the command boundary rejects the same names earlier so a job is
/// never persisted with an unknown template.
pub fn resolve_template(name: &Option<String>) -> Result<Option<Template>, CompileError> {
    match name {
        None => Ok(None),
        Some(name) => {
            compiler::validate_template_name(name)?;
            Ok(Some(default_template()))
        }
    }
}

/// Validate the complete bundled package contract, including its declarative
/// manifest and inert Markdown markers.
pub fn bundled_package_is_valid() -> bool {
    let manifest = crate::export::manifest::foundation_manifest();
    crate::export::manifest::validate_manifest(&manifest).is_ok()
        && crate::export::manifest::validate_marker_contract(include_str!(
            "templates/foundation-journal/template.md"
        ))
        .is_ok()
}

/// Build the generated Typst source, applying the template when one is selected.
///
/// When a template is applied the source imports the fixed
/// [`TEMPLATE_PATH`](crate::export::compiler::TEMPLATE_PATH); otherwise the body
/// is compiled directly. The template name never appears in the import path, so
/// a template cannot influence where its own source resolves.
pub fn apply_template(body: &str, template: &Option<Template>) -> String {
    match template {
        Some(_) => format!("#import \"{}\": *\n{}", compiler::TEMPLATE_PATH, body),
        None => body.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::compiler::APPROVED_TEMPLATE_NAME;

    #[test]
    fn default_template_uses_approved_name() {
        assert_eq!(default_template().name, APPROVED_TEMPLATE_NAME);
    }

    #[test]
    fn default_template_source_is_non_empty_and_safe() {
        let source = DEFAULT_TEMPLATE_SOURCE;
        assert!(!source.trim().is_empty());

        let lowered = source.to_ascii_lowercase();
        // The reviewed template must not import, include, or reach out to the
        // network, shell, or filesystem.
        for forbidden in [
            "#import", "#include", "sys.", "curl", "http://", "https://", "read(",
        ] {
            assert!(
                !lowered.contains(forbidden),
                "default template must not contain '{forbidden}'"
            );
        }
    }

    #[test]
    fn font_rendering_change_has_a_new_template_version() {
        assert_eq!(DEFAULT_TEMPLATE_VERSION, "1.0.1");
        assert!(DEFAULT_TEMPLATE_SOURCE.contains("font: \"Libertinus Serif\""));
    }

    #[test]
    fn resolve_none_is_direct_compilation() {
        assert!(resolve_template(&None).unwrap().is_none());
    }

    #[test]
    fn resolve_default_returns_approved_template() {
        let template = resolve_template(&Some("default".to_string()))
            .unwrap()
            .expect("default resolves");
        assert_eq!(template.name, APPROVED_TEMPLATE_NAME);
        assert_eq!(template.source, DEFAULT_TEMPLATE_SOURCE);
    }

    #[test]
    fn resolve_unknown_name_is_rejected() {
        assert_eq!(
            resolve_template(&Some("diary".to_string())).unwrap_err(),
            CompileError::UnknownTemplate
        );
    }

    #[test]
    fn bundled_package_contract_is_valid() {
        assert!(bundled_package_is_valid());
    }

    #[test]
    fn apply_template_imports_fixed_path() {
        let template = Some(default_template());
        let source = apply_template("= Title\n", &template);
        assert!(source.starts_with("#import \"/templates/foundation-journal.typ\": *\n"));
        assert!(source.contains("= Title\n"));
    }

    #[test]
    fn apply_no_template_is_direct() {
        assert_eq!(apply_template("= Title\n", &None), "= Title\n");
    }
}
