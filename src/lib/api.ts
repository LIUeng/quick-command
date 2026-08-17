import { invoke } from "@tauri-apps/api/core";
import type { CommandActionKind, CommandExecution, LauncherState, QueryResponse, Settings } from "./types";

const inTauri = () => "__TAURI_INTERNALS__" in window;

const demoState: LauncherState = {
  settings: { shortcut: "CommandOrControl+Shift+Space", defaultWorkspace: null, workspaces: [] },
  activeContext: null,
  history: [],
  indexedDirectoryCount: 0,
};

export async function loadState(): Promise<LauncherState> {
  return inTauri() ? invoke("get_launcher_state") : demoState;
}

export async function search(query: string): Promise<QueryResponse> {
  if (!inTauri()) {
    return { executable: null, directoryQuery: null, results: [], actions: [], history: [] };
  }
  return invoke("search_projects", { query });
}

export async function execute(query: string, targetPath?: string): Promise<CommandExecution> {
  return invoke("execute_command", { query, targetPath: targetPath ?? null });
}

export async function executeAction(query: string, actionKind: CommandActionKind, workspacePath: string): Promise<void> {
  await invoke("execute_action", { query, actionKind, workspacePath });
}

export async function setActiveContext(path: string | null): Promise<LauncherState> {
  return invoke("set_active_context", { path });
}

export async function saveSettings(settings: Settings): Promise<LauncherState> {
  return invoke("save_settings", { settings });
}

export async function reindex(): Promise<LauncherState> {
  return invoke("reindex_workspaces");
}

export async function setAutoHideSuspended(suspended: boolean): Promise<void> {
  await invoke("set_auto_hide_suspended", { suspended });
}
