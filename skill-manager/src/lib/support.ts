/**
 * 支持声明的展示与保存取值。
 *
 * 业务规则在 core：缺省=所属域内全支持；显式空集合=该维度不支持任何目标；
 * 修改其他资料不能清除本机未显示的已声明 Agent。
 */
import type { EnvironmentOs, SupportArg } from "../services/client";

/** OS 声明取值与显示名；取值与 core 的 `oses` 一致。 */
export const OS_OPTIONS = [
  { value: "macos", label: "macOS" },
  { value: "linux", label: "Linux" },
  { value: "windows", label: "Windows" },
] as const;

export function osLabel(value: string): string {
  return OS_OPTIONS.find((option) => option.value === value)?.label ?? value;
}

/**
 * 保存支持声明时的参数：
 * - `allSupported` 为真 → `all`，删除限制键（域内全支持）；
 * - 否则把本机未显示的已声明项与本次勾选合并；合并后为空则写显式空集合。
 */
export function buildSupportArg(
  displayed: string[],
  selected: string[],
  declared: string[] | null,
  allSupported: boolean,
): SupportArg {
  if (allSupported) {
    return "all";
  }
  const hidden = (declared ?? []).filter((key) => !displayed.includes(key));
  const merged = [...new Set([...hidden, ...selected])];
  return merged.length === 0 ? "empty" : { list: merged };
}

/** 声明是否覆盖某个目标：缺省（null）=该域内全支持。 */
export function declarationIncludes(
  declared: string[] | null,
  key: string,
): boolean {
  return declared === null || declared.includes(key);
}

/**
 * 当前系统是否不在技能声明范围内（整项灰显的依据）。
 * 声明缺省=全支持；系统未知时无法判定不适用，因此不灰显。
 */
export function unsupportedOs(
  oses: string[] | null,
  current: EnvironmentOs | null,
): boolean {
  if (oses === null) {
    return false;
  }
  if (current === null || current === "unknown") {
    return false;
  }
  return !oses.includes(current);
}

/**
 * 本次选中的技能是否全部不受当前系统支持：全部不支持时返回可读原因，否则 null。
 * 用于部署入口的阻止；撤销不受支持声明约束，因此不在此判定内。
 */
export function unsupportedDeployReason(
  entries: { name: string; oses: string[] | null }[],
  current: EnvironmentOs | null,
): string | null {
  if (entries.length === 0) {
    return null;
  }
  if (entries.some((entry) => !unsupportedOs(entry.oses, current))) {
    return null;
  }
  const system = current === null ? "未知" : osLabel(current);
  if (entries.length === 1) {
    const oses = entries[0].oses ?? [];
    if (oses.length === 0) {
      return `该技能声明不支持任何系统，当前系统 ${system} 无法新增部署；撤销不受影响。`;
    }
    return `该技能声明支持 ${oses.map(osLabel).join("、")}，当前系统 ${system} 不在其中；新增部署会被拒绝，撤销不受影响。`;
  }
  return `所选 ${entries.length} 项（${entries
    .map((entry) => entry.name)
    .join("、")}）均不支持当前系统 ${system}；新增部署会被拒绝，撤销不受影响。`;
}
