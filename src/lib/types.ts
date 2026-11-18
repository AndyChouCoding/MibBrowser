export type MibNode = {
  name: string;
  oid: string;
  kind: string;
  module?: string | null;
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
