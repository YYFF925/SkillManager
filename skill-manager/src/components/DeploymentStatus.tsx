/**
 * 技能详情的“本机已有部署”：已部署目标亮显、未部署灰暗、支持收窄警告与暂停提示。
 *
 * 部署记录与警告来自本机状态与 core 准入；前端只做亮暗与文案组织，
 * 不提供差异合并界面；暂停项只提示并可复制定位信息交给 Agent 处理。
 */
import { Copy, Info } from "lucide-react";

import type { AgentView, DeploymentStatusView, Domain } from "../services/client";
import { normalizeTarget, targetLabel, targetOptions } from "../lib/targets";
import { Badge, Button, Notice } from "./ui";

export interface DeploymentStatusProps {
  skill: string;
  domain: Domain;
  /** 本机全部部署记录；组件内部按技能筛选。 */
  records: DeploymentStatusView[];
  agents: AgentView[];
  onNotice?: (message: string) => void;
}

export function DeploymentStatus({
  skill,
  domain,
  records,
  agents,
  onNotice,
}: DeploymentStatusProps) {
  const mine = records.filter((record) => record.skill === skill);
  // 记录里的目标是 `agent:<key>`，目标选项与 chips 用裸 key；统一后匹配。
  const byTarget = new Map(mine.map((record) => [normalizeTarget(record.target), record]));
  const options = targetOptions(domain, agents);

  // 显示本机检测到的目标与已有部署的目标（去重、保持目录顺序）。
  const shown: { value: string; label: string }[] = [];
  for (const option of options) {
    if (option.detected) {
      shown.push({ value: option.value, label: option.label });
    }
  }
  for (const record of mine) {
    const value = normalizeTarget(record.target);
    if (!shown.some((item) => item.value === value)) {
      shown.push({ value, label: targetLabel(record.target, agents) });
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
      "以下受管部署副本被外部修改，管理器已暂停覆盖；请检查并整理回 SkillVault 后再同步。",
      ...lines,
    ].join("\n\n");
    try {
      await navigator.clipboard.writeText(text);
      onNotice?.(`已复制 ${paused.length} 项暂停部署的定位信息，可交给 Agent 检查。`);
    } catch (err) {
      onNotice?.(`复制失败：${String(err)}`);
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
      <div className="flex flex-wrap gap-1.5">
        {shown.map((item) => {
          const record = byTarget.get(item.value);
          const deployed = record !== undefined;
          const warned = record?.warning !== undefined;
          return (
            <span
              key={item.value}
              title={
                record
                  ? `${item.label} · ${record.target_path}${record.warning ? ` · ${record.warning}` : ""}`
                  : `${item.label} · 未部署`
              }
              className={`inline-flex items-center gap-1.5 rounded-lg border px-2 py-1 text-[12px] ${
                deployed
                  ? warned
                    ? "border-warn bg-warn-soft text-ink-2 shadow-[0_0_0_1px_var(--c-warn)]"
                    : "border-accent-line bg-accent-soft font-medium text-accent"
                  : "border-line-soft bg-subtle text-faint opacity-70"
              }`}
            >
              <span
                className={`size-1.5 rounded-full ${
                  deployed ? (warned ? "bg-warn" : "bg-accent-light") : "bg-faint"
                }`}
              />
              {item.label}
              {deployed && <span>{warned ? "！" : ""}</span>}
            </span>
          );
        })}
      </div>
      <p className="text-[12px] text-faint">
        亮色：已部署 · 灰暗：未部署{warnings.length > 0 ? " · 感叹号：已有部署因支持范围收窄不再适用" : ""}
      </p>

      {warnings.map((record) => (
        <p key={`${record.target}:${record.target_path}`} className="text-[12px] text-ink-3">
          {targetLabel(record.target, agents)}：{record.warning}（{record.target_path}）
        </p>
      ))}

      {paused.length > 0 && (
        <Notice tone="warn">
          有 {paused.length} 项受管副本被外部修改，覆盖已暂停；请在目标目录检查后整理回 SkillVault，
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
