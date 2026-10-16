/**
 * 预设部署 / 撤销弹窗（US7）。
 *
 * 按预设**当前成员**叠加部署或逐个撤销；准入（域边界、支持声明）与暂停规则仍由 core 逐项执行，
 * 前端只展示结果、不复制业务判断。撤销时如实说明影响范围：同一技能属于多个预设时也会一并撤销
 * （不追踪来源预设或引用计数）。
 */
import { useState } from "react";
import { FolderTree } from "lucide-react";

import {
  deployPreset,
  deployUndoPreset,
  type AgentView,
  type DeployReportView,
  type Domain,
  type PresetView,
} from "../services/client";
import { targetLabel, targetOptions } from "../lib/targets";
import { Badge, Button, Dialog, Notice, Segmented } from "./ui";

type Mode = "deploy" | "undo";

const ACTION_LABELS: Record<string, string> = {
  deployed: "已部署",
  updated: "已更新",
  removed: "已移除",
  would_deploy: "将部署（预演）",
  would_update: "将更新（预演）",
  would_remove: "将移除（预演）",
};

function actionLabel(action: string): string {
  return ACTION_LABELS[action] ?? action;
}

export interface PresetDeployDialogProps {
  domain: Domain;
  preset: PresetView;
  agents: AgentView[];
  /** 同时属于其他预设的成员名（用于如实说明撤销影响范围）。 */
  sharedMembers: string[];
  /**
   * 本次选中的成员全部不受当前系统支持时的原因；只阻止新增部署，撤销不受影响。
   * 为空表示可正常发起组合部署。
   */
  unsupportedReason?: string | null;
  onClose: () => void;
  onDone: (message: string) => void;
}

export function PresetDeployDialog({
  domain,
  preset,
  agents,
  sharedMembers,
  unsupportedReason,
  onClose,
  onDone,
}: PresetDeployDialogProps) {
  const options = targetOptions(domain, agents);
  const detected = options.filter((option) => option.detected);
  const others = options.filter((option) => !option.detected);
  const [mode, setMode] = useState<Mode>("deploy");
  const [targets, setTargets] = useState<string[]>([]);
  const [dryRun, setDryRun] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [report, setReport] = useState<DeployReportView | null>(null);

  const members = preset.members.map((member) => member.name);
  const missing = preset.members.filter((member) => member.missing).map((m) => m.name);
  const deployBlocked = mode === "deploy" && !!unsupportedReason;

  function toggleTarget(value: string) {
    setTargets((current) =>
      current.includes(value) ? current.filter((item) => item !== value) : [...current, value],
    );
  }

  async function run() {
    if (targets.length === 0) {
      setError("请至少选择一个目标。");
      return;
    }
    setBusy(true);
    setError(null);
    setReport(null);
    const [primary, ...rest] = targets;
    try {
      const result =
        mode === "deploy"
          ? await deployPreset(domain, preset.id, primary, rest, dryRun)
          : await deployUndoPreset(domain, preset.id, primary, rest, dryRun);
      setReport(result);
      const verb = mode === "deploy" ? "叠加部署" : "按成员撤销";
      onDone(
        result.errors.length === 0
          ? `${verb}完成：处理 ${result.items.length} 项。`
          : `${verb}部分完成：处理 ${result.items.length} 项，${result.errors.length} 项未完成。`,
      );
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  function renderTargetGroup(title: string, group: typeof options) {
    if (group.length === 0) {
      return null;
    }
    return (
      <div>
        <p className="mb-2 text-[12px] text-faint">
          {title} <span className="text-faint">{group.length}</span>
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
      title={`${mode === "deploy" ? "组合部署" : "按成员撤销"} · ${preset.name}`}
      subtitle="按当前成员逐个处理；组合编辑不自动部署或撤销。"
      onClose={onClose}
      footer={
        <>
          <span className="text-[12px] text-faint">
            将处理 {members.length} 个成员 × {targets.length} 个目标
          </span>
          <div className="flex gap-2.5">
            <Button onClick={onClose}>关闭</Button>
            <Button
              kind="primary"
              disabled={busy || deployBlocked || targets.length === 0}
              title={deployBlocked ? unsupportedReason ?? undefined : undefined}
              onClick={run}
            >
              {busy ? "正在处理…" : dryRun ? "预演" : mode === "deploy" ? "叠加部署" : "撤销"}
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
            预演（只报告，不修改文件与本机记录）
          </label>
        </div>

        {error && <Notice tone="danger">{error}</Notice>}

        {deployBlocked && <Notice tone="warn">{unsupportedReason}</Notice>}

        {mode === "undo" && sharedMembers.length > 0 && (
          <Notice tone="warn">
            以下成员同时属于其他预设，撤销会一并移除它们的受管副本（不追踪来源预设）：
            {sharedMembers.join("、")}。
          </Notice>
        )}
        {missing.length > 0 && (
          <Notice tone="warn">
            缺失成员（库内已无同名技能）：{missing.join("、")}；部署时逐项报告，不阻塞其他成员。
          </Notice>
        )}

        {renderTargetGroup("已检测", detected)}
        {renderTargetGroup("其他支持项", others)}

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
                  </li>
                ))}
              </ul>
            )}
            {report.errors.length > 0 && (
              <ul className="flex flex-col gap-1.5">
                {report.errors.map((item, index) => (
                  <li key={`${item.skill}:${item.target}:${index}`} className="text-[12px] text-ink-3">
                    <span className="font-medium">{item.skill ?? "-"}</span>
                    {item.target && ` → ${targetLabel(item.target, agents)}`}：{item.message}
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}

        <p className="flex items-center gap-1.5 text-[12px] text-faint">
          <FolderTree className="size-3.5 shrink-0" strokeWidth={1.75} />
          部署是叠加：保留目标已有的其他技能；撤销保留库内技能。
        </p>
      </div>
    </Dialog>
  );
}
