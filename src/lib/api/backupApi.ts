import { invoke } from "../tauri";
import type { BackupDto, BackupValidationDto, Workspace } from "../types";

export function createBackup(): Promise<BackupDto> {
  return invoke<BackupDto>("create_backup");
}

export function listBackups(): Promise<BackupDto[]> {
  return invoke<BackupDto[]>("list_backups");
}

export function deleteBackup(path: string): Promise<void> {
  return invoke<void>("delete_backup", { path });
}

export function validateBackup(path: string): Promise<BackupValidationDto> {
  return invoke<BackupValidationDto>("validate_backup", { path });
}

export function restoreBackup(path: string, expectedChecksum: string): Promise<Workspace> {
  return invoke<Workspace>("restore_backup", {
    path,
    expectedChecksum,
  });
}
