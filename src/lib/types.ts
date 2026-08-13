export type SearchResult = {
  path: string;
  name: string;
  score: number;
  lastUsedAt: number | null;
  useCount: number;
};

export type HistoryItem = {
  id: string;
  displayText: string;
  executable: string;
  args: string[];
  targetPath: string | null;
  executedAt: number;
};

export type Workspace = {
  path: string;
  enabled: boolean;
};

export type Settings = {
  shortcut: string;
  defaultWorkspace: string | null;
  workspaces: Workspace[];
};

export type LauncherState = {
  settings: Settings;
  history: HistoryItem[];
  indexedDirectoryCount: number;
};

export type QueryResponse = {
  executable: string | null;
  directoryQuery: string | null;
  results: SearchResult[];
  history: HistoryItem[];
  canCreate: boolean;
};
