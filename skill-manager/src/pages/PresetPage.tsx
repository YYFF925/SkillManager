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
  Pencil,
  Plus,
  Rocket,
  Search,
  Square,
  Trash2,
} from "lucide-react";

import {
  addPresetMembers,
  createPreset,
  deletePreset,
  deployStatus,
  listPresets,
  listSkills,
  removePresetMembers,
  renamePreset,
  reorderPresetMembers,
  reorderPresets,
  setPresetIcon,
  type AgentView,
  type DeploymentStatusView,
  type Domain,
  type EnvironmentOs,
  type PresetView,
  type SkillListResult,
  type SkillSummary,
} from "../services/client";
import { PRESET_ICON_OPTIONS, presetIconOption } from "../lib/presetIcons";
import { reorderVisible } from "../lib/presetOrder";
import { unsupportedDeployReason } from "../lib/support";
import { DeployDialog } from "../components/DeployDialog";
import { QuickDeploy } from "../components/QuickDeploy";
import { PresetDeployDialog } from "../components/PresetDeployDialog";
import { SkillDetail } from "../components/SkillDetail";
import { TagChip } from "../components/TagChip";
import { Badge, Button, ConfirmDialog, Dialog, Notice, Segmented } from "../components/ui";

const UNCATEGORIZED = "__none";
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

/** 图标选择网格（新建与重命名共用；仅改图标也可保存）。 */
function PresetIconPicker({
  value,
  onChange,
}: {
  value: string | null;
  onChange: (key: string) => void;
}) {
  return (
    <div className="grid max-w-[350px] grid-cols-8 gap-1.5">
      {PRESET_ICON_OPTIONS.map((option) => {
        const Icon = option.icon;
        const active = option.key === value;
        return (
          <button
            key={option.key}
            type="button"
            title={`${option.label}（${option.key}）`}
            aria-label={option.label}
            onClick={() => onChange(option.key)}
            className={`flex h-9 items-center justify-center rounded-lg border transition-colors ${
              active
                ? "border-accent-line bg-accent-soft text-accent"
                : "border-line bg-panel text-muted hover:bg-hover"
            }`}
          >
            <Icon
              className="size-[15px]"
              strokeWidth={1.75}
              style={{ color: option.color }}
            />
          </button>
        );
      })}
    </div>
  );
}

export interface PresetPageProps {
  os: EnvironmentOs | null;
  agents: AgentView[];
  reloadToken: number;
  /** 上报当前管理域预设数量，用于页头计数。 */
  onCount: (count: number) => void;
  /** 上报当前管理域标题。 */
  onTitle: (title: string) => void;
  /** 保存库内修改后的统一流程（刷新已有部署并按已确认策略上传）。 */
  onLibrarySaved?: () => Promise<void>;
  /** 从其他页面指定初始管理域（默认编码库）。 */
  initialDomain?: Domain;
}

