import type { MibNode } from "./types";

export function searchNodes(nodes: MibNode[], term: string): MibNode[] {
  const query = term.trim().toLowerCase();
  const hits: MibNode[] = [];
  function visit(node: MibNode) {
    if ([node.name, node.oid, node.description, node.module].some(field => field?.toLowerCase().includes(query))) hits.push(node);
    node.children?.forEach(visit);
  }
  nodes.forEach(visit);
  return hits;
}

export function csv(rows: string[][]): string {
  return rows.map(row => row.map(value => {
    const safe = /^[=+@\-\t\r]/.test(value) ? "'" + value : value;
    return '"' + safe.replaceAll('"', '""') + '"';
  }).join(",")).join("\r\n");
}
