/** 标签筛选：全部 / 无标签 / 多选 AND。 */
export type TagMode = "all" | "none" | "and";

export function toggleTagSelection(
  mode: TagMode,
  selected: string[],
  name: string,
): { mode: TagMode; selected: string[] } {
  if (!name) {
    return { mode: "all", selected: [] };
  }
  if (name === "__none") {
    if (mode === "none") {
      return { mode: "all", selected: [] };
    }
    return { mode: "none", selected: [] };
  }
  const nextMode: TagMode = "and";
  const next = mode === "and" ? [...selected] : [];
  const index = next.indexOf(name);
  if (index >= 0) {
    next.splice(index, 1);
  } else {
    next.push(name);
  }
  if (next.length === 0) {
    return { mode: "all", selected: [] };
  }
  return { mode: nextMode, selected: next };
}

export function pruneSelectedTags(selected: string[], candidates: string[]): string[] {
  return selected.filter((tag) => candidates.includes(tag));
}
