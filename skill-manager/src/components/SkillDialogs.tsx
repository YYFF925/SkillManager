/**
 * 技能相关的管理弹窗：标签选择、移动到分类、分类重命名、全库标签整理。
 *
 * 只收集用户输入并调用 client.ts；标签是否存在、分类是否合法、同名阻止
 * 都由 core 决定。
 */
import { useState } from "react";
import { Pencil, Trash2 } from "lucide-react";

import {
  removeSkillTag,
  renameSkillTag,
  type Domain,
  type ManageItem,
} from "../services/client";
import { TagChip } from "./TagChip";
import { Button, Dialog, Field, Notice } from "./ui";

/** 把输入的新标签拆成列表（支持半角/全角逗号分隔）。 */
function parseNewTags(text: string): string[] {
  return text
    .split(/[,，]/)
    .map((tag) => tag.trim())
    .filter((tag) => tag.length > 0);
}

export interface TagPickerResult {
  /** 本次要应用（添加/移除）的标签，不含技能已有且锁定显示的标签。 */
  tags: string[];
}

/** 标签选择弹窗：从已有标签勾选，也可输入新标签。 */
export function TagPickerDialog({
  title,
  subtitle,
  options,
  locked = [],
  allowNew = false,
  newLabel = "新标签（多个用逗号分隔）",
  newPlaceholder,
  confirmLabel,
  onCancel,
  onConfirm,
}: {
  title: string;
  subtitle: string;
  options: string[];
  /** 已拥有且不可取消的标签（勾选并禁用）。 */
  locked?: string[];
  allowNew?: boolean;
  newLabel?: string;
  newPlaceholder?: string;
  confirmLabel: string;
  onCancel: () => void;
  onConfirm: (result: TagPickerResult) => void;
}) {
  const [checked, setChecked] = useState<string[]>([]);
  const [newTags, setNewTags] = useState("");

  const tags = [
    ...new Set([
      ...checked.filter((tag) => !locked.includes(tag)),
      ...(allowNew ? parseNewTags(newTags) : []),
    ]),
  ];

  return (
    <Dialog
      title={title}
      subtitle={subtitle}
      onClose={onCancel}
      footer={
        <>
          <span className="text-[12px] text-faint" />
          <div className="flex gap-2.5">
            <Button onClick={onCancel}>取消</Button>
            <Button kind="primary" disabled={tags.length === 0} onClick={() => onConfirm({ tags })}>
              {confirmLabel}
            </Button>
          </div>
        </>
      }
    >
      <div className="flex flex-wrap gap-x-6 gap-y-2.5">
        {options.map((tag) => {
          const isLocked = locked.includes(tag);
          return (
            <label key={tag} className="flex items-center gap-1.5 text-[13px]">
              <input
                type="checkbox"
                className="selectbox"
                disabled={isLocked}
                checked={isLocked || checked.includes(tag)}
                onChange={(event) =>
                  setChecked((current) =>
                    event.target.checked
                      ? [...current, tag]
                      : current.filter((item) => item !== tag),
                  )
                }
              />
              <TagChip tag={tag} />
            </label>
          );
        })}
        {options.length === 0 && (
          <span className="text-[13px] text-faint">两个技能库中还没有标签。</span>
        )}
      </div>
      {allowNew && (
        <div className="mt-4">
          <Field label={newLabel}>
            <textarea
              className="field-input !h-[64px] resize-none py-2"
              value={newTags}
              placeholder={newPlaceholder}
              onChange={(event) => setNewTags(event.target.value)}
            />
          </Field>
        </div>
      )}
    </Dialog>
  );
}

/** 移动到分类弹窗：分类是实际目录，最多一级。 */
export function MoveCategoryDialog({
  skill,
  current,
  categories,
  onCancel,
  onConfirm,
}: {
  skill: string;
  current: string | null;
  categories: string[];
  onCancel: () => void;
  onConfirm: (category: string | undefined) => void;
}) {
  const [value, setValue] = useState(current ?? "");
  const next = value.trim();

  return (
    <Dialog
      title={`移动到分类 · ${skill}`}
      subtitle="移动实际库目录，技能名、已有部署与预设归属保持不变。"
      width="w-[420px]"
      onClose={onCancel}
      footer={
        <>
          <span className="text-[12px] text-faint" />
          <div className="flex gap-2.5">
            <Button onClick={onCancel}>取消</Button>
            <Button
              kind="primary"
              onClick={() => onConfirm(next.length === 0 ? undefined : next)}
            >
              移动
            </Button>
          </div>
        </>
      }
    >
      <Field label="分类目录（最多一级，留空为未分类）">
        <input
          className="field-input"
          list="move-category-options"
          value={value}
          onChange={(event) => setValue(event.target.value)}
        />
        <datalist id="move-category-options">
          {categories.map((category) => (
            <option key={category} value={category} />
          ))}
        </datalist>
      </Field>
    </Dialog>
  );
}

/** 重命名一级分类：改实际目录名，该分类下全部技能一并移动。 */
export function RenameCategoryDialog({
  current,
  onCancel,
  onConfirm,
}: {
  current: string;
  onCancel: () => void;
  onConfirm: (next: string) => void | Promise<void>;
}) {
  const [value, setValue] = useState(current);
  const next = value.trim();
  const changed = next.length > 0 && next !== current;

  return (
    <Dialog
      title={`重命名分类 · ${current}`}
      subtitle="重命名实际库目录，该分类下的技能一并移动；技能名、已有部署与预设归属保持不变。"
      width="w-[420px]"
      onClose={onCancel}
      footer={
        <>
          <span className="text-[12px] text-faint" />
          <div className="flex gap-2.5">
            <Button onClick={onCancel}>取消</Button>
            <Button
              kind="primary"
              disabled={!changed}
              onClick={() => void onConfirm(next)}
            >
              保存
            </Button>
          </div>
        </>
      }
    >
      <Field label="分类名称（最多一级）">
        <input
          autoFocus
          className="field-input"
          value={value}
          onChange={(event) => setValue(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && changed) {
              event.preventDefault();
              void onConfirm(next);
            }
          }}
        />
      </Field>
    </Dialog>
  );
}

