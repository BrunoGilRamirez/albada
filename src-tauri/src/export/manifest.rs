//! Declarative journal-template manifest contract.
//!
//! Manifests are data only. This validator deliberately accepts no scripts,
//! imports, URLs, or arbitrary package files. Network installation is outside
//! this module; callers must validate a package before it can be considered an
//! approved template.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const REQUIRED_BASE_FIELDS: [&str; 3] = ["entry.emotions", "entry.mood", "entry.energy"];

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TemplateManifest {
    pub id: String,
    pub version: String,
    pub display_name: String,
    pub locale: String,
    pub description: String,
    pub min_app_version: String,
    pub fields: Vec<TemplateField>,
    pub sections: Vec<TemplateSection>,
    pub cleanup: CleanupRules,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TemplateField {
    pub id: String,
    pub field_type: String,
    pub required: bool,
    #[serde(default)]
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TemplateSection {
    pub id: String,
    pub heading: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CleanupRules {
    pub remove_comments: bool,
    pub remove_guide_blockquotes: bool,
    pub remove_response_markers: bool,
    pub remove_placeholders: bool,
    pub remove_images: bool,
    pub remove_empty_day_markers: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    InvalidJson,
    EmptyField(&'static str),
    InvalidId,
    InvalidVersion,
    MissingBaseField(&'static str),
    DuplicateId(String),
    UnsupportedFieldType(String),
    UnknownFieldReference(String),
    MalformedMarker,
    UnsafePackageFile(String),
    MissingPackageFile(&'static str),
}

const SUPPORTED_FIELD_TYPES: [&str; 7] = [
    "short_text",
    "markdown",
    "textarea",
    "single_select",
    "multi_select",
    "custom_option",
    "bounded_scale",
];

pub fn foundation_manifest() -> TemplateManifest {
    serde_json::from_str(include_str!("templates/foundation-journal/manifest.json"))
        .expect("bundled foundation manifest must be valid")
}

pub fn validate_manifest(manifest: &TemplateManifest) -> Result<(), ManifestError> {
    if manifest.id.trim().is_empty() {
        return Err(ManifestError::EmptyField("id"));
    }
    if !is_safe_id(&manifest.id) {
        return Err(ManifestError::InvalidId);
    }
    if !is_semver(&manifest.version) {
        return Err(ManifestError::InvalidVersion);
    }
    for (name, value) in [
        ("display_name", manifest.display_name.as_str()),
        ("locale", manifest.locale.as_str()),
        ("description", manifest.description.as_str()),
        ("min_app_version", manifest.min_app_version.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ManifestError::EmptyField(name));
        }
    }
    if !is_semver(&manifest.min_app_version) {
        return Err(ManifestError::InvalidVersion);
    }

    let mut field_ids = HashSet::new();
    for field in &manifest.fields {
        if !field_ids.insert(field.id.clone()) {
            return Err(ManifestError::DuplicateId(field.id.clone()));
        }
        if !is_safe_id(&field.id) {
            return Err(ManifestError::InvalidId);
        }
        if !SUPPORTED_FIELD_TYPES.contains(&field.field_type.as_str()) {
            return Err(ManifestError::UnsupportedFieldType(
                field.field_type.clone(),
            ));
        }
    }
    for required in REQUIRED_BASE_FIELDS {
        if !manifest
            .fields
            .iter()
            .any(|field| field.id == required && field.required)
        {
            return Err(ManifestError::MissingBaseField(required));
        }
    }

    let mut section_ids = HashSet::new();
    for section in &manifest.sections {
        if !section_ids.insert(section.id.clone()) {
            return Err(ManifestError::DuplicateId(section.id.clone()));
        }
        if !is_safe_id(&section.id) || section.heading.trim().is_empty() {
            return Err(ManifestError::InvalidId);
        }
        for field in &section.fields {
            if !field_ids.contains(field) {
                return Err(ManifestError::UnknownFieldReference(field.clone()));
            }
        }
    }

    let required_files = ["manifest.json", "template.md"];
    for required in required_files {
        if !manifest.files.iter().any(|file| file == required) {
            return Err(ManifestError::MissingPackageFile(required));
        }
    }
    for file in &manifest.files {
        if !is_safe_package_file(file) {
            return Err(ManifestError::UnsafePackageFile(file.clone()));
        }
    }
    Ok(())
}

pub fn validate_marker_contract(template_markdown: &str) -> Result<(), ManifestError> {
    let has_start = template_markdown.contains("<!-- joi:section id=")
        || template_markdown.contains("<!-- joi:field id=");
    let has_end = template_markdown.contains("<!-- joi:end-section -->")
        || template_markdown.contains("<!-- joi:end-field -->");
    if has_start != has_end {
        return Err(ManifestError::MalformedMarker);
    }
    Ok(())
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

fn is_semver(value: &str) -> bool {
    let mut parts = value.split('.');
    parts.next().is_some_and(|part| part.parse::<u64>().is_ok())
        && parts.next().is_some_and(|part| part.parse::<u64>().is_ok())
        && parts.next().is_some_and(|part| part.parse::<u64>().is_ok())
        && parts.next().is_none()
}

fn is_safe_package_file(value: &str) -> bool {
    (value == "manifest.json"
        || value == "schema.json"
        || value == "template.md"
        || value.starts_with("assets/"))
        && !value.is_empty()
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        && !value.ends_with(".exe")
        && !value.ends_with(".dll")
        && !value.ends_with(".bat")
        && !value.ends_with(".ps1")
        && !value.ends_with(".rs")
        && !value.ends_with(".js")
        && !value.ends_with(".sh")
        && !value.ends_with(".cmd")
        && !value.ends_with(".com")
        && !value.ends_with(".dll")
        && !value.ends_with(".so")
        && !value.ends_with(".dylib")
        && !value.ends_with(".wasm")
        && !value.ends_with(".py")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_manifest_is_valid() {
        let manifest = foundation_manifest();
        validate_manifest(&manifest).expect("bundled manifest");
        validate_marker_contract(include_str!("templates/foundation-journal/template.md"))
            .expect("bundled markers");
    }

    #[test]
    fn missing_base_field_is_rejected() {
        let mut manifest = foundation_manifest();
        manifest.fields.retain(|field| field.id != "entry.energy");
        assert_eq!(
            validate_manifest(&manifest),
            Err(ManifestError::MissingBaseField("entry.energy"))
        );
    }

    #[test]
    fn duplicate_and_unsafe_files_are_rejected() {
        let mut manifest = foundation_manifest();
        manifest.fields.push(manifest.fields[0].clone());
        assert!(matches!(
            validate_manifest(&manifest),
            Err(ManifestError::DuplicateId(_))
        ));
        manifest = foundation_manifest();
        manifest.files.push("scripts/install.ps1".to_string());
        assert!(matches!(
            validate_manifest(&manifest),
            Err(ManifestError::UnsafePackageFile(_))
        ));
    }

    #[test]
    fn unbalanced_markers_are_rejected() {
        assert_eq!(
            validate_marker_contract("<!-- joi:field id=\"entry.mood\" -->"),
            Err(ManifestError::MalformedMarker)
        );
    }

    #[test]
    fn incompatible_minimum_version_is_rejected() {
        let mut manifest = foundation_manifest();
        manifest.min_app_version = "not-a-version".to_string();
        assert_eq!(
            validate_manifest(&manifest),
            Err(ManifestError::InvalidVersion)
        );
    }
}
