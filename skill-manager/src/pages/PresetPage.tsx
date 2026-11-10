/**
 * 预设页（US7 / T065）：组合定义管理与成员全库浏览。
 *
 * - 组合定义（名称/图标/两种显示顺序/成员）与**实际部署分开**：编辑定义不触发部署；
 * - 成员浏览复用技能库的分类栏、标签栏、搜索与网格／列表，分类→标签联动由 core 给出；
 * - 卡片开关直接加入／移出成员，多选可批量调整；缺失成员如实标注且不阻塞其他成员；
 * - 部署是叠加、撤销按成员执行，撤销前说明跨预设成员的影响范围。
 * 前端只提交条件与展示结果，不复制搜索、排序或准入判断。
 */
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  CheckSquare,
  GripVertical,
  LayoutGrid,
  List,
  Rocket,
  Search,
  Square,
} from "lucide-react";

import {
  addPresetMembers,
  deployStatus,
  listPresets,
  listSkills,
  removePresetMembers,
  renameCategory,
  reorderPresetMembers,
  type AgentView,
  type DeploymentStatusView,
  type Domain,
  type EnvironmentOs,
  type PresetView,
  type SkillListResult,
  type SkillSummary,
} from "../services/client";
import { reorderVisible } from "../lib/presetOrder";
import { unsupportedDeployReason } from "../lib/support";
import { DeployDialog } from "../components/DeployDialog";
import { QuickDeploy } from "../components/QuickDeploy";
import { PresetDeployDialog } from "../components/PresetDeployDialog";
import { SkillDetail } from "../components/SkillDetail";
import { TagChip } from "../components/TagChip";
import { toggleTagSelection, pruneSelectedTags, type TagMode } from "../lib/tagFilter";
import { CategoryRail, CATEGORY_UNCATEGORIZED } from "../components/CategoryRail";
import { Badge, Button, Notice, Segmented } from "../components/ui";

const UNCATEGORIZED = CATEGORY_UNCATEGORIZED;
const NO_TAG = "__none";

type Layout = "grid" | "list";
type MemberFilter = "all" | "in" | "out";

function lower(value: string): string {
  return value.toLowerCase();
}

/** 预设成员开关：开=已加入，关=未加入；只修改组合定义。 */
function MemberToggle({
  enabled,
  disabled,
  onToggle,
}: {
  enabled: boolean;
  disabled?: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={enabled}
      aria-label={enabled ? "移出该预设" : "加入该预设"}
      title={enabled ? "移出该预设" : "加入该预设"}
      disabled={disabled}
      onClick={(event) => {
        event.stopPropagation();
        onToggle();
      }}
      className={`relative h-[18px] w-[34px] shrink-0 rounded-full border transition-colors disabled:cursor-default disabled:opacity-40 ${
        enabled ? "border-accent-strong bg-accent-strong" : "border-line bg-subtle"
      }`}
    >
      <span
        className={`absolute top-[1px] size-[14px] rounded-full bg-white shadow transition-all ${
          enabled ? "left-[17px]" : "left-[1px]"
        }`}
      />
    </button>
  );
}


export interface PresetPageProps {
  os: EnvironmentOs | null;
  agents: AgentView[];
  reloadToken: number;
  /** 侧栏选定的管理域（预设列表已迁到 AppShell）。 */
  domain: Domain;
  /** 侧栏选定的预设 id；无预设时为 null。 */
  selectedId: string | null;
  onSelectedIdChange: (id: string | null) => void;
  /** 上报当前管理域预设数量，用于页头计数。 */
  onCount: (count: number) => void;
  /** 上报当前管理域标题。 */
  onTitle: (title: string) => void;
  /** 保存库内修改后的统一流程（刷新已有部署并按已确认策略上传）。 */
  onLibrarySaved?: () => Promise<void>;
}

