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
