/**
 * 配套 Skill 候选入库弹窗（US9 场景 6 / T081）。
 *
 * 复用 US2 的候选获取与选择入库流程：先按来源仓库真实获取并确认候选，再显式选择入库。
 * 单候选也列出（原名、中文注释、用途、来源、勾选与已选数量）；必须由用户勾选；
 * 入库不自动部署；工具版本与各 Skill 来源版本分别展示；不复制技能正文进工具资料。
 */
import { useMemo, useState } from "react";
import { Layers } from "lucide-react";

import {
  importSkills,
  previewRepo,
  setSkill,
  type Domain,
  type ToolSkillCandidate,
  type ToolSkillCandidates as ToolSkillCandidatesData,
} from "../services/client";
import { TagChip } from "./TagChip";
import { Badge, Button, Dialog, Field, Notice } from "./ui";

export interface ToolSkillCandidatesProps {
  data: ToolSkillCandidatesData;
  onClose: () => void;
  onDone: (message: string) => void;
}

/** 库内已存在的候选与待入库候选分开显示，避免把“已入库”当成待办。 */
export function ToolSkillCandidates({ data, onClose, onDone }: ToolSkillCandidatesProps) {
  const pendingCandidates = useMemo(
    () => data.candidates.filter((item) => !item.in_vault),
    [data.candidates],
  );

  const [selected, setSelected] = useState<string[]>(() =>
    pendingCandidates.map((item) => item.name),
  );
  const [domain, setDomain] = useState<Domain>("lobster");
  const [category, setCategory] = useState("");
  const [comment, setComment] = useState("");
  const [tags, setTags] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function toggle(name: string) {
    setSelected((current) =>
      current.includes(name) ? current.filter((item) => item !== name) : [...current, name],
    );
  }

  const chosen = pendingCandidates.filter((item) => selected.includes(item.name));

  async function confirm() {
    if (chosen.length === 0) {
      setError("请至少勾选一个候选技能；未选择不会入库。");
      return;
    }
    setBusy(true);
    setError(null);
    const notes: string[] = [];
    const failures: string[] = [];
    const tagList = tags
      .split(/[,，]/)
      .map((item) => item.trim())
      .filter((item) => item.length > 0);
    const metadata = {
      comment: comment.trim() ? comment.trim() : undefined,
      tags: tagList.length > 0 ? tagList : undefined,
    };

    // 按来源仓库分组：同一仓库只获取一次（复用 US2 的预览 → 入库流程）。
    const groups = new Map<string, ToolSkillCandidate[]>();
    for (const item of chosen) {
      if (!item.repo) {
        failures.push(`${item.name}：该候选没有来源仓库地址，无法入库（请由 Agent 补充 skills.repo）`);
        continue;
      }
      const list = groups.get(item.repo) ?? [];
      list.push(item);
      groups.set(item.repo, list);
    }

    for (const [repo, items] of groups) {
      const jobId = `tool-skill-${data.slug}-${Date.now()}`;
      try {
        await previewRepo(repo, undefined, jobId);
        const subpaths = items.map((item) => item.subpath ?? item.name);
        const outcome = await importSkills(repo, domain, subpaths, {
          category: category.trim() ? category.trim() : undefined,
          uncategorized: category.trim() ? undefined : true,
          jobId,
        });
        for (const item of outcome.items) {
          notes.push(`${domain}/${item.name}`);
          if (metadata.comment || metadata.tags) {
            await setSkill(`${domain}/${item.name}`, metadata);
          }
        }
        for (const item of outcome.errors) {
          failures.push(`${item.skill ?? repo}：${item.message}`);
        }
      } catch (err) {
        failures.push(`${repo}：${String(err)}`);
      }
    }
    setBusy(false);

    const parts: string[] = [];
    if (notes.length > 0) {
      parts.push(`已入库 ${notes.length} 项（${notes.join("、")}），未自动部署`);
    }
    if (failures.length > 0) {
      parts.push(`${failures.length} 项未完成：${failures.join("；")}`);
    }
    onDone(parts.length > 0 ? `${parts.join("；")}。` : "没有可入库的候选。");
  }

  return (
    <Dialog
      title={`配套 Skill 入库 · ${data.slug}`}
      subtitle="进入技能库统一维护；入库不会自动部署到 Agent 或 Hermes 档案。"
      width="w-[880px]"
      onClose={onClose}
      footer={
        <>
          <span className="text-[12px] text-faint">
            工具版本：{data.tool.installed ? `已安装 ${data.tool.installed_version ?? "-"}` : "未安装"}
            {data.tool.latest_version ? ` · 检查所得上游版本 ${data.tool.latest_version}` : " · 尚未检查上游版本"}
            （与下方 Skill 来源版本分别管理）
          </span>
          <div className="flex shrink-0 items-center gap-2.5">
            <Button onClick={onClose}>关闭</Button>
            <Button
              kind="primary"
              disabled={busy || chosen.length === 0}
              onClick={() => void confirm()}
            >
              {busy ? "正在入库…" : "确认入库"}
            </Button>
          </div>
        </>
      }
    >
      {error && (
        <div className="mb-4">
          <Notice tone="danger">{error}</Notice>
        </div>
      )}

      <div className="mb-3.5 rounded-[10px] border border-line p-3.5">
        <div className="mb-2.5 flex items-center gap-2.5">
          <Layers className="size-4 text-muted" strokeWidth={1.75} />
          <span className="text-[13px]">选择要入库的 Skill</span>
          <span className="flex-1" />
          <span className="text-[13px] text-muted">
            已选 {chosen.length} / {pendingCandidates.length}
          </span>
        </div>

        {data.candidates.length === 0 ? (
          <p className="rounded-lg border border-line-soft bg-panel px-3.5 py-4 text-[13px] text-muted">
            该工具尚未登记配套 Skill 关联；可由 Agent 用 `tool set` 或 `tool skill-link` 补充。
          </p>
        ) : (
          <div className="flex flex-col gap-2">
            {data.candidates.map((item) => {
              const checked = selected.includes(item.name);
              return (
                <label
                  key={`${item.domain}/${item.name}`}
                  className={`flex cursor-pointer items-start gap-2.5 rounded-lg border px-3 py-2.5 ${
                    item.in_vault ? "border-line-soft bg-subtle" : "border-line bg-panel"
                  }`}
                >
                  <input
                    type="checkbox"
                    className="selectbox mt-0.5"
                    checked={checked}
                    disabled={item.in_vault || busy}
                    onChange={() => toggle(item.name)}
                  />
                  <span className="min-w-0 flex-1">
                    <span className="flex flex-wrap items-center gap-2">
                      <span className="text-[13px] font-semibold">{item.name}</span>
                      {item.comment && <span className="text-[12px] text-faint">（{item.comment}）</span>}
                      {item.in_vault ? (
                        <Badge tone="green">已入库</Badge>
                      ) : (
                        <Badge tone="amber">待入库</Badge>
                      )}
                      {item.source_version && (
                        <span
                          title={`Skill 来源版本：${item.source_version}`}
                          className="rounded-full border border-line-soft bg-subtle px-2 text-[11px] leading-[18px] text-muted"
                        >
                          Skill 版本 {item.source_version.slice(0, 7)}
                        </span>
                      )}
                    </span>
                    <span className="mt-1 block text-[12px] leading-[18px] text-faint">
                      {item.description ?? (item.in_vault ? "库内没有用途描述" : "未入库：暂无库内资料")}
                    </span>
                    <span
                      className="mt-1 block truncate text-[12px] text-faint"
                      title={item.repo ?? "无来源仓库"}
                    >
                      来源：{item.repo ?? "未记录"}{item.subpath ? ` · ${item.subpath}` : ""}
                    </span>
                    {item.tags.length > 0 && (
                      <span className="mt-1.5 flex flex-wrap gap-1.5">
                        {item.tags.map((tag) => (
                          <TagChip key={tag} tag={tag} />
                        ))}
                      </span>
                    )}
                  </span>
                </label>
              );
            })}
          </div>
        )}
      </div>

      <div className="grid grid-cols-2 gap-3.5">
        <Field label="管理域">
          <select
            value={domain}
            onChange={(event) => setDomain(event.target.value as Domain)}
            className="h-10 w-full rounded-lg border border-line bg-panel px-2 text-[13px]"
          >
            <option value="code">编码技能库</option>
            <option value="lobster">龙虾技能库</option>
          </select>
        </Field>
        <Field label="分类目录" hint="最多一级">
          <input
            value={category}
            onChange={(event) => setCategory(event.target.value)}
            placeholder="留空表示不分类"
            className="h-10 w-full rounded-lg border border-line bg-panel px-3 text-[13px] outline-0 focus:border-accent"
          />
        </Field>
        <Field label="中文注释">
          <input
            value={comment}
            onChange={(event) => setComment(event.target.value)}
            placeholder="例如：浏览器任务"
            className="h-10 w-full rounded-lg border border-line bg-panel px-3 text-[13px] outline-0 focus:border-accent"
          />
        </Field>
        <Field label="标签" hint="逗号分隔">
          <input
            value={tags}
            onChange={(event) => setTags(event.target.value)}
            placeholder="例如：页面交互, 自动化"
            className="h-10 w-full rounded-lg border border-line bg-panel px-3 text-[13px] outline-0 focus:border-accent"
          />
        </Field>
      </div>

      <div className="mt-3.5">
        <Notice>
          仅勾选的 Skill 会入库；下方管理资料只用于本次所选技能。入库不部署，工具版本与 Skill
          来源版本互不代表对方已更新。
        </Notice>
      </div>
    </Dialog>
  );
}
