/**
 * 技能库页面（编码库 / 龙虾库）：组合搜索、分类联动、网格/列表、多选批量标签、部署与撤销。
 *
 * 搜索、筛选、标签候选与部署准入都由 core 决定；前端只提交条件与展示结果。
 * 部署状态筛选、部署/撤销、删除与共享目录路径设置接入本机真实记录；检查全部、更新、
 * 新增、入库等后续里程碑入口不出现或明确标注。
 */
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Archive,
  CheckSquare,
  Copy,
  FileText,
  FolderTree,
  GitBranch,
  Layers,
  LayoutGrid,
  List,
  MoreHorizontal,
  Plus,
  RefreshCw,
  Rocket,
  Search,
  Square,
  Trash2,
  Upload,
} from "lucide-react";

import {
  addSkillTags,
  archiveSkills,
  copySkill,
  deleteSkills,
  deployStatus,
  deploySync,
  detachSkillTags,
  listSkills,
  listPresets,
  moveSkill,
  renameCategory,
  skillTask,
  syncPull,
  type CheckReportView,
  type AgentView,
  type DeploymentStatusView,
  type Domain,
  type EnvironmentOs,
  type SkillCheckView,
  type SkillListResult,
  type SkillSummary,
  type PresetView,
} from "../services/client";
import { osLabel, unsupportedDeployReason, unsupportedOs } from "../lib/support";
import { useUpstream } from "../lib/upstream";
import { targetLabel } from "../lib/targets";
import { QuickDeploy } from "../components/QuickDeploy";
import { TagChip } from "../components/TagChip";
import { SkillDetail } from "../components/SkillDetail";
import { DeployDialog } from "../components/DeployDialog";
import { ImportDialog } from "../components/ImportDialog";
import { UpstreamDialog, SingleSkillUpdateDialog, skillHasChange } from "../components/UpstreamDialog";
import {
  CopyToDomainDialog,
  ManageTagsDialog,
  MoveCategoryDialog,
  TagPickerDialog,
  summarizeManageItems,
} from "../components/SkillDialogs";
import { FetchProgressLine } from "../components/FetchProgress";
import { FilterMenu } from "../components/FilterMenu";
import { CategoryRail, CATEGORY_UNCATEGORIZED } from "../components/CategoryRail";
import { toggleTagSelection, pruneSelectedTags, type TagMode } from "../lib/tagFilter";
import { TaskDialog } from "../components/TaskDialog";
import { Badge, Button, ConfirmDialog, Notice, Segmented } from "../components/ui";

/** 分类栏/标签栏的特殊取值：与 core 的“未分类”“无标签”筛选对应。 */
const UNCATEGORIZED = CATEGORY_UNCATEGORIZED;
const NO_TAG = "__none";

/** 卡片菜单的大致高度，用于贴近窗口底部时的翻转定位。 */
const MENU_HEIGHT = 166;

type Layout = "grid" | "list";
type DeployFilter = "all" | "deployed" | "undeployed";

interface CardMenuState {
  reference: string;
  left: number;
  top: number;
}

function referenceOf(skill: SkillSummary): string {
  return `${skill.domain}/${skill.name}`;
}

function sourceLabel(skill: SkillSummary): string {
  return skill.source?.repo ?? "自建";
}

function osReason(skill: SkillSummary, os: EnvironmentOs | null): string {
  const declared =
    skill.oses === null
      ? "未声明系统支持范围（视为全部支持）"
      : skill.oses.length === 0
        ? "显式声明不支持任何系统"
        : `声明支持 ${skill.oses.map(osLabel).join("、")}`;
  return `${declared}；当前系统 ${os === null ? "未知" : osLabel(os)}。仍可维护管理资料。`;
}

export interface LibraryPageProps {
  domain: Domain;
  scheduledReport?: CheckReportView | null;
  os: EnvironmentOs | null;
  agents: AgentView[];
  reloadToken: number;
  /** 上报当前管理域的技能数量，用于页内标题徽标。 */
  onCount: (count: number) => void;
  /** 保存库内修改后的统一流程（先刷新已有部署，再按已确认策略上传）。 */
  onLibrarySaved?: () => Promise<void>;
  /** 从其他页面（如 Hermes 详情）跳转进来时要直接打开的技能引用。 */
  focusReference?: string | null;
  /** 已消费跳转目标。 */
  onFocusConsumed?: () => void;
  /** 上报当前打开/选中的技能引用，供内置管理 Agent 只收引用。 */
  onSelectionChange?: (refs: string[]) => void;
}

