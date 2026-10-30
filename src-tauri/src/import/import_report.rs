use serde::{Deserialize, Serialize};

use super::warnings::ImportWarning;

/// One aggregated count keyed by entity type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityCount {
    pub entity_type: String,
    pub count: i64,
}

/// One item that requires a human decision before it becomes an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewItem {
    pub id: String,
    pub source_path: String,
    pub proposed_entity_type: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
    pub confidence: String,
    /// SHA-256 over the exact source file content, hex-encoded.
    pub source_hash: String,
}

/// The report produced by a single import run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReport {
    pub batch_id: String,
    pub source_root: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub counts: Vec<EntityCount>,
    pub warnings: Vec<ImportWarning>,
    pub review_items: Vec<ReviewItem>,
    /// Deterministic fingerprint for commit verification.
    pub preview_fingerprint: String,
}

/// The subset of a report persisted into `import_batches.summary_json` so a
/// batch can be reconstructed later by `get_import_batch`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSummary {
    pub counts: Vec<EntityCount>,
    pub warnings: Vec<ImportWarning>,
    pub review_items: Vec<ReviewItem>,
    pub preview_fingerprint: String,
}

impl ImportReport {
    pub fn summary(&self) -> ImportSummary {
        ImportSummary {
            counts: self.counts.clone(),
            warnings: self.warnings.clone(),
            review_items: self.review_items.clone(),
            preview_fingerprint: self.preview_fingerprint.clone(),
        }
    }
}
