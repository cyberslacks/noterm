import { useEffect, useMemo, useState } from "react";
import CodeMirror from "@uiw/react-codemirror";
import { markdown } from "@codemirror/lang-markdown";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { api } from "./api";
import type { FileNode, Note, Vault } from "./types";

type Panel = "preview" | "source" | "split";
type Overlay = "settings" | "fabric" | null;

export function App() {
  const [vaults, setVaults] = useState<Vault[]>([]);
  const [vaultId, setVaultId] = useState("");
  const [files, setFiles] = useState<FileNode[]>([]);
  const [note, setNote] = useState<Note | null>(null);
  const [body, setBody] = useState("");
  const [panel, setPanel] = useState<Panel>("split");
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState("Ready");
  const [overlay, setOverlay] = useState<Overlay>(null);
  const [configText, setConfigText] = useState("");
  const [patterns, setPatterns] = useState<string[]>([]);
  const [pattern, setPattern] = useState("");
  const [fabricResult, setFabricResult] = useState("");
  const activeVault = useMemo(() => vaults.find(v => v.id === vaultId), [vaults, vaultId]);

  useEffect(() => { api.vaults().then(v => { setVaults(v); setVaultId(v[0]?.id ?? ""); }).catch(showError); }, []);
  useEffect(() => { if (vaultId) refreshFiles(); }, [vaultId]);

  function showError(error: unknown) { setStatus(error instanceof Error ? error.message : String(error)); }
  async function refreshFiles() { try { setFiles(await api.files(vaultId)); } catch (error) { showError(error); } }
  async function open(path: string) { try { const loaded = await api.note(vaultId, path); setNote(loaded); setBody(loaded.body); setStatus(`Opened ${loaded.relative_path}`); } catch (error) { showError(error); } }
  async function save() { if (!note) return; try { await api.save(vaultId, note.relative_path, body); setNote({ ...note, body }); setStatus("Saved"); } catch (error) { showError(error); } }
  async function publish() { if (!note) return; try { await api.save(vaultId, note.relative_path, body); await api.publish(vaultId, note.relative_path); setStatus("Published to Kazam"); } catch (error) { showError(error); } }
  async function create() { const title = window.prompt("Note title"); if (!title) return; try { const created = await api.create(vaultId, title); await refreshFiles(); setNote(created); setBody(created.body); } catch (error) { showError(error); } }
  async function createCollection() { const name = window.prompt("Collection name"); if (!name) return; try { await api.createCollection(vaultId, name); await refreshFiles(); setStatus(`Created collection: ${name}`); } catch (error) { showError(error); } }
  async function openVaultFolder() { try { await api.openVaultFolder(vaultId); setStatus("Opened vault folder"); } catch (error) { showError(error); } }
  async function openVaultTerminal() { try { await api.openVaultTerminal(vaultId); setStatus("Opened terminal in vault"); } catch (error) { showError(error); } }
  async function search() { if (!query.trim()) return; try { const results = await api.search(vaultId, query); if (results[0]) open(results[0].relative_path); setStatus(results.length ? `${results.length} results` : "No results"); } catch (error) { showError(error); } }
  async function openSettings() { try { setConfigText(JSON.stringify(await api.config(), null, 2)); setOverlay("settings"); } catch (error) { showError(error); } }
  async function saveSettings() { try { await api.saveConfig(JSON.parse(configText)); setStatus("Configuration saved"); setOverlay(null); } catch (error) { showError(error); } }
  async function openFabric() { try { const available = await api.fabricPatterns(); setPatterns(available); setPattern(available[0] ?? ""); setFabricResult(""); setOverlay("fabric"); } catch (error) { showError(error); } }
  async function runFabric() { if (!pattern || !note) return; try { setFabricResult(await api.fabricRun(pattern, body)); } catch (error) { showError(error); } }
  function applyFabric(mode: "append" | "replace") { if (!fabricResult) return; setBody(mode === "append" ? `${body.trimEnd()}\n\n${fabricResult}` : fabricResult); setOverlay(null); setStatus("Fabric result applied; save when ready"); }
  async function saveFabricToInbox() { if (!fabricResult) return; const title = window.prompt("Inbox note title", `Fabric: ${pattern}`); if (!title) return; try { const saved = await api.saveInbox(vaultId, title, fabricResult); await refreshFiles(); setNote(saved); setBody(saved.body); setOverlay(null); setStatus("Fabric output saved to notes/inbox"); } catch (error) { showError(error); } }
  async function sync(direction: "push" | "pull") { try { setStatus(`${direction === "push" ? "Pushing" : "Pulling"} vault…`); await api.sync(vaultId, direction); await refreshFiles(); setStatus(`Vault ${direction} complete`); } catch (error) { showError(error); } }

  return <main className="app-shell">
    <header><strong>NOTERM</strong><select value={vaultId} onChange={e => setVaultId(e.target.value)}>{vaults.map(v => <option key={v.id} value={v.id}>{v.name}</option>)}</select><span>{activeVault?.path}</span><button onClick={openVaultFolder}>Open folder</button><button onClick={openVaultTerminal}>Open terminal</button><button onClick={() => sync("pull")}>Pull</button><button onClick={() => sync("push")}>Push</button><button onClick={openFabric} disabled={!note}>Fabric</button><button onClick={openSettings}>Settings</button><button onClick={createCollection}>New collection</button><button onClick={create}>New note</button><button onClick={publish} disabled={!note}>Publish</button><button onClick={save} disabled={!note}>Save</button></header>
    <aside className="sidebar"><div className="search"><input value={query} onChange={e => setQuery(e.target.value)} onKeyDown={e => e.key === "Enter" && search()} placeholder="Search this vault" /><button onClick={search}>Search</button></div><nav>{files.filter(file => !file.is_dir).map(file => <button key={file.path} className={note?.relative_path === file.relative_path ? "selected" : ""} style={{ paddingLeft: `${12 + file.depth * 16}px` }} onClick={() => open(file.relative_path)}>{file.name}</button>)}</nav></aside>
    <section className="workspace"><div className="toolbar"><span>{note?.title ?? "Select a note"}</span><div>{(["preview", "source", "split"] as Panel[]).map(view => <button key={view} className={panel === view ? "selected" : ""} onClick={() => setPanel(view)}>{view}</button>)}</div></div><div className={`document ${panel}`}>{panel !== "source" && <article><ReactMarkdown remarkPlugins={[remarkGfm]}>{body}</ReactMarkdown></article>}{panel !== "preview" && <CodeMirror value={body} height="100%" extensions={[markdown()]} onChange={setBody} />}</div></section>
    <footer>{status}</footer>
    {overlay === "settings" && <div className="modal-backdrop"><section className="modal settings"><h2>Configuration</h2><p>Edit all non-secret configuration fields. Secret values are stored through the operating-system credential service.</p><textarea value={configText} onChange={event => setConfigText(event.target.value)} spellCheck={false} /><div><button onClick={() => setOverlay(null)}>Cancel</button><button onClick={saveSettings}>Save configuration</button></div></section></div>}
    {overlay === "fabric" && <div className="modal-backdrop"><section className="modal fabric"><h2>Fabric pattern</h2><p>Run a local Fabric pattern against the current note, then review before applying it.</p><select value={pattern} onChange={event => setPattern(event.target.value)}>{patterns.map(item => <option key={item}>{item}</option>)}</select><button onClick={runFabric} disabled={!pattern}>Run pattern</button>{fabricResult && <><textarea value={fabricResult} readOnly /><div><button onClick={() => applyFabric("append")}>Append to note</button><button onClick={() => applyFabric("replace")}>Replace editor</button><button onClick={saveFabricToInbox}>Save to inbox</button></div></>}<button onClick={() => setOverlay(null)}>Close</button></section></div>}
  </main>;
}
