import { Channel, invoke } from "@tauri-apps/api/core";

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
export type Binding = { oid: string; kind: string; value: string; durationMs: number };
type QueryRequest = {
  id: string; host: string; port: number; credentials: Credentials; oid: string;
  operation: "get" | "getNext" | "walk" | "bulkWalk"; timeoutMs: number; retries: number;
};
export type ProfileSettings = { name: string; host: string; port: number; timeoutMs: number; retries: number };
export type HistoryItem = { host: string; oid: string; operation: string; resultCount: number };
export const backend = {
  saveProfile: (settings: ProfileSettings, credentials: Credentials) => invoke<void>("save_profile", { settings, credentials }),
  profileNames: () => invoke<string[]>("profile_names"),
  loadProfile: (name: string) => invoke<[ProfileSettings, Credentials]>("load_profile", { name }),
  recordQuery: (item: HistoryItem) => invoke<void>("record_query", { item }),
  history: () => invoke<[string, HistoryItem][]>("query_history"),
  query: (request: QueryRequest, append: (rows: Binding[]) => void) => {
    const onRows = new Channel<Binding[]>(append);
    return invoke<number>("query_device", { request, onRows });
  },
  cancel: (id: string) => invoke<void>("cancel_query", { id }),
  exportResults: (format: string, content: string) => invoke<boolean>("export_results", {format, content}),
  health: () => invoke<HealthStatus>("health_check"),
  bundledModules: () => invoke<MibModuleSummary[]>("list_bundled_modules"),
  loadMibFiles: (paths: string[]) => invoke<MibSnapshot>("load_mib_files", { paths }),
};
