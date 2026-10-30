use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::domain::types::{JournalSource, TherapyPrivacyScope, TherapyRecordType};
use crate::error::AppError;
use crate::id;
use crate::repos::import_batch_repo::{ImportBatchRepo, SqliteImportBatchRepo};
use crate::repos::imported_record_repo::{ImportedRecordRepo, SqliteImportedRecordRepo};
use crate::repos::journal_entry_repo::{JournalEntryRepo, SqliteJournalEntryRepo};
use crate::repos::therapy_collection_repo::{SqliteTherapyCollectionRepo, TherapyCollectionRepo};
use crate::repos::therapy_record_repo::{SqliteTherapyRecordRepo, TherapyRecordRepo};
use crate::time;

use super::classifier::{classify, Classification, EntityKind, CONFIDENCE_REVIEW};
use super::discovery::{discover, filter_by_include_paths, DiscoveredFile};
use super::import_report::{EntityCount, ImportReport, ReviewItem};
use super::warnings::{detect_warnings, ImportWarning, WarningKind};

/// Compute a deterministic source key for import idempotency.
///
/// The source key is composed of:
/// - canonical source_root (normalized to forward slashes, no trailing slash)
/// - normalized relative path (forward slashes)
/// - target entity type
/// - logical discriminator (for multiple entities from one file, e.g., checkbox index)
///
/// This mirrors the fingerprint normalization in `compute_preview_fingerprint`.
fn compute_source_key(
    source_root: &str,
    relative_path: &str,
    entity_type: &str,
    discriminator: Option<&str>,
) -> String {
    let canonical_root = source_root
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string();
    let normalized_path = relative_path.replace('\\', "/");
    let mut key = format!("{}|{}|{}", canonical_root, normalized_path, entity_type);
    if let Some(disc) = discriminator {
        key.push('|');
        key.push_str(disc);
    }
    key
}

/// Generate a deterministic review item ID from source_path and proposed_entity_type.
fn review_item_id(source_path: &str, proposed_entity_type: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"review-item:");
    hasher.update(source_path.as_bytes());
    hasher.update(b":");
    hasher.update(proposed_entity_type.as_bytes());
    let hash = hasher.finalize();
    // Use first 16 bytes as UUID-like string
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]),
        u16::from_be_bytes([hash[4], hash[5]]),
        u16::from_be_bytes([hash[6], hash[7]]),
        u16::from_be_bytes([hash[8], hash[9]]),
        u64::from_be_bytes([hash[10], hash[11], hash[12], hash[13], hash[14], hash[15], 0, 0])
    )
}

/// Compute the deterministic preview fingerprint.
///
/// The fingerprint is a SHA-256 hash over:
/// - workspace_id
/// - canonical source_root (normalized to forward slashes, no trailing slash)
/// - include_paths (sorted, normalized, joined with '|')
/// - ordered list of (relative_path, content_hash, proposed_entity_type, confidence) for all review items
fn compute_preview_fingerprint(
    workspace_id: &str,
    source_root: &str,
    include_paths: Option<&[String]>,
    review_items: &[ReviewItem],
) -> String {
    let mut hasher = Sha256::new();

    // workspace_id
    hasher.update(workspace_id.as_bytes());
    hasher.update(b"\0");

    // canonical source_root
    let canonical_root = source_root
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string();
    hasher.update(canonical_root.as_bytes());
    hasher.update(b"\0");

    // include_paths (sorted, normalized)
    let include_str = match include_paths {
        Some(paths) if !paths.is_empty() => {
            let mut normalized: Vec<String> = paths
                .iter()
                .map(|p| p.replace('\\', "/").trim_end_matches('/').to_string())
                .collect();
            normalized.sort();
            normalized.join("|")
        }
        _ => String::new(),
    };
    hasher.update(include_str.as_bytes());
    hasher.update(b"\0");

    // ordered review items: (relative_path, content_hash, proposed_entity_type, confidence)
    // Review items are already sorted by source_path in discovery, but we sort again for safety
    let mut sorted_items = review_items.to_vec();
    sorted_items.sort_by(|a, b| a.source_path.cmp(&b.source_path));

    for item in sorted_items {
        hasher.update(item.source_path.as_bytes());
        hasher.update(b"\0");
        hasher.update(item.source_hash.as_bytes());
        hasher.update(b"\0");
        hasher.update(item.proposed_entity_type.as_bytes());
        hasher.update(b"\0");
        hasher.update(item.confidence.as_bytes());
        hasher.update(b"\0");
    }

    format!("{:x}", hasher.finalize())
}

/// Run a full read-only legacy import inside a single transaction.
///
/// The source tree is only read; no source file is modified. Records are
/// inserted with provenance and matched idempotently by `(source_path,
/// entity_type)` within the workspace, so re-running an import updates metadata
/// instead of duplicating entities.
///
/// `include_paths`, when provided, restricts the import to files under the
/// given relative prefixes (normalized and validated). When `None` or empty,
/// the whole source root is imported.
pub fn run_import(
    conn: &Connection,
    workspace_id: &str,
    source_root: &str,
    include_paths: Option<&[String]>,
) -> Result<ImportReport, AppError> {
    let batch = SqliteImportBatchRepo.create(conn, workspace_id, source_root)?;
    run_import_with_batch(conn, workspace_id, source_root, include_paths, &batch.id)
}

/// Run the discovery/classification/persistence portion of an import against a
/// batch that was already created by the caller.
///
/// This is the long-running half of the import. The command boundary creates
/// the batch in a short transaction, returns its ID, and then runs this
/// function on a controlled blocking worker so the Tauri command call and the
/// React thread are never held open for the discovery work.
///
/// The source tree is only read; no source file is modified. Records are
/// inserted with provenance and matched idempotently by `(source_path,
/// entity_type)` within the workspace, so re-running an import updates metadata
/// instead of duplicating entities.
pub fn run_import_with_batch(
    conn: &Connection,
    workspace_id: &str,
    source_root: &str,
    include_paths: Option<&[String]>,
    batch_id: &str,
) -> Result<ImportReport, AppError> {
    let files = discover(Path::new(source_root))?;
    let files = match include_paths {
        Some(paths) => filter_by_include_paths(files, paths)?,
        None => files,
    };

    let mut counts: BTreeMap<String, i64> = BTreeMap::new();
    let mut warnings: Vec<ImportWarning> = Vec::new();
    let mut review_items: Vec<ReviewItem> = Vec::new();

    let therapy_collection_id = ensure_therapy_collection(conn, workspace_id)?;

    for file in &files {
        let file_warnings = detect_warnings(file)?;
        warnings.extend(file_warnings);

        let mut checkbox_index: usize = 0;
        for classification in classify(file) {
            if classification.confidence == CONFIDENCE_REVIEW {
                review_items.push(ReviewItem {
                    id: id::new_uuid(),
                    source_path: classification.source_path.clone(),
                    proposed_entity_type: classification.entity_kind.as_str().to_string(),
                    title: classification.title.clone(),
                    snippet: classification.snippet.clone(),
                    confidence: classification.confidence.to_string(),
                    source_hash: classification.source_hash.clone(),
                });
                continue;
            }

            let entity_type = classification.entity_kind.as_str();
            let warnings_json = serde_json::to_string(&detect_warnings(file)?).ok();

            // Compute deterministic source_key for idempotency.
            // For checkbox-derived entities, use a discriminator to distinguish multiple entities from one file.
            let discriminator = if classification.entity_kind == EntityKind::Task {
                checkbox_index += 1;
                Some(format!("checkbox-{}", checkbox_index))
            } else {
                None
            };
            let source_key = compute_source_key(
                source_root,
                &classification.source_path,
                entity_type,
                discriminator.as_deref(),
            );
            let content_hash = &classification.source_hash;

            // Idempotency: match by source_key within the workspace.
            if let Some(existing) =
                SqliteImportedRecordRepo.find_existing(conn, workspace_id, &source_key)?
            {
                // Same source_key found. Check content hash.
                if existing.content_hash.as_deref() == Some(content_hash) {
                    // Same source_key and same content hash: skip insertion, associate with new batch.
                    SqliteImportedRecordRepo.insert(
                        conn,
                        batch_id,
                        entity_type,
                        &existing.entity_id,
                        &classification.source_path,
                        &classification.source_date,
                        classification.confidence,
                        &warnings_json,
                        &source_key,
                        content_hash,
                    )?;
                    continue;
                } else {
                    // Same source_key but changed hash: do not overwrite user-edited content.
                    // Record a warning and continue (the existing entity remains associated with its original batch).
                    warnings.push(ImportWarning::new(
                    WarningKind::AmbiguousCheckbox,
                    "warning",
                    &classification.source_path,
                    None,
                    format!(
                        "Source content changed since last import for '{}' (entity type: {}). Existing entity retained; not overwritten.",
                        classification.source_path, entity_type
                    ),
                ));
                    continue;
                }
            }

            let entity_id = persist_classification(
                conn,
                workspace_id,
                &therapy_collection_id,
                &classification,
                file,
            )?;

            SqliteImportedRecordRepo.insert(
                conn,
                batch_id,
                entity_type,
                &entity_id,
                &classification.source_path,
                &classification.source_date,
                classification.confidence,
                &warnings_json,
                &source_key,
                content_hash,
            )?;

            *counts.entry(entity_type.to_string()).or_insert(0) += 1;
        }
    }

    let counts: Vec<EntityCount> = counts
        .into_iter()
        .map(|(entity_type, count)| EntityCount { entity_type, count })
        .collect();

    // Compute fingerprint for the completed import (same as preview but with workspace_id)
    let preview_fingerprint =
        compute_preview_fingerprint(workspace_id, source_root, include_paths, &review_items);

    let report = ImportReport {
        batch_id: batch_id.to_string(),
        source_root: source_root.to_string(),
        status: "completed".to_string(),
        started_at: time::now_rfc3339(),
        completed_at: Some(time::now_rfc3339()),
        counts,
        warnings,
        review_items,
        preview_fingerprint,
    };

    let summary_json = serde_json::to_string(&report.summary())
        .map_err(|_| AppError::database("Could not serialize the import summary."))?;
    SqliteImportBatchRepo.complete(conn, batch_id, &summary_json)?;

    Ok(report)
}

