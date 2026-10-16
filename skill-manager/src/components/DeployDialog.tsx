/**
 * 部署弹窗：单项/多技能 × 多目标的部署与撤销。
 *
 * 目标列表来自内置 Agent 名单与本机检测结果；准入（域边界、支持声明）与暂停规则由 core
 * 逐项执行。批量结果逐项展示，过滤项（域边界/支持声明不适用）与暂停项分别表达，其余继续；
 * 不提供差异合并界面，暂停项只提示并可复制定位信息交给 Agent。
 * 共享目录路径设置放在这里：设置页属 M5，而 shared 目标的实际使用处就是本弹窗。
 */
import { useState } from "react";
import { Copy, FolderTree } from "lucide-react";

import {
  deployTo,
  deployUndo,
  type AgentView,
  type DeployReportView,
  type Domain,
  type ItemErrorView,
} from "../services/client";
import { targetLabel, targetOptions } from "../lib/targets";
import { Badge, Button, Dialog, Field, Notice, Segmented } from "./ui";

type Mode = "deploy" | "undo";

const ACTION_LABELS: Record<string, string> = {
  deployed: "已部署",
  updated: "已更新",
  unchanged: "未变化",
  removed: "已移除",
  would_deploy: "将部署（预演）",
  would_update: "将更新（预演）",
  would_remove: "将移除（预演）",
};

function actionLabel(action: string): string {
  return ACTION_LABELS[action] ?? action;
}

/** 暂停项：受管副本被外部修改，覆盖被暂停。 */
function isPaused(error: ItemErrorView): boolean {
  return error.message.includes("暂停");
}

export interface DeployDialogProps {
  domain: Domain;
  /** 本次处理的技能引用（单项或多选，均属于同一管理域）。 */
  references: string[];
  agents: AgentView[];
  sharedDir: string | null;
  /** 默认勾选的目标（例如从某个 Agent 页面发起）。 */
  initialTargets?: string[];
  onSharedDirChange: (path: string | null) => Promise<void>;
  onClose: () => void;
  onDone: (message: string) => void;
}

