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

export type MibNodeSummary = {
  name: string;
  oid: string;
  kind: string;
  module: string | null;
  description: string;
  children: MibNodeSummary[];
};

export type MibSnapshot = {
  modules: MibModuleSummary[];
  roots: MibNodeSummary[];
  nodeCount: number;
};

export const backend = {
  query: (host: string, port: number, community: string, oid: string, operation: "get" | "getNext" | "walk") =>
    invoke<{oid: string; kind: string; value: string; durationMs: number}[]>("query_v2c", {host, port, community, oid, operation}),
  exportResults: (format: string, content: string) => invoke<boolean>("export_results", {format, content}),
  health: () => invoke<HealthStatus>("health_check"),
  bundledModules: () => invoke<MibModuleSummary[]>("list_bundled_modules"),
  loadMibFiles: (paths: string[]) => invoke<MibSnapshot>("load_mib_files", { paths }),
};