export function PresetPage({
  os,
  agents,
  reloadToken,
  domain,
  selectedId,
  onSelectedIdChange,
  onCount,
  onTitle,
  onLibrarySaved,
}: PresetPageProps) {
  const [presets, setPresets] = useState<PresetView[]>([]);
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [category, setCategory] = useState<string | null>(null);
  const [tagMode, setTagMode] = useState<TagMode>("all");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [memberFilter, setMemberFilter] = useState<MemberFilter>("all");
  const [layout, setLayout] = useState<Layout>("grid");
  const [multi, setMulti] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const [all, setAll] = useState<SkillListResult | null>(null);
  const [filtered, setFiltered] = useState<SkillListResult | null>(null);
  const [version, setVersion] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [deployOpen, setDeployOpen] = useState(false);
  const [detail, setDetail] = useState<string | null>(null);
  const [skillDeploy, setSkillDeploy] = useState<string[] | null>(null);
  const [records, setRecords] = useState<DeploymentStatusView[]>([]);
  const [dragMember, setDragMember] = useState<string | null>(null);

  const preset = useMemo(
    () => presets.find((item) => item.id === selectedId) ?? null,
    [presets, selectedId],
  );
  useEffect(() => {
    onTitle(preset?.name ?? "预设");
    onCount(preset?.members.length ?? 0);
  }, [
    onCount,
    onTitle,
    preset?.id,
    preset?.members.length,
    preset?.name,
  ]);

  // 搜索去抖：core 每次查询都会重扫库文件。
  useEffect(() => {
    const timer = setTimeout(() => setDebouncedQuery(query), 250);
    return () => clearTimeout(timer);
  }, [query]);

  const refreshPresets = useCallback(async () => {
    try {
      const list = await listPresets(domain);
      setPresets(list);
      const next =
        selectedId && list.some((item) => item.id === selectedId)
          ? selectedId
          : list[0]?.id ?? null;
      if (next !== selectedId) {
        onSelectedIdChange(next);
      }
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, [domain, onSelectedIdChange, selectedId]);

  useEffect(() => {
    void refreshPresets();
  }, [refreshPresets, version, reloadToken]);

  // 本机部署记录：技能详情与部署状态展示用它（core 真实记录，不用模拟数据）。
  useEffect(() => {
    void deployStatus()
      .then(setRecords)
      .catch((err) => setError(String(err)));
  }, [version, reloadToken]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const base = await listSkills({ domain });
        if (cancelled) {
          return;
        }
        const hasFilter =
          debouncedQuery.trim().length > 0 || category !== null || tagMode !== "all";
        const result = hasFilter
          ? await listSkills({
              domain,
              query: debouncedQuery.trim() || undefined,
              category: category !== null && category !== UNCATEGORIZED ? category : undefined,
              uncategorized: category === UNCATEGORIZED || undefined,
              tags: tagMode === "and" ? selectedTags : undefined,
              noTag: tagMode === "none" || undefined,
            })
          : base;
        if (cancelled) {
          return;
        }
        setAll(base);
        setFiltered(result);
        setError(null);
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [domain, debouncedQuery, category, tagMode, selectedTags, version, reloadToken]);

  // 切换管理域或筛选时清空多选与选择。
  useEffect(() => {
    setSelected([]);
  }, [domain, debouncedQuery, category, tagMode, selectedTags, memberFilter]);

  useEffect(() => {
    setQuery("");
    setDebouncedQuery("");
    setCategory(null);
    setTagMode("all");
    setSelectedTags([]);
    setMemberFilter("all");
    setMulti(false);
  }, [domain]);

  const memberNames = useMemo(
    () => (preset?.members ?? []).map((member) => member.name),
    [preset],
  );
  const memberSet = useMemo(
    () => new Set(memberNames.map(lower)),
    [memberNames],
  );

  /** 成员同时属于的其他预设（用于如实说明撤销影响范围）。 */
  const sharedMembers = useMemo(() => {
    if (!all || !preset) {
      return [];
    }
    return memberNames.filter((name) => {
      const skill = all.skills.find((item) => lower(item.name) === lower(name));
      return (skill?.presets ?? []).some((ref) => ref.id !== preset.id);
    });
  }, [all, preset, memberNames]);

  const skills = useMemo(() => {
    const base = filtered?.skills ?? [];
    if (!preset) {
      return base;
    }
    const withMemberState = base.map((skill) => ({
      skill,
      isMember: memberSet.has(lower(skill.name)),
    }));
    const visible =
      memberFilter === "all"
        ? withMemberState
        : withMemberState.filter((item) =>
            memberFilter === "in" ? item.isMember : !item.isMember,
          );
    if (memberFilter === "all") {
      // 默认已加入成员优先；不改变定义，只影响展示顺序。
      return [...visible]
        .sort((a, b) => Number(b.isMember) - Number(a.isMember))
        .map((item) => item.skill);
    }
    return visible.map((item) => item.skill);
  }, [filtered, preset, memberSet, memberFilter]);

  const tagCandidates = useMemo(() => filtered?.tag_candidates ?? [], [filtered]);
  useEffect(() => {
    if (tagMode !== "and") {
      return;
    }
    const next = pruneSelectedTags(selectedTags, tagCandidates);
    if (next.length !== selectedTags.length) {
      if (next.length === 0) {
        setTagMode("all");
        setSelectedTags([]);
      } else {
        setSelectedTags(next);
      }
    }
  }, [tagMode, selectedTags, tagCandidates]);

  const railCategories = useMemo(() => {
    const counts = new Map<string, number>();
    let uncategorized = 0;
    for (const skill of all?.skills ?? []) {
      if (skill.category === null) {
        uncategorized += 1;
      } else {
        counts.set(skill.category, (counts.get(skill.category) ?? 0) + 1);
      }
    }
    return { namedCounts: counts, uncategorized };
  }, [all]);

  const selectedSkills = skills.filter((skill) => selected.includes(referenceOf(skill)));
  const missingMemberCount = preset
    ? preset.members.filter((member) => member.missing).length
    : 0;

  function referenceOf(skill: SkillSummary): string {
    return `${skill.domain}/${skill.name}`;
  }

  async function runMemberChange(names: string[], mode: "add" | "remove") {
    if (!preset || names.length === 0) {
      return;
    }
    try {
      const updated =
        mode === "add"
          ? await addPresetMembers(domain, preset.id, names)
          : await removePresetMembers(domain, preset.id, names);
      setNotice(
        mode === "add"
          ? `已加入 ${names.length} 个成员到「${updated.name}」（不改动任何部署）。`
          : `已从「${updated.name}」移出 ${names.length} 个成员（不改动任何部署）。`,
      );
      setError(null);
      setVersion((value) => value + 1);
      await onLibrarySaved?.();
    } catch (err) {
      setError(String(err));
    }
  }

  async function toggleMember(skill: SkillSummary) {
    const isMember = memberSet.has(lower(skill.name));
    await runMemberChange([skill.name], isMember ? "remove" : "add");
  }

  async function renameRailCategory(from: string, to: string) {
    try {
      const result = await renameCategory(domain, from, to);
      setNotice(`已将分类「${result.from}」重命名为「${result.to}」，涉及 ${result.affected} 个技能。`);
      setError(null);
      if (category === from) {
        setCategory(result.to);
      }
      setVersion((value) => value + 1);
      await onLibrarySaved?.();
    } catch (err) {
      setError(String(err));
    }
  }


  async function reorderMemberList(targetName: string) {
    if (!preset || !dragMember || dragMember === targetName) {
      return;
    }
    const visible = skills.map((skill) => skill.name);
    if (!visible.includes(dragMember) || !visible.includes(targetName)) {
      return;
    }
    const from = visible.indexOf(dragMember);
    const to = visible.indexOf(targetName);
    const nextVisible = [...visible];
    const [moved] = nextVisible.splice(from, 1);
    nextVisible.splice(to, 0, moved);
    setDragMember(null);
    // 只调整可见条目的相对位置：非可见成员保持原位。
    const full = reorderVisible(memberNames, nextVisible);
    try {
      const updated = await reorderPresetMembers(domain, preset.id, full);
      setNotice("已调整成员显示顺序（不增减成员、不动目录、不改变部署）。");
      setError(null);
      setPresets((current) =>
        current.map((item) => (item.id === updated.id ? updated : item)),
      );
      await onLibrarySaved?.();
    } catch (err) {
      setError(String(err));
    }
  }


  const filterActive =
    debouncedQuery.trim().length > 0 ||
    category !== null ||
    tagMode !== "all" ||
    memberFilter !== "all";

  function onToggleTag(name: string) {
    const next = toggleTagSelection(tagMode, selectedTags, name);
    setTagMode(next.mode);
    setSelectedTags(next.selected);
  }

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col">
        {error && (
          <div className="mb-3.5 shrink-0">
            <Notice tone="danger">{error}</Notice>
          </div>
        )}
        {notice && (
          <div className="mb-3.5 shrink-0">
            <Notice>{notice}</Notice>
          </div>
        )}

        {!preset ? (
          <p className="px-4 py-8 text-[13px] text-muted">
            该库还没有预设，请从左侧新建。
          </p>
        ) : (
          <>
            <div className="mb-3.5 flex shrink-0 flex-wrap items-center justify-between gap-3">
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <h2 className="text-[15px] font-semibold">{preset.name}</h2>
                  <span className="rounded-md bg-subtle px-1.5 py-0.5 font-mono text-[11px] text-muted">
                    {preset.id}
                  </span>
                  <span className="text-[12px] text-faint">
                    成员 {preset.members.length} 个
                    {missingMemberCount > 0 && (
                      <span className="text-warn">（{missingMemberCount} 个缺失）</span>
                    )}
                  </span>
                </div>
                {(preset.missing_members?.length ?? 0) > 0 && (
                  <p className="mt-1 text-[12px] text-warn">
                    缺失成员：{preset.missing_members?.join("、")}
                    （库内已无同名技能；不阻塞其他成员）
                  </p>
                )}
              </div>
              <div className="flex items-center gap-2.5">
                <Button compact onClick={() => setDeployOpen(true)}>
                  <Rocket className="size-3.5" strokeWidth={1.75} />
                  组合部署…
                </Button>
              </div>
            </div>

            <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
              <div className="flex flex-wrap items-center gap-2.5">
                <label className="flex h-10 w-[260px] items-center gap-2.5 rounded-lg border border-line bg-panel px-3 text-faint">
                  <Search className="size-4 shrink-0" strokeWidth={1.75} />
                  <input
                    value={query}
                    placeholder="搜索技能名、注释、标签…"
                    onChange={(event) => setQuery(event.target.value)}
                    className="w-full border-0 bg-transparent text-[13px] text-ink-3 outline-0 placeholder:text-faint"
                  />
                </label>
                <Segmented
                  options={[
                    { value: "all", label: "全部" },
                    { value: "in", label: "已加入" },
                    { value: "out", label: "未加入" },
                  ]}
                  value={memberFilter}
                  onChange={(value) => setMemberFilter(value as MemberFilter)}
                />
              </div>
              <div className="flex items-center gap-2.5">
                <div className="flex h-10 items-center rounded-lg border border-line bg-panel p-[3px]">
                  {(
                    [
                      ["grid", "网格", LayoutGrid],
                      ["list", "列表", List],
                    ] as const
                  ).map(([value, label, Icon]) => (
                    <button
                      key={value}
                      type="button"
                      title={label}
                      aria-label={label}
                      onClick={() => setLayout(value)}
                      className={`grid h-8 w-[30px] place-items-center rounded-[5px] ${
                        layout === value ? "bg-selected text-ink-3" : "text-muted"
                      }`}
                    >
                      <Icon className="size-4" strokeWidth={1.75} />
                    </button>
                  ))}
                  <button
                    type="button"
                    onClick={() => {
                      setMulti((current) => !current);
                      setSelected([]);
                    }}
                    className={`ml-1.5 flex h-8 items-center gap-1.5 rounded-[5px] border-l border-line px-2.5 text-[13px] ${
                      multi ? "bg-selected text-ink-2" : "text-muted"
                    }`}
                  >
                    {multi ? (
                      <CheckSquare className="size-4" strokeWidth={1.75} />
                    ) : (
                      <Square className="size-4" strokeWidth={1.75} />
                    )}
                    {multi ? "取消多选" : "多选"}
                  </button>
                </div>
              </div>
            </div>

            <div className="mb-3.5 flex min-h-[23px] shrink-0 flex-wrap items-center gap-1.5">
              <span className="mr-0.5 text-[12px] text-faint">标签</span>
              <button
                type="button"
                onClick={() => onToggleTag("")}
                aria-pressed={tagMode === "all"}
                className={`rounded-full px-2 text-[12px] leading-5 ${
                  tagMode === "all" ? "bg-accent text-white" : "bg-subtle text-muted"
                }`}
              >
                全部标签
              </button>
              <button
                type="button"
                onClick={() => onToggleTag(NO_TAG)}
                aria-pressed={tagMode === "none"}
                className={`rounded-full border border-line px-2 text-[12px] leading-[18px] ${
                  tagMode === "none" ? "bg-accent text-white" : "text-muted"
                }`}
              >
                无标签
              </button>
              {tagCandidates.map((candidate) => {
                const on = tagMode === "and" && selectedTags.includes(candidate);
                return (
                  <button
                    key={candidate}
                    type="button"
                    onClick={() => onToggleTag(candidate)}
                    title={candidate}
                    aria-pressed={on}
                    className={`inline-flex items-center rounded-full text-[12px] leading-5 ${
                      on ? "bg-accent px-2 text-white" : "px-0"
                    }`}
                  >
                    {on ? candidate : <TagChip tag={candidate} />}
                  </button>
                );
              })}
            </div>

            {multi && (
              <div className="mb-3.5 flex shrink-0 flex-wrap items-center gap-2.5 rounded-lg border border-line bg-panel p-2.5">
                <Button
                  compact
                  disabled={skills.length === 0 || selectedSkills.length === skills.length}
                  onClick={() => setSelected(skills.map((skill) => referenceOf(skill)))}
                >
                  全选
                </Button>
                <Button compact disabled={selected.length === 0} onClick={() => setSelected([])}>
                  全不选
                </Button>
                <span className="text-[13px] text-muted">已选择 {selected.length}</span>
                <Button
                  compact
                  disabled={selected.length === 0}
                  onClick={() => void runMemberChange(selectedSkills.map((s) => s.name), "add")}
                >
                  批量加入成员
                </Button>
                <Button
                  compact
                  disabled={selected.length === 0}
                  onClick={() => void runMemberChange(selectedSkills.map((s) => s.name), "remove")}
                >
                  批量移出成员
                </Button>
              </div>
            )}

            <div className="flex min-h-0 flex-1 items-stretch gap-4">
              <CategoryRail
                domain={domain}
                total={all?.total ?? 0}
                namedCounts={railCategories.namedCounts}
                uncategorizedCount={railCategories.uncategorized}
                selected={category}
                onSelect={setCategory}
                onRenameNamed={(from, to) => void renameRailCategory(from, to)}
              />

              <div className="card-pane">
                <div
                  className={
                    layout === "grid"
                      ? "grid grid-cols-[repeat(auto-fill,minmax(min(100%,400px),1fr))] gap-3"
                      : "grid grid-cols-1 gap-3"
                  }
                >
                {skills.map((skill) => {
                  const reference = referenceOf(skill);
                  const isMember = memberSet.has(lower(skill.name));
                  const isSelected = selected.includes(reference);
                  const canDragMember = memberFilter === "in";
                  return (
                    <article
                      key={reference}
                      draggable={canDragMember && !multi}
                      onDragStart={() => canDragMember && setDragMember(skill.name)}
                      onDragOver={(event) => canDragMember && event.preventDefault()}
                      onDrop={() => void reorderMemberList(skill.name)}
                      className={`group min-w-0 overflow-hidden rounded-xl border border-line bg-panel shadow-card transition-shadow hover:shadow-card-hover ${
                        multi && isSelected
                          ? "border-accent shadow-[0_0_0_1px_rgba(5,150,105,0.33)]"
                          : ""
                      } ${layout === "list" ? "grid grid-cols-[340px_1fr_260px] items-center" : "flex flex-col"}`}
                    >
                      <div
                        className={`flex items-center gap-2.5 ${
                          layout === "list" ? "px-3 py-3" : "min-h-9 px-3.5 pt-2.5"
                        }`}
                      >
                        {multi ? (
                          <input
                            type="checkbox"
                            className="selectbox shrink-0"
                            checked={isSelected}
                            onChange={() =>
                              setSelected((current) =>
                                current.includes(reference)
                                  ? current.filter((item) => item !== reference)
                                  : [...current, reference],
                              )
                            }
                          />
                        ) : canDragMember ? (
                          <span
                            title="拖动调整成员显示顺序"
                            className="grid size-4 shrink-0 cursor-grab place-items-center text-faint"
                          >
                            <GripVertical className="size-3.5" strokeWidth={1.75} />
                          </span>
                        ) : (
                          <span
                            title={isMember ? "已加入该预设" : "未加入该预设"}
                            className={`size-2 shrink-0 rounded-full ${
                              isMember ? "bg-accent-light" : "bg-faint"
                            }`}
                          />
                        )}
                        <h3 className="flex min-w-0 flex-1 items-baseline gap-1.5">
                          <button
                            type="button"
                            title={skill.name}
                            onClick={() => setDetail(reference)}
                            className="max-w-full shrink-0 text-left text-[14px] leading-5 font-semibold [overflow-wrap:anywhere] group-hover:text-accent"
                          >
                            {skill.name}
                          </button>
                          {skill.comment && (
                            <span title={skill.comment} className="min-w-0 truncate text-[11px] font-normal text-faint">
                              （{skill.comment}）
                            </span>
                          )}
                        </h3>
                        <MemberToggle
                          enabled={isMember}
                          onToggle={() => void toggleMember(skill)}
                        />
                      </div>

                      <div
                        role="presentation"
                        onClick={() => setDetail(reference)}
                        className={`min-w-0 ${layout === "list" ? "cursor-pointer px-2.5 py-2.5" : "cursor-pointer px-3.5 pt-1 pb-3"}`}
                      >
                        <p className="truncate text-[13px] leading-[19px] text-faint">
                          {skill.description ?? ""}
                        </p>
                        <div className="mt-2.5 flex flex-wrap items-center gap-1.5">
                          {skill.tags.map((item) => (
                            <TagChip key={item} tag={item} />
                          ))}
                          {skill.translation_stale && (
                            <Badge tone="amber" title="原文已变化，译文尚未维护">
                              译文待更新
                            </Badge>
                          )}
                        </div>
                      </div>
                      <div className="mt-auto flex items-center border-t border-line-faint px-3.5 py-2">
                        <QuickDeploy deploymentBlocks={skill.deployment_blocks} reference={reference} domain={domain} agents={agents} records={records}
                          onChange={async () => setRecords(await deployStatus())} onError={setError} />
                      </div>
                    </article>
                  );
                })}

                {skills.length === 0 && (
                  <div className="col-span-full px-4 py-8 text-center text-[13px] text-muted">
                    <p>没有符合条件的技能。</p>
                    {filterActive && (
                      <div className="mt-2 flex justify-center">
                        <Button
                          compact
                          onClick={() => {
                            setQuery("");
                            setDebouncedQuery("");
                            setCategory(null);
                            setTagMode("all");
                            setSelectedTags([]);
                            setMemberFilter("all");
                          }}
                        >
                          清除筛选
                        </Button>
                      </div>
                    )}
                  </div>
                )}
                </div>
              </div>
            </div>
          </>
        )}

      {deployOpen && preset && (
        <PresetDeployDialog
          domain={domain}
          preset={preset}
          agents={agents}
          sharedMembers={sharedMembers}
          unsupportedReason={unsupportedDeployReason(
            preset.members.map((member) => ({
              name: member.name,
              oses:
                all?.skills.find(
                  (skill) => skill.name.toLowerCase() === member.name.toLowerCase(),
                )?.oses ?? null,
            })),
            os,
          )}
          onClose={() => { setDeployOpen(false); setVersion((value) => value + 1); }}
          onDone={(message) => {
            setNotice(message);
            setError(null);
            setVersion((value) => value + 1);
          }}
        />
      )}


      {detail && (
        <SkillDetail
          reference={detail}
          os={os}
          agents={agents}
          tagCandidates={all?.tag_candidates ?? []}
          records={records}
          reloadToken={reloadToken + version}
          onClose={() => setDetail(null)}
          onNotice={(message) => setNotice(message)}
          onOpenMove={() => setNotice("移动到分类请在技能库页面执行。")}
          onDeploy={(reference) => setSkillDeploy([reference])}
          onDelete={() => setNotice("删除技能请在技能库页面执行。")}
          onSyncDeployments={() => {
            void (onLibrarySaved ? onLibrarySaved() : Promise.resolve());
          }}
          onDeploymentsChanged={async () => setRecords(await deployStatus())}
        />
      )}

      {skillDeploy && (
        <DeployDialog
          domain={domain}
          references={skillDeploy}
          agents={agents}
          onClose={() => { setSkillDeploy(null); setVersion((value) => value + 1); }}
          onDone={(message) => {
            setNotice(message);
            setError(null);
            setVersion((value) => value + 1);
          }}
        />
      )}
    </div>
  );
}
