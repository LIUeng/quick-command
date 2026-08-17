import { Check, FolderPlus, ShieldCheck, X } from "lucide-react";
import type { OperationConfirmation } from "../lib/types";

type ConfirmationViewProps = {
  confirmation: OperationConfirmation;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
};

export function OperationConfirmationView({ confirmation, busy, onCancel, onConfirm }: ConfirmationViewProps) {
  const confirmLabel = confirmation.kind === "create-project-directory-and-open" ? "创建并打开" : "创建目录";
  return (
    <section className="grid h-full place-items-center px-6 py-8">
      <div className="w-full max-w-lg rounded-2xl border border-amber-300/15 bg-amber-300/[0.055] p-6">
        <div className="flex items-start gap-4">
          <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-amber-300/10 text-amber-200"><FolderPlus className="h-5 w-5" /></span>
          <div className="min-w-0 flex-1">
            <h2 className="font-semibold text-zinc-100">{confirmation.title}</h2>
            <p className="mt-1 text-sm leading-5 text-zinc-500">{confirmation.description}</p>
          </div>
        </div>
        <div className="mt-5 rounded-xl border border-white/10 bg-zinc-950/65 p-4">
          <div className="text-[10px] uppercase tracking-widest text-zinc-600">最终路径</div>
          <div className="mt-1 break-all font-mono text-xs leading-5 text-zinc-300">{confirmation.targetPath}</div>
          {confirmation.pathsToCreate.length > 0 && (
            <div className="mt-3 border-t border-white/5 pt-3">
              <div className="text-[10px] uppercase tracking-widest text-zinc-600">将创建 {confirmation.pathsToCreate.length} 个目录</div>
              <div className="mt-2 grid gap-1.5">
                {confirmation.pathsToCreate.map((path) => <div className="break-all font-mono text-[11px] leading-4 text-zinc-500" key={path}>{path}</div>)}
              </div>
            </div>
          )}
          <div className="mt-3 flex items-center gap-2 border-t border-white/5 pt-3 text-xs text-zinc-600"><ShieldCheck className="h-4 w-4 text-emerald-400" /><span className="truncate" title={confirmation.workspacePath}>限制在工作区 {confirmation.workspacePath}</span></div>
        </div>
        <div className="mt-5 flex items-center justify-between">
          <span className="text-xs text-zinc-600">Enter 确认 · Esc 取消</span>
          <div className="flex gap-2">
            <button className="secondary-button" onClick={onCancel} disabled={busy}>取消</button>
            <button className="primary-button inline-flex items-center gap-2" onClick={onConfirm} disabled={busy}><FolderPlus className="h-4 w-4" />{confirmLabel}</button>
          </div>
        </div>
      </div>
    </section>
  );
}

type CompletedViewProps = {
  title: string;
  message: string;
  path: string;
  onClose: () => void;
};

export function OperationCompletedView({ title, message, path, onClose }: CompletedViewProps) {
  return (
    <section className="grid h-full place-items-center px-6 py-10">
      <div className="relative w-full max-w-md rounded-2xl border border-emerald-400/15 bg-emerald-400/[0.06] px-6 py-7 text-center">
        <button className="icon-button absolute right-2 top-2" onClick={onClose} aria-label="关闭操作结果"><X className="h-4 w-4" /></button>
        <span className="mx-auto flex h-12 w-12 items-center justify-center rounded-2xl bg-emerald-400/10 text-emerald-300"><Check className="h-6 w-6" /></span>
        <div className="mt-4 text-sm font-medium text-emerald-200">{title}</div>
        <div className="mt-2 break-all font-mono text-xs leading-5 text-zinc-300">{path}</div>
        <p className="mt-4 text-xs leading-5 text-zinc-500">{message}</p>
      </div>
    </section>
  );
}
