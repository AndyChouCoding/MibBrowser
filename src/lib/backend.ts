import { invoke } from "@tauri-apps/api/core";

export type HealthStatus = {
  status: "ready";
  version: string;
};

export type MibModuleSummary = {
  name: string;
  source: string;
  enabled: boolean;
};

export const backend = {
  health: () => invoke<HealthStatus>("health_check"),
  bundledModules: () => invoke<MibModuleSummary[]>("list_bundled_modules"),
};
