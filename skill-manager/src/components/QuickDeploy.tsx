import { useState } from "react";
import { Bot, Share2 } from "lucide-react";
import {
  deployStatus, deployTo, deployUndo,
  type AgentView, type DeploymentStatusView, type Domain,
} from "../services/client";
import {
  normalizeTarget,
  SHARED_TARGET,
  targetOptions,
  type TargetOption,
} from "../lib/targets";
import { FastTooltip } from "./FastTooltip";

export function AgentIcon({ option }: { option: TargetOption }) {
  if (option.value === SHARED_TARGET) {
    return <Share2 className="size-4" strokeWidth={1.75} />;
  }
  return option.icon ? (
    <img src={option.icon} alt="" className={`size-4 object-contain ${option.invertInDark ? "dark:invert" : ""}`} />
  ) : <Bot className="size-4" strokeWidth={1.75} />;
}

export function QuickDeploy({
  reference, domain, agents, records, unsupportedReason, onChange, onError,
}: {
  reference?: string | null;
  domain: Domain;
  agents: AgentView[];
  records: DeploymentStatusView[];
  unsupportedReason?: string | null;
  onChange: () => void | Promise<void>;
  onError: (message: string) => void;
}) {
  const [busy, setBusy] = useState(false);
  const options: TargetOption[] =
    domain === "code"
      ? [
          { value: SHARED_TARGET, label: "共享 Agent", icon: null, invertInDark: false },
          ...targetOptions(domain, agents),
        ]
      : targetOptions(domain, agents);
  async function toggle(target: string) {
    if (!reference) return;
    setBusy(true);
    try {
      const current = await deployStatus();
      const deployed = current.some((record) => record.skill === reference && normalizeTarget(record.target) === target);
      if (!deployed && unsupportedReason) throw new Error(unsupportedReason);
      const result = deployed ? await deployUndo(target, [reference]) : await deployTo(target, [reference]);
      await onChange();
      const messages = [...result.errors.map((item) => item.message), ...result.items.flatMap((item) => item.message ? [item.message] : [])];
      if (messages.length) onError(messages.join("\n"));
    } catch (err) {
      onError(String(err));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="ml-auto flex shrink-0 flex-wrap justify-end gap-0" onClick={(event) => event.stopPropagation()}>
      {options.map((option) => {
        const deployed = records.some((record) => record.skill === reference && normalizeTarget(record.target) === option.value);
        const blocked = !reference || (!deployed && !!unsupportedReason);
        const label = !reference ? `${option.label} · 先纳入管理` : `${deployed ? "撤销部署" : "部署到"} ${option.label}`;
        return (
          <FastTooltip key={option.value} label={option.label}>
            <button
              type="button"
              aria-label={label}
              aria-pressed={deployed}
              disabled={busy || blocked}
              onClick={() => void toggle(option.value)}
              className={`grid size-[22px] place-items-center rounded-[5px] transition-all hover:bg-hover disabled:cursor-default ${
                deployed ? "bg-accent-soft text-accent opacity-100"
                  : "text-muted opacity-35 grayscale hover:opacity-70"
              }`}
            >
              <AgentIcon option={option} />
            </button>
          </FastTooltip>
        );
      })}
    </div>
  );
}
