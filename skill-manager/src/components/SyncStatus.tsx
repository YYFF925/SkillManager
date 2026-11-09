/**
 * 同步状态（T055）：连接状态、库变化与上游变化、最近同步时间、冲突提示。
 *
 * 只读 core 的 `sync status` / `sync conflicts` 事实；冲突只提示，并给出可复制给 Agent 的
 * 定位信息，不提供专用合并界面。未连接时说明本地管理与部署仍可继续。
 */
import { useCallback, useEffect, useState } from "react";
import { Copy, RefreshCw } from "lucide-react";

import { Badge, Button, Notice } from "./ui";
import {
  syncConflicts,
  syncStatus,
  type ConflictView,
  type SyncStatusView,
} from "../services/client";

export interface SyncStatusProps {
  /** 外部触发重新读取（上传/拉取/重试后递增）。 */
  reloadToken?: number;
  /** 最近一次上传失败的可读原因；无失败为 null。 */
  failure?: string | null;
  /** 显式重试最近一次已同意的上传。 */
  onRetryUpload?: () => Promise<void>;
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-4 border-b border-line-faint px-4 py-3 last:border-b-0">
      <span className="shrink-0 text-[13px] text-muted">{label}</span>
      <div className="min-w-0 flex-1 text-right text-[13px] text-ink-3">{children}</div>
    </div>
  );
}

export function SyncStatus({ reloadToken = 0, failure = null, onRetryUpload }: SyncStatusProps) {
  const [status, setStatus] = useState<SyncStatusView | null>(null);
  const [conflict, setConflict] = useState<ConflictView | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retrying, setRetrying] = useState(false);
  const [copied, setCopied] = useState(false);

  const reload = useCallback(async () => {
    try {
      const view = await syncStatus();
      setStatus(view);
      setError(null);
      setConflict(view.conflict ? await syncConflicts() : null);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload, reloadToken]);

  async function retryUpload() {
    if (!onRetryUpload) {
      return;
    }
    setRetrying(true);
    try {
      await onRetryUpload();
      await reload();
    } finally {
      setRetrying(false);
    }
  }

  async function copyConflict() {
    if (!conflict) {
      return;
    }
    const text = [
      conflict.message,
      ...conflict.conflicts,
      ...conflict.status,
    ]
      .filter((line) => line.trim().length > 0)
      .join("\n");
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      setError("复制失败，请手动选择下方信息复制。");
    }
  }

  return (
    <div className="flex flex-col gap-3">
      {error && <Notice tone="danger">{error}</Notice>}

      <div className="flex items-center justify-between">
        <span className="text-[13px] text-muted">同步状态</span>
        <Button compact disabled={loading} onClick={() => void reload()}>
          <RefreshCw className="size-3.5" strokeWidth={1.75} />
          刷新
        </Button>
      </div>

      {status && (
        <div className="rounded-[10px] border border-line bg-panel">
          <Row label="连接状态">
            {status.connected ? (
              <Badge tone="green">已连接</Badge>
            ) : (
              <Badge tone="amber">未连接</Badge>
            )}
          </Row>
          <Row label="远端">
            <span className="break-all font-mono text-[12px]">
              {status.remote ?? "未配置远端"}
            </span>
          </Row>
          <Row label="分支">
            {status.branch ?? "—"}
            {status.upstream ? ` · 跟踪 ${status.upstream}` : " · 无上游跟踪"}
          </Row>
          <Row label="库变化（本机）">
            {status.uncommitted.length > 0 ? (
              <span className="break-all">
                有 {status.uncommitted.length} 项未提交改动：
                {status.uncommitted.slice(0, 5).join("、")}
                {status.uncommitted.length > 5 ? " 等" : ""}
                。与远端同一文件重叠时拉取会暂停，请先上传或丢弃这些改动。
              </span>
            ) : (
              "无未提交改动"
            )}
          </Row>
          <Row label="上游变化">
            {status.behind === null || status.behind === undefined ? (
              "无上游信息"
            ) : status.behind > 0 ? (
              <span className="text-ink-2">远端有 {status.behind} 个提交待拉取</span>
            ) : status.ahead && status.ahead > 0 ? (
              `本机领先 ${status.ahead} 个提交待上传`
            ) : (
              "与远端一致"
            )}
          </Row>
          <Row label="最近同步">
            {status.last_sync_at ?? "尚未成功拉取"}
            <span className="ml-1 text-[11px] text-faint">（仅记录最近一次拉取）</span>
          </Row>
        </div>
      )}

      {status && !status.connected && (
        <Notice tone="warn">
          远端不可用，无法拉取和上传。
          {status.connection_error ? `原因：${status.connection_error}` : ""}
        </Notice>
      )}

      {status?.conflict && (
        <Notice tone="danger">
          <div className="flex items-center justify-between gap-3">
            <span>库同步发生冲突，本次同步已暂停；请由 Agent 检查后裁决并继续。</span>
            <Button compact onClick={() => void copyConflict()}>
              <Copy className="size-3.5" strokeWidth={1.75} />
              {copied ? "已复制" : "复制定位信息"}
            </Button>
          </div>
          {conflict && (
            <pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap font-mono text-[11px]">
              {[conflict.message, ...conflict.conflicts, ...conflict.status]
                .filter((line) => line.trim().length > 0)
                .join("\n")}
            </pre>
          )}
        </Notice>
      )}

      {failure && (
        <Notice tone="danger">
          <div className="flex items-center justify-between gap-3">
            <span>上次上传失败：{failure}</span>
            {onRetryUpload && (
              <Button compact disabled={retrying} onClick={() => void retryUpload()}>
                <RefreshCw className="size-3.5" strokeWidth={1.75} />
                {retrying ? "正在重试…" : "重试上传"}
              </Button>
            )}
          </div>
        </Notice>
      )}
    </div>
  );
}
