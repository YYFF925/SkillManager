/**
 * 部署目标的展示辅助：目标标签、图标与分组。
 *
 * 只做展示映射；准入（域边界、支持声明）与真实路径由 core 决定。
 */
import type { AgentView, Domain } from "../services/client";

export const SHARED_TARGET = "shared";
export const HERMES_DEFAULT_TARGET = "hermes:default";

export interface TargetOption {
  /** 目标标识：Agent key、`shared` 或 `hermes:default`。 */
  value: string;
  label: string;
  domain: Domain;
  /** 本机检测目录存在（Agent 目标）；共享目录与 Hermes default 视为可用。 */
  detected: boolean;
  /** 品牌图标文件名（相对 public/）。 */
  icon: string | null;
  invertInDark: boolean;
  isAgent: boolean;
}

/** 某管理域可用的部署目标：共享目录 / Hermes default + 内置 Agent 名单。 */
export function targetOptions(domain: Domain, agents: AgentView[]): TargetOption[] {
  const specials: TargetOption[] =
    domain === "code"
      ? [
          {
            value: SHARED_TARGET,
            label: "共享目录",
            domain: "code",
            detected: true,
            icon: null,
            invertInDark: false,
            isAgent: false,
          },
        ]
      : [
          {
            value: HERMES_DEFAULT_TARGET,
            label: "Hermes default 档案",
            domain: "lobster",
            detected: true,
            icon: null,
            invertInDark: false,
            isAgent: false,
          },
        ];
  const agentOptions = agents
    .filter((agent) => agent.domain === domain)
    .map((agent) => ({
      value: agent.key,
      label: agent.display_name,
      domain: agent.domain,
      detected: agent.detected,
      icon: agent.icon,
      invertInDark: agent.invert_in_dark,
      isAgent: true,
    }));
  return [...specials, ...agentOptions];
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
