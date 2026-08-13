import { invoke } from "@tauri-apps/api/core";
import type { LauncherState, QueryResponse, Settings } from "./types";

const inTauri = () => "__TAURI_INTERNALS__" in window;

const demoState: LauncherState = {
  settings: { shortcut: "CommandOrControl+Shift+Space", defaultWorkspace: null, workspaces: [] },
  history: [],
  indexedDirectoryCount: 0,
};

export async function loadState(): Promise<LauncherState> {
  return inTauri() ? invoke("get_launcher_state") : demoState;
}

export async function search(query: string): Promise<QueryResponse> {
  if (!inTauri()) {
    return { executable: null, directoryQuery: null, results: [], history: [], canCreate: false };
  }
  return invoke("search_projects", { query });
}

export async function execute(query: string, targetPath?: string): Promise<void> {
  await invoke("execute_command", { query, targetPath: targetPath ?? null });
}

export async function createAndExecute(query: string): Promise<void> {
  await invoke("create_and_execute", { query });
}

export async function saveSettings(settings: Settings): Promise<LauncherState> {
  return invoke("save_settings", { settings });
}

export async function reindex(): Promise<LauncherState> {
  return invoke("reindex_workspaces");
}
