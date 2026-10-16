/**
 * 部署目标的展示辅助：目标标签、图标与分组。
 *
 * 只做展示映射；准入（域边界、支持声明）与真实路径由 core 决定。
 */
import type { AgentView, Domain } from "../services/client";

export const SHARED_TARGET = "shared";
export const HERMES_DEFAULT_TARGET = "hermes:default";

export interface TargetOption {
  /** Agent key。 */
  value: string;
  label: string;
  /** 品牌图标文件名（相对 public/）。 */
  icon: string | null;
  invertInDark: boolean;
}

/** 某管理域可用的部署目标：已检测且启用的 Agent。 */
export function targetOptions(domain: Domain, agents: AgentView[]): TargetOption[] {
  const agentOptions = agents
    .filter((agent) => agent.domain === domain && agent.detected && agent.enabled)
    .map((agent) => ({
      value: agent.key,
      label: agent.display_name,
      icon: agent.icon,
      invertInDark: agent.invert_in_dark,
    }));
  return agentOptions;
}

/**
 * 部署记录里的目标标识（`agent:<key>`）与目标选项标识（Agent 用裸 key）统一为可用于查找的取值。
 */
export function normalizeTarget(value: string): string {
  return value.startsWith("agent:") ? value.slice("agent:".length) : value;
}

/** 目标标识的可读名称；未知目标回退为标识本身。 */
export function targetLabel(value: string, agents: AgentView[]): string {
  const key = normalizeTarget(value);
  if (key === SHARED_TARGET) {
    return "共享目录";
  }
  if (key === HERMES_DEFAULT_TARGET) {
    return "Hermes default 档案";
  }
  return agents.find((agent) => agent.key === key)?.display_name ?? value;
}
