/**
 * 诊断面板（T085/T087）：可获取的运行日志、崩溃记录与诊断导出。
 *
 * 内容全部来自 core 的诊断报告（已脱敏、含真实文件路径）；由用户主动获取，不自动上传。
 * 导出写出真实文件并显示其路径，供 Agent 直接读取排查。
 */
import { useCallback, useEffect, useState } from "react";
import { FileText, RefreshCw, Upload } from "lucide-react";

import {
  diagnosticsExport,
  diagnosticsReport,
  type DiagnosticsLogEntry,
  type DiagnosticsReport,
} from "../services/client";
import { Badge, Button, Notice } from "./ui";

const LEVEL_TONES: Record<string, "green" | "gray" | "amber" | "blue"> = {
  info: "gray",
  warn: "amber",
  error: "amber",
  panic: "blue",
  raw: "gray",
};

export interface DiagnosticsPanelProps {
  reloadToken: number;
}

function LogList({ entries, empty }: { entries: DiagnosticsLogEntry[]; empty: string }) {
  if (entries.length === 0) {
    return <p className="text-[12px] text-faint">{empty}</p>;
  }
  return (
    <ul className="flex max-h-[220px] flex-col gap-1 overflow-auto">
      {entries.map((entry, index) => (
        <li key={`${entry.at}:${index}`} className="flex flex-wrap items-baseline gap-2 text-[12px]">
          <Badge tone={LEVEL_TONES[entry.level] ?? "gray"}>{entry.level}</Badge>
          <span className="shrink-0 font-mono text-[11px] text-faint">{entry.at}</span>
          <span className="text-muted">{entry.action}</span>
          <span className="min-w-0 flex-1 break-all text-ink-3">{entry.message}</span>
        </li>
      ))}
    </ul>
  );
}

export function DiagnosticsPanel({ reloadToken }: DiagnosticsPanelProps) {
  const [report, setReport] = useState<DiagnosticsReport | null>(null);
  const [exported, setExported] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      setReport(await diagnosticsReport(100));
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load, reloadToken]);

  async function runExport() {
    setBusy(true);
    setError(null);
    try {
      const result = await diagnosticsExport(200);
      setReport(result.report);
      setExported(result.path);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  if (!report) {
    return error ? <Notice tone="danger">{error}</Notice> : <p className="text-[12px] text-faint">正在读取诊断资料…</p>;
  }

  return (
    <div className="flex flex-col gap-3.5">
      {error && <Notice tone="danger">{error}</Notice>}

      <div className="grid grid-cols-2 gap-x-5 gap-y-1.5 text-[12px]">
        <span className="text-faint">应用版本</span>
        <span className="text-ink-3">{report.version}</span>
        <span className="text-faint">平台</span>
        <span className="text-ink-3">
          {report.os} / {report.arch}
        </span>
        <span className="text-faint">SkillVault</span>
        <span className="break-all font-mono text-[11px] text-ink-3">
          {report.vault ?? "（尚未接入）"}
        </span>
        <span className="text-faint">状态目录</span>
        <span className="break-all font-mono text-[11px] text-ink-3">{report.state_dir}</span>
        <span className="text-faint">运行日志</span>
        <span className="break-all font-mono text-[11px] text-ink-3">{report.log_path}</span>
        <span className="text-faint">崩溃记录</span>
        <span className="break-all font-mono text-[11px] text-ink-3">{report.crash_path}</span>
        <span className="text-faint">最近检查 / 同步</span>
        <span className="text-ink-3">
          {report.last_check_at ?? "（无记录）"} / {report.last_sync_at ?? "（无记录）"}
        </span>
      </div>

      <div className="flex flex-wrap items-center gap-2.5">
        <Button compact disabled={busy} onClick={() => void runExport()}>
          <Upload className="size-3.5" strokeWidth={1.75} />
          {busy ? "正在导出…" : "导出诊断资料"}
        </Button>
        <Button compact disabled={busy} onClick={() => void load()}>
          <RefreshCw className="size-3.5" strokeWidth={1.75} />
          刷新
        </Button>
      </div>

      {exported && (
        <Notice>
          已导出诊断资料到真实文件：
          <span className="ml-1 break-all font-mono text-[11px]">{exported}</span>
        </Notice>
      )}

      <div>
        <p className="mb-1.5 flex items-center gap-1.5 text-[12px] text-faint">
          <FileText className="size-3.5 shrink-0" strokeWidth={1.75} />
          最近运行日志（含部署异常与检查失败）
        </p>
        <LogList entries={report.recent_logs} empty="暂无运行日志。" />
      </div>

      <div>
        <p className="mb-1.5 text-[12px] text-faint">崩溃记录</p>
        <LogList entries={report.crashes} empty="没有崩溃记录。" />
      </div>

      <ul className="flex flex-col gap-1 text-[12px] text-faint">
        {report.notes.map((note) => (
          <li key={note}>· {note}</li>
        ))}
      </ul>
    </div>
  );
}