/// Discover and classify a source tree without creating a database batch or
/// inserting any records. Path-derived classifications are returned as review
/// items so the caller can show a preview before a future commit step.
pub fn preview_import(
    workspace_id: &str,
    source_root: &str,
    include_paths: Option<&[String]>,
) -> Result<ImportReport, AppError> {
    let started_at = time::now_rfc3339();
    let files = discover(Path::new(source_root))?;
    let files = match include_paths {
        Some(paths) => filter_by_include_paths(files, paths)?,
        None => files,
    };

    let mut counts: BTreeMap<String, i64> = BTreeMap::new();
    let mut warnings = Vec::new();
    let mut review_items = Vec::new();

    for file in &files {
        warnings.extend(detect_warnings(file)?);
        for classification in classify(file) {
            *counts
                .entry(classification.entity_kind.as_str().to_string())
                .or_insert(0) += 1;

            if classification.confidence == CONFIDENCE_REVIEW {
                review_items.push(ReviewItem {
                    id: review_item_id(
                        &classification.source_path,
                        classification.entity_kind.as_str(),
                    ),
                    source_path: classification.source_path,
                    proposed_entity_type: classification.entity_kind.as_str().to_string(),
                    title: classification.title,
                    snippet: classification.snippet,
                    confidence: classification.confidence.to_string(),
                    source_hash: classification.source_hash,
                });
            }
        }
    }

    // Compute fingerprint for commit verification using the actual workspace_id
    let preview_fingerprint =
        compute_preview_fingerprint(workspace_id, source_root, include_paths, &review_items);

    Ok(ImportReport {
        batch_id: format!("preview-{}", id::new_uuid()),
        source_root: source_root.to_string(),
        status: "preview".to_string(),
        started_at,
        completed_at: Some(time::now_rfc3339()),
        counts: counts
            .into_iter()
            .map(|(entity_type, count)| EntityCount { entity_type, count })
            .collect(),
        warnings,
        review_items,
        preview_fingerprint,
    })
}

/// Insert the classified entity into its target table and return its ID.
fn persist_classification(
    conn: &Connection,
    workspace_id: &str,
    therapy_collection_id: &str,
    classification: &Classification,
    file: &DiscoveredFile,
) -> Result<String, AppError> {
    match classification.entity_kind {
        EntityKind::JournalEntry => {
            let occurred_on = classification
                .source_date
                .clone()
                .unwrap_or_else(time::today_utc);
            let title = classification
                .title
                .clone()
                .unwrap_or_else(|| file.relative_path.clone());
            let entry = SqliteJournalEntryRepo.create(
                conn,
                workspace_id,
                &occurred_on,
                &title,
                &file.content,
                &None,
                &None,
                &BTreeMap::new(),
                &None,
                &None,
                &[],
                false,
                JournalSource::Import,
            )?;
            Ok(entry.id)
        }
        EntityKind::TherapyRecord => {
            let record_type = classification
                .therapy_record_type
                .as_deref()
                .and_then(|t| t.parse().ok())
                .unwrap_or(TherapyRecordType::Note);
            let title = classification
                .title
                .clone()
                .unwrap_or_else(|| file.relative_path.clone());
            let record = SqliteTherapyRecordRepo.create(
                conn,
                therapy_collection_id,
                record_type,
                &title,
                &file.content,
                &classification.source_date,
            )?;
            Ok(record.id)
        }
        // Task, goal, project, year theme, quarter, and note are never
        // auto-created from legacy content; they are surfaced as review
        // items for a human decision.
        _ => Err(AppError::validation(
            "This entity type requires human review and cannot be imported automatically.",
        )),
    }
}

/// Return (or create) the single private therapy collection for a workspace.
fn ensure_therapy_collection(conn: &Connection, workspace_id: &str) -> Result<String, AppError> {
    let existing = SqliteTherapyCollectionRepo.list_including_archived(conn, workspace_id)?;
    if let Some(collection) = existing.first() {
        return Ok(collection.id.clone());
    }
    let collection = SqliteTherapyCollectionRepo.create(
        conn,
        workspace_id,
        "Therapy",
        TherapyPrivacyScope::TherapyPrivate,
        &None,
    )?;
    Ok(collection.id)
}

/// A fixed, content-safe summary persisted when an import batch fails. It never
/// contains source paths, snippets, or any imported content, so a failed batch
/// can be reconstructed by `get_import_batch` without exposing source data.
pub fn fixed_failed_summary_json() -> String {
    let summary = crate::import::import_report::ImportSummary {
        counts: Vec::new(),
        warnings: Vec::new(),
        review_items: Vec::new(),
        preview_fingerprint: String::new(),
    };
    serde_json::to_string(&summary).unwrap_or_else(|_| "{}".to_string())
}

/// Run the long discovery/import transaction on a controlled blocking worker.
///
/// This function owns a clone of the shared database handle (never Tauri
/// `State`) and is intended to run inside `tauri::async_runtime::spawn_blocking`.
/// It is infallible at the top level: on any failure it marks the pre-created
/// batch `failed` with a fixed, content-safe summary so the batch is never left
/// permanently `running`.
pub fn run_import_worker(
    db: crate::state::Db,
    workspace_id: String,
    source_root: String,
    include_paths: Option<Vec<String>>,
    batch_id: String,
) {
    let result = db.with_tx(|conn| {
        run_import_with_batch(
            conn,
            &workspace_id,
            &source_root,
            include_paths.as_deref(),
            &batch_id,
        )
    });

    if let Err(err) = result {
        log::error!("import worker failed for batch {batch_id}: {err:?}");
        // Best-effort terminal state. The transaction already rolled back, so
        // no partial records remain; only the batch row is marked failed.
        let _ = db.with_conn(|conn| {
            SqliteImportBatchRepo.fail(conn, &batch_id, &fixed_failed_summary_json())
        });
    }
}

