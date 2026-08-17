import { FormEvent, KeyboardEvent as ReactKeyboardEvent, MouseEvent as ReactMouseEvent, useEffect, useRef, useState } from "react";
import { Clock3, Command, FilePlus2, Folder, FolderPlus, Keyboard, LoaderCircle, Search, Settings2 } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { execute, executeAction, loadState, saveSettings, search, setActiveContext } from "./lib/api";
import type { CommandAction, CommandExecution, LauncherState, PresentationEntry, PresentationOutput, QueryResponse, SearchResult, Settings, Workspace } from "./lib/types";
import { PresentationView } from "./components/PresentationView";
import { WorkspaceList } from "./components/WorkspaceList";
import { WorkspacePicker } from "./components/WorkspacePicker";

const emptyQuery: QueryResponse = {
  executable: null,
  directoryQuery: null,
  results: [],
  actions: [],
  history: [],
};

function describeError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function isTauri() {
  return "__TAURI_INTERNALS__" in window;
}

async function hideLauncher() {
  if (!isTauri()) return;
  try {
    await getCurrentWindow().hide();
  } catch (reason) {
    if (import.meta.env.DEV) console.error("Failed to hide launcher window", reason);
  }
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
  const [pendingAction, setPendingAction] = useState<CommandAction | null>(null);
  const [contextPickerOpen, setContextPickerOpen] = useState(false);
  const [pendingContextQuery, setPendingContextQuery] = useState<string | null>(null);
  const [presentation, setPresentation] = useState<PresentationOutput | null>(null);

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
          setPendingAction(null);
          setContextPickerOpen(false);
          setPendingContextQuery(null);
          setPresentation(null);
          setError(null);
        })
        .catch((reason) => setError(describeError(reason)));
    }, 80);
    return () => window.clearTimeout(timer);
  }, [query]);

  const visibleHistory = query.trim() === "" ? state?.history ?? [] : response.history;
  const choices = response.results;
  const enabledWorkspaces = state?.settings.workspaces.filter((workspace) => workspace.enabled) ?? [];
  const choosingWorkspace = pendingAction !== null || contextPickerOpen || pendingContextQuery !== null;
  const selectionCount = choosingWorkspace ? enabledWorkspaces.length : presentation ? 0 : choices.length + response.actions.length;

  async function applyExecutionResult(command: string, result: CommandExecution) {
    if (result.kind === "needs-context") {
      if (!enabledWorkspaces.length) {
        setError("请先在设置中添加工作区，再执行需要目录上下文的命令");
        return;
      }
      setPendingContextQuery(command);
      setSelected(0);
      setError(null);
      return;
    }
    setState(await loadState());
    if (result.kind === "presented") {
      setPresentation(result.output);
      setPendingContextQuery(null);
      return;
    }
    setQuery("");
    setPresentation(null);
    await hideLauncher();
  }

  async function run(target?: SearchResult) {
    if (!query.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const command = query;
      await applyExecutionResult(command, await execute(command, target?.path));
    } catch (reason) {
      setError(describeError(reason));
    } finally {
      setBusy(false);
      inputRef.current?.focus();
    }
  }

  async function runAction(action: CommandAction, workspace: Workspace) {
    setBusy(true);
    setError(null);
    try {
      await executeAction(query, action.kind, workspace.path);
      setQuery("");
      setPendingAction(null);
      setPresentation(null);
      setState(await loadState());
      await hideLauncher();
    } catch (reason) {
      setError(describeError(reason));
    } finally {
      setBusy(false);
    }
  }

  async function chooseContext(workspace: Workspace) {
    setBusy(true);
    setError(null);
    try {
      const nextState = await setActiveContext(workspace.path);
      setState(nextState);
      setContextPickerOpen(false);
      setSelected(0);
      if (pendingContextQuery) {
        const command = pendingContextQuery;
        setPendingContextQuery(null);
        await applyExecutionResult(command, await execute(command));
      }
    } catch (reason) {
      setError(describeError(reason));
    } finally {
      setBusy(false);
      inputRef.current?.focus();
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
      if (pendingAction && enabledWorkspaces[selected]) void runAction(pendingAction, enabledWorkspaces[selected]);
      else if (pendingContextQuery && enabledWorkspaces[selected]) void chooseContext(enabledWorkspaces[selected]);
      else if (contextPickerOpen && enabledWorkspaces[selected]) void chooseContext(enabledWorkspaces[selected]);
      else if (selected < choices.length) void run(choices[selected]);
      else if (response.actions[selected - choices.length]) {
        if (!enabledWorkspaces.length) setError("请先在设置中添加工作区");
        else { setPendingAction(response.actions[selected - choices.length]); setSelected(0); }
      }
      else void run();
    } else if (event.key === "Escape") {
      if (pendingAction) { setPendingAction(null); setSelected(0); }
      else if (pendingContextQuery) { setPendingContextQuery(null); setSelected(0); }
      else if (contextPickerOpen) { setContextPickerOpen(false); setSelected(0); }
      else if (settingsOpen) setSettingsOpen(false);
      else if (presentation) setPresentation(null);
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
            onChange={(event) => { setPresentation(null); setQuery(event.target.value); }}
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

        <div className={`min-h-0 flex-1 p-2 ${presentation ? "overflow-hidden" : "overflow-y-auto"}`}>
          {error && <div className="m-2 rounded-xl border border-red-400/20 bg-red-400/10 px-4 py-3 text-sm text-red-200">{error}</div>}

          {query.trim() !== "" && !choosingWorkspace && !presentation && choices.map((item, index) => (
            <button key={item.path} className={`result-row ${selected === index ? "result-row-active" : ""}`} onClick={() => void run(item)}>
              <Folder className="h-5 w-5 shrink-0 text-indigo-400" />
              <span className="min-w-0 flex-1 text-left">
                <span className="block font-medium">{item.name}</span>
                <span className="block truncate text-xs text-zinc-500">{item.path}</span>
              </span>
              <span className="text-xs text-zinc-600">使用 {item.useCount} 次</span>
            </button>
          ))}

          {query.trim() !== "" && !choosingWorkspace && !presentation && response.actions.map((action, actionIndex) => {
            const index = choices.length + actionIndex;
            const ActionIcon = action.kind === "open-file" ? FilePlus2 : FolderPlus;
            return <button key={action.id} className={`result-row ${selected === index ? "result-row-active" : ""}`} onClick={() => {
              if (!enabledWorkspaces.length) setError("请先在设置中添加工作区");
              else { setPendingAction(action); setSelected(0); }
            }}>
              <ActionIcon className={`h-5 w-5 ${action.kind === "open-file" ? "text-sky-400" : "text-emerald-400"}`} />
              <span className="text-left"><span className="block font-medium">{action.label}</span><span className="text-xs text-zinc-500">{action.description}</span></span>
            </button>;
          })}

          {query.trim() !== "" && pendingAction && <WorkspacePicker title={`为“${pendingAction.label}”选择工作区`} workspaces={enabledWorkspaces} activeContext={state.activeContext} selected={selected} onBack={() => { setPendingAction(null); setSelected(0); }} onSelect={(workspace) => void runAction(pendingAction, workspace)} />}

          {pendingContextQuery && <WorkspacePicker title="选择本次命令的目录上下文" workspaces={enabledWorkspaces} activeContext={state.activeContext} selected={selected} onBack={() => { setPendingContextQuery(null); setSelected(0); }} onSelect={(workspace) => void chooseContext(workspace)} />}

          {contextPickerOpen && !pendingContextQuery && <WorkspacePicker title="选择活动上下文" workspaces={enabledWorkspaces} activeContext={state.activeContext} selected={selected} onBack={() => { setContextPickerOpen(false); setSelected(0); }} onSelect={(workspace) => void chooseContext(workspace)} />}

          {presentation && !choosingWorkspace && <PresentationView output={presentation} onClose={() => setPresentation(null)} onOpenEntry={(entry: PresentationEntry) => {
            const nextQuery = `${entry.kind === "directory" ? "ls" : "cat"} ${JSON.stringify(entry.path)}`;
            setPresentation(null);
            setQuery(nextQuery);
          }} />}

          {query.trim() === "" && !choosingWorkspace && !presentation && (
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
          <button className="flex min-w-0 items-center gap-2 hover:text-zinc-300" onClick={() => {
            if (!enabledWorkspaces.length) setError("请先在设置中添加工作区");
            else { setContextPickerOpen(true); setPendingAction(null); setPendingContextQuery(null); setPresentation(null); setSelected(0); }
          }} title={state.activeContext ?? "选择活动上下文"}>
            <Folder className="h-3.5 w-3.5 shrink-0" />
            <span className="max-w-52 truncate">{state.activeContext?.split("/").pop() ?? "选择上下文"}</span>
          </button>
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
  const [workspaces, setWorkspaces] = useState(state.settings.workspaces);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [recordingShortcut, setRecordingShortcut] = useState(false);

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
      const next = await saveSettings({ ...form, defaultWorkspace: null, workspaces });
      onChange(next);
      onClose();
    } catch (reason) {
      setError(describeError(reason));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="fixed inset-3 z-20 grid place-items-center overflow-hidden rounded-2xl bg-black/70 p-6 backdrop-blur-sm" onMouseDown={onClose}>
      <form className="settings-surface max-h-full w-full max-w-xl overflow-y-auto rounded-2xl border border-white/10 bg-zinc-900 p-6" onSubmit={submit} onMouseDown={(event) => event.stopPropagation()}>
        <h2 className="text-lg font-semibold">设置</h2>
        <label className="field-label">全局快捷键
          <button type="button" className={`shortcut-recorder ${recordingShortcut ? "shortcut-recorder-active" : ""}`} onClick={() => { setError(null); setRecordingShortcut(true); }}>
            <Keyboard className="h-4 w-4 text-zinc-500" />
            <ShortcutKeys shortcut={form.shortcut} />
            <span className={`ml-auto text-xs ${recordingShortcut ? "text-indigo-300" : "text-zinc-600"}`}>{recordingShortcut ? "请按下组合键…" : "点击开始录制"}</span>
          </button>
        </label>
        <WorkspaceList workspaces={workspaces} disabled={busy} onChange={setWorkspaces} onError={(message) => setError(message || null)} />
        {error && <div className="mt-4 rounded-xl border border-red-400/20 bg-red-400/10 px-4 py-3 text-sm text-red-200">{error}</div>}
        <div className="mt-5 flex items-center justify-between">
          <span className="text-xs text-zinc-600">保存后自动重建索引</span>
          <div className="flex items-center gap-2"><button type="button" className="secondary-button" onClick={onClose}>取消</button><button className="primary-button" disabled={busy}>保存</button></div>
        </div>
      </form>
    </div>
  );
}

export default App;
