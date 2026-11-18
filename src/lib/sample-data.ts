import type { MibNode, QueryResult } from "./types";

export const mibTree: MibNode[] = [
  {
    name: "iso",
    oid: "1",
    kind: "folder",
    children: [
      {
        name: "org",
        oid: "1.3",
        kind: "folder",
        children: [
          {
            name: "internet",
            oid: "1.3.6.1",
            kind: "folder",
            children: [
              {
                name: "mib-2",
                oid: "1.3.6.1.2.1",
                kind: "folder",
                children: [
                  {
                    name: "system",
                    oid: "1.3.6.1.2.1.1",
                    kind: "folder",
                    children: [
                      {
                        name: "sysDescr",
                        oid: "1.3.6.1.2.1.1.1",
                        kind: "scalar",
                        access: "read-only",
                        syntax: "DisplayString",
                        description: "A textual description of the managed entity.",
                      },
                      {
                        name: "sysUpTime",
                        oid: "1.3.6.1.2.1.1.3",
                        kind: "scalar",
                        access: "read-only",
                        syntax: "TimeTicks",
                        description: "Time since the network management portion was initialized.",
                      },
                      {
                        name: "sysName",
                        oid: "1.3.6.1.2.1.1.5",
                        kind: "scalar",
                        access: "read-write",
                        syntax: "DisplayString",
                        description: "The administratively assigned name for this managed node.",
                      },
                    ],
                  },
                  { name: "interfaces", oid: "1.3.6.1.2.1.2", kind: "table" },
                ],
              },
            ],
          },
        ],
      },
    ],
  },
];

export const sampleResults: QueryResult[] = [
  {
    oid: "1.3.6.1.2.1.1.1.0",
    name: "SNMPv2-MIB::sysDescr.0",
    type: "OCTET STRING",
    value: "Network appliance / lab",
    durationMs: 18,
  },
  {
    oid: "1.3.6.1.2.1.1.3.0",
    name: "SNMPv2-MIB::sysUpTime.0",
    type: "TimeTicks",
    value: "14 days, 07:32:11.42",
    durationMs: 19,
  },
  {
    oid: "1.3.6.1.2.1.1.5.0",
    name: "SNMPv2-MIB::sysName.0",
    type: "OCTET STRING",
    value: "core-switch-01",
    durationMs: 18,
  },
];
