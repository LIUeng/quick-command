import { FormEvent, KeyboardEvent as ReactKeyboardEvent, MouseEvent as ReactMouseEvent, useEffect, useMemo, useRef, useState } from "react";
import { Clock3, Command, Folder, Keyboard, LoaderCircle, Plus, RefreshCw, Search, Settings2 } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
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

function isTauri() {
  return "__TAURI_INTERNALS__" in window;
}

async function hideLauncher() {
  if (isTauri()) await getCurrentWindow().hide();
}

async function startWindowDrag(event: ReactMouseEvent<HTMLElement>) {
  if (event.button !== 0 || !isTauri()) return;
  const target = event.target as HTMLElement;
  if (target.closest("input, button, textarea, [data-no-drag]")) return;
  event.preventDefault();
  await getCurrentWindow().startDragging();
}

const keyLabels: Record<string, string> = {
  CommandOrControl: "⌘",
  Command: "⌘",
  Control: "⌃",
  Alt: "⌥",
  Option: "⌥",
  Shift: "⇧",
  Space: "Space",
  Enter: "↩",
  Escape: "Esc",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Backspace: "⌫",
  Delete: "⌦",
  Tab: "⇥",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Backquote: "`",
};

function acceleratorKeyFromCode(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;

  const supportedCodes = new Set([
    "Space", "Enter", "Escape", "Tab", "Backspace", "Delete", "Insert",
    "Home", "End", "PageUp", "PageDown", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight",
    "Minus", "Equal", "BracketLeft", "BracketRight", "Backslash", "Semicolon",
    "Quote", "Comma", "Period", "Slash", "Backquote",
  ]);
  return supportedCodes.has(code) ? code : null;
}

function shortcutParts(shortcut: string) {
  return shortcut.split("+").filter(Boolean).map((part) => keyLabels[part] ?? (part.length === 1 ? part.toUpperCase() : part));
}

