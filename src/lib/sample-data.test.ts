import { describe, expect, it } from "vitest";
import { mibTree, sampleResults } from "./sample-data";
import type { MibNode } from "./types";

function flatten(nodes: MibNode[]): MibNode[] {
  return nodes.flatMap((node) => [node, ...flatten(node.children ?? [])]);
}

describe("MIB preview data", () => {
  const nodes = flatten(mibTree);

  it("uses unique numeric OIDs", () => {
    const oids = nodes.map((node) => node.oid);
    expect(new Set(oids).size).toBe(oids.length);
    expect(oids.every((oid) => /^\d+(\.\d+)*$/.test(oid))).toBe(true);
  });

  it("keeps child OIDs under their parent", () => {
    for (const node of nodes) {
      for (const child of node.children ?? []) {
        expect(child.oid.startsWith(`${node.oid}.`)).toBe(true);
      }
    }
  });

  it("maps sample instance results to known scalar nodes", () => {
    const scalarOids = new Set(
      nodes.filter((node) => node.kind === "scalar").map((node) => `${node.oid}.0`),
    );

    expect(sampleResults.every((result) => scalarOids.has(result.oid))).toBe(true);
  });
});