/// Reviewed-decision commit boundary for legacy import.
///
/// This function:
/// 1. Re-discovers and re-classifies the source tree (same as preview).
/// 2. Recomputes the preview fingerprint with the actual workspace_id and verifies
///    it matches the provided `preview_fingerprint`.
/// 3. Verifies that every discovered review item has an explicit decision.
/// 4. Verifies that source files still exist and their content hashes match.
/// 5. Applies decisions atomically in a single transaction:
///    - "ignore" -> skip
///    - "journal_entry" -> create JournalEntry
///    - "therapy_record" -> create TherapyRecord (only via explicit decision)
///    - other entity types -> rejected (require separate workflows)
/// 6. Creates an import batch with provenance and marks it completed.
///
/// Therapy content is ONLY imported when the decision explicitly chooses
/// `therapy_record`; path-derived therapy classification is never auto-applied.
pub fn commit_legacy_import(
    conn: &Connection,
    workspace_id: &str,
    source_root: &str,
    include_paths: Option<&[String]>,
    preview_fingerprint: &str,
    decisions: &[crate::commands::dto::ReviewDecisionDto],
) -> Result<ImportReport, AppError> {
    // Step 1: Re-discover and re-classify (same as preview)
    let files = discover(Path::new(source_root))?;
    let files = match include_paths {
        Some(paths) => filter_by_include_paths(files, paths)?,
        None => files,
    };

    let mut all_review_items: Vec<ReviewItem> = Vec::new();
    let mut warnings = Vec::new();

    for file in &files {
        warnings.extend(detect_warnings(file)?);
        for classification in classify(file) {
            if classification.confidence == CONFIDENCE_REVIEW {
                all_review_items.push(ReviewItem {
                    id: review_item_id(
                        &classification.source_path,
                        classification.entity_kind.as_str(),
                    ),
                    source_path: classification.source_path.clone(),
                    proposed_entity_type: classification.entity_kind.as_str().to_string(),
                    title: classification.title.clone(),
                    snippet: classification.snippet.clone(),
                    confidence: classification.confidence.to_string(),
                    source_hash: classification.source_hash.clone(),
                });
            }
        }
    }

    // Step 2: Recompute fingerprint and verify it matches
    let computed_fingerprint =
        compute_preview_fingerprint(workspace_id, source_root, include_paths, &all_review_items);

    if computed_fingerprint != preview_fingerprint {
        return Err(AppError::validation(
            "Preview fingerprint mismatch. The source tree or preview parameters have changed since the preview was generated. Please re-run preview.",
        ));
    }

    // Step 3: Verify all review items have decisions
    let decision_map: std::collections::HashMap<_, _> = decisions
        .iter()
        .map(|d| (d.review_item_id.clone(), d.entity_type.clone()))
        .collect();

    if decision_map.len() != decisions.len() {
        return Err(AppError::validation(
            "Duplicate review_item_id in decisions.",
        ));
    }

    if decision_map.len() != all_review_items.len() {
        return Err(AppError::validation(
            "Incomplete decisions: every review item from the preview must have an explicit decision (entity_type or 'ignore').",
        ));
    }

    // Step 4: Verify source files still exist and content hashes match
    for item in &all_review_items {
        let decision = decision_map.get(&item.id).ok_or_else(|| {
            AppError::validation(format!("Missing decision for review item {}", item.id))
        })?;

        // Verify file exists and hash matches
        let absolute_path = Path::new(source_root).join(&item.source_path);
        if !absolute_path.exists() {
            return Err(AppError::validation(format!(
                "Source file no longer exists: {}",
                item.source_path
            )));
        }

        let current_hash = super::discovery::compute_content_hash(&absolute_path)?;
        if current_hash != item.source_hash {
            return Err(AppError::validation(format!(
                "Source file content has changed since preview: {}",
                item.source_path
            )));
        }

        // Verify decision is valid
        match decision.as_str() {
            "ignore" | "journal_entry" | "therapy_record" => {}
            _ => {
                return Err(AppError::validation(format!(
                    "Invalid entity_type '{}' for review item {}. Valid types: ignore, journal_entry, therapy_record.",
                    decision, item.id
                )));
            }
        }
    }

    // Step 5: Apply decisions atomically
    let batch = SqliteImportBatchRepo.create(conn, workspace_id, source_root)?;
    let therapy_collection_id = ensure_therapy_collection(conn, workspace_id)?;

    let mut counts: BTreeMap<String, i64> = BTreeMap::new();

    for item in &all_review_items {
        let decision = decision_map.get(&item.id).unwrap();

        if decision == "ignore" {
            continue;
        }

        // Find the corresponding file for content
        let file = files
            .iter()
            .find(|f| f.relative_path == item.source_path)
            .ok_or_else(|| {
                AppError::validation(format!(
                    "Source file not found in discovery: {}",
                    item.source_path
                ))
            })?;

        // Compute deterministic source_key for idempotency (same logic as run_import_with_batch)
        let source_key = compute_source_key(source_root, &item.source_path, decision, None);
        let content_hash = &item.source_hash;

        // Check for existing record with same source_key in this workspace
        let existing_in_workspace =
            SqliteImportedRecordRepo.find_existing(conn, workspace_id, &source_key)?;

        // If found in workspace, check if the entity is archived first (per ADR: archived entities
        // should warn and not create duplicates, regardless of content hash)
        let mut should_skip_due_to_archived = false;
        if let Some(ref existing) = existing_in_workspace {
            let is_archived = match decision.as_str() {
                "journal_entry" => match SqliteJournalEntryRepo.get(conn, &existing.entity_id) {
                    Ok(entry) => entry.archived_at.is_some(),
                    Err(e) if e.code == crate::error::ErrorCode::NotFound => false,
                    Err(e) => return Err(e),
                },
                "therapy_record" => match SqliteTherapyRecordRepo.get(conn, &existing.entity_id) {
                    Ok(record) => record.archived_at.is_some(),
                    Err(e) if e.code == crate::error::ErrorCode::NotFound => false,
                    Err(e) => return Err(e),
                },
                _ => false,
            };
            if is_archived {
                should_skip_due_to_archived = true;
            }
        }

        if should_skip_due_to_archived {
            // Prior entity was archived: retain that entity and show the changed/archived condition for review.
            // Do not create a duplicate by default. Record a warning.
            warnings.push(ImportWarning::new(
                WarningKind::AmbiguousCheckbox,
                "warning",
                &item.source_path,
                None,
                format!(
                    "Prior entity for '{}' (entity type: {}) was archived. Not creating a duplicate; review required.",
                    item.source_path, decision
                ),
            ));
            continue;
        }

        // Check for existing record with same source_key (in workspace)
        if let Some(existing) = existing_in_workspace {
            // Same source_key found. Check content hash.
            if existing.content_hash.as_deref() == Some(content_hash) {
                // Same source_key and same content hash: skip insertion, associate with new batch.
                SqliteImportedRecordRepo.insert(
                    conn,
                    &batch.id,
                    decision,
                    &existing.entity_id,
                    &item.source_path,
                    &None,
                    "review",
                    &None,
                    &source_key,
                    content_hash,
                )?;
                *counts.entry(decision.clone()).or_insert(0) += 1;
                continue;
            } else {
                // Same source_key but changed hash: do not overwrite user-edited content.
                // Record a warning and skip this item (the existing entity remains associated with its original batch).
                warnings.push(ImportWarning::new(
                    WarningKind::AmbiguousCheckbox,
                    "warning",
                    &item.source_path,
                    None,
                    format!(
                        "Source content changed since last import for '{}' (entity type: {}). Existing entity retained; not overwritten.",
                        item.source_path, decision
                    ),
                ));
                continue;
            }
        }

        // Check if the prior entity (by source_key) was archived - look up across all batches
        // (for cases where the prior import was in a different workspace or the batch was deleted)
        if let Some(archived_record) =
            SqliteImportedRecordRepo.find_by_source_key(conn, &source_key)?
        {
            let is_archived = match decision.as_str() {
                "journal_entry" => {
                    match SqliteJournalEntryRepo.get(conn, &archived_record.entity_id) {
                        Ok(entry) => entry.archived_at.is_some(),
                        Err(e) if e.code == crate::error::ErrorCode::NotFound => false,
                        Err(e) => return Err(e),
                    }
                }
                "therapy_record" => {
                    match SqliteTherapyRecordRepo.get(conn, &archived_record.entity_id) {
                        Ok(record) => record.archived_at.is_some(),
                        Err(e) if e.code == crate::error::ErrorCode::NotFound => false,
                        Err(e) => return Err(e),
                    }
                }
                _ => false,
            };

            if is_archived {
                // Prior entity was archived: retain that entity and show the changed/archived condition for review.
                // Do not create a duplicate by default. Record a warning.
                warnings.push(ImportWarning::new(
                    WarningKind::AmbiguousCheckbox,
                    "warning",
                    &item.source_path,
                    None,
                    format!(
                        "Prior entity for '{}' (entity type: {}) was archived. Not creating a duplicate; review required.",
                        item.source_path, decision
                    ),
                ));
                continue;
            }
        }

        match decision.as_str() {
            "journal_entry" => {
                let occurred_on = time::today_utc();
                let title = item
                    .title
                    .clone()
                    .unwrap_or_else(|| item.source_path.clone());
                let entry = SqliteJournalEntryRepo.create(
                    conn,
                    workspace_id,
                    &occurred_on,
                    &title,
                    &file.content,
                    &None,
                    &None,
                    &BTreeMap::new(),
                    &None,
                    &None,
                    &[],
                    false,
                    JournalSource::Import,
                )?;

                SqliteImportedRecordRepo.insert(
                    conn,
                    &batch.id,
                    "journal_entry",
                    &entry.id,
                    &item.source_path,
                    &None,
                    "review",
                    &None,
                    &source_key,
                    content_hash,
                )?;

                *counts.entry("journal_entry".to_string()).or_insert(0) += 1;
            }
            "therapy_record" => {
                let record_type = TherapyRecordType::Note;
                let title = item
                    .title
                    .clone()
                    .unwrap_or_else(|| item.source_path.clone());
                let record = SqliteTherapyRecordRepo.create(
                    conn,
                    &therapy_collection_id,
                    record_type,
                    &title,
                    &file.content,
                    &None,
                )?;

                SqliteImportedRecordRepo.insert(
                    conn,
                    &batch.id,
                    "therapy_record",
                    &record.id,
                    &item.source_path,
                    &None,
                    "review",
                    &None,
                    &source_key,
                    content_hash,
                )?;

                *counts.entry("therapy_record".to_string()).or_insert(0) += 1;
            }
            _ => unreachable!("Validated above"),
        }
    }

    let counts: Vec<EntityCount> = counts
        .into_iter()
        .map(|(entity_type, count)| EntityCount { entity_type, count })
        .collect();

    let report = ImportReport {
        batch_id: batch.id.clone(),
        source_root: source_root.to_string(),
        status: "completed".to_string(),
        started_at: time::now_rfc3339(),
        completed_at: Some(time::now_rfc3339()),
        counts,
        warnings,
        review_items: all_review_items,
        preview_fingerprint: preview_fingerprint.to_string(),
    };

    let summary_json = serde_json::to_string(&report.summary())
        .map_err(|_| AppError::database("Could not serialize the import summary."))?;
    SqliteImportBatchRepo.complete(conn, &batch.id, &summary_json)?;

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_conn;
    use std::fs;
    use std::path::PathBuf;

    fn legacy_fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/legacy")
    }

    fn write_fixture(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(path, content).expect("write fixture");
    }

    fn fixture_root() -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("joi-import-test-{}", id::new_uuid()));
        dir
    }

    #[test]
    fn synthetic_legacy_fixtures_cover_review_privacy_and_ignored_paths() {
        let root = legacy_fixture_root();
        let source = root.join("records/2026-Foundation/Q2/diary/2026-06-25.md");
        let before = fs::read(&source).expect("read synthetic fixture before import");
        let discovered = discover(&root).expect("discover synthetic fixture tree");
        let paths: Vec<&str> = discovered
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect();

        assert!(paths.iter().any(|path| path.ends_with("example.txt")));
        assert!(paths.iter().all(|path| {
            !path.ends_with(".tex") && !path.ends_with(".pdf") && !path.ends_with(".png")
        }));
        for ignored in [".git/", ".opencode/", ".venv/", "node_modules/", "target/"] {
            assert!(
                paths.iter().all(|path| !path.contains(ignored)),
                "{ignored}"
            );
        }

        let report = preview_import("synthetic-workspace", root.to_str().unwrap(), None)
            .expect("preview synthetic fixture tree");
        for kind in [
            WarningKind::DateMismatch,
            WarningKind::Template,
            WarningKind::AmbiguousCheckbox,
            WarningKind::TherapyInGeneralDiary,
        ] {
            assert!(
                report.warnings.iter().any(|warning| warning.kind == kind),
                "missing reviewable warning {kind:?}"
            );
        }
        assert!(report.review_items.iter().any(|item| {
            item.source_path.ends_with("2000-00-00-raw.md")
                && item.proposed_entity_type == "journal_entry"
        }));
        assert!(report.review_items.iter().any(|item| {
            item.source_path.ends_with("nota-sintetica.md")
                && item.proposed_entity_type == "therapy_record"
                && item.confidence == CONFIDENCE_REVIEW
        }));
        assert!(report.review_items.iter().any(|item| {
            item.source_path.ends_with("synthetic-project.md")
                && item.proposed_entity_type == "project"
        }));
        assert!(report.review_items.iter().any(|item| {
            item.source_path.ends_with("synthetic-project.md")
                && item.proposed_entity_type == "task"
                && item.title.as_deref() == Some("Validate the sample web preview")
        }));

        let conn = test_conn();
        let imported = run_import(&conn, "w1", root.to_str().unwrap(), None)
            .expect("run read-only synthetic import");
        assert!(!imported.review_items.is_empty());
        let task_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
            .expect("count executable tasks");
        let dependency_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM task_dependencies", [], |row| {
                row.get(0)
            })
            .expect("count executable dependencies");
        let project_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
            .expect("count executable projects");
        let journal_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM journal_entries", [], |row| row.get(0))
            .expect("count journal entries before review");
        let therapy_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM therapy_records", [], |row| row.get(0))
            .expect("count therapy records before review");
        assert_eq!(task_count, 0, "task candidates require a review decision");
        assert_eq!(dependency_count, 0, "dependency candidates stay unlinked");
        assert_eq!(project_count, 0, "project candidates require review");
        assert_eq!(
            journal_count, 0,
            "journal candidates require a review decision"
        );
        assert_eq!(
            therapy_count, 0,
            "therapy candidates require explicit consent"
        );
        assert_eq!(fs::read(source).expect("read source after import"), before);
    }

    #[test]
    fn synthetic_possible_therapy_content_is_persisted_separately_only_after_choice() {
        let root = legacy_fixture_root();
        let source_path = "records/2026-Foundation/Q2/diary/personal-notes-raw/2026-06-25-raw.md";
        let include_paths = vec![source_path.to_string()];
        let conn = test_conn();
        let preview = preview_import("w1", root.to_str().unwrap(), Some(&include_paths))
            .expect("preview selected synthetic diary");
        assert!(preview
            .warnings
            .iter()
            .any(|warning| warning.kind == WarningKind::TherapyInGeneralDiary));

        let decisions: Vec<crate::commands::dto::ReviewDecisionDto> = preview
            .review_items
            .iter()
            .map(|item| crate::commands::dto::ReviewDecisionDto {
                review_item_id: item.id.clone(),
                entity_type: if item.proposed_entity_type == "journal_entry" {
                    "therapy_record".to_string()
                } else {
                    "ignore".to_string()
                },
            })
            .collect();
        let committed = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            Some(&include_paths),
            &preview.preview_fingerprint,
            &decisions,
        )
        .expect("explicitly classify synthetic entry as therapy");
        assert_eq!(committed.counts[0].entity_type, "therapy_record");
        let journal_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM journal_entries", [], |row| row.get(0))
            .expect("count journal entries");
        let therapy_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM therapy_records", [], |row| row.get(0))
            .expect("count therapy records");
        assert_eq!(journal_count, 0);
        assert_eq!(therapy_count, 1);
        let therapy_scope: String = conn
            .query_row(
                "SELECT privacy_scope FROM therapy_records LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("read therapy privacy scope");
        assert_eq!(therapy_scope, "therapy_private");
    }

    #[test]
    fn preview_does_not_create_batches_or_records() {
        let root = fixture_root();
        write_fixture(
            &root,
            "journal/entries/2026-06-25.md",
            "# Journal entry\n\nContent for preview only.\n",
        );

        let conn = test_conn();
        let report = preview_import("w1", root.to_str().unwrap(), None).expect("preview import");

        assert_eq!(report.status, "preview");
        assert!(!report.review_items.is_empty());
        let batch_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM import_batches", [], |row| row.get(0))
            .expect("count import batches");
        assert_eq!(batch_count, 0);
        let entries = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert!(entries.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn import_does_not_modify_source_files() {
        let root = fixture_root();
        let content = "# Diario 2026-06-25\n\nHoy fue un día tranquilo.\n";
        write_fixture(&root, "journal/entries/2026-06-25-raw.md", content);
        let before = fs::read_to_string(root.join("journal/entries/2026-06-25-raw.md"))
            .expect("read before");

        let conn = test_conn();
        let report = run_import(&conn, "w1", root.to_str().unwrap(), None).expect("run import");

        let after =
            fs::read_to_string(root.join("journal/entries/2026-06-25-raw.md")).expect("read after");
        assert_eq!(before, after, "source file must be unchanged");
        assert_eq!(report.status, "completed");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn review_preserves_original_paths_and_language() {
        let root = fixture_root();
        let content = "# Diario personal — 2026-06-25\n\nMe sentí motivado y con energía.\n";
        write_fixture(
            &root,
            "records/2026-Foundation/Q2/diary/personal-notes-raw/2026-06-25-raw.md",
            content,
        );

        let conn = test_conn();
        let report = run_import(&conn, "w1", root.to_str().unwrap(), None).expect("run import");

        let entries = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert!(
            entries.is_empty(),
            "path-derived journal imports need review"
        );
        let item = report
            .review_items
            .iter()
            .find(|item| item.source_path.ends_with("2026-06-25-raw.md"))
            .expect("review item");
        assert_eq!(item.proposed_entity_type, "journal_entry");
        assert_eq!(item.title.as_deref(), Some("Diario personal — 2026-06-25"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn therapy_content_requires_review_before_persistence() {
        let root = fixture_root();
        write_fixture(
            &root,
            "salud-mental/Notas/terapia-10-06-2026.md",
            "# Terapia\n\nHoy trabajé la regulación emocional.\n",
        );

        let conn = test_conn();
        let report = run_import(&conn, "w1", root.to_str().unwrap(), None).expect("run import");

        let collections = SqliteTherapyCollectionRepo
            .list(&conn, "w1")
            .expect("list collections");
        assert_eq!(collections.len(), 1);
        let records = SqliteTherapyRecordRepo
            .list_for_collection(&conn, &collections[0].id)
            .expect("list records");
        assert!(records.is_empty());

        // No journal entry should be created for therapy content.
        let entries = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert!(entries.is_empty());

        // The report proposes a therapy record but does not persist it.
        assert!(report
            .review_items
            .iter()
            .any(|item| item.proposed_entity_type == "therapy_record"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn repeated_review_does_not_create_records() {
        let root = fixture_root();
        write_fixture(
            &root,
            "records/2026-Foundation/Q2/diary/personal-notes-raw/2026-06-25-raw.md",
            "# Diario 2026-06-25\n\nContenido.\n",
        );

        let conn = test_conn();
        run_import(&conn, "w1", root.to_str().unwrap(), None).expect("first import");
        run_import(&conn, "w1", root.to_str().unwrap(), None).expect("second import");

        let entries = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert!(entries.is_empty(), "review items must not be auto-imported");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn txt_files_are_reviewed_before_journal_import() {
        let root = fixture_root();
        write_fixture(
            &root,
            "journal/entries/plain-note.txt",
            "Notas sueltas sin encabezado.\nSegunda línea.\n",
        );

        let conn = test_conn();
        let report = run_import(&conn, "w1", root.to_str().unwrap(), None).expect("run import");

        let entries = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert!(entries.is_empty());
        assert!(report
            .review_items
            .iter()
            .any(|item| item.proposed_entity_type == "journal_entry"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn tex_and_pdf_are_silently_ignored() {
        let root = fixture_root();
        write_fixture(&root, "records/export.tex", "\\documentclass{article}\n");
        write_fixture(&root, "records/export.pdf", "%PDF-1.4\n");
        write_fixture(
            &root,
            "journal/entries/2026-06-25-raw.md",
            "# Diario 2026-06-25\n\nContenido.\n",
        );

        let conn = test_conn();
        let report = run_import(&conn, "w1", root.to_str().unwrap(), None).expect("run import");

        // No journal or therapy records should be created from .tex/.pdf.
        let entries = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert!(entries.is_empty(), "the .md file also requires review");
        let collections = SqliteTherapyCollectionRepo
            .list(&conn, "w1")
            .expect("list collections");
        // The private therapy collection is always ensured, but it must hold no
        // records because .tex/.pdf are not imported as entries.
        for collection in &collections {
            let records = SqliteTherapyRecordRepo
                .list_for_collection(&conn, &collection.id)
                .expect("list records");
            assert!(records.is_empty());
        }

        // .tex/.pdf are silently ignored: no Artifact review items and no
        // warnings are produced for them.
        assert_eq!(report.review_items.len(), 1);
        assert!(report
            .warnings
            .iter()
            .all(|w| !w.source_path.ends_with(".tex") && !w.source_path.ends_with(".pdf")));
        assert!(report
            .review_items
            .iter()
            .any(|item| item.source_path.ends_with("2026-06-25-raw.md")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn title_comes_from_first_h1() {
        let root = fixture_root();
        write_fixture(
            &root,
            "journal/entries/reflection.md",
            "# Mi reflexión\n\nContenido del día.\n",
        );

        let conn = test_conn();
        let report = run_import(&conn, "w1", root.to_str().unwrap(), None).expect("run import");

        let item = report
            .review_items
            .iter()
            .find(|item| item.source_path.ends_with("reflection.md"))
            .expect("review item");
        assert_eq!(item.title.as_deref(), Some("Mi reflexión"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn generic_title_derived_from_date_and_name() {
        let root = fixture_root();
        write_fixture(
            &root,
            "journal/entries/2026-06-25.md",
            "Contenido sin encabezado.\n",
        );

        let conn = test_conn();
        let report = run_import(&conn, "w1", root.to_str().unwrap(), None).expect("run import");

        let item = report
            .review_items
            .iter()
            .find(|item| item.source_path.ends_with("2026-06-25.md"))
            .expect("review item");
        assert_eq!(item.title.as_deref(), Some("Journal entry — 2026-06-25"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn include_paths_restrict_import_and_keep_therapy_routing() {
        let root = fixture_root();
        write_fixture(
            &root,
            "salud-mental/Notas/terapia.md",
            "# Terapia\n\nContenido terapéutico.\n",
        );
        write_fixture(
            &root,
            "records/2026-Foundation/Q2/diary/personal-notes-raw/2026-06-25-raw.md",
            "# Diario\n\nContenido personal.\n",
        );
        write_fixture(
            &root,
            "records/2026-Foundation/Q2/planning/plan-Q2.md",
            "# Plan Q2\n\nContenido de planificación.\n",
        );

        let conn = test_conn();
        let include = vec![
            "salud-mental".to_string(),
            "records/2026-Foundation/Q2/diary/personal-notes-raw".to_string(),
        ];
        let report =
            run_import(&conn, "w1", root.to_str().unwrap(), Some(&include)).expect("run import");

        // Therapy content is proposed for the private collection but is not
        // persisted without a review decision.
        let collections = SqliteTherapyCollectionRepo
            .list(&conn, "w1")
            .expect("list collections");
        assert_eq!(collections.len(), 1);
        let records = SqliteTherapyRecordRepo
            .list_for_collection(&conn, &collections[0].id)
            .expect("list records");
        assert!(records.is_empty());

        // Personal notes are proposed as journal entries for review.
        let entries = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert!(entries.is_empty());

        // The planning file is excluded by the filter.
        assert!(!report
            .review_items
            .iter()
            .any(|r| r.source_path.contains("planning")));
        assert!(report
            .review_items
            .iter()
            .any(|item| item.proposed_entity_type == "therapy_record"));
        assert!(report
            .review_items
            .iter()
            .any(|item| item.proposed_entity_type == "journal_entry"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn include_paths_reject_absolute_paths() {
        let root = fixture_root();
        write_fixture(
            &root,
            "salud-mental/Notas/terapia.md",
            "# Terapia\n\nContenido.\n",
        );

        let conn = test_conn();
        let include = vec!["/etc/passwd".to_string()];
        let result = run_import(&conn, "w1", root.to_str().unwrap(), Some(&include));
        assert!(result.is_err());
        let _ = fs::remove_dir_all(&root);
    }

    /// Build a `Db` with an in-memory connection and a seeded workspace `w1`,
    /// mirroring `test_conn` but returning the cloneable handle the worker owns.
    fn worker_db() -> crate::state::Db {
        let db = crate::state::Db::open_in_memory().expect("open in-memory db");
        let now = crate::time::now_rfc3339();
        db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES ('w1', 'Test', ?1, ?1)",
                [&now],
            )
            .expect("seed workspace");
            Ok(())
        })
        .expect("seed workspace");
        db
    }

    #[test]
    fn run_import_with_batch_uses_precreated_batch() {
        let root = fixture_root();
        write_fixture(
            &root,
            "journal/entries/2026-06-25.md",
            "# Diario 2026-06-25\n\nContenido sintético.\n",
        );

        let conn = test_conn();
        let batch = SqliteImportBatchRepo
            .create(&conn, "w1", root.to_str().unwrap())
            .expect("create batch");
        assert_eq!(batch.status, "running");

        let report = run_import_with_batch(&conn, "w1", root.to_str().unwrap(), None, &batch.id)
            .expect("run import with batch");

        assert_eq!(report.batch_id, batch.id);
        assert_eq!(report.status, "completed");

        let reloaded = SqliteImportBatchRepo
            .get(&conn, &batch.id)
            .expect("get batch");
        assert_eq!(reloaded.status, "completed");
        assert!(reloaded.summary_json.is_some());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn worker_completes_batch_and_does_not_modify_source() {
        let root = fixture_root();
        let content = "# Diario 2026-06-25\n\nHoy fue un día tranquilo.\n";
        write_fixture(&root, "journal/entries/2026-06-25-raw.md", content);
        let before = fs::read_to_string(root.join("journal/entries/2026-06-25-raw.md"))
            .expect("read before");

        let db = worker_db();
        let batch = db
            .with_conn(|conn| SqliteImportBatchRepo.create(conn, "w1", root.to_str().unwrap()))
            .expect("create batch");
        let batch_id = batch.id.clone();

        run_import_worker(
            db.clone(),
            "w1".to_string(),
            root.to_str().unwrap().to_string(),
            None,
            batch_id.clone(),
        );

        let after =
            fs::read_to_string(root.join("journal/entries/2026-06-25-raw.md")).expect("read after");
        assert_eq!(before, after, "source file must be unchanged");

        let reloaded = db
            .with_conn(|conn| SqliteImportBatchRepo.get(conn, &batch_id))
            .expect("get batch");
        assert_eq!(reloaded.status, "completed");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn worker_marks_batch_failed_on_error() {
        // A source root that does not exist makes discovery fail inside the
        // worker transaction; the batch must be marked failed, never left
        // running.
        let db = worker_db();
        let missing = fixture_root(); // never created
        let batch = db
            .with_conn(|conn| SqliteImportBatchRepo.create(conn, "w1", missing.to_str().unwrap()))
            .expect("create batch");
        let batch_id = batch.id.clone();

        run_import_worker(
            db.clone(),
            "w1".to_string(),
            missing.to_str().unwrap().to_string(),
            None,
            batch_id.clone(),
        );

        let reloaded = db
            .with_conn(|conn| SqliteImportBatchRepo.get(conn, &batch_id))
            .expect("get batch");
        assert_eq!(reloaded.status, "failed");
        assert!(reloaded.completed_at.is_some());
        // The failed summary is fixed and content-safe: no source paths.
        let summary = reloaded.summary_json.expect("failed summary");
        assert!(!summary.contains("journal"));
        assert!(!summary.contains("source_path"));
    }

    #[test]
    fn fixed_failed_summary_is_content_safe() {
        let json = fixed_failed_summary_json();
        let parsed: crate::import::import_report::ImportSummary =
            serde_json::from_str(&json).expect("parse summary");
        assert!(parsed.counts.is_empty());
        assert!(parsed.warnings.is_empty());
        assert!(parsed.review_items.is_empty());
    }

    #[test]
    fn commit_legacy_import_succeeds_with_valid_decisions() {
        let root = fixture_root();
        let content = "# Diario 2026-06-25\n\nHoy fue un día tranquilo.\n";
        write_fixture(&root, "journal/entries/2026-06-25-raw.md", content);

        let conn = test_conn();

        // First, run preview to get review items and fingerprint
        let preview = preview_import("w1", root.to_str().unwrap(), None).expect("preview import");
        assert_eq!(preview.status, "preview");
        assert!(!preview.review_items.is_empty());

        // Provide decisions for ALL review items (main file + any checkboxes)
        let decisions: Vec<crate::commands::dto::ReviewDecisionDto> = preview
            .review_items
            .iter()
            .map(|item| {
                let entity_type = if item.proposed_entity_type == "journal_entry" {
                    "journal_entry"
                } else {
                    "ignore" // Ignore checkbox items
                };
                crate::commands::dto::ReviewDecisionDto {
                    review_item_id: item.id.clone(),
                    entity_type: entity_type.to_string(),
                }
            })
            .collect();

        let report = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            None,
            &preview.preview_fingerprint,
            &decisions,
        )
        .expect("commit import");

        assert_eq!(report.status, "completed");
        assert_eq!(
            report
                .counts
                .iter()
                .find(|c| c.entity_type == "journal_entry")
                .map(|c| c.count),
            Some(1)
        );

        // Verify the journal entry was created
        let entries = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Diario 2026-06-25");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn commit_legacy_import_rejects_stale_fingerprint() {
        let root = fixture_root();
        let content = "# Diario 2026-06-25\n\nHoy fue un día tranquilo.\n";
        write_fixture(&root, "journal/entries/2026-06-25-raw.md", content);

        let conn = test_conn();

        let preview = preview_import("w1", root.to_str().unwrap(), None).expect("preview import");

        // Provide decisions for all review items
        let decisions: Vec<crate::commands::dto::ReviewDecisionDto> = preview
            .review_items
            .iter()
            .map(|item| crate::commands::dto::ReviewDecisionDto {
                review_item_id: item.id.clone(),
                entity_type: "journal_entry".to_string(),
            })
            .collect();

        // Use a wrong fingerprint
        let result = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            None,
            "wrong-fingerprint",
            &decisions,
        );

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("fingerprint mismatch"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn commit_legacy_import_rejects_incomplete_decisions() {
        let root = fixture_root();
        let content = "# Diario 2026-06-25\n\nHoy fue un día tranquilo.\n";
        write_fixture(&root, "journal/entries/2026-06-25-raw.md", content);

        let conn = test_conn();

        let preview = preview_import("w1", root.to_str().unwrap(), None).expect("preview import");

        // Provide no decisions (incomplete)
        let decisions: Vec<crate::commands::dto::ReviewDecisionDto> = vec![];

        let result = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            None,
            &preview.preview_fingerprint,
            &decisions,
        );

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Incomplete decisions"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn commit_legacy_import_rejects_missing_source_file() {
        let root = fixture_root();
        let content = "# Diario 2026-06-25\n\nHoy fue un día tranquilo.\n";
        write_fixture(&root, "journal/entries/2026-06-25-raw.md", content);

        let conn = test_conn();

        let preview = preview_import("w1", root.to_str().unwrap(), None).expect("preview import");

        // Delete the source file after preview
        fs::remove_file(root.join("journal/entries/2026-06-25-raw.md")).expect("remove file");

        // Provide decisions for all review items
        let decisions: Vec<crate::commands::dto::ReviewDecisionDto> = preview
            .review_items
            .iter()
            .map(|item| crate::commands::dto::ReviewDecisionDto {
                review_item_id: item.id.clone(),
                entity_type: "journal_entry".to_string(),
            })
            .collect();

        let result = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            None,
            &preview.preview_fingerprint,
            &decisions,
        );

        assert!(result.is_err());
        // Fingerprint mismatch is detected first because re-discovery finds different files
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("fingerprint mismatch"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn commit_legacy_import_rejects_changed_source_content() {
        let root = fixture_root();
        let content = "# Diario 2026-06-25\n\nHoy fue un día tranquilo.\n";
        write_fixture(&root, "journal/entries/2026-06-25-raw.md", content);

        let conn = test_conn();

        let preview = preview_import("w1", root.to_str().unwrap(), None).expect("preview import");

        // Modify the source file after preview
        let modified_content = "# Diario 2026-06-25\n\nContenido modificado.\n";
        fs::write(
            root.join("journal/entries/2026-06-25-raw.md"),
            modified_content,
        )
        .expect("write modified");

        // Provide decisions for all review items
        let decisions: Vec<crate::commands::dto::ReviewDecisionDto> = preview
            .review_items
            .iter()
            .map(|item| crate::commands::dto::ReviewDecisionDto {
                review_item_id: item.id.clone(),
                entity_type: "journal_entry".to_string(),
            })
            .collect();

        let result = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            None,
            &preview.preview_fingerprint,
            &decisions,
        );

        assert!(result.is_err());
        // Fingerprint mismatch is detected first because content hash changes
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("fingerprint mismatch"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn commit_legacy_import_therapy_only_via_explicit_decision() {
        let root = fixture_root();
        let content = "# Terapia\n\nHoy trabajé la regulación emocional.\n";
        write_fixture(&root, "salud-mental/Notas/terapia.md", content);

        let conn = test_conn();

        let preview = preview_import("w1", root.to_str().unwrap(), None).expect("preview import");

        // Provide decisions for all review items - therapy_record for the main file, ignore for checkboxes
        let decisions: Vec<crate::commands::dto::ReviewDecisionDto> = preview
            .review_items
            .iter()
            .map(|item| {
                let entity_type = if item.proposed_entity_type == "therapy_record" {
                    "therapy_record"
                } else {
                    "ignore"
                };
                crate::commands::dto::ReviewDecisionDto {
                    review_item_id: item.id.clone(),
                    entity_type: entity_type.to_string(),
                }
            })
            .collect();

        let report = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            None,
            &preview.preview_fingerprint,
            &decisions,
        )
        .expect("commit import");

        assert_eq!(report.status, "completed");
        assert_eq!(
            report
                .counts
                .iter()
                .find(|c| c.entity_type == "therapy_record")
                .map(|c| c.count),
            Some(1)
        );

        // Verify the therapy record was created
        let collections = SqliteTherapyCollectionRepo
            .list(&conn, "w1")
            .expect("list collections");
        assert_eq!(collections.len(), 1);
        let records = SqliteTherapyRecordRepo
            .list_for_collection(&conn, &collections[0].id)
            .expect("list records");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].title, "Terapia");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn commit_legacy_import_ignore_decision_skips_item() {
        let root = fixture_root();
        let content = "# Diario 2026-06-25\n\nHoy fue un día tranquilo.\n";
        write_fixture(&root, "journal/entries/2026-06-25-raw.md", content);

        let conn = test_conn();

        let preview = preview_import("w1", root.to_str().unwrap(), None).expect("preview import");

        // Provide ignore decisions for all review items
        let decisions: Vec<crate::commands::dto::ReviewDecisionDto> = preview
            .review_items
            .iter()
            .map(|item| crate::commands::dto::ReviewDecisionDto {
                review_item_id: item.id.clone(),
                entity_type: "ignore".to_string(),
            })
            .collect();

        let report = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            None,
            &preview.preview_fingerprint,
            &decisions,
        )
        .expect("commit import");

        assert_eq!(report.status, "completed");
        assert!(report.counts.is_empty() || report.counts.iter().all(|c| c.count == 0));

        // Verify no journal entry was created
        let entries = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert!(entries.is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn commit_legacy_import_rejects_invalid_entity_type() {
        let root = fixture_root();
        let content = "# Diario 2026-06-25\n\nHoy fue un día tranquilo.\n";
        write_fixture(&root, "journal/entries/2026-06-25-raw.md", content);

        let conn = test_conn();

        let preview = preview_import("w1", root.to_str().unwrap(), None).expect("preview import");

        // Provide decisions for all review items, but use invalid "task" for journal_entry items
        let decisions: Vec<crate::commands::dto::ReviewDecisionDto> = preview
            .review_items
            .iter()
            .map(|item| {
                let entity_type = if item.proposed_entity_type == "journal_entry" {
                    "task" // Invalid for commit
                } else {
                    "ignore"
                };
                crate::commands::dto::ReviewDecisionDto {
                    review_item_id: item.id.clone(),
                    entity_type: entity_type.to_string(),
                }
            })
            .collect();

        let result = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            None,
            &preview.preview_fingerprint,
            &decisions,
        );

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid entity_type"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn preview_includes_source_hash_and_fingerprint() {
        let root = fixture_root();
        let content = "# Diario 2026-06-25\n\nHoy fue un día tranquilo.\n";
        write_fixture(&root, "journal/entries/2026-06-25-raw.md", content);

        let _conn = test_conn();
        let preview = preview_import("w1", root.to_str().unwrap(), None).expect("preview import");

        assert_eq!(preview.status, "preview");
        assert!(!preview.review_items.is_empty());
        assert!(!preview.preview_fingerprint.is_empty());

        // All review items should have source_hash
        for item in &preview.review_items {
            assert!(
                !item.source_hash.is_empty(),
                "review item missing source_hash"
            );
            // Verify it's a valid hex string (64 chars for SHA-256)
            assert_eq!(item.source_hash.len(), 64);
            assert!(item.source_hash.chars().all(|c| c.is_ascii_hexdigit()));
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn compute_preview_fingerprint_is_deterministic() {
        let root = fixture_root();
        let content = "# Diario 2026-06-25\n\nHoy fue un día tranquilo.\n";
        write_fixture(&root, "journal/entries/2026-06-25-raw.md", content);

        let preview1 =
            preview_import("w1", root.to_str().unwrap(), None).expect("preview import 1");
        let preview2 =
            preview_import("w1", root.to_str().unwrap(), None).expect("preview import 2");

        // Fingerprints should be identical for same input
        assert_eq!(preview1.preview_fingerprint, preview2.preview_fingerprint);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn run_import_idempotent_recommit_skips_unchanged() {
        // This test verifies idempotency for commit_legacy_import (where entities are actually created).
        // run_import only creates review items for journal/therapy, so idempotency is tested via commit.
        let root = fixture_root();
        let content = "# Diario 2026-06-25\n\nHoy fue un día tranquilo.\n";
        write_fixture(&root, "journal/entries/2026-06-25-raw.md", content);

        let conn = test_conn();

        // First, commit a journal entry via preview + commit
        let preview1 =
            preview_import("w1", root.to_str().unwrap(), None).expect("preview import 1");
        let decisions1: Vec<crate::commands::dto::ReviewDecisionDto> = preview1
            .review_items
            .iter()
            .map(|item| crate::commands::dto::ReviewDecisionDto {
                review_item_id: item.id.clone(),
                entity_type: "journal_entry".to_string(),
            })
            .collect();
        let report1 = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            None,
            &preview1.preview_fingerprint,
            &decisions1,
        )
        .expect("commit import 1");
        assert_eq!(report1.status, "completed");

        let entries1 = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert_eq!(entries1.len(), 1);
        let entry_id = entries1[0].id.clone();

        let records1 = SqliteImportedRecordRepo
            .list_for_batch(&conn, &report1.batch_id)
            .expect("list batch 1");

        // Second commit with same content - should skip and associate with new batch
        let preview2 =
            preview_import("w1", root.to_str().unwrap(), None).expect("preview import 2");
        let decisions2: Vec<crate::commands::dto::ReviewDecisionDto> = preview2
            .review_items
            .iter()
            .map(|item| crate::commands::dto::ReviewDecisionDto {
                review_item_id: item.id.clone(),
                entity_type: "journal_entry".to_string(),
            })
            .collect();
        let report2 = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            None,
            &preview2.preview_fingerprint,
            &decisions2,
        )
        .expect("commit import 2");
        assert_eq!(report2.status, "completed");

        let entries2 = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert_eq!(entries2.len(), 1, "should not create duplicate entry");
        assert_eq!(entries2[0].id, entry_id, "should reuse same entity");

        let records2 = SqliteImportedRecordRepo
            .list_for_batch(&conn, &report2.batch_id)
            .expect("list batch 2");
        assert_eq!(records2.len(), 1);
        assert_eq!(records1[0].entity_id, entry_id);
        assert_eq!(records2[0].entity_id, entry_id);
        assert_eq!(
            records1[0].source_key, records2[0].source_key,
            "source_key should be same"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn commit_legacy_import_changed_hash_warns_without_overwrite() {
        let root = fixture_root();
        let content = "# Diario 2026-06-25\n\nHoy fue un día tranquilo.\n";
        write_fixture(&root, "journal/entries/2026-06-25-raw.md", content);

        let conn = test_conn();

        // First, commit a journal entry
        let preview1 =
            preview_import("w1", root.to_str().unwrap(), None).expect("preview import 1");
        let decisions1: Vec<crate::commands::dto::ReviewDecisionDto> = preview1
            .review_items
            .iter()
            .map(|item| crate::commands::dto::ReviewDecisionDto {
                review_item_id: item.id.clone(),
                entity_type: "journal_entry".to_string(),
            })
            .collect();
        let report1 = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            None,
            &preview1.preview_fingerprint,
            &decisions1,
        )
        .expect("commit import 1");
        assert_eq!(report1.status, "completed");

        let entries1 = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert_eq!(entries1.len(), 1);
        let entry_id = entries1[0].id.clone();
        let original_content = entries1[0].content_markdown.clone();

        // Modify the source file
        let modified_content = "# Diario 2026-06-25\n\nContenido modificado.\n";
        fs::write(
            root.join("journal/entries/2026-06-25-raw.md"),
            modified_content,
        )
        .expect("write modified");

        // Second commit - should warn but not overwrite
        let preview2 =
            preview_import("w1", root.to_str().unwrap(), None).expect("preview import 2");
        let decisions2: Vec<crate::commands::dto::ReviewDecisionDto> = preview2
            .review_items
            .iter()
            .map(|item| crate::commands::dto::ReviewDecisionDto {
                review_item_id: item.id.clone(),
                entity_type: "journal_entry".to_string(),
            })
            .collect();
        let report2 = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            None,
            &preview2.preview_fingerprint,
            &decisions2,
        )
        .expect("commit import 2");
        assert_eq!(report2.status, "completed");

        // Should have a warning about changed content
        assert!(report2
            .warnings
            .iter()
            .any(|w| w.message.contains("changed since last import")));

        // Original entity should be unchanged
        let entries2 = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert_eq!(entries2.len(), 1);
        assert_eq!(entries2[0].id, entry_id);
        assert_eq!(
            entries2[0].content_markdown, original_content,
            "user content should not be overwritten"
        );

        // Second batch should not have a new imported_record for this entity
        let records2 = SqliteImportedRecordRepo
            .list_for_batch(&conn, &report2.batch_id)
            .expect("list batch 2");
        // The changed entity should not be associated with the new batch
        assert!(records2.is_empty() || records2.iter().all(|r| r.entity_id != entry_id));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn commit_legacy_import_archived_entity_no_duplicate() {
        let root = fixture_root();
        let content = "# Diario 2026-06-25\n\nHoy fue un día tranquilo.\n";
        write_fixture(&root, "journal/entries/2026-06-25-raw.md", content);

        let conn = test_conn();

        // First, commit a journal entry
        let preview1 = preview_import("w1", root.to_str().unwrap(), None).expect("preview import");
        let decisions1: Vec<crate::commands::dto::ReviewDecisionDto> = preview1
            .review_items
            .iter()
            .map(|item| crate::commands::dto::ReviewDecisionDto {
                review_item_id: item.id.clone(),
                entity_type: "journal_entry".to_string(),
            })
            .collect();
        let report1 = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            None,
            &preview1.preview_fingerprint,
            &decisions1,
        )
        .expect("commit import 1");
        assert_eq!(report1.status, "completed");

        let entries = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert_eq!(entries.len(), 1);
        let entry_id = entries[0].id.clone();

        // Archive the journal entry
        SqliteJournalEntryRepo
            .archive(&conn, &entry_id)
            .expect("archive entry");

        // Verify it's archived
        let archived_entry = SqliteJournalEntryRepo
            .get(&conn, &entry_id)
            .expect("get archived entry");
        assert!(archived_entry.archived_at.is_some());

        let records1 = SqliteImportedRecordRepo
            .list_for_batch(&conn, &report1.batch_id)
            .expect("list batch 1");
        let source_key = records1[0].source_key.clone().unwrap();
        let _found = SqliteImportedRecordRepo
            .find_by_source_key(&conn, &source_key)
            .expect("find by source_key");

        // Now try to commit again with same source - should warn about archived entity and not create duplicate
        let preview2 =
            preview_import("w1", root.to_str().unwrap(), None).expect("preview import 2");
        let decisions2: Vec<crate::commands::dto::ReviewDecisionDto> = preview2
            .review_items
            .iter()
            .map(|item| crate::commands::dto::ReviewDecisionDto {
                review_item_id: item.id.clone(),
                entity_type: "journal_entry".to_string(),
            })
            .collect();
        let report2 = commit_legacy_import(
            &conn,
            "w1",
            root.to_str().unwrap(),
            None,
            &preview2.preview_fingerprint,
            &decisions2,
        )
        .expect("commit import 2");
        assert_eq!(report2.status, "completed");

        // Should have a warning about archived entity
        assert!(report2
            .warnings
            .iter()
            .any(|w| w.message.contains("was archived")));

        // Should not create a new journal entry (the archived one still exists but is not listed)
        let archived_entry = SqliteJournalEntryRepo
            .get(&conn, &entry_id)
            .expect("get archived entry");
        assert!(
            archived_entry.archived_at.is_some(),
            "original entry should still be archived"
        );
        let entries2 = SqliteJournalEntryRepo
            .list(&conn, "w1")
            .expect("list entries");
        assert_eq!(
            entries2.len(),
            0,
            "should not create duplicate for archived entity (list excludes archived)"
        );

        let _ = fs::remove_dir_all(&root);
    }
}