export function LibraryPage({
  domain,
  scheduledReport,
  os,
  agents,
  reloadToken,
  onCount,
  onLibrarySaved,
  focusReference,
  onFocusConsumed,
  onSelectionChange,
}: LibraryPageProps) {
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [category, setCategory] = useState<string | null>(null);
  const [tagMode, setTagMode] = useState<TagMode>("all");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [layout, setLayout] = useState<Layout>("grid");
  const [multi, setMulti] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const [all, setAll] = useState<SkillListResult | null>(null);
  const [filtered, setFiltered] = useState<SkillListResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [version, setVersion] = useState(0);
  const [menu, setMenu] = useState<CardMenuState | null>(null);
  const [detail, setDetail] = useState<string | null>(null);
  const [tagDialog, setTagDialog] = useState<
    { mode: "add" | "remove"; targets: string[] } | null
  >(null);
  const [moveTarget, setMoveTarget] = useState<SkillSummary | null>(null);
  const [manageTags, setManageTags] = useState(false);
  const [deployTarget, setDeployTarget] = useState<string[] | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<string[] | null>(null);
  const [archiveTarget, setArchiveTarget] = useState<string[] | null>(null);
  const [records, setRecords] = useState<DeploymentStatusView[]>([]);
  const [deployFilter, setDeployFilter] = useState<DeployFilter>("all");
  const [deployedTo, setDeployedTo] = useState("");
  const [preset, setPreset] = useState("");
  const [presets, setPresets] = useState<PresetView[]>([]);
  const [importOpen, setImportOpen] = useState(false);
  const [upstreamOpen, setUpstreamOpen] = useState(false);
  const [copyTarget, setCopyTarget] = useState<SkillSummary | null>(null);
  const [singleSkill, setSingleSkill] = useState<SkillCheckView | null>(null);
  const [syncing, setSyncing] = useState(false);
  /** 生成维护任务的目标技能（单项或多选）。 */
  const [taskTargets, setTaskTargets] = useState<string[] | null>(null);
  const upstream = useUpstream(scheduledReport);

  // 搜索输入去抖：core 每次查询都会重扫库文件。
  useEffect(() => {
    const timer = setTimeout(() => setDebouncedQuery(query), 250);
    return () => clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      try {
        const [base, presetList] = await Promise.all([listSkills({ domain }), listPresets(domain)]);
        if (cancelled) {
          return;
        }
        const hasFilter =
          debouncedQuery.trim().length > 0 ||
          category !== null ||
          tagMode !== "all" ||
          !!deployedTo ||
          !!preset;
        const result = hasFilter
          ? await listSkills({
              domain,
              query: debouncedQuery.trim() || undefined,
              category: category !== null && category !== UNCATEGORIZED ? category : undefined,
              uncategorized: category === UNCATEGORIZED || undefined,
              tags: tagMode === "and" ? selectedTags : undefined,
              noTag: tagMode === "none" || undefined,
              deployedTo: deployedTo || undefined,
              preset: preset || undefined,
            })
          : base;
        if (cancelled) {
          return;
        }
        setAll(base);
        setPresets(presetList);
        setFiltered(result);
        setError(null);
        onCount(base.total);
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [domain, debouncedQuery, category, tagMode, selectedTags, deployedTo, preset, reloadToken, version, onCount]);

  // 筛选或搜索变化时清空批量选择（与原型一致，避免作用于不可见条目）。
  useEffect(() => {
    setSelected([]);
  }, [domain, debouncedQuery, category, tagMode, selectedTags, deployedTo, preset, deployFilter]);

  // 从其他页面（Hermes 详情“前往技能库编辑”）跳转进来：直接打开目标技能详情。
  useEffect(() => {
    if (focusReference) {
      setDetail(focusReference);
      onFocusConsumed?.();
    }
  }, [focusReference, onFocusConsumed]);

  // 本机部署记录：部署状态筛选与详情亮暗都用它（core 真实记录，不用模拟数据）。
  const refreshRecords = useCallback(async () => {
    try {
      setRecords(await deployStatus());
    } catch (err) {
      setError(String(err));
    }
  }, []);

  useEffect(() => {
    void refreshRecords();
  }, [refreshRecords, reloadToken, version, domain]);

  useEffect(() => {
    const refs = detail ? [detail] : selected;
    onSelectionChange?.(refs);
  }, [detail, selected, onSelectionChange]);


  /**
   * GUI 保存影响库内容的修改后走统一流程：先刷新本机已有部署，再按已确认策略上传
   * （由 onLibrarySaved 提供）；刷新部署记录由本页负责。未接线时退回仅刷新部署。
   */
  const syncDeployments = useCallback(() => {
    const flow = onLibrarySaved
      ? onLibrarySaved()
      : deploySync().then(() => undefined);
    void Promise.resolve(flow)
      .catch((err) => setError(String(err)))
      .finally(() => void refreshRecords());
  }, [onLibrarySaved, refreshRecords]);

  /** 手动同步：拉取同一 SkillManagerVault；无冲突静默应用并刷新已有部署。 */
  async function manualSync() {
    setSyncing(true);
    setError(null);
    try {
      const report = await syncPull();
      setNotice(report.message);
      setVersion((value) => value + 1);
    } catch (err) {
      setError(String(err));
    } finally {
      setSyncing(false);
    }
  }

  const deployedTargets = useMemo(() => {
    const map = new Map<string, Set<string>>();
    for (const record of records) {
      const set = map.get(record.skill) ?? new Set<string>();
      set.add(record.target);
      map.set(record.skill, set);
    }
    return map;
  }, [records]);

  const skills = useMemo(() => {
    const base = filtered?.skills ?? [];
    const visible =
      deployFilter === "all"
        ? base
        : base.filter((skill) => {
            const deployed = (deployedTargets.get(referenceOf(skill))?.size ?? 0) > 0;
            return deployFilter === "deployed" ? deployed : !deployed;
          });
    return [...visible].sort((left, right) => {
      const leftUnsupported = unsupportedOs(left.oses, os) ? 1 : 0;
      const rightUnsupported = unsupportedOs(right.oses, os) ? 1 : 0;
      return leftUnsupported - rightUnsupported;
    });
  }, [filtered, deployFilter, deployedTargets, os]);
  const deployedOptions = useMemo(() => [...new Set(records
    .filter((record) => record.skill.startsWith(`${domain}/`))
    .map((record) => record.target))], [records, domain]);
  const tagCandidates = useMemo(() => filtered?.tag_candidates ?? [], [filtered]);

  // 上游状态：仅展示 core 检查报告，卡片与工具栏据此表达“有更新/新增”。
  const updateCount = useMemo(
    () =>
      (upstream.report?.skills ?? [])
        .filter((item) => item.skill.startsWith(`${domain}/`))
        .filter(skillHasChange).length,
    [upstream.report, domain],
  );
  const candidateCount = useMemo(
    () =>
      (upstream.report?.candidates ?? []).filter((item) => item.domain === domain).length,
    [upstream.report, domain],
  );
  function checkOf(reference: string): SkillCheckView | undefined {
    return upstream.report?.skills.find((item) => item.skill === reference);
  }

  // 分类栏按管理域全量统计；标签候选按当前分类联动（core 给出）。
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

  const categoryNames = [...railCategories.namedCounts.keys()].sort((left, right) =>
    left < right ? -1 : 1,
  );

  // 切换分类后失效的标签筛选自动清除。
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

  // 卡片菜单：点外部或 Escape 关闭。
  useEffect(() => {
    if (!menu) {
      return;
    }
    const close = () => setMenu(null);
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        close();
      }
    };
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [menu]);

  function toggleSelect(reference: string) {
    setSelected((current) =>
      current.includes(reference)
        ? current.filter((item) => item !== reference)
        : [...current, reference],
    );
  }

  function openDetail(reference: string) {
    if (multi) {
      toggleSelect(reference);
      return;
    }
    setDetail(reference);
  }

  function menuAt(event: React.MouseEvent<HTMLButtonElement>, reference: string) {
    event.stopPropagation();
    if (menu?.reference === reference) {
      setMenu(null);
      return;
    }
    const rect = event.currentTarget.getBoundingClientRect();
    const below = rect.bottom + 5;
    const top =
      below + MENU_HEIGHT > window.innerHeight
        ? Math.max(8, rect.top - MENU_HEIGHT - 5)
        : below;
    setMenu({
      reference,
      left: Math.max(8, Math.min(window.innerWidth - 186, rect.right - 178)),
      top,
    });
  }

  const filterActive =
    debouncedQuery.trim().length > 0 ||
    category !== null ||
    tagMode !== "all" ||
    deployFilter !== "all" ||
    !!deployedTo ||
    !!preset;

  function clearFilters() {
    setQuery("");
    setDebouncedQuery("");
    setCategory(null);
    setTagMode("all");
    setSelectedTags([]);
    setDeployFilter("all");
    setDeployedTo("");
    setPreset("");
  }

  function onToggleTag(name: string) {
    const next = toggleTagSelection(tagMode, selectedTags, name);
    setTagMode(next.mode);
    setSelectedTags(next.selected);
  }

  const selectedSkills = skills.filter((skill) => selected.includes(referenceOf(skill)));
  const selectedChanged = selectedSkills.filter((skill) => {
    const check = checkOf(referenceOf(skill));
    return check !== undefined && skillHasChange(check);
  }).length;
  const removableTags = [
    ...new Set(selectedSkills.flatMap((skill) => skill.tags)),
  ].sort();
  const allVisibleSelected = skills.length > 0 && selectedSkills.length === skills.length;

  async function runTagBatch(mode: "add" | "remove", tags: string[]) {
    const targets = tagDialog?.targets ?? [];
    if (targets.length === 0 || tags.length === 0) {
      setTagDialog(null);
      return;
    }
    try {
      const items =
        mode === "add"
          ? await addSkillTags(targets, tags)
          : await detachSkillTags(targets, tags);
      setNotice(summarizeManageItems(items, mode === "add" ? "添加标签" : "移除标签"));
      setError(null);
      setVersion((value) => value + 1);
      void syncDeployments();
    } catch (err) {
      setError(String(err));
    } finally {
      setTagDialog(null);
    }
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
      void syncDeployments();
    } catch (err) {
      setError(String(err));
    }
  }

  async function move(reference: string, target: string | undefined) {
    try {
      await moveSkill(reference, target);
      setNotice(
        target === undefined
          ? `已移动到未分类（管理域根下）：${reference}`
          : `已移动到分类「${target}」：${reference}`,
      );
      setError(null);
      setVersion((value) => value + 1);
      void syncDeployments();
    } catch (err) {
      setError(String(err));
    } finally {
      setMoveTarget(null);
    }
  }

  async function confirmArchive() {
    const targets = archiveTarget ?? [];
    setArchiveTarget(null);
    if (targets.length === 0) {
      return;
    }
    try {
      const result = await archiveSkills(targets);
      const parts = [
        `已归档 ${result.archived.length} 项`,
        `卸载副本 ${result.cleanup.length} 项`,
      ];
      if (result.errors.length > 0) {
        parts.push(`未完成 ${result.errors.length} 项：${result.errors.map((item) => item.message).join("；")}`);
      }
      setNotice(parts.join("，"));
      setError(result.errors.length > 0 ? result.errors.map((item) => item.message).join("；") : null);
      setSelected([]);
      setVersion((value) => value + 1);
      void syncDeployments();
    } catch (err) {
      setError(String(err));
    }
  }

  async function confirmDelete() {
    const targets = deleteTarget ?? [];
    setDeleteTarget(null);
    if (targets.length === 0) {
      return;
    }
    try {
      const result = await deleteSkills(targets);
      const parts = [
        `已删除库内技能 ${result.deleted.length} 项`,
        `清理副本 ${result.cleanup.length} 项`,
      ];
      if (result.errors.length > 0) {
        parts.push(
          `${result.errors.length} 项未完成：${result.errors
            .map((item) => `${item.skill ?? "-"}（${item.message}）`)
            .join("；")}`,
        );
      }
      setNotice(`${parts.join("；")}。`);
      setError(null);
      setSelected([]);
      setVersion((value) => value + 1);
      void syncDeployments();
    } catch (err) {
      setError(String(err));
    }
  }

  async function copyTo(skill: SkillSummary, to: Domain, category?: string) {
    try {
      const created = await copySkill(referenceOf(skill), to, {
        category,
        uncategorized: category === undefined,
      });
      setNotice(
        `已复制到${to === "code" ? "编码" : "龙虾"}库：${to}/${created.name}（不部署，两份独立维护）。`,
      );
      setError(null);
      setVersion((value) => value + 1);
      void syncDeployments();
    } catch (err) {
      setError(String(err));
    } finally {
      setCopyTarget(null);
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col">
      {upstream.busy && (
        <div className="mb-3.5 shrink-0">
          <FetchProgressLine progress={upstream.progress} />
        </div>
      )}

      <div className="mb-3.5 flex shrink-0 flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-2.5">
          <label className="flex h-10 w-[280px] items-center gap-2.5 rounded-lg border border-line bg-panel px-3 text-faint">
            <Search className="size-4 shrink-0" strokeWidth={1.75} />
            <input
              value={query}
              placeholder="搜索名称、中文注释、正文或标签…"
              onChange={(event) => setQuery(event.target.value)}
              className="w-full border-0 bg-transparent text-[13px] text-ink-3 outline-0 placeholder:text-faint"
            />
          </label>
          <Segmented
            options={[
              { value: "all", label: "全部" },
              { value: "deployed", label: "本机已部署" },
              { value: "undeployed", label: "本机未部署" },
            ]}
            value={deployFilter}
            onChange={(value) => setDeployFilter(value as DeployFilter)}
          />
          <FilterMenu
            ariaLabel="指定部署目标"
            value={deployedTo}
            onChange={setDeployedTo}
            options={[
              { value: "", label: "全部部署目标" },
              ...deployedOptions.map((target) => ({
                value: target,
                label: targetLabel(target, agents),
              })),
            ]}
          />
          <FilterMenu
            ariaLabel="所属预设"
            value={preset}
            onChange={setPreset}
            options={[
              { value: "", label: "全部预设" },
              ...presets.map((item) => ({ value: item.id, label: item.name })),
            ]}
          />
        </div>
        <div className="flex flex-wrap items-center justify-end gap-2.5">
          <Button compact disabled={upstream.busy} onClick={() => void upstream.checkAll()}>
            <RefreshCw className="size-3.5" strokeWidth={1.75} />
            检查全部
          </Button>
          <Button
            compact
            disabled={syncing}
            title="拉取同一 SkillManagerVault；无冲突静默应用并刷新已有部署"
            onClick={() => void manualSync()}
          >
            <GitBranch className="size-3.5" strokeWidth={1.75} />
            {syncing ? "正在同步…" : "同步"}
          </Button>
          <Button compact disabled={upstream.busy} onClick={() => setUpstreamOpen(true)}>
            更新{updateCount > 0 ? ` (${updateCount})` : ""}
          </Button>
          <Button compact disabled={upstream.busy} onClick={() => setUpstreamOpen(true)}>
            新增{candidateCount > 0 ? ` (${candidateCount})` : ""}
          </Button>
          <Button compact kind="primary" onClick={() => setImportOpen(true)}>
            <Upload className="size-3.5" strokeWidth={1.75} />
            入库
          </Button>
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
        <button
          type="button"
          title="管理标签"
          aria-label="管理标签"
          onClick={() => setManageTags(true)}
          className="grid size-5 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-accent"
        >
          <MoreHorizontal className="size-4" strokeWidth={1.75} />
        </button>
      </div>

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

      {multi && (
        <div className="mb-3.5 flex shrink-0 flex-wrap items-center gap-2.5 rounded-lg border border-line bg-panel p-2.5">
          <Button
            compact
            disabled={skills.length === 0 || allVisibleSelected}
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
            onClick={() => setTagDialog({ mode: "add", targets: selected })}
          >
            批量添加标签
          </Button>
          <Button
            compact
            disabled={selected.length === 0 || removableTags.length === 0}
            title={
              removableTags.length === 0 ? "所选技能没有可移除的标签" : undefined
            }
            onClick={() => setTagDialog({ mode: "remove", targets: selected })}
          >
            批量移除标签
          </Button>
          <Button
            compact
            disabled={selected.length === 0}
            onClick={() => setDeployTarget(selected)}
          >
            <Rocket className="size-3.5" strokeWidth={1.75} />
            部署…
          </Button>
          <Button
            compact
            disabled={selected.length === 0}
            onClick={() => setArchiveTarget(selected)}
          >
            <Archive className="size-3.5" strokeWidth={1.75} />
            归档
          </Button>
          <Button
            compact
            disabled={selected.length === 0}
            onClick={() => setDeleteTarget(selected)}
          >
            <Trash2 className="size-3.5" strokeWidth={1.75} />
            删除技能
          </Button>
          <Button
            compact
            disabled={selected.length === 0}
            title="生成可复制的维护任务说明，交给外部 Agent"
            onClick={() => setTaskTargets(selected)}
          >
            <FileText className="size-3.5" strokeWidth={1.75} />
            生成维护任务
          </Button>
          {selectedChanged > 0 && (
            <Button compact onClick={() => setUpstreamOpen(true)}>
              <RefreshCw className="size-3.5" strokeWidth={1.75} />
              查看上游变化（{selectedChanged}）
            </Button>
          )}
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
                ? "grid grid-cols-2 gap-3 min-[1301px]:grid-cols-3"
                : "grid grid-cols-1 gap-3"
            }
          >
          {skills.map((skill) => {
            const reference = referenceOf(skill);
            const unsupported = unsupportedOs(skill.oses, os);
            const isSelected = selected.includes(reference);
            const check = checkOf(reference);
            const hasChange = check !== undefined && skillHasChange(check);
            return (
              <article
                key={reference}
                className={`group min-w-0 overflow-hidden rounded-xl border border-line bg-panel shadow-card transition-shadow hover:shadow-card-hover ${
                  unsupported ? "opacity-[0.43]" : ""
                } ${
                  multi && isSelected ? "border-accent shadow-[0_0_0_1px_rgba(5,150,105,0.33)]" : ""
                } ${layout === "list" ? "grid grid-cols-[340px_1fr_260px] items-center" : "flex flex-col"}`}
              >
                <div
                  className={`flex items-center gap-2.5 ${
                    layout === "list" ? "px-3 py-3" : "h-9 px-3.5 pt-2.5"
                  }`}
                >
                  {multi ? (
                    <input
                      type="checkbox"
                      className="selectbox shrink-0"
                      checked={isSelected}
                      onChange={() => toggleSelect(reference)}
                    />
                  ) : (
                    <span
                      title="已在 SkillManagerVault 管理"
                      className="size-2 shrink-0 rounded-full bg-accent-light shadow-[0_0_0_3px_rgba(5,150,105,0.05)]"
                    />
                  )}
                  <h3 className="flex min-w-0 flex-1 items-baseline gap-1.5">
                    <button
                      type="button"
                      title={skill.name}
                      onClick={() => openDetail(reference)}
                      className="truncate text-[14px] leading-5 font-semibold group-hover:text-accent"
                    >
                      {skill.name}
                    </button>
                    {skill.comment && (
                      <span className="shrink-0 text-[11px] font-normal text-faint">
                        （{skill.comment}）
                      </span>
                    )}
                  </h3>
                  <button
                    type="button"
                    title={`更多操作 · ${skill.name}`}
                    aria-label={`更多操作 · ${skill.name}`}
                    onClick={(event) => menuAt(event, reference)}
                    className="grid size-6 shrink-0 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-accent"
                  >
                    <MoreHorizontal className="size-4" strokeWidth={1.75} />
                  </button>
                </div>

                <div
                  role="presentation"
                  onClick={() => openDetail(reference)}
                  className={`min-w-0 ${layout === "list" ? "cursor-pointer px-2.5 py-2.5" : "cursor-pointer px-3.5 pt-1 pb-3"}`}
                >
                  <p className="truncate text-[13px] leading-[19px] text-faint">
                    {skill.description ?? ""}
                  </p>
                  <div className="mt-2.5 flex flex-wrap items-center gap-1.5">
                    {hasChange && check && (
                      <button
                        type="button"
                        title="有上游变化；先预览变化再确认，只更新当前条目与已有部署"
                        onClick={(event) => {
                          event.stopPropagation();
                          setSingleSkill(check);
                        }}
                        className="badge-tone-amber inline-flex items-center gap-1 rounded-full px-2 text-[12px] leading-5"
                      >
                        <RefreshCw className="size-3" strokeWidth={2} />
                        更新
                      </button>
                    )}
                    {skill.tags.map((item) => (
                      <TagChip key={item} tag={item} />
                    ))}
                    {unsupported && (
                      <Badge tone="gray" title={osReason(skill, os)}>
                        不支持当前系统
                      </Badge>
                    )}
                    {skill.translation_stale && (
                      <Badge tone="amber" title="原文已变化，译文尚未维护">
                        译文待更新
                      </Badge>
                    )}
                    {skill.manager_error && (
                      <Badge tone="amber" title={skill.manager_error}>
                        .manager 读取异常
                      </Badge>
                    )}
                  </div>
                </div>

                <div
                  className={`flex min-h-[43px] flex-wrap items-center gap-2.5 px-3.5 py-2.5 ${
                    layout === "list" ? "" : "mt-auto border-t border-line-faint"
                  }`}
                >
                  <span
                    title={skill.source ? `来源：${skill.source.repo}` : "自建技能：内容在本机 SkillManagerVault 内维护"}
                    className="flex max-w-[170px] items-center gap-1 truncate rounded-full border border-line-soft bg-subtle px-2 text-[11px] leading-[18px] text-muted"
                  >
                    {skill.source ? (
                      <GitBranch className="size-3 shrink-0" strokeWidth={1.75} />
                    ) : (
                      <FolderTree className="size-3 shrink-0" strokeWidth={1.75} />
                    )}
                    <span className="truncate">{sourceLabel(skill)}</span>
                  </span>
                  {skill.presets.map((preset) => (
                    <span
                      key={preset.id}
                      title={`属于预设：${preset.name}（${preset.id}）`}
                      className="max-w-[130px] truncate rounded-full border border-line-soft bg-subtle px-2 text-[11px] leading-[18px] text-muted"
                    >
                      {preset.name}
                    </span>
                  ))}
                  <QuickDeploy deploymentBlocks={skill.deployment_blocks} reference={reference} domain={domain} agents={agents} records={records}
                    unsupportedReason={unsupported ? osReason(skill, os) : null}
                    onChange={refreshRecords} onError={setError} />
                </div>
              </article>
            );
          })}

          {!loading && skills.length === 0 && (
            <div className="col-span-full px-4 py-8 text-center text-[13px] text-muted">
              <Layers className="mx-auto mb-2 size-7 text-faint" strokeWidth={1.5} />
              <p>没有符合条件的技能。</p>
              {filterActive && (
                <div className="mt-2 flex justify-center">
                  <Button compact onClick={clearFilters}>
                    清除筛选
                  </Button>
                </div>
              )}
            </div>
          )}
          </div>
        </div>
      </div>

      {menu && (
        <div
          className="card-menu"
          role="menu"
          aria-label="技能操作"
          style={{ left: menu.left, top: menu.top }}
          onMouseDown={(event) => event.stopPropagation()}
        >
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              const skill = (all?.skills ?? []).find(
                (item) => referenceOf(item) === menu.reference,
              );
              setMenu(null);
              if (skill) {
                setCopyTarget(skill);
              }
            }}
          >
            <Copy className="size-3.5 text-muted" strokeWidth={1.75} />
            复制到另一库…
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              const skill = (all?.skills ?? []).find(
                (item) => referenceOf(item) === menu.reference,
              );
              setMenu(null);
              if (skill) {
                setMoveTarget(skill);
              }
            }}
          >
            <FolderTree className="size-3.5 text-muted" strokeWidth={1.75} />
            移动到分类…
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              setTagDialog({ mode: "add", targets: [menu.reference] });
              setMenu(null);
            }}
          >
            <Plus className="size-3.5 text-muted" strokeWidth={1.75} />
            添加标签…
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              setTaskTargets([menu.reference]);
              setMenu(null);
            }}
          >
            <FileText className="size-3.5 text-muted" strokeWidth={1.75} />
            生成维护任务…
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              setDeployTarget([menu.reference]);
              setMenu(null);
            }}
          >
            <Rocket className="size-3.5 text-muted" strokeWidth={1.75} />
            部署…
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              setArchiveTarget([menu.reference]);
              setMenu(null);
            }}
          >
            <Archive className="size-3.5 text-muted" strokeWidth={1.75} />
            归档…
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              setDeleteTarget([menu.reference]);
              setMenu(null);
            }}
          >
            <Trash2 className="size-3.5 text-danger" strokeWidth={1.75} />
            删除技能…
          </button>
        </div>
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
          onNotice={(message) => {
            setNotice(message);
            setError(null);
          }}
          onOpenMove={(skill) => setMoveTarget(skill)}
          onDeploy={(reference) => setDeployTarget([reference])}
          onDelete={(reference) => setDeleteTarget([reference])}
          onArchive={(reference) => setArchiveTarget([reference])}
          onSyncDeployments={syncDeployments}
          onDeploymentsChanged={refreshRecords}
        />
      )}

      {tagDialog && tagDialog.mode === "add" && (
        <TagPickerDialog
          title={
            tagDialog.targets.length === 1
              ? `添加标签 · ${tagDialog.targets[0]}`
              : `批量添加标签 · ${tagDialog.targets.length} 个技能`
          }
          subtitle="从已有标签选择，也可以输入新标签；保留技能现有标签。"
          options={all?.tag_candidates ?? []}
          locked={
            tagDialog.targets.length === 1
              ? ((all?.skills ?? []).find(
                  (skill) => referenceOf(skill) === tagDialog.targets[0],
                )?.tags ?? [])
              : []
          }
          allowNew
          newLabel="新标签（多个用逗号分隔）"
          newPlaceholder="例如：日常运维，常用"
          confirmLabel="添加"
          onCancel={() => setTagDialog(null)}
          onConfirm={(result) => runTagBatch("add", result.tags)}
        />
      )}

      {tagDialog && tagDialog.mode === "remove" && (
        <TagPickerDialog
          title={`批量移除标签 · ${tagDialog.targets.length} 个技能`}
          subtitle="仅解除所选技能与这些标签的关联，其他标签保持原样。"
          options={removableTags}
          confirmLabel="移除"
          onCancel={() => setTagDialog(null)}
          onConfirm={(result) => runTagBatch("remove", result.tags)}
        />
      )}


      {moveTarget && (
        <MoveCategoryDialog
          skill={moveTarget.name}
          current={moveTarget.category}
          categories={categoryNames}
          onCancel={() => setMoveTarget(null)}
          onConfirm={(target) => {
            void move(referenceOf(moveTarget), target);
          }}
        />
      )}

      {manageTags && (
        <ManageTagsDialog
          tags={all?.tag_candidates ?? []}
          onClose={() => setManageTags(false)}
          onChanged={(message) => {
            setNotice(message);
            setError(null);
            setVersion((value) => value + 1);
            void syncDeployments();
          }}
        />
      )}

      {deployTarget && (
        <DeployDialog
          domain={domain}
          references={deployTarget}
          agents={agents}
          unsupportedReason={unsupportedDeployReason(
            deployTarget.map((reference) => ({
              name: reference,
              oses: all?.skills.find((skill) => referenceOf(skill) === reference)?.oses ?? null,
            })),
            os,
          )}
          onClose={() => { setDeployTarget(null); setVersion((value) => value + 1); }}
          onDone={(message) => {
            setNotice(message);
            setError(null);
            setVersion((value) => value + 1);
          }}
        />
      )}

      {archiveTarget && (
        <ConfirmDialog
          title="归档技能"
          subtitle="归档会先从所有已安装的 Agent 卸载，再移出技能库；有本地变更的副本会中止归档。"
          message={`将归档 ${archiveTarget.length} 项：${archiveTarget.join("、")}。还原后保留分类与标签，不自动重新部署。`}
          confirmLabel="归档"
          onCancel={() => setArchiveTarget(null)}
          onConfirm={() => void confirmArchive()}
        />
      )}

      {deleteTarget && (
        <ConfirmDialog
          title="删除库内技能"
          subtitle="删除会一并清理受管部署；有本地变更的副本先按暂停规则处理。"
          message={`将删除 ${deleteTarget.length} 项并清理其受管部署：${deleteTarget.join("、")}。`}
          confirmLabel="删除"
          danger
          onCancel={() => setDeleteTarget(null)}
          onConfirm={() => void confirmDelete()}
        />
      )}

      {taskTargets && (
        <TaskDialog
          title={
            taskTargets.length === 1
              ? `生成维护任务 · ${taskTargets[0]}`
              : `生成维护任务 · ${taskTargets.length} 个技能`
          }
          requirementLabel="希望 Agent 做什么"
          generate={(requirement) => skillTask(taskTargets, requirement || undefined)}
          onClose={() => setTaskTargets(null)}
          onNotice={(message) => {
            setNotice(message);
            setError(null);
          }}
        />
      )}

      {copyTarget && (
        <CopyToDomainDialog
          reference={referenceOf(copyTarget)}
          from={domain}
          onCancel={() => setCopyTarget(null)}
          onConfirm={(to, category) => void copyTo(copyTarget, to, category)}
        />
      )}

      {importOpen && (
        <ImportDialog
          domain={domain}
          onClose={() => setImportOpen(false)}
          onDone={(message) => {
            setNotice(message);
            setError(null);
            setVersion((value) => value + 1);
            void syncDeployments();
          }}
        />
      )}

      {upstreamOpen && (
        <UpstreamDialog
          domain={domain}
          upstream={upstream}
          onClose={() => setUpstreamOpen(false)}
          onChanged={(message) => {
            setNotice(message);
            setError(null);
            setVersion((value) => value + 1);
            void syncDeployments();
          }}
        />
      )}

      {singleSkill && (
        <SingleSkillUpdateDialog
          skill={singleSkill}
          upstream={upstream}
          onClose={() => setSingleSkill(null)}
          onChanged={(message) => {
            setSingleSkill(null);
            setNotice(message);
            setError(null);
            setVersion((value) => value + 1);
            void syncDeployments();
          }}
        />
      )}
    </div>
  );
}
