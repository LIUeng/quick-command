import { FormEvent, KeyboardEvent, useEffect, useMemo, useRef, useState } from "react";
import { Clock3, Command, Folder, LoaderCircle, Plus, RefreshCw, Search, Settings2 } from "lucide-react";
import { createAndExecute, execute, loadState, reindex, saveSettings, search } from "./lib/api";
import type { LauncherState, QueryResponse, SearchResult, Settings } from "./lib/types";

const emptyQuery: QueryResponse = {
  executable: null,
  directoryQuery: null,
  results: [],
  history: [],
  canCreate: false,
};

function describeError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function App() {
  const inputRef = useRef<HTMLInputElement>(null);
  const [state, setState] = useState<LauncherState | null>(null);
  const [query, setQuery] = useState("");
  const [response, setResponse] = useState<QueryResponse>(emptyQuery);
  const [selected, setSelected] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);

  useEffect(() => {
    loadState().then(setState).catch((reason) => setError(describeError(reason)));
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      search(query)
        .then((next) => {
          setResponse(next);
          setSelected(0);
          setError(null);
        })
        .catch((reason) => setError(describeError(reason)));
    }, 80);
    return () => window.clearTimeout(timer);
  }, [query]);

  const visibleHistory = query.trim() === "" ? state?.history ?? [] : response.history;
  const choices = response.results;
  const selectionCount = choices.length + (response.canCreate ? 1 : 0);

  async function run(target?: SearchResult) {
    if (!query.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await execute(query, target?.path);
      setQuery("");
      setState(await loadState());
    } catch (reason) {
      setError(describeError(reason));
    } finally {
      setBusy(false);
      inputRef.current?.focus();
    }
  }

  async function createProject() {
    setBusy(true);
    setError(null);
    try {
      await createAndExecute(query);
      setQuery("");
      setState(await loadState());
    } catch (reason) {
      setError(describeError(reason));
    } finally {
      setBusy(false);
    }
  }

  function onKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown" && selectionCount) {
      event.preventDefault();
      setSelected((value) => (value + 1) % selectionCount);
    } else if (event.key === "ArrowUp" && selectionCount) {
      event.preventDefault();
      setSelected((value) => (value - 1 + selectionCount) % selectionCount);
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (selected < choices.length) void run(choices[selected]);
      else if (response.canCreate) void createProject();
      else void run();
    } else if (event.key === "Escape") {
      setQuery("");
      setError(null);
    }
  }

  if (!state) {
    return <main className="grid min-h-screen place-items-center text-zinc-400"><LoaderCircle className="animate-spin" /></main>;
  }

  return (
    <main className="min-h-screen bg-zinc-950 p-3 text-zinc-100">
      <section className="mx-auto max-w-3xl overflow-hidden rounded-2xl border border-white/10 bg-zinc-900 shadow-2xl shadow-black/50">
        <div className="flex items-center gap-3 border-b border-white/10 px-5 py-4">
          <Search className="h-5 w-5 text-zinc-500" />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={onKeyDown}
            className="min-w-0 flex-1 bg-transparent text-lg outline-none placeholder:text-zinc-600"
            placeholder="输入命令，例如 code example"
            spellCheck={false}
          />
          {busy && <LoaderCircle className="h-5 w-5 animate-spin text-indigo-400" />}
          <button className="icon-button" onClick={() => setSettingsOpen(true)} aria-label="打开设置">
            <Settings2 className="h-5 w-5" />
          </button>
        </div>

        <div className="max-h-[480px] overflow-y-auto p-2">
          {error && <div className="m-2 rounded-xl border border-red-400/20 bg-red-400/10 px-4 py-3 text-sm text-red-200">{error}</div>}

          {query.trim() !== "" && choices.map((item, index) => (
            <button key={item.path} className={`result-row ${selected === index ? "result-row-active" : ""}`} onClick={() => void run(item)}>
              <Folder className="h-5 w-5 shrink-0 text-indigo-400" />
              <span className="min-w-0 flex-1 text-left">
                <span className="block font-medium">{item.name}</span>
                <span className="block truncate text-xs text-zinc-500">{item.path}</span>
              </span>
              <span className="text-xs text-zinc-600">使用 {item.useCount} 次</span>
            </button>
          ))}

          {query.trim() !== "" && response.canCreate && (
            <button className={`result-row ${selected === choices.length ? "result-row-active" : ""}`} onClick={() => void createProject()}>
              <Plus className="h-5 w-5 text-emerald-400" />
              <span className="text-left"><span className="block font-medium">创建并打开项目</span><span className="text-xs text-zinc-500">在默认工作区中创建 {response.directoryQuery}</span></span>
            </button>
          )}

          {query.trim() === "" && (
            <>
              <div className="flex items-center justify-between px-3 pb-2 pt-1 text-xs uppercase tracking-widest text-zinc-600">
                <span>最近使用</span><span>{state.indexedDirectoryCount} 个项目已索引</span>
              </div>
              {visibleHistory.length ? visibleHistory.map((item) => (
                <button key={item.id} className="result-row" onClick={() => setQuery(item.displayText)}>
                  <Clock3 className="h-5 w-5 text-zinc-500" />
                  <span className="min-w-0 flex-1 truncate text-left">{item.displayText}</span>
                  <span className="truncate text-xs text-zinc-600">{item.targetPath}</span>
                </button>
              )) : <EmptyState />}
            </>
          )}
        </div>

        <footer className="flex items-center justify-between border-t border-white/10 px-5 py-3 text-xs text-zinc-600">
          <span className="flex items-center gap-2"><Command className="h-3.5 w-3.5" /> Enter 执行 · ↑↓ 选择 · Esc 清空</span>
          <span>{state.settings.shortcut}</span>
        </footer>
      </section>

      {settingsOpen && <SettingsDialog state={state} onClose={() => setSettingsOpen(false)} onChange={setState} />}
    </main>
  );
}