export function DeployDialog({
  domain,
  references,
  agents,
  sharedDir,
  initialTargets,
  onSharedDirChange,
  onClose,
  onDone,
}: DeployDialogProps) {
  const options = targetOptions(domain, agents);
  const detected = options.filter((option) => option.detected);
  const others = options.filter((option) => !option.detected);

  const [mode, setMode] = useState<Mode>("deploy");
  const [targets, setTargets] = useState<string[]>(initialTargets ?? []);
  const [dryRun, setDryRun] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [report, setReport] = useState<DeployReportView | null>(null);
  const [sharedInput, setSharedInput] = useState(sharedDir ?? "");
  const [sharedBusy, setSharedBusy] = useState(false);

  const needsShared = domain === "code" && targets.includes("shared") && !sharedDir;

  function toggleTarget(value: string) {
    setTargets((current) =>
      current.includes(value)
        ? current.filter((item) => item !== value)
        : [...current, value],
    );
  }

  async function run() {
    if (targets.length === 0) {
      setError("请至少选择一个部署目标。");
      return;
    }
    setBusy(true);
    setError(null);
    setReport(null);
    const [primary, ...rest] = targets;
    try {
      const result =
        mode === "deploy"
          ? await deployTo(primary, references, rest, dryRun)
          : await deployUndo(primary, references, rest, dryRun, false);
      setReport(result);
      const ok = result.items.length;
      const failed = result.errors.length;
      const verb = mode === "deploy" ? "部署" : "撤销";
      if (failed === 0) {
        onDone(`${verb}完成：处理 ${ok} 项。`);
      } else {
        onDone(
          `${verb}部分完成：处理 ${ok} 项，${failed} 项未完成（逐项原因见部署结果）。`,
        );
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function saveSharedDir() {
    setSharedBusy(true);
    setError(null);
    try {
      await onSharedDirChange(sharedInput.trim() === "" ? null : sharedInput.trim());
    } catch (err) {
      setError(String(err));
    } finally {
      setSharedBusy(false);
    }
  }

  const paused = (report?.errors ?? []).filter(isPaused);
  const filtered = (report?.errors ?? []).filter((item) => !isPaused(item));

  async function copyPaused() {
    const text = [
      "以下受管部署副本被外部修改，管理器已暂停覆盖；请检查并整理回 SkillVault 后再同步。",
      ...paused.map(
        (item) =>
          `${item.skill ?? "-"} → ${item.target ?? "-"}\n路径：${item.path ?? "未知"}\n${item.message}`,
      ),
    ].join("\n\n");
    try {
      await navigator.clipboard.writeText(text);
      onDone(`已复制 ${paused.length} 项暂停部署的定位信息。`);
    } catch (err) {
      setError(`复制失败：${String(err)}`);
    }
  }

  function renderTargetGroup(title: string, group: typeof options, count: number) {
    if (group.length === 0) {
      return null;
    }
    return (
      <div>
        <p className="mb-2 text-[12px] text-faint">
          {title} <span className="text-faint">{count}</span>
        </p>
        <div className="flex flex-wrap gap-1.5">
          {group.map((option) => {
            const active = targets.includes(option.value);
            return (
              <button
                key={option.value}
                type="button"
                onClick={() => toggleTarget(option.value)}
                className={`inline-flex items-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-[12px] transition-colors ${
                  active
                    ? "border-accent-line bg-accent-soft font-medium text-accent"
                    : "border-line bg-panel text-muted hover:bg-hover"
                }`}
              >
                <span
                  className={`size-1.5 rounded-full ${active ? "bg-accent-light" : "bg-faint"}`}
                />
                {option.label}
              </button>
            );
          })}
        </div>
      </div>
    );
  }

  return (
    <Dialog
      title={`${mode === "deploy" ? "部署" : "撤销部署"} · ${
        references.length === 1 ? references[0] : `${references.length} 个技能`
      }`}
      subtitle="按管理域与支持声明合成独立副本；不适用项逐项过滤并报告，其余继续。"
      onClose={onClose}
      footer={
        <>
          <span className="text-[12px] text-faint">
            将处理 {references.length} 个技能 × {targets.length} 个目标
          </span>
          <div className="flex gap-2.5">
            <Button onClick={onClose}>关闭</Button>
            <Button kind="primary" disabled={busy || targets.length === 0} onClick={run}>
              {busy ? "正在处理…" : dryRun ? "预演" : mode === "deploy" ? "部署" : "撤销"}
            </Button>
          </div>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <div className="flex flex-wrap items-center gap-3">
          <Segmented
            options={[
              { value: "deploy", label: "部署" },
              { value: "undo", label: "撤销" },
            ]}
            value={mode}
            onChange={(value) => setMode(value)}
          />
          <label className="flex items-center gap-1.5 text-[13px]">
            <input
              type="checkbox"
              className="selectbox"
              checked={dryRun}
              onChange={(event) => setDryRun(event.target.checked)}
            />
            预演（只报告技能、目标与路径，不修改文件与本机记录）
          </label>
        </div>

        {error && <Notice tone="danger">{error}</Notice>}

        {renderTargetGroup("已检测", detected, detected.length)}
        {renderTargetGroup("其他支持项", others, others.length)}

        {needsShared && (
          <Field label="共享技能目录（shared 是编码 Agent 的部署目标）">
            <div className="flex items-center gap-2.5">
              <input
                className="field-input"
                placeholder="例如：/Users/you/.agents/skills（绝对路径）"
                value={sharedInput}
                onChange={(event) => setSharedInput(event.target.value)}
              />
              <Button compact disabled={sharedBusy} onClick={saveSharedDir}>
                保存路径
              </Button>
            </div>
            <p className="mt-1.5 text-[12px] text-faint">
              尚未设置共享目录，shared 目标暂时无法部署；先保存绝对路径再执行。
            </p>
          </Field>
        )}

        {report && (
          <div className="flex flex-col gap-3 rounded-lg border border-line-soft p-3.5">
            <p className="text-[13px] font-medium">
              处理结果：成功 {report.items.length} 项，未完成 {report.errors.length} 项
              {report.dry_run && "（预演，未修改文件与本机记录）"}
            </p>

            {report.items.length > 0 && (
              <ul className="flex flex-col gap-1.5">
                {report.items.map((item) => (
                  <li
                    key={`${item.skill}:${item.target}`}
                    className="flex flex-wrap items-center gap-2 text-[12px] text-ink-3"
                  >
                    <Badge tone="green">{actionLabel(item.action)}</Badge>
                    <span className="font-medium">
                      {item.skill} → {targetLabel(item.target, agents)}
                    </span>
                    <span className="truncate font-mono text-[11px] text-faint">{item.path}</span>
                    {item.message && <span className="text-warn">{item.message}</span>}
                  </li>
                ))}
              </ul>
            )}

            {filtered.length > 0 && (
              <div>
                <p className="mb-1.5 text-[12px] text-faint">过滤 / 不适用（未处理，其余继续）</p>
                <ul className="flex flex-col gap-1.5">
                  {filtered.map((item, index) => (
                    <li key={`${item.skill}:${item.target}:${index}`} className="text-[12px] text-ink-3">
                      <span className="font-medium">{item.skill ?? "-"}</span>
                      {item.target && ` → ${targetLabel(item.target, agents)}`}：{item.message}
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {paused.length > 0 && (
              <div>
                <p className="mb-1.5 text-[12px] text-warn">暂停（本地副本已被外部修改，未覆盖）</p>
                <ul className="flex flex-col gap-1.5">
                  {paused.map((item, index) => (
                    <li key={`${item.skill}:${item.target}:${index}`} className="text-[12px] text-ink-3">
                      <span className="font-medium">{item.skill ?? "-"}</span>
                      {item.target && ` → ${targetLabel(item.target, agents)}`}：{item.message}
                      {item.path && (
                        <span className="ml-1 font-mono text-[11px] text-faint">{item.path}</span>
                      )}
                    </li>
                  ))}
                </ul>
                <div className="mt-2">
                  <Button compact onClick={() => void copyPaused()}>
                    <Copy className="size-3.5" strokeWidth={1.75} />
                    复制定位信息给 Agent
                  </Button>
                </div>
              </div>
            )}
          </div>
        )}

        <p className="flex items-center gap-1.5 text-[12px] text-faint">
          <FolderTree className="size-3.5 shrink-0" strokeWidth={1.75} />
          撤销保留库内技能；暂停项需由 Agent 检查、用户裁决后再显式按库覆盖，不提供合并界面。
        </p>
      </div>
    </Dialog>
  );
}
