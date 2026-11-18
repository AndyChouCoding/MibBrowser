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
import { OidTree } from "./components/OidTree";
import { backend } from "./lib/backend";
import { mibTree, sampleResults } from "./lib/sample-data";
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
  const [tree, setTree] = useState(mibTree);
  const [queryTab, setQueryTab] = useState("Walk");
  const [connected, setConnected] = useState(false);
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

  async function importMibs() {
    const selection = await open({
      multiple: true,
      filters: [{ name: "MIB files", extensions: ["mib", "smi", "txt", "my"] }],
    });
    if (!selection) return;

    const paths = Array.isArray(selection) ? selection : [selection];
    setImportStatus("Parsing MIB files…");
    try {
      const snapshot = await backend.loadMibFiles(paths);
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
            <input defaultValue="192.168.10.1" aria-label="Target address" />
          </label>
          <label className="field compact">
            <span>Version</span>
            <button className="select-button">SNMP v2c <ChevronDown size={13} /></button>
          </label>
          <label className="field compact community-field">
            <span>Community</span>
            <input type="password" defaultValue="public" aria-label="Community" />
          </label>
          <button className={`connect-button ${connected ? "connected" : ""}`} onClick={() => setConnected(!connected)}>
            <span className="status-dot" />
            {connected ? "Connected" : "Connect"}
          </button>
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
            <button className="icon-button small" aria-label="Add MIB" onClick={importMibs}><Plus size={14} /></button>
          </div>
          <div className="search-box"><Search size={14} /><input placeholder="Filter modules" /></div>
          <div className="module-list">
            {modules.map((module) => (
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
          <div className="search-box"><Search size={14} /><input placeholder="Search name, OID, description" /></div>
          <div className="tree-scroll">
            <OidTree nodes={tree} selectedOid={selectedOid} onSelect={(node) => setSelectedOid(node.oid)} />
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
              <div><dt>Module</dt><dd>{selected.module ?? "SNMPv2-MIB"}</dd></div>
              <div><dt>Status</dt><dd>current</dd></div>
              <div><dt>Instance</dt><dd>{selected.kind === "scalar" ? ".0" : "—"}</dd></div>
            </dl>
            <div className="description"><span>Description</span><p>{selected.description ?? "A branch within the management information tree."}</p></div>
            <button className="primary-action"><Play size={14} /> Query this object</button>
          </div>
        </aside>
      </section>

      <section className="query-panel">
        <div className="query-header">
          <div className="tabs">
            {["Get", "Get next", "Walk", "Set", "History"].map((tab) => <button key={tab} className={queryTab === tab ? "active" : ""} onClick={() => setQueryTab(tab)}>{tab}</button>)}
          </div>
          <div className="query-actions"><span className="result-summary">3 objects · 55 ms</span><button className="secondary-button"><Download size={13} /> Export</button><button className="run-button"><Play size={13} /> Run {queryTab}</button></div>
        </div>
        <div className="query-input"><span>Root OID</span><code>SNMPv2-MIB::system</code><small>1.3.6.1.2.1.1</small></div>
        <div className="results-table">
          <div className="result-row table-head"><span>Name</span><span>OID</span><span>Type</span><span>Value</span><span>Time</span></div>
          {sampleResults.map((result) => <div className="result-row" key={result.oid}><span>{result.name}</span><code>{result.oid}</code><span className="type-value">{result.type}</span><span>{result.value}</span><span>{result.durationMs} ms</span></div>)}
        </div>
      </section>

      <footer><span><span className={`status-dot ${backendStatus === "ready" ? "online" : ""}`} /> Backend {backendStatus}</span><span>{connected ? "Session active" : "UDP · Timeout 2s · Retries 1"}</span></footer>
    </main>
  );
}
