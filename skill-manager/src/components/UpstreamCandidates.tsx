/**
 * 上游新增候选列表（US4）：列出“当前仍存在、未入库、未拒绝”的候选，单个也列出。
 *
 * 检查只展示候选；勾选后随仓库批量更新入库并补充分类；每项可明确“不入库并记住”，
 * 由 core 记录并随库共享，之后不再提示。返回列表或未操作不算拒绝。
 */
import { FolderPlus } from "lucide-react";

import type { CandidateView } from "../services/client";
import { Badge, Button, Field } from "./ui";

/** 分类目录（最多一级）留空表示不分类。 */
export function UpstreamCandidates({
  candidates,
  selected,
  onToggle,
  onSelectAll,
  onReject,
  category,
  onCategoryChange,
  busy,
}: {
  candidates: CandidateView[];
  /** 勾选入库的子路径。 */
  selected: string[];
  onToggle: (subpath: string) => void;
  onSelectAll: (checked: boolean) => void;
  onReject: (subpath: string) => void;
  category: string;
  onCategoryChange: (value: string) => void;
  busy: boolean;
}) {
  const allSelected = candidates.length > 0 && selected.length === candidates.length;

  return (
    <div>
      <div className="mb-2.5 flex items-center gap-3">
        <label className="flex items-center gap-1.5 text-[13px]">
          <input
            type="checkbox"
            className="selectbox"
            checked={allSelected}
            disabled={busy || candidates.length === 0}
            onChange={(event) => onSelectAll(event.target.checked)}
          />
          新增技能
        </label>
        <span className="text-[12px] text-faint">勾选才入库</span>
        <span className="flex-1" />
        <span className="text-[13px] text-muted">
          已选 {selected.length} / {candidates.length}
        </span>
      </div>

      {candidates.length === 0 ? (
        <p className="rounded-lg border border-line-soft bg-panel px-3.5 py-4 text-[13px] text-muted">
          该仓库没有待决定的新增候选。
        </p>
      ) : (
        <div className="overflow-hidden rounded-lg border border-line-soft">
          {candidates.map((candidate) => {
            const checked = selected.includes(candidate.subpath);
            return (
              <div
                key={candidate.subpath}
                className="flex items-start gap-3 border-t border-line-faint px-3.5 py-2.5 first:border-t-0"
              >
                <input
                  type="checkbox"
                  className="selectbox mt-0.5 shrink-0"
                  checked={checked}
                  disabled={busy}
                  onChange={() => onToggle(candidate.subpath)}
                />
                <span className="min-w-0 flex-1">
                  <span className="flex items-center gap-2">
                    <span className="truncate text-[14px] font-semibold">{candidate.name}</span>
                    {candidate.is_new ? (
                      <Badge tone="green">后来新增</Badge>
                    ) : (
                      <Badge tone="gray">首次未选</Badge>
                    )}
                  </span>
                  <span className="mt-0.5 block truncate text-[12px] text-faint">
                    {candidate.description ?? "（无用途说明）"}
                  </span>
                  <span className="mt-0.5 block truncate font-mono text-[11px] text-faint">
                    {candidate.subpath}
                  </span>
                </span>
                <Button compact disabled={busy} onClick={() => onReject(candidate.subpath)}>
                  <FolderPlus className="size-3.5" strokeWidth={1.75} />
                  不入库并记住
                </Button>
              </div>
            );
          })}
        </div>
      )}

      <div className="mt-3.5">
        <Field label="新增技能的分类目录（最多一级，留空为不分类）">
          <input
            className="field-input"
            value={category}
            placeholder="例如：matt；留空表示进入管理域根下"
            onChange={(event) => onCategoryChange(event.target.value)}
          />
        </Field>
      </div>
    </div>
  );
}
