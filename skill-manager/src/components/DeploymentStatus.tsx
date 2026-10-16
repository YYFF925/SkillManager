/**
 * 技能详情的“本机已有部署”：已部署目标亮显、未部署灰暗、支持收窄警告与暂停提示。
 *
 * 部署记录与警告来自本机状态与 core 准入；前端只做亮暗与文案组织，
 * 不提供差异合并界面；暂停项只提示并可复制定位信息交给 Agent 处理。
 */
import { useEffect, useState } from "react";
import { Copy, Info } from "lucide-react";

import {
  deployStatus,
  deployTo,
  deployUndo,
  type AgentView,
  type DeploymentStatusView,
  type Domain,
} from "../services/client";
import { type TargetOption, normalizeTarget, SHARED_TARGET, targetLabel, targetOptions } from "../lib/targets";
import { AgentIcon } from "./QuickDeploy";
import { FastTooltip } from "./FastTooltip";
import { Badge, Button, Notice } from "./ui";

export interface DeploymentStatusProps {
  skill: string;
  domain: Domain;
  /** 本机全部部署记录；组件内部按技能筛选。 */
  records: DeploymentStatusView[];
  agents: AgentView[];
  deploymentBlocks: Record<string, string>;
  onNotice?: (message: string) => void;
  /** 直接部署/撤销后通知父页面刷新本机记录。 */
  onChanged?: () => void | Promise<void>;
}

export function DeploymentStatus({
  skill,
  domain,
  records,
  agents,
  deploymentBlocks,
  onNotice,
  onChanged,
}: DeploymentStatusProps) {
  const [currentRecords, setCurrentRecords] = useState(records);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    setCurrentRecords(records);
  }, [records]);

  const mine = currentRecords.filter((record) => record.skill === skill);
  // 记录里的目标是 `agent:<key>`，目标选项与 chips 用裸 key；统一后匹配。
  const byTarget = new Map(mine.map((record) => [normalizeTarget(record.target), record]));
  const options: TargetOption[] =
    domain === "code"
      ? [
          { value: SHARED_TARGET, label: "共享目录", icon: null, invertInDark: false },
          ...targetOptions(domain, agents),
        ]
      : targetOptions(domain, agents);

  // 显示本机检测到的目标与已有部署的目标（去重、保持目录顺序）。
  const shown: TargetOption[] = [];
  for (const option of options) {
    shown.push(option);
  }
  for (const record of mine) {
    const value = normalizeTarget(record.target);
    if (!shown.some((item) => item.value === value)) {
      shown.push({
        value,
        label: targetLabel(record.target, agents),
        icon: null,
        invertInDark: false,
      });
    }
  }

  const warnings = mine.filter((record) => record.warning);
  const paused = mine.filter((record) => record.status === "paused_local_change");

  async function copyLocator() {
    const lines = paused.map(
      (record) =>
        `${record.skill} → ${record.target}\n路径：${record.target_path}\n状态：本地副本已被外部修改，覆盖已暂停`,
    );
    const text = [
      "以下受管部署副本被外部修改，管理器已暂停覆盖；请检查并整理回 SkillManagerVault 后再同步。",
      ...lines,
    ].join("\n\n");
    try {
      await navigator.clipboard.writeText(text);
      onNotice?.(`已复制 ${paused.length} 项暂停部署的定位信息，可交给 Agent 检查。`);
    } catch (err) {
      onNotice?.(`复制失败：${String(err)}`);
    }
  }

  async function toggle(item: TargetOption) {
    if (busy !== null) {
      return;
    }
    const record = byTarget.get(item.value);
    setBusy(item.value);
    try {
      const result = record
        ? await deployUndo(item.value, [skill])
        : await deployTo(item.value, [skill]);
      const messages = [
        ...result.errors.map((entry) => entry.message),
        ...result.items.flatMap((entry) => (entry.message ? [entry.message] : [])),
      ];
      const next = await deployStatus();
      setCurrentRecords(next);
      await onChanged?.();
      if (messages.length > 0) {
        onNotice?.(messages.join("\n"));
      } else {
        onNotice?.(
          record
            ? `已从 ${item.label} 撤销部署：${skill}`
            : `已部署到 ${item.label}：${skill}`,
        );
      }
    } catch (err) {
      onNotice?.(String(err));
    } finally {
      setBusy(null);
    }
  }

  if (shown.length === 0 && mine.length === 0) {
    return (
      <p className="flex items-center gap-2 text-[13px] text-muted">
        <Info className="size-4 shrink-0" strokeWidth={1.75} />
        本机没有可用部署目标，也还没有部署记录。
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap gap-1.5 rounded-[10px] border border-line p-3.5">
        {shown.map((item) => {
          const record = byTarget.get(item.value);
          const deployed = record !== undefined;
          const reason = deploymentBlocks[item.value.includes(":") || item.value === SHARED_TARGET ? item.value : `agent:${item.value}`];
          const warned = record?.warning !== undefined;
          return (
            <FastTooltip key={item.value} label={!deployed && reason ? `${item.label}：${reason}` : item.label}>
              <button
                type="button"
                disabled={busy !== null || (!deployed && !!reason)}
                aria-label={`${item.label} · ${deployed ? "已部署，点击撤销" : "未部署，点击部署"}`}
                title={!deployed ? reason : undefined}
                onClick={() => void toggle(item)}
                className={`relative grid size-8 place-items-center rounded-lg border transition-colors hover:bg-hover disabled:cursor-default ${
                  deployed
                    ? warned
                      ? "border-warn bg-warn-soft text-ink-2 shadow-[0_0_0_1px_var(--c-warn)]"
                      : "border-accent-line bg-accent-soft text-accent"
                    : "border-line-soft bg-subtle text-faint opacity-70"
                }`}
              >
                <AgentIcon option={item} />
                {deployed && warned && (
                  <span className="absolute -top-1 -right-1 grid size-3.5 place-items-center rounded-full bg-warn text-[9px] text-white">
                    !
                  </span>
                )}
              </button>
            </FastTooltip>
          );
        })}
      </div>
      {warnings.map((record) => (
        <p key={`${record.target}:${record.target_path}`} className="text-[12px] text-ink-3">
          {targetLabel(record.target, agents)}：{record.warning}（{record.target_path}）
        </p>
      ))}

      {paused.length > 0 && (
        <Notice tone="warn">
          有 {paused.length} 项受管副本被外部修改，覆盖已暂停；请在目标目录检查后整理回 SkillManagerVault，
          或由 Agent 裁决后显式按库覆盖。
        </Notice>
      )}
      {paused.length > 0 && (
        <div className="flex flex-wrap gap-2">
          <Button compact onClick={() => void copyLocator()}>
            <Copy className="size-3.5" strokeWidth={1.75} />
            复制定位信息给 Agent
          </Button>
          {mine.some((record) => !record.library_exists) && (
            <Badge tone="amber">库内条目已删除，副本待处置</Badge>
          )}
        </div>
      )}
    </div>
  );
}
