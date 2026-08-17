import { Check, FolderOpen, X } from "lucide-react";

type ContextUpdateViewProps = {
  path: string;
  onClose: () => void;
};

export function ContextUpdateView({ path, onClose }: ContextUpdateViewProps) {
  const name = path.split("/").pop() || path;
  return (
    <section className="grid h-full place-items-center px-6 py-10">
      <div className="relative w-full max-w-md rounded-2xl border border-emerald-400/15 bg-emerald-400/[0.06] px-6 py-7 text-center">
        <button className="icon-button absolute right-2 top-2" onClick={onClose} aria-label="关闭上下文提示"><X className="h-4 w-4" /></button>
        <span className="mx-auto flex h-12 w-12 items-center justify-center rounded-2xl bg-emerald-400/10 text-emerald-300">
          <FolderOpen className="h-6 w-6" />
        </span>
        <div className="mt-4 flex items-center justify-center gap-1.5 text-sm font-medium text-emerald-200"><Check className="h-4 w-4" />已切换活动上下文</div>
        <div className="mt-2 truncate text-base font-semibold text-zinc-100">{name}</div>
        <div className="mt-1 truncate text-xs text-zinc-500" title={path}>{path}</div>
        <p className="mt-4 text-xs leading-5 text-zinc-500">后续相对路径命令将从这个目录开始解析。</p>
      </div>
    </section>
  );
}
