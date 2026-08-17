import { Box, ChevronRight, EyeOff, FileCode2, FileText, Folder, Link2, ListTree, X } from "lucide-react";
import type { DirectoryPresentation, PresentationEntry, PresentationOutput, TextFilePresentation } from "../lib/types";

type PresentationViewProps = {
  output: PresentationOutput;
  onClose: () => void;
  onOpenEntry: (entry: PresentationEntry) => void;
};

const dateFormatter = new Intl.DateTimeFormat("zh-CN", {
  month: "2-digit",
  day: "2-digit",
  hour: "2-digit",
  minute: "2-digit",
});

function formatBytes(bytes: number | null) {
  if (bytes === null) return "—";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
}

function formatDate(timestamp: number | null) {
  return timestamp ? dateFormatter.format(new Date(timestamp * 1000)) : "—";
}

function EntryIcon({ kind }: { kind: PresentationEntry["kind"] }) {
  if (kind === "directory") return <Folder className="h-[18px] w-[18px] text-indigo-400" />;
  if (kind === "file") return <FileText className="h-[18px] w-[18px] text-sky-400" />;
  if (kind === "symlink") return <Link2 className="h-[18px] w-[18px] text-amber-400" />;
  return <Box className="h-[18px] w-[18px] text-zinc-500" />;
}

function Header({ title, path, onClose }: { title: string; path: string; onClose: () => void }) {
  return (
    <div className="border-b border-white/10 px-4 py-3">
      <div className="flex items-start gap-3">
        <div className="min-w-0 flex-1">
          <div className="truncate text-sm font-semibold text-zinc-100">{title}</div>
          <div className="mt-0.5 truncate text-xs text-zinc-600" title={path}>{path}</div>
        </div>
        <button className="icon-button -mr-1 -mt-1" onClick={onClose} aria-label="关闭展示结果"><X className="h-4 w-4" /></button>
      </div>
    </div>
  );
}

function DirectoryView({ output, onClose, onOpenEntry }: { output: DirectoryPresentation } & Omit<PresentationViewProps, "output">) {
  return (
    <section className="presentation-panel">
      <Header title={output.path.split("/").pop() || output.path} path={output.path} onClose={onClose} />
      <div className="flex flex-wrap items-center gap-2 border-b border-white/5 px-4 py-2.5 text-xs text-zinc-500">
        <span className="presentation-badge"><Folder className="h-3.5 w-3.5" />{output.directoryCount} 个文件夹</span>
        <span className="presentation-badge"><FileText className="h-3.5 w-3.5" />{output.fileCount} 个文件</span>
        {output.hiddenCount > 0 && <span className="presentation-badge"><EyeOff className="h-3.5 w-3.5" />{output.hiddenCount} 个隐藏项</span>}
        {output.detailed && <span className="ml-auto flex items-center gap-1 text-indigo-300"><ListTree className="h-3.5 w-3.5" />详细视图</span>}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        {output.detailed && output.entries.length > 0 && (
          <div className="flex items-center gap-3 px-3 pb-1.5 text-[10px] uppercase tracking-wider text-zinc-700">
            <span className="w-[18px]" /><span className="min-w-0 flex-1">名称</span><span className="w-20 text-right">大小</span><span className="w-24 text-right">修改时间</span><span className="w-4" />
          </div>
        )}
        {output.entries.length ? output.entries.map((entry) => {
          const navigable = entry.kind === "directory" || entry.kind === "file";
          return (
            <button key={entry.path} className="group flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left transition hover:bg-white/5 disabled:cursor-default" onClick={() => navigable && onOpenEntry(entry)} disabled={!navigable}>
              <EntryIcon kind={entry.kind} />
              <span className={`min-w-0 flex-1 truncate text-sm ${entry.hidden ? "text-zinc-500" : "text-zinc-300"}`}>{entry.name}</span>
              {output.detailed && <><span className="w-20 text-right text-xs tabular-nums text-zinc-600">{formatBytes(entry.size)}</span><span className="w-24 text-right text-xs tabular-nums text-zinc-600">{formatDate(entry.modifiedAt)}</span></>}
              {navigable && <ChevronRight className="h-4 w-4 text-zinc-700 transition group-hover:text-zinc-400" />}
            </button>
          );
        }) : <div className="grid place-items-center px-6 py-16 text-sm text-zinc-600">这个目录是空的</div>}
        {output.truncated && <div className="mx-3 my-2 rounded-lg bg-amber-400/10 px-3 py-2 text-xs text-amber-200/70">目录项目较多，仅展示前 500 项。</div>}
      </div>
    </section>
  );
}

function TextFileView({ output, onClose }: { output: TextFilePresentation; onClose: () => void }) {
  const lines = output.content === "" ? [] : output.content.split(/\r?\n/);
  if (output.content.endsWith("\n")) lines.pop();
  return (
    <section className="presentation-panel">
      <Header title={output.name} path={output.path} onClose={onClose} />
      <div className="flex items-center gap-2 border-b border-white/5 px-4 py-2.5 text-xs text-zinc-500">
        <span className="presentation-badge"><FileCode2 className="h-3.5 w-3.5" />{output.language ?? "文本"}</span>
        <span>{output.lineCount} 行</span><span>·</span><span>{formatBytes(output.size)}</span>
      </div>
      <div className="min-h-0 flex-1 overflow-auto bg-zinc-950/55 py-3 font-mono text-[12px] leading-5">
        {lines.length ? lines.map((line, index) => (
          <div className="flex min-w-max px-3 hover:bg-white/[0.025]" key={index}>
            <span className="mr-4 w-10 shrink-0 select-none text-right tabular-nums text-zinc-700">{index + 1}</span>
            <span className="whitespace-pre text-zinc-300">{line || " "}</span>
          </div>
        )) : <div className="grid place-items-center px-6 py-16 font-sans text-sm text-zinc-600">这个文件是空的</div>}
      </div>
    </section>
  );
}

export function PresentationView({ output, onClose, onOpenEntry }: PresentationViewProps) {
  return output.type === "directory"
    ? <DirectoryView output={output} onClose={onClose} onOpenEntry={onOpenEntry} />
    : <TextFileView output={output} onClose={onClose} />;
}
