/**
 * 技能库分类栏：六点手柄改序、悬停新建、行内重命名（无弹窗）。
 * 对应原型 `.category-rail`。
 */
import { useEffect, useMemo, useRef, useState } from "react";
import { GripVertical, Pencil, Plus } from "lucide-react";

import type { Domain } from "../services/client";
import {
  freshCategoryName,
  loadCategoryRail,
  orderedCategoryNames,
  saveCategoryRail,
} from "../lib/categoryRail";

const UNCATEGORIZED = "__none";

export interface CategoryRailProps {
  domain: Domain;
  total: number;
  /** 真实分类名 → 技能数。 */
  namedCounts: Map<string, number>;
  uncategorizedCount: number;
  selected: string | null;
  onSelect: (category: string | null) => void;
  /** 有技能的分类重命名走 vault；空壳分类只改本机列表。 */
  onRenameNamed: (from: string, to: string) => Promise<void> | void;
  editable?: boolean;
}

export function CategoryRail({
  domain,
  total,
  namedCounts,
  uncategorizedCount,
  selected,
  onSelect,
  onRenameNamed,
  editable = true,
}: CategoryRailProps) {
  const [order, setOrder] = useState<string[]>([]);
  const [extras, setExtras] = useState<string[]>([]);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [dragId, setDragId] = useState<string | null>(null);
  const renameRef = useRef<HTMLInputElement>(null);
  const focusSelect = useRef(false);

  // 切换管理域时重载本机顺序与空壳。
  useEffect(() => {
    const stored = loadCategoryRail(domain);
    setOrder(stored.order);
    setExtras(stored.extras);
    setRenaming(null);
  }, [domain]);

  const present = useMemo(() => {
    const names = [...namedCounts.keys(), ...extras];
    return [...new Set(names)];
  }, [namedCounts, extras]);

  const categories = useMemo(
    () => orderedCategoryNames(present, order),
    [present, order],
  );

  useEffect(() => {
    setOrder((current) => {
      const next = orderedCategoryNames(present, current);
      if (next.join("\0") !== current.join("\0")) {
        saveCategoryRail(domain, { order: next, extras });
        return next;
      }
      return current;
    });
  }, [present, domain, extras]);

  useEffect(() => {
    if (!renaming) {
      return;
    }
    renameRef.current?.focus();
    if (focusSelect.current) {
      renameRef.current?.select();
      focusSelect.current = false;
    }
  }, [renaming]);

  function persist(nextOrder: string[], nextExtras: string[]) {
    setOrder(nextOrder);
    setExtras(nextExtras);
    saveCategoryRail(domain, { order: nextOrder, extras: nextExtras });
  }

  function beginRename(name: string, selectAll: boolean) {
    if (!editable) {
      return;
    }
    focusSelect.current = selectAll;
    setRenaming(name);
    setDraft(name);
  }

  function createCategory() {
    if (!editable) {
      return;
    }
    const name = freshCategoryName([...categories, ...extras]);
    const nextExtras = [...extras, name];
    const nextOrder = [...categories, name];
    persist(nextOrder, nextExtras);
    onSelect(name);
    beginRename(name, true);
  }

  async function commitRename() {
    if (!renaming) {
      return;
    }
    const from = renaming;
    const to = draft.trim();
    setRenaming(null);
    if (!to || to === from) {
      return;
    }
    if (categories.includes(to) || extras.includes(to) || namedCounts.has(to)) {
      return;
    }
    const isExtra = extras.includes(from) && !namedCounts.has(from);
    if (isExtra) {
      persist(
        order.map((item) => (item === from ? to : item)),
        extras.map((item) => (item === from ? to : item)),
      );
      if (selected === from) {
        onSelect(to);
      }
      return;
    }
    await onRenameNamed(from, to);
    persist(
      order.map((item) => (item === from ? to : item)),
      extras.filter((item) => item !== from),
    );
  }

  function onDropOn(targetId: string) {
    if (!dragId || dragId === targetId) {
      setDragId(null);
      return;
    }
    // 未分类不参与命名顺序拖拽；只排真实/空壳名。
    const named = categories.filter((name) => namedCounts.has(name) || extras.includes(name));
    const from = named.indexOf(dragId);
    const to = named.indexOf(targetId);
    if (from < 0 || to < 0) {
      setDragId(null);
      return;
    }
    const next = [...named];
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    persist(next, extras);
    setDragId(null);
  }

  function rowClass(active: boolean): string {
    return `group mb-0.5 flex h-[35px] w-full items-center gap-0.5 rounded-md pr-1 text-[14px] ${
      active ? "bg-selected text-ink" : "text-ink-3 hover:bg-hover"
    }`;
  }

  return (
    <aside className="category-rail w-[148px] shrink-0">
      <div className={rowClass(selected === null)}>
        <span className="grid size-4 shrink-0" aria-hidden="true" />
        <button
          type="button"
          onClick={() => onSelect(null)}
          className="min-w-0 flex-1 truncate px-1 text-left"
        >
          全部分类
          <span className="ml-1.5 inline-grid h-[18px] min-w-[20px] place-items-center rounded-full bg-subtle px-1.5 text-[11px] text-muted align-middle">
            {total}
          </span>
        </button>
        {editable && (
          <button
            type="button"
            title="新建分类"
            aria-label="新建分类"
            onClick={createCategory}
            className="grid size-5 shrink-0 place-items-center rounded-[5px] text-faint opacity-0 hover:bg-hover hover:text-accent group-hover:opacity-100 group-focus-within:opacity-100"
          >
            <Plus className="size-3.5" strokeWidth={1.75} />
          </button>
        )}
      </div>

      {uncategorizedCount > 0 && (
        <div className={rowClass(selected === UNCATEGORIZED)}>
          <span className="grid size-4 shrink-0" aria-hidden="true" />
          <button
            type="button"
            onClick={() => onSelect(UNCATEGORIZED)}
            className="min-w-0 flex-1 truncate px-1 text-left"
          >
            未分类
            <span className="ml-1.5 inline-grid h-[18px] min-w-[20px] place-items-center rounded-full bg-subtle px-1.5 text-[11px] text-muted align-middle">
              {uncategorizedCount}
            </span>
          </button>
        </div>
      )}

      {categories.map((name) => {
        const count = namedCounts.get(name) ?? 0;
        const active = selected === name;
        if (renaming === name) {
          return (
            <div key={name} className={rowClass(active)}>
              <span className="grid size-4 shrink-0 place-items-center text-faint">
                <GripVertical className="size-3.5" strokeWidth={1.75} />
              </span>
              <input
                ref={renameRef}
                aria-label="分类名称"
                value={draft}
                onChange={(event) => setDraft(event.target.value)}
                onBlur={() => void commitRename()}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    void commitRename();
                  }
                  if (event.key === "Escape") {
                    setRenaming(null);
                  }
                }}
                className="mx-0.5 h-[26px] min-w-0 flex-1 rounded-md border border-accent bg-panel px-1.5 text-[13px] text-ink outline-0"
              />
            </div>
          );
        }
        return (
          <div
            key={name}
            className={rowClass(active)}
            onDragOver={(event) => event.preventDefault()}
            onDrop={() => onDropOn(name)}
          >
            {editable ? (
              <button
                type="button"
                draggable
                title="按住拖动排序"
                aria-label={`拖动排序 ${name}`}
                onDragStart={() => setDragId(name)}
                onDragEnd={() => setDragId(null)}
                className="grid size-4 shrink-0 cursor-grab place-items-center text-[#c4c4cc] active:cursor-grabbing group-hover:text-muted"
              >
                <GripVertical className="size-3.5" strokeWidth={1.75} />
              </button>
            ) : (
              <span className="grid size-4 shrink-0" aria-hidden="true" />
            )}
            <button
              type="button"
              title={name}
              onClick={() => onSelect(name)}
              className="min-w-0 flex-1 truncate px-1 text-left"
            >
              {name}
              <span className="ml-1.5 inline-grid h-[18px] min-w-[20px] place-items-center rounded-full bg-subtle px-1.5 text-[11px] text-muted align-middle">
                {count}
              </span>
            </button>
            {editable && (
              <button
                type="button"
                title="重命名"
                aria-label={`重命名分类 ${name}`}
                onClick={() => {
                  onSelect(name);
                  beginRename(name, false);
                }}
                className="grid size-5 shrink-0 place-items-center rounded-[5px] text-faint opacity-0 hover:bg-hover hover:text-accent group-hover:opacity-100 group-focus-within:opacity-100"
              >
                <Pencil className="size-3.5" strokeWidth={1.75} />
              </button>
            )}
          </div>
        );
      })}
    </aside>
  );
}

export { UNCATEGORIZED as CATEGORY_UNCATEGORIZED };
