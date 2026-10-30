import { invoke } from "../tauri";
import type { EnvironmentDiagnostics } from "../types";

export function getEnvironmentDiagnostics(): Promise<EnvironmentDiagnostics> {
  return invoke<EnvironmentDiagnostics>("get_environment_diagnostics");
}
