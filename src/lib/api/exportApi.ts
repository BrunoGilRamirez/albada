import { invoke } from "../tauri";
import { openPath } from "@tauri-apps/plugin-opener";
import type {
  ExportJob,
  ExportPreview,
  ExportScopeType,
  ExportType,
  SaveExportArtifactCopyResult,
  TemplateCapability,
  TemplateManifest,
} from "../types";

export interface CreateExportJobInput {
  workspace_id: string;
  export_type: ExportType;
  scope_type: ExportScopeType;
  title: string;
  template_name?: string | null;
  selection: ExportSelectionInput;
  options?: Record<string, unknown> | null;
  preview_fingerprint?: string | null;
}

export interface ExportSelectionInput {
  journal_entry_ids?: string[] | null;
  therapy_record_ids?: string[] | null;
  task_ids?: string[] | null;
  project_ids?: string[] | null;
  goal_ids?: string[] | null;
  quarter_ids?: string[] | null;
  date_range?: { from: string; to: string } | null;
  export_all?: boolean | null;
}

export function createExportJob(input: CreateExportJobInput): Promise<ExportJob> {
  return invoke<ExportJob>("create_export_job", { dto: input });
}

export function listExportJobs(workspaceId: string): Promise<ExportJob[]> {
  return invoke<ExportJob[]>("list_export_jobs", { workspaceId });
}

export function deleteExportJob(workspaceId: string, jobId: string): Promise<void> {
  return invoke<void>("delete_export_job", { workspaceId, jobId });
}

export function openExportArtifact(path: string): Promise<void> {
  return openPath(path);
}

export function runExportJob(jobId: string): Promise<ExportJob> {
  // Tauri command arguments use the Rust command parameter name at the IPC
  // boundary. The command parameter is `job_id`, which Tauri exposes as
  // `jobId` in its camelCase wire format.
  return invoke<ExportJob>("run_export_job", { jobId });
}

export function previewExportSelection(input: CreateExportJobInput): Promise<ExportPreview> {
  return invoke<ExportPreview>("preview_export_selection", { dto: input });
}

export function cancelExportJob(jobId: string): Promise<ExportJob> {
  return invoke<ExportJob>("cancel_export_job", { jobId });
}

export function exportTemplateCapability(): Promise<TemplateCapability> {
  return invoke<TemplateCapability>("export_template_capability");
}

export function exportTemplateCatalog(): Promise<TemplateManifest[]> {
  return invoke<TemplateManifest[]>("export_template_catalog");
}

export function saveExportArtifactCopy(
  workspaceId: string,
  jobId: string,
  artifactId: string,
): Promise<SaveExportArtifactCopyResult> {
  return invoke<SaveExportArtifactCopyResult>("save_export_artifact_copy", {
    workspaceId,
    jobId,
    artifactId,
  });
}