function EmptyState() {
  return <div className="grid place-items-center gap-2 px-6 py-16 text-center text-zinc-600"><Command className="h-8 w-8" /><p>添加工作区并建立索引后即可搜索项目</p></div>;
}

function SettingsDialog({ state, onClose, onChange }: { state: LauncherState; onClose: () => void; onChange: (state: LauncherState) => void }) {
  const [form, setForm] = useState<Settings>(state.settings);
  const [workspaceText, setWorkspaceText] = useState(state.settings.workspaces.map((item) => item.path).join("\n"));
  const [busy, setBusy] = useState(false);
  const workspaceCount = useMemo(() => workspaceText.split("\n").filter((line) => line.trim()).length, [workspaceText]);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    try {
      const workspaces = workspaceText.split("\n").map((path) => path.trim()).filter(Boolean).map((path) => ({ path, enabled: true }));
      const next = await saveSettings({ ...form, defaultWorkspace: workspaces[0]?.path ?? null, workspaces });
      onChange(next);
      onClose();
    } finally {
      setBusy(false);
    }
  }

  async function rebuild() {
    setBusy(true);
    try { onChange(await reindex()); } finally { setBusy(false); }
  }

  return (
    <div className="fixed inset-0 grid place-items-center bg-black/70 p-6 backdrop-blur-sm" onMouseDown={onClose}>
      <form className="w-full max-w-xl rounded-2xl border border-white/10 bg-zinc-900 p-6 shadow-2xl" onSubmit={submit} onMouseDown={(event) => event.stopPropagation()}>
        <h2 className="text-lg font-semibold">设置</h2>
        <label className="field-label">全局快捷键<input className="field-input" value={form.shortcut} onChange={(event) => setForm({ ...form, shortcut: event.target.value })} /></label>
        <label className="field-label">工作区目录（每行一个）<textarea className="field-input min-h-32 resize-y" value={workspaceText} onChange={(event) => setWorkspaceText(event.target.value)} placeholder="/Users/you/Projects" /></label>
        <div className="mt-5 flex items-center justify-between">
          <button type="button" className="secondary-button" onClick={() => void rebuild()} disabled={busy}><RefreshCw className="h-4 w-4" />重建索引</button>
          <div className="flex items-center gap-2"><span className="mr-2 text-xs text-zinc-600">{workspaceCount} 个工作区</span><button type="button" className="secondary-button" onClick={onClose}>取消</button><button className="primary-button" disabled={busy}>保存</button></div>
        </div>
      </form>
    </div>
  );
}

export default App;
