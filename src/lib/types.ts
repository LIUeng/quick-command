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
  action: HistoryAction | null;
  executedAt: number;
};

export type HistoryActionKind =
  | "launch-command"
  | "open-project"
  | "open-file"
  | "create-directory-and-open"
  | "list-directory"
  | "read-text-file"
  | "change-context"
  | "create-directory";

export type HistoryAction = {
  kind: HistoryActionKind;
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
  activeContext: string | null;
  history: HistoryItem[];
  indexedDirectoryCount: number;
  startupNotice: string | null;
};

export type QueryResponse = {
  executable: string | null;
  directoryQuery: string | null;
  results: SearchResult[];
  actions: CommandAction[];
  history: HistoryItem[];
};

export type CommandActionKind = "open-file" | "create-directory";

export type CommandAction = {
  id: string;
  kind: CommandActionKind;
  label: string;
  description: string;
  requiresWorkspace: boolean;
};

export type PresentationEntryKind = "directory" | "file" | "symlink" | "other";

export type PresentationEntry = {
  name: string;
  path: string;
  kind: PresentationEntryKind;
  size: number | null;
  modifiedAt: number | null;
  hidden: boolean;
};

export type DirectoryPresentation = {
  type: "directory";
  path: string;
  entries: PresentationEntry[];
  directoryCount: number;
  fileCount: number;
  hiddenCount: number;
  detailed: boolean;
  truncated: boolean;
};

export type TextFilePresentation = {
  type: "text-file";
  path: string;
  name: string;
  content: string;
  size: number;
  lineCount: number;
  language: string | null;
};

export type PresentationOutput = DirectoryPresentation | TextFilePresentation;

export type OperationKind = "create-directory" | "create-project-directory-and-open";

export type OperationConfirmation = {
  kind: OperationKind;
  title: string;
  description: string;
  targetPath: string;
  workspacePath: string;
  pathsToCreate: string[];
};

export type CommandExecution =
  | { kind: "launched" }
  | { kind: "needs-context"; message: string }
  | { kind: "context-updated"; path: string }
  | { kind: "confirmation"; confirmation: OperationConfirmation }
  | { kind: "operation-completed"; title: string; message: string; path: string }
  | { kind: "presented"; output: PresentationOutput };
