export type MibNode = {
  name: string;
  oid: string;
  kind: "folder" | "scalar" | "table";
  access?: "read-only" | "read-write" | "not-accessible";
  syntax?: string;
  description?: string;
  children?: MibNode[];
};

export type QueryResult = {
  oid: string;
  name: string;
  type: string;
  value: string;
  durationMs: number;
};
