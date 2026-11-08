/**
 * 分类栏显示顺序与空分类（尚未挂技能）的本机持久化。
 * 正式部署名与库目录仍由 vault 决定；这里只保留界面排序与空壳分类。
 */
import type { Domain } from "../services/client";

export interface CategoryRailState {
  order: string[];
  extras: string[];
}

const PREFIX = "skill-manager.category-rail.v1";

function key(domain: Domain): string {
  return `${PREFIX}:${domain}`;
}

export function loadCategoryRail(domain: Domain): CategoryRailState {
  try {
    const raw = localStorage.getItem(key(domain));
    if (!raw) {
      return { order: [], extras: [] };
    }
    const parsed = JSON.parse(raw) as Partial<CategoryRailState>;
    return {
      order: Array.isArray(parsed.order) ? parsed.order.filter((item) => typeof item === "string") : [],
      extras: Array.isArray(parsed.extras)
        ? parsed.extras.filter((item) => typeof item === "string")
        : [],
    };
  } catch {
    return { order: [], extras: [] };
  }
}

export function saveCategoryRail(domain: Domain, state: CategoryRailState): void {
  localStorage.setItem(key(domain), JSON.stringify(state));
}

/**
 * 合并库内已有分类、空壳分类与持久化顺序。
 * `present` 可含真实分类名；空壳 extras 也会出现在结果中。
 */
export function orderedCategoryNames(present: string[], storedOrder: string[]): string[] {
  const universe = [...new Set([...storedOrder, ...present])];
  const known = storedOrder.filter((name) => universe.includes(name));
  const missing = universe.filter((name) => !known.includes(name));
  return [...known, ...missing];
}

export function freshCategoryName(existing: string[]): string {
  const names = new Set(existing);
  let name = "新分类";
  let n = 2;
  while (names.has(name)) {
    name = `新分类 ${n}`;
    n += 1;
  }
  return name;
}