export function PresetPage({
  os,
  agents,
  reloadToken,
  onCount,
  onTitle,
  onLibrarySaved,
  initialDomain,
}: PresetPageProps) {
  const [domain, setDomain] = useState<Domain>(initialDomain ?? "code");
  const [presets, setPresets] = useState<PresetView[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [category, setCategory] = useState<string | null>(null);
  const [tag, setTag] = useState<string | null>(null);
  const [memberFilter, setMemberFilter] = useState<MemberFilter>("all");
  const [layout, setLayout] = useState<Layout>("grid");
  const [multi, setMulti] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const [all, setAll] = useState<SkillListResult | null>(null);
  const [filtered, setFiltered] = useState<SkillListResult | null>(null);
  const [version, setVersion] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [editOpen, setEditOpen] = useState(false);
  const [deployOpen, setDeployOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<PresetView | null>(null);
  const [detail, setDetail] = useState<string | null>(null);
  const [skillDeploy, setSkillDeploy] = useState<string[] | null>(null);
  const [records, setRecords] = useState<DeploymentStatusView[]>([]);
  const [dragPreset, setDragPreset] = useState<string | null>(null);
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
      setSelectedId((current) =>
        current && list.some((item) => item.id === current) ? current : list[0]?.id ?? null,
      );
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, [domain]);

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
          debouncedQuery.trim().length > 0 || category !== null || tag !== null;
        const result = hasFilter
          ? await listSkills({
              domain,
              query: debouncedQuery.trim() || undefined,
              category: category !== null && category !== UNCATEGORIZED ? category : undefined,
              uncategorized: category === UNCATEGORIZED || undefined,
              tags: tag !== null && tag !== NO_TAG ? [tag] : undefined,
              noTag: tag === NO_TAG || undefined,
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
  }, [domain, debouncedQuery, category, tag, version, reloadToken]);

  // 切换管理域或筛选时清空多选与选择。
  useEffect(() => {
    setSelected([]);
  }, [domain, debouncedQuery, category, tag, memberFilter]);

  useEffect(() => {
    setQuery("");
    setDebouncedQuery("");
    setCategory(null);
    setTag(null);
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
    if (tag !== null && tag !== NO_TAG && !tagCandidates.includes(tag)) {
      setTag(null);
    }
  }, [tag, tagCandidates]);

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
    return {
      named: [...counts.entries()].sort((left, right) => (left[0] < right[0] ? -1 : 1)),
      uncategorized,
    };
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

  async function reorderPresetList(targetId: string) {
    if (!dragPreset || dragPreset === targetId) {
      return;
    }
    const from = presets.findIndex((item) => item.id === dragPreset);
    const to = presets.findIndex((item) => item.id === targetId);
    if (from < 0 || to < 0) {
      return;
    }
    const next = [...presets];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    setDragPreset(null);
    try {
      const updated = await reorderPresets(domain, next.map((item) => item.id));
      setPresets(updated);
      setNotice("已调整预设显示顺序（不改变成员集合或实际部署）。");
      setError(null);
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

  async function confirmDelete() {
    const target = deleteTarget;
    setDeleteTarget(null);
    if (!target) {
      return;
    }
    try {
      await deletePreset(domain, target.id);
      setNotice(`已删除预设「${target.name}」（实际部署未改变）。`);
      setError(null);
      if (selectedId === target.id) {
        setSelectedId(null);
      }
      setVersion((value) => value + 1);
      await onLibrarySaved?.();
    } catch (err) {
      setError(String(err));
    }
  }

  const filterActive =
    debouncedQuery.trim().length > 0 ||
    category !== null ||
    tag !== null ||
    memberFilter !== "all";

  return (
    <div className="flex items-start gap-4">
      {/* 左侧预设列表 */}
      <aside className="w-[180px] shrink-0">
        <div className="mb-2 flex items-center justify-between px-1">
          <Segmented
            options={[
              { value: "code", label: "编码库" },
              { value: "lobster", label: "龙虾库" },
            ]}
            value={domain}
            onChange={(value) => setDomain(value as Domain)}
            className="h-8"
          />
        </div>
        <div className="flex flex-col gap-0.5">
          {presets.map((item) => {
            const option = presetIconOption(item.icon);
            const Icon = option.icon;
            const active = item.id === selectedId;
            return (
              <div
                key={item.id}
                draggable
                onDragStart={() => setDragPreset(item.id)}
                onDragOver={(event) => event.preventDefault()}
                onDrop={() => void reorderPresetList(item.id)}
                className={`group flex items-center gap-1.5 rounded-md px-2 py-1.5 text-[13px] ${
                  active ? "bg-selected font-medium text-ink" : "text-ink-2 hover:bg-hover"
                }`}
              >
                <span
                  title="拖动调整预设顺序"
                  className="flex size-4 shrink-0 cursor-grab items-center justify-center text-faint opacity-0 group-hover:opacity-100"
                >
                  <GripVertical className="size-3.5" strokeWidth={1.75} />
                </span>
                <Icon className="size-4 shrink-0" strokeWidth={1.75} style={{ color: option.color }} />
                <button
                  type="button"
                  title={`${item.name}（${item.id}）`}
                  onClick={() => setSelectedId(item.id)}
                  className="min-w-0 flex-1 truncate text-left"
                >
                  {item.name}
                </button>
                {(item.missing_members?.length ?? 0) > 0 && (
                  <span title="含缺失成员" className="shrink-0 text-[11px] text-warn">
                    !
                  </span>
                )}
                <button
                  type="button"
                  title="重命名 / 图标"
                  aria-label={`编辑预设 ${item.name}`}
                  onClick={() => {
                    setSelectedId(item.id);
                    setEditOpen(true);
                  }}
                  className="grid size-5 shrink-0 place-items-center rounded-[5px] text-faint opacity-0 hover:bg-hover hover:text-accent group-hover:opacity-100"
                >
                  <Pencil className="size-3.5" strokeWidth={1.75} />
                </button>
                <button
                  type="button"
                  title="删除预设"
                  aria-label={`删除预设 ${item.name}`}
                  onClick={() => setDeleteTarget(item)}
                  className="grid size-5 shrink-0 place-items-center rounded-[5px] text-faint opacity-0 hover:bg-hover hover:text-danger group-hover:opacity-100"
                >
                  <Trash2 className="size-3.5" strokeWidth={1.75} />
                </button>
              </div>
            );
          })}
        </div>
        <button
          type="button"
          onClick={() => setCreateOpen(true)}
          className="mt-1.5 flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] text-muted hover:bg-hover"
        >
          <Plus className="size-4 shrink-0" strokeWidth={1.75} />
          新建预设
        </button>
      </aside>

      {/* 右侧成员浏览 */}
      <div className="min-w-0 flex-1">
        {error && (
          <div className="mb-3.5">
            <Notice tone="danger">{error}</Notice>
          </div>
        )}
        {notice && (
          <div className="mb-3.5">
            <Notice>{notice}</Notice>
          </div>
        )}

        {!preset ? (
          <p className="px-4 py-8 text-[13px] text-muted">
            该管理域还没有预设，请从左侧新建预设。
          </p>
        ) : (
          <>
            <div className="mb-3.5 flex flex-wrap items-center justify-between gap-3">
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

            <div className="mb-3.5 flex min-h-[23px] flex-wrap items-center gap-1.5">
              <span className="mr-0.5 text-[12px] text-faint">标签</span>
              <button
                type="button"
                onClick={() => setTag(null)}
                className={`rounded-full px-2 text-[12px] leading-5 ${
                  tag === null ? "bg-accent text-white" : "bg-subtle text-muted"
                }`}
              >
                全部标签
              </button>
              <button
                type="button"
                onClick={() => setTag(NO_TAG)}
                className={`rounded-full border border-line px-2 text-[12px] leading-[18px] ${
                  tag === NO_TAG ? "bg-accent text-white" : "text-muted"
                }`}
              >
                无标签
              </button>
              {tagCandidates.map((candidate) => (
                <button
                  key={candidate}
                  type="button"
                  onClick={() => setTag(candidate)}
                  title={candidate}
                  className={`rounded-full px-2 text-[12px] leading-5 ${
                    tag === candidate ? "bg-accent text-white" : ""
                  }`}
                >
                  {tag === candidate ? candidate : <TagChip tag={candidate} />}
                </button>
              ))}
            </div>

            {multi && (
              <div className="mb-3.5 flex flex-wrap items-center gap-2.5 rounded-lg border border-line bg-panel p-2.5">
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

            <div className="flex items-start gap-4">
              <aside className="w-[132px] shrink-0">
                <button
                  type="button"
                  onClick={() => setCategory(null)}
                  className={`mb-0.5 flex h-[35px] w-full items-center gap-1.5 rounded-md px-2.5 text-left text-[14px] ${
                    category === null ? "bg-selected text-ink" : "text-ink-3 hover:bg-hover"
                  }`}
                >
                  全部分类
                  <span className="grid h-[18px] min-w-[20px] place-items-center rounded-full bg-subtle px-1.5 text-[11px] text-muted">
                    {all?.total ?? 0}
                  </span>
                </button>
                {railCategories.named.map(([name, count]) => (
                  <button
                    key={name}
                    type="button"
                    title={name}
                    onClick={() => setCategory(name)}
                    className={`mb-0.5 flex h-[35px] w-full items-center gap-1.5 rounded-md px-2.5 text-left text-[14px] ${
                      category === name ? "bg-selected text-ink" : "text-ink-3 hover:bg-hover"
                    }`}
                  >
                    <span className="truncate">{name}</span>
                    <span className="ml-auto grid h-[18px] min-w-[20px] place-items-center rounded-full bg-subtle px-1.5 text-[11px] text-muted">
                      {count}
                    </span>
                  </button>
                ))}
                {railCategories.uncategorized > 0 && (
                  <button
                    type="button"
                    onClick={() => setCategory(UNCATEGORIZED)}
                    className={`mb-0.5 flex h-[35px] w-full items-center gap-1.5 rounded-md px-2.5 text-left text-[14px] ${
                      category === UNCATEGORIZED
                        ? "bg-selected text-ink"
                        : "text-ink-3 hover:bg-hover"
                    }`}
                  >
                    未分类
                    <span className="ml-auto grid h-[18px] min-w-[20px] place-items-center rounded-full bg-subtle px-1.5 text-[11px] text-muted">
                      {railCategories.uncategorized}
                    </span>
                  </button>
                )}
              </aside>

              <div
                className={
                  layout === "grid"
                    ? "grid min-w-0 flex-1 grid-cols-[repeat(auto-fill,minmax(min(100%,400px),1fr))] gap-3"
                    : "grid min-w-0 flex-1 grid-cols-1 gap-3"
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
                            setTag(null);
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
          </>
        )}
      </div>

      {createOpen && (
        <CreatePresetDialog
          domain={domain}
          onCancel={() => setCreateOpen(false)}
          onConfirm={async (name, icon, targetDomain) => {
            try {
              const created = await createPreset(targetDomain, name, icon);
              setNotice(`已新建预设「${created.name}」（id ${created.id}；尚未部署）。`);
              setError(null);
              setCreateOpen(false);
              if (targetDomain !== domain) {
                setDomain(targetDomain);
              }
              setSelectedId(created.id);
              setVersion((value) => value + 1);
              await onLibrarySaved?.();
            } catch (err) {
              setError(String(err));
            }
          }}
        />
      )}

      {editOpen && preset && (
        <EditPresetDialog
          preset={preset}
          onCancel={() => setEditOpen(false)}
          onConfirm={async (name, icon) => {
            try {
              if (name !== preset.name) {
                await renamePreset(domain, preset.id, name);
              }
              if ((icon ?? null) !== (preset.icon ?? null)) {
                await setPresetIcon(domain, preset.id, icon);
              }
              setNotice("已更新预设名称／图标（成员与实际部署未改变）。");
              setError(null);
              setEditOpen(false);
              setVersion((value) => value + 1);
              await onLibrarySaved?.();
            } catch (err) {
              setError(String(err));
            }
          }}
        />
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

      {deleteTarget && (
        <ConfirmDialog
          title="删除预设"
          subtitle="只删除组合定义，不撤销已部署的副本；成员的技能与部署保持不变。"
          message={`将删除预设「${deleteTarget.name}」（${deleteTarget.id}，${deleteTarget.members.length} 个成员）。`}
          confirmLabel="删除"
          danger
          onCancel={() => setDeleteTarget(null)}
          onConfirm={() => void confirmDelete()}
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

/** 新建预设：名称 + 图标（图标可后改）。 */
function CreatePresetDialog({
  domain,
  onCancel,
  onConfirm,
}: {
  domain: Domain;
  onCancel: () => void;
  onConfirm: (name: string, icon: string, domain: Domain) => void | Promise<void>;
}) {
  const [name, setName] = useState("");
  const [icon, setIcon] = useState(PRESET_ICON_OPTIONS[0].key);
  const [targetDomain, setTargetDomain] = useState<Domain>(domain);
  return (
    <Dialog
      title="新建预设"
      subtitle="创建后进入对应技能库，用卡片开关选择组合成员。"
      onClose={onCancel}
      footer={
        <>
          <span className="text-[12px] text-faint">成员稍后在预设详情中用卡片开关选择。</span>
          <div className="flex gap-2.5">
            <Button onClick={onCancel}>取消</Button>
            <Button
              kind="primary"
              disabled={name.trim() === ""}
              onClick={() => void onConfirm(name.trim(), icon, targetDomain)}
            >
              创建
            </Button>
          </div>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <div className="grid grid-cols-[1fr_230px] gap-4">
          <div>
            <label className="mb-2 block text-[12px] text-faint">预设名称</label>
            <input
              autoFocus
              className="field-input"
              value={name}
              placeholder="例如：Rust 依赖检查"
              onChange={(event) => setName(event.target.value)}
            />
          </div>
          <div>
            <label className="mb-2 block text-[12px] text-faint">管理域</label>
            <select
              className="field-input"
              value={targetDomain}
              onChange={(event) => setTargetDomain(event.target.value as Domain)}
            >
              <option value="code">编码技能库</option>
              <option value="lobster">龙虾技能库</option>
            </select>
          </div>
        </div>
        <div>
          <label className="mb-2 block text-[12px] text-faint">图标</label>
          <PresetIconPicker value={icon} onChange={setIcon} />
        </div>
      </div>
    </Dialog>
  );
}

/** 重命名 / 改图标：仅改图标也可保存。 */
function EditPresetDialog({
  preset,
  onCancel,
  onConfirm,
}: {
  preset: PresetView;
  onCancel: () => void;
  onConfirm: (name: string, icon: string | null) => void | Promise<void>;
}) {
  const [name, setName] = useState(preset.name);
  const [icon, setIcon] = useState<string | null>(preset.icon ?? null);
  const changed = name.trim() !== preset.name || (icon ?? null) !== (preset.icon ?? null);
  return (
    <Dialog
      title="重命名 / 图标"
      onClose={onCancel}
      footer={
        <>
          <div className="flex gap-2.5">
            <Button onClick={onCancel}>取消</Button>
            <Button
              kind="primary"
              disabled={!changed || name.trim() === ""}
              onClick={() => void onConfirm(name.trim(), icon)}
            >
              保存
            </Button>
          </div>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <div>
          <label className="mb-2 block text-[12px] text-faint">预设名称</label>
          <input
            autoFocus
            className="field-input"
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
        </div>
        <div>
          <label className="mb-2 block text-[12px] text-faint">
            图标（当前：{presetIconOption(icon).label}）
          </label>
          <PresetIconPicker value={icon} onChange={setIcon} />
          <button
            type="button"
            onClick={() => setIcon(null)}
            className="mt-2 text-[12px] text-muted hover:text-accent"
          >
            清除图标
          </button>
        </div>
      </div>
    </Dialog>
  );
}
