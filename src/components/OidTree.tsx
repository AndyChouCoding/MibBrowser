import { ChevronDown, ChevronRight, CircleDot, Table2 } from "lucide-react";
import { useState } from "react";
import type { MibNode } from "../lib/types";

type Props = {
  nodes: MibNode[];
  selectedOid: string;
  onSelect: (node: MibNode) => void;
  depth?: number;
};

export function OidTree({ nodes, selectedOid, onSelect, depth = 0 }: Props) {
  return (
    <div role={depth === 0 ? "tree" : "group"}>
      {nodes.map((node) => (
        <OidTreeItem
          key={node.oid}
          node={node}
          selectedOid={selectedOid}
          onSelect={onSelect}
          depth={depth}
        />
      ))}
    </div>
  );
}

type OidTreeItemProps = {
  node: MibNode;
  selectedOid: string;
  onSelect: (node: MibNode) => void;
  depth: number;
};

function OidTreeItem({ node, selectedOid, onSelect, depth }: OidTreeItemProps) {
  const [open, setOpen] = useState(depth < 5);
  const hasChildren = Boolean(node.children?.length);
  const Icon = node.kind === "table" ? Table2 : CircleDot;

  return (
    <div>
      <button
        className={`tree-row ${selectedOid === node.oid ? "is-selected" : ""}`}
        style={{ paddingLeft: `${12 + depth * 17}px` }}
        role="treeitem"
        aria-expanded={hasChildren ? open : undefined}
        aria-selected={selectedOid === node.oid}
        onKeyDown={event => {
          if (hasChildren && (event.key === "ArrowRight" || event.key === "ArrowLeft")) {
            event.preventDefault(); setOpen(event.key === "ArrowRight");
          }
        }}
        onClick={() => onSelect(node)}
        onDoubleClick={() => hasChildren && setOpen((value) => !value)}
      >
        <span
          className="tree-toggle"
          onClick={(event) => {
            event.stopPropagation();
            if (hasChildren) setOpen((value) => !value);
          }}
        >
          {hasChildren ? open ? <ChevronDown size={14} /> : <ChevronRight size={14} /> : <Icon size={12} />}
        </span>
        <span className="tree-name">{node.name}</span>
        <span className="tree-oid">{node.oid}</span>
      </button>
      {open && node.children ? (
        <OidTree nodes={node.children} selectedOid={selectedOid} onSelect={onSelect} depth={depth + 1} />
      ) : null}
    </div>
  );
}
