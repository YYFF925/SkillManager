/**
 * 预设成员显示顺序的纯计算（US7 / T065）。
 *
 * 筛选中拖动只调整**可见条目**的相对位置：非可见成员保持原位。
 * core 的 `reorder-members` 只接受当前成员的完整排列，因此这里返回完整顺序。
 */

function lower(value: string): string {
  return value.toLowerCase();
}

/** 把可见条目的新顺序套回完整成员数组，只调整可见条目的相对位置。 */
export function reorderVisible(full: string[], visibleOrdered: string[]): string[] {
  const visible = new Set(visibleOrdered.map(lower));
  const queue = [...visibleOrdered];
  return full.map((name) => (visible.has(lower(name)) ? (queue.shift() ?? name) : name));
}
