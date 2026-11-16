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
 * `present` 为当前真实分类与未落库空壳；`storedOrder` 只提供显示顺序，不复活已消失或串域的分类。
 */
export function orderedCategoryNames(present: string[], storedOrder: string[]): string[] {
  const presentSet = new Set(present);
  const known = storedOrder.filter((name) => presentSet.has(name));
  const knownSet = new Set(known);
  const missing = present.filter((name) => !knownSet.has(name));
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