/**
 * 全库标签整理：重命名同时作用于两个技能库，删除只解除关联、不删除技能。
 */
export function ManageTagsDialog({
  tags,
  onClose,
  onChanged,
}: {
  tags: string[];
  onClose: () => void;
  onChanged: (message: string) => void;
}) {
  const [renaming, setRenaming] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function rename(from: string) {
    const to = draft.trim();
    if (to.length === 0 || to === from) {
      setRenaming(null);
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const result = await renameSkillTag(from, to);
      onChanged(`标签「${result.from}」已重命名为「${result.to}」，影响 ${result.affected} 个技能。`);
      setRenaming(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function remove(tag: string) {
    setBusy(true);
    setError(null);
    try {
      const result = await removeSkillTag(tag);
      onChanged(`已解除标签「${result.tag}」的关联，影响 ${result.affected} 个技能；技能未被删除。`);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog
      title="管理标签"
      subtitle="两个技能库的标签可统一整理；删除标签只解除关联，不删除技能。"
      width="w-[560px]"
      onClose={onClose}
      footer={
        <>
          <span className="text-[12px] text-faint" />
          <Button onClick={onClose}>完成</Button>
        </>
      }
    >
      {error && (
        <div className="mb-3">
          <Notice tone="danger">{error}</Notice>
        </div>
      )}
      <div className="overflow-hidden rounded-lg border border-line-soft">
        {tags.map((tag) => (
          <div
            key={tag}
            className="flex min-h-[52px] items-center gap-3 border-t border-line-faint px-3.5 py-2.5 first:border-t-0"
          >
            <TagChip tag={tag} />
            {renaming === tag ? (
              <>
                <input
                  autoFocus
                  className="field-input h-8 flex-1"
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                />
                <Button compact kind="primary" disabled={busy} onClick={() => rename(tag)}>
                  确定
                </Button>
                <Button compact onClick={() => setRenaming(null)}>
                  取消
                </Button>
              </>
            ) : (
              <>
                <span className="flex-1" />
                <Button
                  compact
                  disabled={busy}
                  onClick={() => {
                    setRenaming(tag);
                    setDraft(tag);
                  }}
                >
                  <Pencil className="size-3.5" strokeWidth={1.75} />
                  改名
                </Button>
                <button
                  type="button"
                  title={`移除标签 ${tag}`}
                  aria-label={`移除标签 ${tag}`}
                  disabled={busy}
                  onClick={() => remove(tag)}
                  className="grid size-6 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-danger disabled:opacity-40"
                >
                  <Trash2 className="size-4" strokeWidth={1.75} />
                </button>
              </>
            )}
          </div>
        ))}
        {tags.length === 0 && (
          <p className="px-3.5 py-6 text-[13px] text-muted">两个技能库中还没有标签。</p>
        )}
      </div>
    </Dialog>
  );
}

/**
 * 跨管理域复制：保留原件与来源记录，两份独立维护、不部署、不互相同步。
 */
export function CopyToDomainDialog({
  reference,
  from,
  onCancel,
  onConfirm,
}: {
  reference: string;
  from: Domain;
  onCancel: () => void;
  onConfirm: (to: Domain, category?: string) => void;
}) {
  const to: Domain = from === "code" ? "lobster" : "code";
  const [category, setCategory] = useState("");
  const name = reference.slice(reference.indexOf("/") + 1);

  return (
    <Dialog
      title={`复制到另一库 · ${name}`}
      subtitle="复制保留原件与来源记录；两份独立维护、不互相同步，复制不会部署。"
      width="w-[460px]"
      onClose={onCancel}
      footer={
        <>
          <span className="text-[12px] text-faint">目标库内同名技能会被阻止。</span>
          <div className="flex gap-2.5">
            <Button onClick={onCancel}>取消</Button>
            <Button
              kind="primary"
              onClick={() => onConfirm(to, category.trim() || undefined)}
            >
              复制
            </Button>
          </div>
        </>
      }
    >
      <Field label="目标管理域">
        <p className="text-[13px] text-ink-3">
          {to === "code" ? "编码技能库（code）" : "龙虾技能库（lobster）"}
        </p>
      </Field>
      <div className="mt-3.5">
        <Field label="副本分类目录（最多一级，留空为不分类）">
          <input
            className="field-input"
            value={category}
            placeholder="留空表示进入目标库管理域根下"
            onChange={(event) => setCategory(event.target.value)}
          />
        </Field>
      </div>
    </Dialog>
  );
}

/** 批量标签操作的结果摘要：逐项表达成功与未处理原因。 */
export function summarizeManageItems(items: ManageItem[], action: string): string {
  const ok = items.filter((item) => item.ok).length;
  const failed = items.filter((item) => !item.ok);
  const parts = [`已${action} ${ok} 个技能`];
  if (failed.length > 0) {
    parts.push(
      `${failed.length} 个未处理：${failed
        .map((item) => `${item.skill}（${item.message ?? "未说明原因"}）`)
        .join("；")}`,
    );
  }
  return `${parts.join("；")}。`;
}
