import { expect, it } from "vitest";
import { csv, searchNodes } from "./browse";
import { mibTree } from "./sample-data";

it("finds nested scalars by numeric OID or case-insensitive description", () => {
  expect(searchNodes(mibTree, "ADMINISTRATIVELY")[0].name).toBe("sysName");
  expect(searchNodes(mibTree, "1.3.6.1.2.1.1.5")[0].name).toBe("sysName");
});

it("quotes multiline CSV values, doubles quotes and neutralizes formulas", () => {
  expect(csv([['a,"b"\nc', '=1+1']])).toBe('"a,""b""\nc","\'=1+1"');
});
