import { ArrowLeft, Check, Folder } from "lucide-react";
import type { Workspace } from "../lib/types";

type WorkspacePickerProps = {
  title: string;
  workspaces: Workspace[];
  activeContext: string | null;
  selected: number;
  onBack: () => void;
  onSelect: (workspace: Workspace) => void;
};

export function WorkspacePicker({ title, workspaces, activeContext, selected, onBack, onSelect }: WorkspacePickerProps) {
  return (
    <div>
      <button className="mb-1 flex items-center gap-2 rounded-lg px-3 py-2 text-xs text-zinc-500 hover:bg-white/5 hover:text-zinc-300" onClick={onBack}>
        <ArrowLeft className="h-4 w-4" />返回
      </button>
      <div className="px-3 pb-2 text-xs uppercase tracking-widest text-zinc-600">{title}</div>
      {workspaces.map((workspace, index) => {
        const isActive = activeContext === workspace.path || activeContext?.startsWith(`${workspace.path}/`);
        return (
          <button key={workspace.path} className={`result-row ${selected === index ? "result-row-active" : ""}`} onClick={() => onSelect(workspace)}>
            <Folder className="h-5 w-5 text-indigo-400" />
            <span className="min-w-0 flex-1 text-left">
              <span className="block truncate font-medium">{workspace.path.split("/").pop()}</span>
              <span className="block truncate text-xs text-zinc-500">{workspace.path}</span>
            </span>
            {isActive && <span className="flex items-center gap-1 text-xs text-emerald-400"><Check className="h-3.5 w-3.5" />当前</span>}
          </button>
        );
      })}
    </div>
  );
}
