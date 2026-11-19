export type MibNode = {
  name: string;
  oid: string;
  kind: string;
  module?: string | null;
  indexes?: string[];
  enumeration?: string[];
  access?: string | null;
  syntax?: string | null;
  status?: string | null;
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
