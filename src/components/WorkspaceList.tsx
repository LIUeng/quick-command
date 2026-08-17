import { open } from "@tauri-apps/plugin-dialog";
import { Folder, FolderPlus, LoaderCircle, Trash2 } from "lucide-react";
import { useState } from "react";
import { setAutoHideSuspended } from "../lib/api";
import type { Workspace } from "../lib/types";

type WorkspaceListProps = {
  workspaces: Workspace[];
  disabled?: boolean;
  onChange: (workspaces: Workspace[]) => void;
  onError: (message: string) => void;
};

function workspaceName(path: string) {
  const normalized = path.replace(/\/$/, "");
  return normalized.split("/").pop() || normalized;
}

function describeError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export function WorkspaceList({ workspaces, disabled, onChange, onError }: WorkspaceListProps) {
  const [selecting, setSelecting] = useState(false);

  async function addWorkspaces() {
    setSelecting(true);
    onError("");
    let autoHideSuspended = false;
    try {
      await setAutoHideSuspended(true);
      autoHideSuspended = true;
      const selected = await open({
        directory: true,
        multiple: true,
        title: "选择 Quick Command 工作区",
      });
      if (!selected) return;

      const paths = Array.isArray(selected) ? selected : [selected];
      const existing = new Set(workspaces.map((workspace) => workspace.path));
      const additions = paths
        .filter((path) => !existing.has(path))
        .map((path) => ({ path, enabled: true }));
      if (additions.length) onChange([...workspaces, ...additions]);
    } catch (error) {
      onError(describeError(error));
    } finally {
      if (autoHideSuspended) {
        try {
          await setAutoHideSuspended(false);
        } catch (error) {
          if (import.meta.env.DEV) console.error("Failed to restore launcher auto-hide", error);
        }
      }
      setSelecting(false);
    }
  }

  return (
    <section className="mt-5">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-sm text-zinc-400">工作区</span>
        <span className="text-xs text-zinc-600">{workspaces.length} 个</span>
      </div>

      <div className="max-h-48 overflow-y-auto rounded-xl border border-white/10 bg-zinc-950">
        {workspaces.length ? workspaces.map((workspace) => (
          <div key={workspace.path} className="group flex items-center gap-3 border-b border-white/5 px-3 py-3 last:border-b-0">
            <Folder className="h-5 w-5 shrink-0 text-indigo-400" />
            <span className="min-w-0 flex-1">
              <span className="block truncate text-sm font-medium text-zinc-200">{workspaceName(workspace.path)}</span>
              <span className="block truncate text-xs text-zinc-600" title={workspace.path}>{workspace.path}</span>
            </span>
            <button
              type="button"
              className="icon-button text-zinc-600 hover:text-red-300"
              onClick={() => onChange(workspaces.filter((item) => item.path !== workspace.path))}
              disabled={disabled}
              aria-label={`移除工作区 ${workspaceName(workspace.path)}`}
              title="从列表移除，不会删除本地文件"
            >
              <Trash2 className="h-4 w-4" />
            </button>
          </div>
        )) : (
          <div className="px-4 py-6 text-center text-sm text-zinc-600">尚未添加工作区</div>
        )}
      </div>

      <button type="button" className="secondary-button mt-3 w-full justify-center" onClick={() => void addWorkspaces()} disabled={disabled || selecting}>
        {selecting ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <FolderPlus className="h-4 w-4" />}
        添加工作区
      </button>
      <p className="mt-2 text-xs text-zinc-600">移除工作区只会更新搜索范围，不会删除本地文件。</p>
    </section>
  );
}
