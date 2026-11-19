export type MibNode = {
  name: string;
  oid: string;
  kind: string;
  module?: string | null;
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
