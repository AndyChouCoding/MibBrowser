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

export type Credentials = {version: "v1" | "v2c"; community: string} | {
  version: "v3"; username: string; context: string; level: "noAuthNoPriv" | "authNoPriv" | "authPriv"; authPassword: string; privacyPassword: string
};
export const backend = {
  query: (host: string, port: number, credentials: Credentials, oid: string, operation: "get" | "getNext" | "walk") =>
    invoke<{oid: string; kind: string; value: string; durationMs: number}[]>("query_device", {host, port, credentials, oid, operation}),
  exportResults: (format: string, content: string) => invoke<boolean>("export_results", {format, content}),
  health: () => invoke<HealthStatus>("health_check"),
  bundledModules: () => invoke<MibModuleSummary[]>("list_bundled_modules"),
  loadMibFiles: (paths: string[]) => invoke<MibSnapshot>("load_mib_files", { paths }),
};
