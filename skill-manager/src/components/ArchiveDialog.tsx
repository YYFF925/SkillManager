/**
 * 归档管理：列出已归档技能，提供还原与删除。
 */
import { useEffect, useState } from "react";

import {
  deleteArchivedSkills,
  listArchivedSkills,
  restoreArchivedSkills,
  type ArchivedSkillView,
  type Domain,
} from "../services/client";
import { Button, ConfirmDialog, Dialog, Notice } from "./ui";

export function ArchiveDialog({
  onClose,
  onChanged,
}: {
  onClose: () => void;
  onChanged?: (message: string) => void;
}) {
  const [items, setItems] = useState<ArchivedSkillView[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<ArchivedSkillView | null>(null);

  async function reload() {
    try {
      setItems(await listArchivedSkills());
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }

  useEffect(() => {
    void reload();
  }, []);

  function referenceOf(item: ArchivedSkillView): string {
    return `${item.domain}/${item.name}`;
  }

  function domainLabel(domain: Domain): string {
    return domain === "code" ? "编码库" : "龙虾库";
  }

  async function restore(item: ArchivedSkillView) {
    setBusy(true);
    try {
      const result = await restoreArchivedSkills([referenceOf(item)]);
      if (result.errors.length > 0) {
        setError(result.errors.map((entry) => entry.message).join("；"));
        return;
      }
      onChanged?.(`已还原 ${referenceOf(item)}，保留原分类与标签，未重新部署。`);
      await reload();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function confirmDelete() {
    const item = pendingDelete;
    setPendingDelete(null);
    if (!item) {
      return;
    }
    setBusy(true);
    try {
      const result = await deleteArchivedSkills([referenceOf(item)]);
      if (result.errors.length > 0) {
        setError(result.errors.map((entry) => entry.message).join("；"));
        return;
      }
      onChanged?.(`已从归档删除 ${referenceOf(item)}。`);
      await reload();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <Dialog
        title="已归档技能"
        subtitle="归档后的技能不出现在技能库列表中。还原保留原分类与标签，不自动重新部署。"
        width="w-[640px]"
        onClose={onClose}
        footer={
          <div className="flex w-full items-center justify-end">
            <Button onClick={onClose}>关闭</Button>
          </div>
        }
      >
        {error && (
          <div className="mb-4">
            <Notice tone="danger">{error}</Notice>
          </div>
        )}
        {items === null ? (
          <p className="text-[13px] text-muted">正在读取归档…</p>
        ) : items.length === 0 ? (
          <p className="text-[13px] text-muted">当前没有已归档技能。</p>
        ) : (
          <ul className="divide-y divide-line-faint">
            {items.map((item) => (
              <li key={referenceOf(item)} className="flex items-start justify-between gap-3 py-3 first:pt-0 last:pb-0">
                <div className="min-w-0">
                  <p className="truncate text-[14px] font-medium text-ink">{item.name}</p>
                  <p className="mt-0.5 truncate text-[12px] text-muted">
                    {domainLabel(item.domain)} · {item.category ?? "未分类"}
                    {item.comment ? ` · ${item.comment}` : ""}
                  </p>
                  {item.tags.length > 0 && (
                    <p className="mt-0.5 truncate text-[12px] text-faint">{item.tags.join("、")}</p>
                  )}
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  <Button compact disabled={busy} onClick={() => void restore(item)}>
                    还原
                  </Button>
                  <Button compact disabled={busy} onClick={() => setPendingDelete(item)}>
                    删除
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </Dialog>
      {pendingDelete && (
        <ConfirmDialog
          title="从归档删除"
          subtitle="此操作不可恢复。"
          message={`将永久删除归档中的 ${referenceOf(pendingDelete)}，技能文件不再保留。`}
          confirmLabel="删除"
          danger
          onCancel={() => setPendingDelete(null)}
          onConfirm={() => void confirmDelete()}
        />
      )}
    </>
  );
}