function ShortcutKeys({ shortcut }: { shortcut: string }) {
  return <span className="inline-flex items-center gap-1" aria-label={shortcut}>{shortcutParts(shortcut).map((part, index) => <kbd className="keycap" key={`${part}-${index}`}>{part}</kbd>)}</span>;
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
      await hideLauncher();
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
      await hideLauncher();
    } catch (reason) {
      setError(describeError(reason));
    } finally {
      setBusy(false);
    }
  }

  function onKeyDown(event: ReactKeyboardEvent<HTMLInputElement>) {
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
      if (settingsOpen) setSettingsOpen(false);
      else void hideLauncher();
    }
  }

  if (!state) {
    return <main className="grid min-h-screen place-items-center text-zinc-400"><LoaderCircle className="animate-spin" /></main>;
  }

  return (
    <main className="flex h-screen w-screen bg-transparent p-3 text-zinc-100">
      <section className="launcher-surface relative flex min-h-0 w-full flex-col overflow-hidden rounded-2xl border border-white/10 bg-zinc-900">
        <div onMouseDown={startWindowDrag} className="absolute inset-x-0 top-0 z-10 h-3 cursor-grab active:cursor-grabbing" aria-hidden="true" />
        <div onMouseDown={startWindowDrag} className="flex cursor-grab items-center gap-3 border-b border-white/10 px-5 py-4 active:cursor-grabbing">
          <Search className="h-5 w-5 text-zinc-500" />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={onKeyDown}
            className="min-w-0 flex-1 cursor-text bg-transparent text-lg outline-none placeholder:text-zinc-600"
            placeholder="输入命令，例如 code example"
            spellCheck={false}
          />
          {busy && <LoaderCircle className="h-5 w-5 animate-spin text-indigo-400" />}
          <button className="icon-button" onClick={() => setSettingsOpen(true)} aria-label="打开设置">
            <Settings2 className="h-5 w-5" />
          </button>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto p-2">
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
          <span className="flex items-center gap-2"><Command className="h-3.5 w-3.5" /> Enter 执行 · ↑↓ 选择 · Esc 隐藏</span>
          <ShortcutKeys shortcut={state.settings.shortcut} />
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
  const [error, setError] = useState<string | null>(null);
  const [recordingShortcut, setRecordingShortcut] = useState(false);
  const workspaceCount = useMemo(() => workspaceText.split("\n").filter((line) => line.trim()).length, [workspaceText]);

  useEffect(() => {
    if (!recordingShortcut) return;

    function captureShortcut(event: globalThis.KeyboardEvent) {
      event.preventDefault();
      event.stopPropagation();
      if (event.repeat || ["Meta", "Control", "Alt", "Shift"].includes(event.key)) return;

      const parts: string[] = [];
      if (event.metaKey) parts.push("Command");
      if (event.ctrlKey) parts.push("Control");
      if (event.altKey) parts.push("Alt");
      if (event.shiftKey) parts.push("Shift");
      if (!parts.length) {
        setError("快捷键至少需要包含 ⌘、⌃、⌥ 中的一个修饰键");
        return;
      }

      const key = acceleratorKeyFromCode(event.code);
      if (!key) {
        setError("暂不支持这个按键，请使用字母、数字、方向键或功能键");
        return;
      }
      parts.push(key);
      setForm((current) => ({ ...current, shortcut: parts.join("+") }));
      setError(null);
      setRecordingShortcut(false);
    }

    window.addEventListener("keydown", captureShortcut, true);
    return () => window.removeEventListener("keydown", captureShortcut, true);
  }, [recordingShortcut]);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const workspaces = workspaceText.split("\n").map((path) => path.trim()).filter(Boolean).map((path) => ({ path, enabled: true }));
      const next = await saveSettings({ ...form, defaultWorkspace: workspaces[0]?.path ?? null, workspaces });
      onChange(next);
      onClose();
    } catch (reason) {
      setError(describeError(reason));
    } finally {
      setBusy(false);
    }
  }

  async function rebuild() {
    setBusy(true);
    setError(null);
    try { onChange(await reindex()); } catch (reason) { setError(describeError(reason)); } finally { setBusy(false); }
  }

  return (
    <div className="fixed inset-3 z-20 grid place-items-center overflow-hidden rounded-2xl bg-black/70 p-6 backdrop-blur-sm" onMouseDown={onClose}>
      <form className="settings-surface w-full max-w-xl overflow-hidden rounded-2xl border border-white/10 bg-zinc-900 p-6" onSubmit={submit} onMouseDown={(event) => event.stopPropagation()}>
        <h2 className="text-lg font-semibold">设置</h2>
        <label className="field-label">全局快捷键
          <button type="button" className={`shortcut-recorder ${recordingShortcut ? "shortcut-recorder-active" : ""}`} onClick={() => { setError(null); setRecordingShortcut(true); }}>
            <Keyboard className="h-4 w-4 text-zinc-500" />
            <ShortcutKeys shortcut={form.shortcut} />
            <span className={`ml-auto text-xs ${recordingShortcut ? "text-indigo-300" : "text-zinc-600"}`}>{recordingShortcut ? "请按下组合键…" : "点击开始录制"}</span>
          </button>
        </label>
        <label className="field-label">工作区目录（每行一个）<textarea className="field-input min-h-32 resize-y" value={workspaceText} onChange={(event) => setWorkspaceText(event.target.value)} placeholder="/Users/you/Projects" /></label>
        {error && <div className="mt-4 rounded-xl border border-red-400/20 bg-red-400/10 px-4 py-3 text-sm text-red-200">{error}</div>}
        <div className="mt-5 flex items-center justify-between">
          <button type="button" className="secondary-button" onClick={() => void rebuild()} disabled={busy}><RefreshCw className="h-4 w-4" />重建索引</button>
          <div className="flex items-center gap-2"><span className="mr-2 text-xs text-zinc-600">{workspaceCount} 个工作区</span><button type="button" className="secondary-button" onClick={onClose}>取消</button><button className="primary-button" disabled={busy}>保存</button></div>
        </div>
      </form>
    </div>
  );
}

export default App;
