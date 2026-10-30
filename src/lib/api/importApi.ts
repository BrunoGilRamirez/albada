import { invoke } from "../tauri";
import type { CommitLegacyImportDto, ImportBatchStatusDto, ImportReportDto } from "../types";

export function runLegacyImport(
  sourceRoot: string,
  includePaths: string[] | null = null,
): Promise<ImportReportDto> {
  return invoke<ImportReportDto>("run_legacy_import", {
    dto: { source_root: sourceRoot, include_paths: includePaths },
  });
}

export function startLegacyImport(
  sourceRoot: string,
  includePaths: string[] | null = null,
): Promise<ImportBatchStatusDto> {
  return invoke<ImportBatchStatusDto>("start_legacy_import", {
    dto: { source_root: sourceRoot, include_paths: includePaths },
  });
}

export function previewLegacyImport(
  sourceRoot: string,
  includePaths: string[] | null = null,
): Promise<ImportReportDto> {
  return invoke<ImportReportDto>("preview_legacy_import", {
    dto: { source_root: sourceRoot, include_paths: includePaths },
  });
}

export function commitLegacyImport(dto: CommitLegacyImportDto): Promise<ImportReportDto> {
  return invoke<ImportReportDto>("commit_legacy_import", { dto });
}

export function getImportBatch(batchId: string): Promise<ImportReportDto> {
  return invoke<ImportReportDto>("get_import_batch", { batchId });
}

export function getImportBatchStatus(batchId: string): Promise<ImportBatchStatusDto> {
  return invoke<ImportBatchStatusDto>("get_import_batch_status", { batchId });
}
