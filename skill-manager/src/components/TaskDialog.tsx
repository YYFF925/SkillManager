/**
 * 维护任务说明弹窗（US9/T078）。
 *
 * 只生成可复制文本：不调用 Agent、不建任务队列、不安装、不改库、不上传。
 * 生成不代表已开始或已完成；实际执行由外部 Agent 依 CLI 与配套维护 Skill 完成。
 */
import { useCallback, useEffect, useState } from "react";
import { Copy, RefreshCw } from "lucide-react";

import { Button, Dialog, Notice } from "./ui";

export interface TaskDialogProps {
  title: string;
  subtitle?: string;
  requirementLabel?: string;
  requirementPlaceholder?: string;
  /** 依据用户要求生成任务文本。 */
  generate: (requirement: string) => Promise<string>;
  onClose: () => void;
  /** 生成成功后的侧栏提示（可选）。 */
  onNotice?: (message: string) => void;
}

export function TaskDialog({
  title,
  subtitle,
  requirementLabel = "希望 Agent 做什么",
  requirementPlaceholder = "例如：安装到最新版本，并核实回写本机版本与配置位置。",
  generate,
  onClose,
  onNotice,
}: TaskDialogProps) {
  const [requirement, setRequirement] = useState("");
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const run = useCallback(
    async (current: string) => {
      setBusy(true);
      setCopied(false);
      try {
        const generated = await generate(current);
        setText(generated);
        setError(null);
        return generated;
      } catch (err) {
        setError(String(err));
        return null;
      } finally {
        setBusy(false);
      }
    },
    [generate],
  );

  useEffect(() => {
    void run("");
  }, [run]);

  async function copy() {
    const generated = await run(requirement);
    if (generated == null) {
      return;
    }
    try {
      await navigator.clipboard.writeText(generated);
      setCopied(true);
      onNotice?.("任务说明已复制到剪贴板；请交给外部 Agent 执行，生成任务不代表已完成。");
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <Dialog
      title={title}
      subtitle={subtitle ?? "复制任务说明交给外部 Agent，由 Agent 实际执行与回写。"}
      width="w-[880px]"
      onClose={onClose}
      footer={
        <>
          <span className="text-[12px] text-faint">
            这里只生成可复制的任务说明：尚未调用 Agent 或执行维护。
          </span>
          <div className="flex shrink-0 items-center gap-2.5">
            <Button onClick={onClose}>关闭</Button>
            <Button disabled={busy} onClick={() => void run(requirement)}>
              <RefreshCw className="size-3.5" strokeWidth={1.75} />
              按当前要求刷新
            </Button>
            <Button kind="primary" disabled={busy} onClick={() => void copy()}>
              <Copy className="size-3.5" strokeWidth={1.75} />
              {copied ? "已复制" : "复制任务说明"}
            </Button>
          </div>
        </>
      }
    >
      {error && (
        <div className="mb-4">
          <Notice tone="danger">生成任务说明失败：{error}</Notice>
        </div>
      )}

      <div className="rounded-[10px] border border-line p-3.5">
        <label className="mb-2.5 block text-[12px] text-faint">{requirementLabel}</label>
        <textarea
          value={requirement}
          placeholder={requirementPlaceholder}
          onChange={(event) => {
            setRequirement(event.target.value);
            setCopied(false);
          }}
          rows={3}
          className="w-full resize-y rounded-lg border border-line bg-panel p-3 text-[13px] leading-[21px] text-ink-3 outline-0 focus:border-accent"
        />
      </div>

      <div className="mt-3.5 rounded-[10px] border border-line p-3.5">
        <div className="mb-2.5 flex items-center justify-between">
          <span className="text-[12px] text-faint">任务说明 · 随上方要求更新</span>
          {busy && <span className="text-[12px] text-muted">正在生成…</span>}
        </div>
        <pre className="max-h-[46vh] overflow-auto rounded-lg bg-subtle p-3 text-[12px] leading-[19px] whitespace-pre-wrap break-all text-ink-3">
          {text || (busy ? "" : "尚未生成任务说明。")}
        </pre>
      </div>
    </Dialog>
  );
}
