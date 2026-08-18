import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { CheckCircle2, Download, LoaderCircle, RefreshCw } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { loadUpdateConfiguration } from "../lib/api";
import type { UpdateConfiguration } from "../lib/types";

type UpdatePhase = "idle" | "checking" | "latest" | "available" | "downloading" | "restarting" | "error";

function updateErrorMessage(error: unknown) {
  if (import.meta.env.DEV) {
    return error instanceof Error ? error.message : String(error);
  }
  return "暂时无法完成更新，请检查网络后重试，或前往 GitHub Releases 手动下载。";
}

export function UpdateSection() {
  const updateRef = useRef<Update | null>(null);
  const [configuration, setConfiguration] = useState<UpdateConfiguration | null>(null);
  const [phase, setPhase] = useState<UpdatePhase>("idle");
  const [availableVersion, setAvailableVersion] = useState<string | null>(null);
  const [releaseNotes, setReleaseNotes] = useState<string | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    loadUpdateConfiguration()
      .then(setConfiguration)
      .catch((error) => {
        setPhase("error");
        setMessage(updateErrorMessage(error));
      });

    return () => {
      void updateRef.current?.close();
    };
  }, []);

  async function checkForUpdate() {
    if (!configuration?.configured) {
      setPhase("error");
      setMessage("当前构建尚未配置更新签名，仍可从 GitHub Releases 手动下载安装。");
      return;
    }

    setPhase("checking");
    setMessage(null);
    setProgress(null);
    try {
      if (updateRef.current) await updateRef.current.close();
      const update = await check({ timeout: 15_000 });
      updateRef.current = update;
      if (!update) {
        setAvailableVersion(null);
        setReleaseNotes(null);
        setPhase("latest");
        return;
      }

      setAvailableVersion(update.version);
      setReleaseNotes(update.body?.trim() || null);
      setPhase("available");
    } catch (error) {
      setPhase("error");
      setMessage(updateErrorMessage(error));
    }
  }

  async function installUpdate() {
    const update = updateRef.current;
    if (!update) return;

    let downloaded = 0;
    let total: number | undefined;
    setPhase("downloading");
    setProgress(0);
    setMessage(null);
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength;
          downloaded = 0;
          setProgress(total ? 0 : null);
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          if (total) setProgress(Math.min(100, Math.round((downloaded / total) * 100)));
        } else {
          setProgress(100);
        }
      });
      updateRef.current = null;
      setPhase("restarting");
      await relaunch();
    } catch (error) {
      setPhase("error");
      setMessage(updateErrorMessage(error));
    }
  }

  const busy = phase === "checking" || phase === "downloading" || phase === "restarting";

  return (
    <section className="mt-5 border-t border-white/10 pt-5">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h3 className="text-sm font-medium text-zinc-300">软件更新</h3>
          <p className="mt-1 text-xs text-zinc-600">当前版本 v{configuration?.currentVersion ?? "—"}</p>
        </div>
        {phase !== "available" && (
          <button type="button" className="secondary-button shrink-0" onClick={() => void checkForUpdate()} disabled={busy || !configuration}>
            {phase === "checking" ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <RefreshCw className="h-4 w-4" />}
            {phase === "checking" ? "检查中" : "检查更新"}
          </button>
        )}
      </div>

      {phase === "latest" && (
        <div className="mt-3 flex items-center gap-2 rounded-xl border border-emerald-400/15 bg-emerald-400/5 px-3 py-2.5 text-sm text-emerald-200">
          <CheckCircle2 className="h-4 w-4 shrink-0" />当前已是最新版本
        </div>
      )}

      {phase === "available" && availableVersion && (
        <div className="mt-3 rounded-xl border border-indigo-400/20 bg-indigo-400/5 p-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="text-sm font-medium text-zinc-100">发现 v{availableVersion}</p>
              <p className="mt-1 text-xs text-zinc-500">安装完成后 Quick Command 将自动重启</p>
            </div>
            <button type="button" className="primary-button inline-flex shrink-0 items-center gap-2" onClick={() => void installUpdate()}>
              <Download className="h-4 w-4" />下载并安装
            </button>
          </div>
          {releaseNotes && <p className="mt-3 max-h-28 overflow-y-auto whitespace-pre-wrap border-t border-white/5 pt-3 text-xs leading-5 text-zinc-400">{releaseNotes}</p>}
        </div>
      )}

      {phase === "downloading" && (
        <div className="mt-3 rounded-xl border border-white/10 bg-zinc-950 px-3 py-3">
          <div className="flex items-center justify-between text-xs text-zinc-400">
            <span>正在下载并安装更新…</span><span>{progress === null ? "—" : `${progress}%`}</span>
          </div>
          <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-white/5">
            <div className={`h-full rounded-full bg-indigo-400 transition-[width] ${progress === null ? "w-1/3 animate-pulse" : ""}`} style={progress === null ? undefined : { width: `${progress}%` }} />
          </div>
        </div>
      )}

      {phase === "restarting" && <p className="mt-3 text-xs text-zinc-400">更新已安装，正在重新启动…</p>}
      {phase === "error" && message && <div className="mt-3 rounded-xl border border-red-400/20 bg-red-400/10 px-3 py-2.5 text-xs leading-5 text-red-200">{message}</div>}
      {configuration && !configuration.configured && phase === "idle" && (
        <p className="mt-3 text-xs leading-5 text-zinc-600">开发构建未注入更新公钥；正式发布构建会启用应用内更新。</p>
      )}
    </section>
  );
}
