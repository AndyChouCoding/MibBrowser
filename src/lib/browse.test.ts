import { expect, it } from "vitest";
import { csv, enabledTree, searchNodes } from "./browse";
import { mibTree } from "./sample-data";

it("finds nested scalars by numeric OID or case-insensitive description", () => {
  expect(searchNodes(mibTree, "ADMINISTRATIVELY")[0].name).toBe("sysName");
  expect(searchNodes(mibTree, "1.3.6.1.2.1.1.5")[0].name).toBe("sysName");
});

it("quotes multiline CSV values, doubles quotes and neutralizes formulas", () => {
  expect(csv([['a,"b"\nc', '=1+1']])).toBe('"a,""b""\nc","\'=1+1"');
});

it("keeps children from enabled modules when their parent module is disabled", () => {
  const child = {name: "child", oid: "1.2", kind: "scalar", module: "B"};
  const parent = {name: "parent", oid: "1", kind: "folder", module: "A", children: [child]};
  const visible = enabledTree([parent], new Set(["A"]));
  expect(visible.map(node => node.name)).toEqual(["child"]);
  expect(searchNodes(visible, "parent")).toEqual([]);
  expect(parent.children).toEqual([child]);
});
