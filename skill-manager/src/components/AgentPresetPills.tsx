/**
 * Agent 详情搜索栏上方的轻量预设 pill：一键部署 / 取消安装。
 * 对应原型 `.agent-presets` / `.agent-pill`。
 */
import { useEffect, useMemo, useState } from "react";
import { Check } from "lucide-react";

import {
  deployPreset,
  deployStatus,
  deployUndoPreset,
  listPresets,
  type AgentView,
  type DeploymentStatusView,
  type Domain,
  type PresetView,
} from "../services/client";
import { presetIconOption } from "../lib/presetIcons";

export interface AgentPresetPillsProps {
  domain: Domain;
  /** 部署目标键，例如 agent:codex 或 shared。 */
  target: string;
  agents: AgentView[];
  reloadToken?: number;
  onNotice: (message: string) => void;
  onChanged?: () => void;
}

type Coverage = "off" | "partial" | "on";

function coverage(
  preset: PresetView,
  target: string,
  records: DeploymentStatusView[],
): { state: Coverage; installed: number; total: number } {
  const members = preset.members.map((member) => `${preset.domain}/${member.name}`);
  const total = members.length;
  if (total === 0) {
    return { state: "off", installed: 0, total: 0 };
  }
  const installed = members.filter((reference) =>
    records.some((record) => record.skill === reference && record.target === target),
  ).length;
  const state: Coverage = !installed ? "off" : installed === total ? "on" : "partial";
  return { state, installed, total };
}

export function AgentPresetPills({
  domain,
  target,
  reloadToken = 0,
  onNotice,
  onChanged,
}: AgentPresetPillsProps) {
  const [presets, setPresets] = useState<PresetView[]>([]);
  const [records, setRecords] = useState<DeploymentStatusView[]>([]);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [version, setVersion] = useState(0);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [list, status] = await Promise.all([listPresets(domain), deployStatus()]);
        if (!cancelled) {
          setPresets(list);
          setRecords(status);
        }
      } catch {
        if (!cancelled) {
          setPresets([]);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [domain, reloadToken, version]);

  const deployTarget =
    target === "shared" || target.includes(":") ? target : `agent:${target}`;

  const rows = useMemo(
    () =>
      presets.map((preset) => ({
        preset,
        ...coverage(preset, deployTarget, records),
      })),
    [presets, records, deployTarget],
  );

  if (rows.length === 0) {
    return null;
  }

  async function toggle(preset: PresetView, state: Coverage) {
    setBusyId(preset.id);
    try {
      if (state === "on") {
        await deployUndoPreset(domain, preset.id, deployTarget);
        onNotice(`已取消安装预设「${preset.name}」`);
      } else {
        await deployPreset(domain, preset.id, deployTarget);
        onNotice(`已部署预设「${preset.name}」到当前 Agent`);
      }
      setVersion((value) => value + 1);
      onChanged?.();
    } catch (err) {
      onNotice(String(err));
    } finally {
      setBusyId(null);
    }
  }

  return (
    <div className="mb-3 flex items-center gap-2.5">
      <span className="shrink-0 text-[12px] text-[#8b8b94]">预设</span>
      <div className="flex min-w-0 flex-wrap gap-2">
        {rows.map(({ preset, state, installed, total }) => {
          const option = presetIconOption(preset.icon);
          const Icon = option.icon;
          const tone =
            state === "on"
              ? "border-[#b7e4d4] bg-white text-[#3f3f46]"
              : state === "partial"
                ? "border-[#f5c58a] bg-white text-[#3f3f46]"
                : "border-[#e6e6ea] bg-[#f4f4f5] text-[#a1a1aa]";
          return (
            <button
              key={preset.id}
              type="button"
              disabled={busyId === preset.id || total === 0}
              title={state === "on" ? "取消安装到当前 Agent" : "部署到当前 Agent"}
              onClick={() => void toggle(preset, state)}
              className={`inline-flex h-7 items-center gap-1.5 rounded-full border py-0 pr-2.5 pl-1.5 text-[13px] font-medium disabled:opacity-50 ${tone}`}
            >
              <span
                className="grid size-[18px] place-items-center rounded-[5px]"
                style={{ background: `${option.color}22` }}
              >
                <Icon className="size-3" strokeWidth={1.75} style={{ color: option.color }} />
              </span>
              <span>{preset.name}</span>
              {state === "on" && <Check className="size-3.5 text-accent" strokeWidth={2} />}
              {state === "partial" && (
                <span className="rounded-full bg-[#fff4e8] px-1.5 text-[11px] leading-4 text-[#ef7800]">
                  {installed}/{total}
                </span>
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}
