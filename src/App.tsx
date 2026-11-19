import { open } from "@tauri-apps/plugin-dialog";
import {
  Activity,
  BookOpen,
  Box,
  ChevronDown,
  Copy,
  Download,
  Moon,
  MoreHorizontal,
  Play,
  Plus,
  Search,
  Settings,
  ShieldCheck,
  Sun,
  TerminalSquare,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { csv, searchNodes } from "./lib/browse";
import { OidTree } from "./components/OidTree";
import { backend } from "./lib/backend";
import { mibTree } from "./lib/sample-data";
import type { MibNode } from "./lib/types";

const previewModules = [
  { name: "SNMPv2-MIB", nodes: 72, active: true, source: "preview" },
  { name: "IF-MIB", nodes: 107, active: true, source: "preview" },
  { name: "IP-MIB", nodes: 184, active: true, source: "preview" },
  { name: "TCP-MIB", nodes: 64, active: false, source: "preview" },
];

function findNode(nodes: MibNode[], oid: string): MibNode | undefined {
  for (const node of nodes) {
    if (node.oid === oid) return node;
    const match = node.children ? findNode(node.children, oid) : undefined;
    if (match) return match;
  }
}

export function App() {
  const [theme, setTheme] = useState<"dark" | "light">("dark");
  const [selectedOid, setSelectedOid] = useState("1.3.6.1.2.1.1.5");
  const [moduleFilter, setModuleFilter] = useState("");
  const [search, setSearch] = useState("");
  const [pathsLoaded, setPathsLoaded] = useState<string[]>([]);
  const [importing, setImporting] = useState(false);
  const [format, setFormat] = useState("csv");
  const [tree, setTree] = useState(mibTree);
  const hits = useMemo(() => search.trim() ? searchNodes(tree, search) : [], [tree, search]);
  const [queryTab, setQueryTab] = useState("Walk");
  const [host, setHost] = useState("");
  const [port, setPort] = useState("161");
  const [community, setCommunity] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("Ready");
  const [rows, setRows] = useState<Awaited<ReturnType<typeof backend.query>>>([]);
  const [modules, setModules] = useState(previewModules);
  const [backendStatus, setBackendStatus] = useState<"checking" | "ready" | "unavailable">("checking");
  const [importStatus, setImportStatus] = useState<string | null>(null);
  const selected = useMemo(
    () => findNode(tree, selectedOid) ?? tree[0] ?? mibTree[0],
    [selectedOid, tree],
  );

  useEffect(() => {
    Promise.all([backend.health(), backend.bundledModules()])
      .then(([, bundledModules]) => {
        setModules(
          bundledModules.map((module) => ({
            name: module.name,
            nodes: 0,
            active: module.enabled,
            source: module.source,
          })),
        );
        setBackendStatus("ready");
      })
      .catch(() => setBackendStatus("unavailable"));
  }, []);

  async function runQuery() {
    if (busy) return;
    const operation = queryTab === "Get" ? "get" : queryTab === "Get next" ? "getNext" : "walk";
    const oid = selected.oid + (operation === "get" && selected.kind === "scalar" ? ".0" : "");
    setBusy(true);
    setRows([]);
    setStatus(`Querying ${host}:${port}…`);
    try {
      const result = await backend.query(host, Number(port), community, oid, operation);
      setRows(result);
      setStatus(`${host}:${port} · ${oid} · ${result.length} results`);
    } catch (error) {
      setStatus(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function importMibs() {
    setImporting(true);
    try {
    const selection = await open({
      multiple: true,
      filters: [{ name: "MIB files", extensions: ["mib", "smi", "txt", "my"] }],
    });
    if (!selection) return;

    const paths = Array.isArray(selection) ? selection : [selection];
    setImportStatus("Parsing MIB files…");
      const merged = [...new Set([...pathsLoaded, ...paths])];
      const snapshot = await backend.loadMibFiles(merged);
      setPathsLoaded(merged);
      setModules(
        snapshot.modules.map((module) => ({
          name: module.name,
          nodes: 0,
          active: module.enabled,
          source: module.source,
        })),
      );
      setTree(snapshot.roots);
      if (snapshot.roots[0]) setSelectedOid(snapshot.roots[0].oid);
      setImportStatus(`${snapshot.nodeCount} OID nodes loaded`);
    } catch (error) {
      setImportStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setImporting(false);
    }
  }

  return (
    <main className="app" data-theme={theme}>
      <header className="topbar">
        <div className="brand">
          <div className="brand-mark"><Activity size={17} /></div>
          <span>MIB Browser</span>
          <span className="version">0.1</span>
        </div>
        <div className="connection-bar">
          <label className="field compact target-field">
            <span>Target</span>
            <input disabled={busy} value={host} placeholder="Device address" onChange={(e) => setHost(e.target.value)} aria-label="Target address" />
          </label>
          <label className="field compact">
            <span>Version</span>
            <button className="select-button">SNMP v2c <ChevronDown size={13} /></button>
          </label>
          <label className="field compact community-field">
            <span>Community</span>
            <input type="password" disabled={busy} value={community} onChange={(e) => setCommunity(e.target.value)} aria-label="Community" />
          </label>
          <label className="field compact"><span>Port</span><input type="number" min="1" max="65535" value={port} disabled={busy} onChange={(e) => setPort(e.target.value)} /></label>
        </div>
        <div className="toolbar">
          <button className="icon-button" aria-label="Toggle theme" onClick={() => setTheme(theme === "dark" ? "light" : "dark")}>
            {theme === "dark" ? <Moon size={16} /> : <Sun size={16} />}
          </button>
          <button className="icon-button" aria-label="Settings"><Settings size={16} /></button>
        </div>
      </header>

      <section className="workspace">
        <aside className="panel modules-panel">
          <div className="panel-heading">
            <div><BookOpen size={14} /><strong>MIB modules</strong><span className="count">{modules.length}</span></div>
            <button className="icon-button small" aria-label="Add MIB" disabled={importing} onClick={importMibs}><Plus size={14} /></button>
          </div>
          <div className="search-box"><Search size={14} /><input value={moduleFilter} onChange={e => setModuleFilter(e.target.value)} placeholder="Filter modules" /></div>
          <div className="module-list">
            {modules.filter(m => m.name.toLowerCase().includes(moduleFilter.trim().toLowerCase())).map((module) => (
              <button className="module-row" key={module.name}>
                <span className={`module-status ${module.active ? "active" : ""}`} />
                <span><strong>{module.name}</strong><small>{module.nodes > 0 ? `${module.nodes} objects` : module.source}</small></span>
                <MoreHorizontal size={14} />
              </button>
            ))}
          </div>
          <div className="module-footer"><ShieldCheck size={13} /> {importStatus ?? `${modules.filter((module) => module.active).length} modules active`}</div>
        </aside>

        <section className="panel tree-panel">
          <div className="panel-heading">
            <div><Box size={14} /><strong>OID tree</strong></div>
            <span className="subtle">SMIv2</span>
          </div>
          <div className="search-box"><Search size={14} /><input value={search} onChange={e => setSearch(e.target.value)} placeholder="Search name, OID, description" /></div>
          <div className="tree-scroll">
            {search.trim() ? (hits.length ? hits.slice(0, 200).map(node => <button key={node.oid} className="tree-row" onClick={() => setSelectedOid(node.oid)}>{node.name} · {node.oid}</button>) : <p>No matching objects</p>) : <OidTree nodes={tree} selectedOid={selectedOid} onSelect={(node) => setSelectedOid(node.oid)} />}
          </div>
        </section>

        <aside className="panel details-panel">
          <div className="panel-heading"><div><TerminalSquare size={14} /><strong>Object details</strong></div><button className="icon-button small"><Copy size={13} /></button></div>
          <div className="details-content">
            <div className="object-icon">{selected.kind === "table" ? "T" : "O"}</div>
            <h2>{selected.name}</h2>
            <code>{selected.oid}</code>
            <div className="tag-row"><span className="tag blue">{selected.syntax ?? "OBJECT IDENTIFIER"}</span><span className="tag green">{selected.access ?? "branch"}</span></div>
            <dl className="metadata">
              <div><dt>Module</dt><dd>{selected.module ?? "—"}</dd></div>
              <div><dt>Status</dt><dd>{selected.status ?? "—"}</dd></div>
              <div><dt>Instance</dt><dd>{selected.kind === "scalar" ? ".0" : "—"}</dd></div>
            </dl>
            <div className="description"><span>Description</span><p>{selected.description ?? "A branch within the management information tree."}</p></div>
            <button className="primary-action" disabled={busy || !host || !community} onClick={runQuery}><Play size={14} /> Query this object</button>
          </div>
        </aside>
      </section>

      <section className="query-panel">
        <div className="query-header">
          <div className="tabs">
            {["Get", "Get next", "Walk", "Set", "History"].map((tab) => <button key={tab} disabled={busy || tab === "Set" || tab === "History"} className={queryTab === tab ? "active" : ""} onClick={() => setQueryTab(tab)}>{tab}</button>)}
          </div>
          <div className="query-actions"><span className="result-summary">{status}</span><select aria-label="Export format" value={format} onChange={e => setFormat(e.target.value)}><option value="csv">CSV</option><option value="json">JSON</option></select><button className="secondary-button" disabled={!rows.length || busy} onClick={async () => {
            try {
              const content = format === "json" ? JSON.stringify(rows, null, 2) : csv([["OID","Type","Value","Time (ms)"], ...rows.map(row => [row.oid,row.kind,row.value,String(row.durationMs)])]);
              if (await backend.exportResults(format, content)) setStatus("Results exported");
            } catch (error) { setStatus(String(error)); }
          }}><Download size={13} /> Export</button><button className="run-button" disabled={busy || !host || !community} onClick={runQuery}><Play size={13} /> Run {queryTab}</button></div>
        </div>
        <div className="query-input"><span>Root OID</span><code>{selected.name}</code><small>{selected.oid}{queryTab === "Get" && selected.kind === "scalar" ? ".0" : ""}</small></div>
        <div className="results-table">
          <div className="result-row table-head"><span>Name</span><span>OID</span><span>Type</span><span>Value</span><span>Time</span></div>
          {rows.map((result) => <div className="result-row" key={result.oid}><span>{findNode(tree, result.oid)?.name ?? findNode(tree, result.oid.replace(/\.0$/, ""))?.name ?? "—"}</span><code>{result.oid}</code><span className="type-value">{result.kind}</span><span>{result.value}</span><span>{result.durationMs} ms</span></div>)}
        </div>
      </section>

      <footer><span><span className={`status-dot ${backendStatus === "ready" ? "online" : ""}`} /> Backend {backendStatus}</span><span>SNMP v2c · UDP</span></footer>
    </main>
  );
}
