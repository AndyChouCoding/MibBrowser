import { version as appVersion } from "../package.json";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Activity,
  BookOpen,
  Box,
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
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { isTauri } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { csv, enabledTree, searchNodes } from "./lib/browse";
import { OidTree } from "./components/OidTree";
import { backend, type Binding, type Credentials, type HistoryItem, type SetPreview } from "./lib/backend";
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
  const [theme, setTheme] = useState<"dark" | "light">(() => { try { return localStorage.getItem("theme") === "light" ? "light" : "dark"; } catch { return "dark"; } });
  const [selectedOid, setSelectedOid] = useState("1.3.6.1.2.1.1.5");
  const [manualOid, setManualOid] = useState("");
  const searchInput = useRef<HTMLInputElement>(null);
  const [moduleFilter, setModuleFilter] = useState("");
  const [search, setSearch] = useState("");
  const [pathsLoaded, setPathsLoaded] = useState<string[]>([]);
  const pendingImports = useRef<string[]>([]);
  const importLock = useRef(false);
  const [importing, setImporting] = useState(false);
  const [format, setFormat] = useState("csv");
  const [allNodes, setTree] = useState(mibTree);
  const [disabledModules, setDisabledModules] = useState<Set<string>>(() => {
    try {
      const names: unknown = JSON.parse(localStorage.getItem("workspace.disabledModules") ?? "[]");
      if (Array.isArray(names) && names.every(name => typeof name === "string")) return new Set(names);
    } catch { /* Start with every module enabled when storage is unavailable. */ }
    return new Set();
  });
  const tree = useMemo(() => enabledTree(allNodes, disabledModules), [allNodes, disabledModules]);
  const hits = useMemo(() => search.trim() ? searchNodes(tree, search) : [], [tree, search]);
  const [queryTab, setQueryTab] = useState("Walk");
  const [host, setHost] = useState("");
  const [port, setPort] = useState("161");
  const [version, setVersion] = useState<"v1" | "v2c" | "v3">("v2c");
  const [level, setLevel] = useState<"noAuthNoPriv" | "authNoPriv" | "authPriv">("authPriv");
  const [username, setUsername] = useState("");
  const [context, setContext] = useState("");
  const [authPassword, setAuthPassword] = useState("");
  const [privacyPassword, setPrivacyPassword] = useState("");
  const [community, setCommunity] = useState("");
  const queryId = useRef("");
  const [timeoutMs, setTimeoutMs] = useState(2000);
  const [retries, setRetries] = useState(1);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [profileName, setProfileName] = useState("");
  const [profileNames, setProfileNames] = useState<string[]>([]);
  const [history, setHistory] = useState<[string, HistoryItem][]>([]);
  const [instance, setInstance] = useState("");
  const [newValue, setNewValue] = useState("");
  const [setPreview, setSetPreview] = useState<SetPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("Ready");
  const [rows, setRows] = useState<Binding[]>([]);
  const [modules, setModules] = useState(previewModules);
  const [backendStatus, setBackendStatus] = useState<"checking" | "ready" | "unavailable">("checking");
  const [importStatus, setImportStatus] = useState<string | null>(null);
  const selected = useMemo(
    () => findNode(tree, selectedOid) ?? tree[0] ?? mibTree[0],
    [selectedOid, tree],
  );

  const loadWorkspace = useCallback(async (paths: string[]) => {
    if (importLock.current) return;
    importLock.current = true;
    setImporting(true);
    setImportStatus("Parsing MIB files…");
    try {
      const snapshot = await backend.loadMibFiles(paths);
      setModules(snapshot.modules.map(module => ({
        name: module.name, nodes: 0, active: module.enabled, source: module.source,
      })));
      setTree(snapshot.roots);
      pendingImports.current = [];
      setPathsLoaded(paths);
      if (snapshot.roots[0]) setSelectedOid(snapshot.roots[0].oid);
      setImportStatus(`${snapshot.nodeCount} OID nodes loaded`);
      try {
        localStorage.setItem("workspace.mibPaths", JSON.stringify(paths));
      } catch {
        setImportStatus("MIB files loaded. Workspace paths could not be saved.");
      }
    } catch (error) {
      pendingImports.current = String(error).includes("unresolved import") ? paths : [];
      setImportStatus(String(error));
    } finally {
      importLock.current = false;
      setImporting(false);
    }
  }, []);

  useEffect(() => {
    let active = true;
    Promise.all([backend.health(), backend.loadMibFiles([])])
      .then(([, foundation]) => {
        if (!active) return;
        setTree(foundation.roots);
        if (foundation.roots[0]) setSelectedOid(foundation.roots[0].oid);
        setModules(foundation.modules.map(module => ({
          name: module.name, nodes: 0, active: module.enabled, source: module.source,
        })));
        setBackendStatus("ready");
        try {
          const saved: unknown = JSON.parse(localStorage.getItem("workspace.mibPaths") ?? "[]");
          if (Array.isArray(saved) && saved.length > 0 && saved.every(path => typeof path === "string")) {
            void loadWorkspace(saved);
          }
        } catch {
          setImportStatus("Unable to restore saved MIB paths");
        }
      })
      .catch(() => { if (active) setBackendStatus("unavailable"); });
    return () => { active = false; };
  }, [loadWorkspace]);

  useEffect(() => {
    if (!isTauri() || backendStatus !== "ready") return;
    let active = true;
    const listener = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (active && payload.type === "drop") {
        void loadWorkspace([...new Set([...pathsLoaded, ...pendingImports.current, ...payload.paths])]);
      }
    }).catch(error => { if (active) setImportStatus(String(error)); });
    return () => { active = false; void listener.then(stop => stop?.()); };
  }, [backendStatus, pathsLoaded, loadWorkspace]);

  function toggleModule(name: string) {
    const next = new Set(disabledModules);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    setDisabledModules(next);
    try {
      localStorage.setItem("workspace.disabledModules", JSON.stringify([...next]));
    } catch { setImportStatus("Module selection could not be saved"); }
  }

  useEffect(() => {
    function focusSearch(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "f") {
        event.preventDefault(); searchInput.current?.focus();
      }
    }
    window.addEventListener("keydown", focusSearch);
    return () => window.removeEventListener("keydown", focusSearch);
  }, []);

  function selectOid(oid: string) {
    setSelectedOid(oid); setManualOid(""); setInstance("");
  }

  function currentCredentials(): Credentials {
    if (version !== "v3") return { version, community };
    return { version, username, context, level,
      authPassword: level === "noAuthNoPriv" ? "" : authPassword,
      privacyPassword: level === "authPriv" ? privacyPassword : "" };
  }

  async function saveProfile() {
    try {
      await backend.saveProfile({ name: profileName, host, port: Number(port), timeoutMs, retries }, currentCredentials());
      setProfileNames(await backend.profileNames());
      setStatus("Profile saved");
    } catch (error) { setStatus(String(error)); }
  }

  async function loadProfile(name: string) {
    if (!name) return;
    setBusy(true);
    try {
      const [profile, credentials] = await backend.loadProfile(name);
      setProfileName(profile.name); setHost(profile.host); setPort(String(profile.port));
      setTimeoutMs(profile.timeoutMs); setRetries(profile.retries); setVersion(credentials.version);
      setCommunity(credentials.version === "v3" ? "" : credentials.community);
      setAuthPassword(credentials.version === "v3" ? credentials.authPassword : "");
      setPrivacyPassword(credentials.version === "v3" ? credentials.privacyPassword : "");
      if (credentials.version === "v3") {
        setContext(credentials.context); setLevel(credentials.level); setUsername(credentials.username);
      } else if (credentials.version === "v1" && queryTab === "Bulk walk") setQueryTab("Walk");
      setStatus("Profile loaded");
    } catch (error) { setStatus(String(error)); }
    finally { setBusy(false); }
  }

  async function runQuery() {
    if (busy || !tree.length || queryTab === "History") return;
    if (queryTab === "Set") {
      setBusy(true);
      try {
        const preview = await backend.prepareSet({ host, port: Number(port), credentials: currentCredentials(), paths: pathsLoaded,
          objectOid: selected.oid, instance: selected.kind === "scalar" ? "0" : instance, input: newValue, timeoutMs });
        setSetPreview(preview);
      } catch (error) { setStatus(String(error)); setBusy(false); }
      return;
    }
    const operation = queryTab === "Get" ? "get" : queryTab === "Get next" ? "getNext" : queryTab === "Bulk walk" ? "bulkWalk" : "walk";
    const oid = manualOid.trim() || selected.oid + (operation === "get" && selected.kind === "scalar" ? ".0" : "");
    const id = crypto.randomUUID();
    queryId.current = id;
    setBusy(true);
    setRows([]);
    setStatus(`Querying ${host}:${port}…`);
    try {
      const result = await backend.query({ id, host, port: Number(port), credentials: currentCredentials(), oid, operation, timeoutMs, retries }, batch => {
        if (queryId.current === id) setRows(previous => [...previous, ...batch]);
      });
      setStatus(`${host}:${port} · ${oid} · ${result} results`);
      try { await backend.recordQuery({ host, oid, operation, resultCount: result }); }
      catch { setStatus(`${result} results · history could not be saved`); }
    } catch (error) {
      setStatus(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function importMibs() {
    try {
      const selection = await open({
        multiple: true,
        filters: [{ name: "MIB files", extensions: ["mib", "smi", "txt", "my"] }],
      });
      if (!selection) return;
      const paths = Array.isArray(selection) ? selection : [selection];
      await loadWorkspace([...new Set([...pathsLoaded, ...pendingImports.current, ...paths])]);
    } catch (error) {
      setImportStatus(String(error));
    }
  }

  return (
    <main className="app" data-theme={theme}>
      <header className="topbar">
        <div className="brand">
          <div className="brand-mark"><Activity size={17} /></div>
          <span>MIB Browser</span>
          <span className="version">{appVersion}</span>
        </div>
        <div className="connection-bar">
          <label className="field compact target-field">
            <span>Target</span>
            <input disabled={busy} value={host} placeholder="Device address" onChange={(e) => setHost(e.target.value)} aria-label="Target address" />
          </label>
          <label className="field compact">
            <span>Version</span>
            <select disabled={busy} value={version} onChange={e => { setVersion(e.target.value as typeof version); if (e.target.value === "v1" && queryTab === "Bulk walk") setQueryTab("Walk"); }}><option>v1</option><option>v2c</option><option>v3</option></select>
          </label>
          {version === "v3" ? <details className="usm"><summary>USM credentials</summary><fieldset disabled={busy}>
            <label>User<input value={username} onChange={e => setUsername(e.target.value)} /></label>
            <label>Context<input value={context} onChange={e => setContext(e.target.value)} /></label>
            <select aria-label="Security level" value={level} onChange={e => setLevel(e.target.value as typeof level)}><option>noAuthNoPriv</option><option>authNoPriv</option><option>authPriv</option></select>
            {level !== "noAuthNoPriv" && <label>SHA-256 password<input type="password" value={authPassword} onChange={e => setAuthPassword(e.target.value)} /></label>}
            {level === "authPriv" && <label>AES-128 password<input type="password" value={privacyPassword} onChange={e => setPrivacyPassword(e.target.value)} /></label>}
          </fieldset></details> : <label className="field compact community-field">
            <span>Community</span>
            <input type="password" disabled={busy} value={community} onChange={(e) => setCommunity(e.target.value)} aria-label="Community" />
          </label>
          }<label className="field compact"><span>Port</span><input type="number" min="1" max="65535" value={port} disabled={busy} onChange={(e) => setPort(e.target.value)} /></label>
        </div>
        <div className="toolbar">
          <button className="icon-button" aria-label="Toggle theme" onClick={() => {
            const next = theme === "dark" ? "light" : "dark"; setTheme(next);
            try { localStorage.setItem("theme", next); } catch { setStatus("Theme could not be saved"); }
          }}>
            {theme === "dark" ? <Moon size={16} /> : <Sun size={16} />}
          </button>
          <button className="icon-button" aria-label="Settings" aria-expanded={settingsOpen} onClick={() => { setSettingsOpen(!settingsOpen); if (!settingsOpen) void backend.profileNames().then(setProfileNames).catch(error => setStatus(String(error))); }}><Settings size={16} /></button>
        </div>
      </header>

      {settingsOpen && <fieldset disabled={busy} className="query-settings">
        <label>Load profile <select value="" onChange={e => void loadProfile(e.target.value)}><option value="">Select profile</option>{profileNames.map(name => <option key={name}>{name}</option>)}</select></label>
        <label>Profile name <input value={profileName} onChange={e => setProfileName(e.target.value)} /></label>
        <button disabled={!profileName.trim()} onClick={saveProfile}>Save new profile</button>
        <label>Timeout (ms) <input type="number" min="100" max="120000" value={timeoutMs} onChange={e => setTimeoutMs(Number(e.target.value))} /></label>
        <label>Retries <input type="number" min="0" max="10" value={retries} onChange={e => setRetries(Number(e.target.value))} /></label>
      </fieldset>}
      <section className="workspace">
        <aside className="panel modules-panel">
          <div className="panel-heading">
            <div><BookOpen size={14} /><strong>MIB modules</strong><span className="count">{modules.length}</span></div>
            <button className="secondary-button" disabled={importing || !pathsLoaded.length} onClick={() => loadWorkspace(pathsLoaded)}>Reload</button>
            <button className="icon-button small" aria-label="Add MIB" disabled={importing || backendStatus !== "ready"} onClick={importMibs}><Plus size={14} /></button>
          </div>
          <div className="search-box"><Search size={14} /><input value={moduleFilter} onChange={e => setModuleFilter(e.target.value)} placeholder="Filter modules" /></div>
          <div className="module-list">
            {modules.filter(m => m.name.toLowerCase().includes(moduleFilter.trim().toLowerCase())).map((module) => (
              <button className="module-row" key={module.name} aria-pressed={!disabledModules.has(module.name)} onClick={() => toggleModule(module.name)}>
                <span className={`module-status ${!disabledModules.has(module.name) ? "active" : ""}`} />
                <span><strong>{module.name}</strong><small>{module.nodes > 0 ? `${module.nodes} objects` : module.source}</small></span>
                <MoreHorizontal size={14} />
              </button>
            ))}
          </div>
          <div className="module-footer"><ShieldCheck size={13} /> {importStatus ?? `${modules.filter(module => !disabledModules.has(module.name)).length} modules active`}</div>
        </aside>

        <section className="panel tree-panel">
          <div className="panel-heading">
            <div><Box size={14} /><strong>OID tree</strong></div>
            <span className="subtle">SMIv2</span>
          </div>
          <div className="search-box"><Search size={14} /><input ref={searchInput} value={search} onChange={e => setSearch(e.target.value)} placeholder="Search name, OID, description" /></div>
          <div className="tree-scroll">
            {search.trim() ? (hits.length ? hits.slice(0, 200).map(node => <button key={node.oid} className="tree-row" onClick={() => selectOid(node.oid)}>{node.name} · {node.oid}</button>) : <p>No matching objects</p>) : <OidTree nodes={tree} selectedOid={selectedOid} onSelect={(node) => selectOid(node.oid)} />}
          </div>
        </section>

        <aside className="panel details-panel">
          <div className="panel-heading"><div><TerminalSquare size={14} /><strong>Object details</strong></div><button className="icon-button small" aria-label="Copy OID" disabled={!tree.length} onClick={() => navigator.clipboard.writeText(selected.oid).then(() => setStatus("OID copied")).catch(() => setStatus("Clipboard unavailable"))}><Copy size={13} /></button></div>
          {tree.length ? <div className="details-content">
            <div className="object-icon">{selected.kind === "table" ? "T" : "O"}</div>
            <h2>{selected.name}</h2>
            <code>{selected.oid}</code>
            <div className="tag-row"><span className="tag blue">{selected.syntax ?? "OBJECT IDENTIFIER"}</span><span className="tag green">{selected.access ?? "branch"}</span></div>
            <dl className="metadata">
              <div><dt>Module</dt><dd>{selected.module ?? "—"}</dd></div>
              {Boolean(selected.indexes?.length) && <div><dt>Index</dt><dd>{selected.indexes?.join(", ")}</dd></div>}
              {Boolean(selected.enumeration?.length) && <div><dt>Enumeration</dt><dd>{selected.enumeration?.join(", ")}</dd></div>}
              <div><dt>Status</dt><dd>{selected.status ?? "—"}</dd></div>
              <div><dt>Instance</dt><dd>{selected.kind === "scalar" ? ".0" : "—"}</dd></div>
            </dl>
            <div className="description"><span>Description</span><p>{selected.description ?? "A branch within the management information tree."}</p></div>
            <button className="primary-action" disabled={busy || queryTab === "History" || !tree.length || !host || !(version === "v3" ? username : community)} onClick={runQuery}><Play size={14} /> Query this object</button>
          </div> : <p>No enabled objects</p>}
        </aside>
      </section>

      <section className="query-panel">
        <div className="query-header">
          <div className="tabs">
            {["Get", "Get next", "Walk", "Bulk walk", "Set", "History"].map((tab) => <button key={tab} disabled={busy || (tab === "Bulk walk" && version === "v1")} className={queryTab === tab ? "active" : ""} onClick={() => { setQueryTab(tab); if (tab === "History") void backend.history().then(setHistory).catch(error => setStatus(String(error))); }}>{tab}</button>)}
          </div>
          <div className="query-actions"><span className="result-summary">{status}</span>{busy && queryTab !== "Set" && <button onClick={() => backend.cancel(queryId.current).catch(error => setStatus(String(error)))}>Cancel</button>}<select aria-label="Export format" value={format} onChange={e => setFormat(e.target.value)}><option value="csv">CSV</option><option value="json">JSON</option></select><button className="secondary-button" disabled={busy || (queryTab === "History" ? !history.length : !rows.length)} onClick={async () => {
            try {
              const content = queryTab === "History"
                ? (format === "json" ? JSON.stringify(history.map(([time, item]) => ({time, ...item})), null, 2) : csv([["Time (UTC)", "Target", "OID", "Operation", "Results"], ...history.map(([time, item]) => [time, item.host, item.oid, item.operation, String(item.resultCount)])]))
                : format === "json" ? JSON.stringify(rows, null, 2) : csv([["OID","Type","Value","Time (ms)"], ...rows.map(row => [row.oid,row.kind,row.value,String(row.durationMs)])]);
              if (await backend.exportResults(format, content)) setStatus("Results exported");
            } catch (error) { setStatus(String(error)); }
          }}><Download size={13} /> Export</button><button className="run-button" disabled={busy || queryTab === "History" || !tree.length || !host || !(version === "v3" ? username : community)} onClick={runQuery}><Play size={13} /> Run {queryTab}</button></div>
        </div>
        {tree.length > 0 && <div className="query-input"><span>Root OID</span><code>{selected.name}</code>{queryTab === "Set" ? <small>{selected.oid}</small> : <input aria-label="Query OID" disabled={busy} value={manualOid} placeholder={selected.oid + (queryTab === "Get" && selected.kind === "scalar" ? ".0" : "")} onChange={e => setManualOid(e.target.value)} />}</div>}
        {queryTab === "Set" && <div className="query-input">
          <label>Instance suffix <input disabled={busy || selected.kind === "scalar"} value={selected.kind === "scalar" ? "0" : instance} onChange={e => setInstance(e.target.value)} /></label>
          <label>New value <input disabled={busy} value={newValue} onChange={e => setNewValue(e.target.value)} /></label>
        </div>}
        {setPreview && <dialog open className="set-review" aria-label="Confirm SNMP SET">
          <h2>Confirm SET</h2><dl><dt>Device</dt><dd>{setPreview.target}</dd><dt>Instance OID</dt><dd>{setPreview.oid}</dd><dt>Current value</dt><dd>{setPreview.current}</dd><dt>New value</dt><dd>{setPreview.proposed}</dd></dl>
          {[false, true].map(confirm => <button key={String(confirm)} onClick={async () => {
            setSetPreview(null);
            try { setStatus(await backend.finishSet(setPreview.token, confirm)); }
            catch (error) { setStatus(String(error)); }
            finally { setBusy(false); }
          }}>{confirm ? "Confirm write" : "Cancel"}</button>)}
        </dialog>}
        {queryTab === "History" ? <div className="results-table" aria-label="Query history">
          <div className="result-row table-head"><span>Time (UTC)</span><span>Target</span><span>Operation</span><span>OID</span><span>Results</span></div>
          {history.length ? history.map(([time, item], index) => <div className="result-row" key={index}><span>{time}</span><span>{item.host}</span><span>{item.operation}</span><span>{item.oid}</span><span>{item.resultCount}</span></div>) : <p>No recorded queries</p>}
        </div> : <div className="results-table">
          <div className="result-row table-head"><span>Name</span><span>OID</span><span>Type</span><span>Value</span><span>Time</span></div>
          {rows.map((result) => <div className="result-row" key={result.oid}><span>{findNode(tree, result.oid)?.name ?? findNode(tree, result.oid.replace(/\.0$/, ""))?.name ?? "—"}</span><code>{result.oid}</code><span className="type-value">{result.kind}</span><span>{result.value}</span><span>{result.durationMs} ms</span></div>)}
        </div>}
      </section>

      <footer><span><span className={`status-dot ${backendStatus === "ready" ? "online" : ""}`} /> Backend {backendStatus}</span><span>SNMP {version} · UDP</span></footer>
    </main>
  );
}
