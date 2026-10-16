/**
 * Hermes 档案页：档案选择、档案技能列表（复用技能库的分类栏、标签栏、卡片网格与分类联动规则）、
 * 明确启停与批量启停、只读技能详情（前往技能库编辑）与档案独有技能收编。
 *
 * 筛选、分类联动的标签候选与启用准入都由 core 决定；前端只提交条件与展示结果。
 * 不提供档案生命周期（创建/删除/改名）、活动切换、多实例或 `SOUL.md` 管理。
 */
import { useEffect, useMemo, useRef, useState } from "react";
import {
  Check,
  CheckSquare,
  ChevronDown,
  FolderTree,
  Layers,
  LayoutGrid,
  List,
  MoreHorizontal,
  RefreshCw,
  Square,
} from "lucide-react";

import {
  hermesAdopt,
  hermesDisable,
  hermesEnable,
  hermesList,
  hermesProfiles,
  type EnvironmentOs,
  type HermesProfileView,
  type HermesSkillView,
  type HermesSkillsView,
} from "../services/client";
import { osLabel, unsupportedOs } from "../lib/support";
import { TagChip } from "../components/TagChip";
import { HermesSkillDetail } from "../components/HermesSkillDetail";
import { Badge, Button, ConfirmDialog, Notice, Segmented } from "../components/ui";

const UNCATEGORIZED = "__none";
const NO_TAG = "__none";

type Layout = "grid" | "list";
type EnabledFilter = "all" | "enabled" | "disabled";

export interface HermesPageProps {
  os: EnvironmentOs | null;
  reloadToken: number;
  /** 前往技能库编辑统一内容（切换侧栏页面并打开详情）。 */
  onOpenInLibrary: (reference: string) => void;
  /** 保存库内修改后的统一流程（先刷新已有部署，再按已确认策略上传）。 */
  onLibrarySaved?: () => Promise<void>;
}

function osReason(skill: HermesSkillView, os: EnvironmentOs | null): string {
  const declared =
    skill.oses === null
      ? "未声明系统支持范围（视为全部支持）"
      : skill.oses.length === 0
        ? "显式声明不支持任何系统"
        : `声明支持 ${skill.oses.map(osLabel).join("、")}`;
  return `${declared}；当前系统 ${os === null ? "未知" : osLabel(os)}。`;
}

/** 档案内一项技能的启停开关。 */
function Toggle({
  enabled,
  disabled,
  onToggle,
}: {
  enabled: boolean;
  disabled: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={enabled}
      aria-label={enabled ? "禁用" : "启用"}
      title={enabled ? "已启用，点击禁用" : "未启用，点击启用"}
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

export function HermesPage({ os, reloadToken, onOpenInLibrary, onLibrarySaved }: HermesPageProps) {
  const [profiles, setProfiles] = useState<HermesProfileView[]>([]);
  const [active, setActive] = useState<string | null>(null);
  const [root, setRoot] = useState<string>("");
  const [pickerOpen, setPickerOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [category, setCategory] = useState<string | null>(null);
  const [tag, setTag] = useState<string | null>(null);
  const [enabledFilter, setEnabledFilter] = useState<EnabledFilter>("all");
  const [layout, setLayout] = useState<Layout>("grid");
  const [multi, setMulti] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const [base, setBase] = useState<HermesSkillsView | null>(null);
  const [filtered, setFiltered] = useState<HermesSkillsView | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [version, setVersion] = useState(0);
  const [detail, setDetail] = useState<string | null>(null);
  const [adoptTarget, setAdoptTarget] = useState<HermesSkillView | null>(null);
  const pickerRef = useRef<HTMLDivElement | null>(null);

  // 档案发现：只读本机事实。
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const view = await hermesProfiles();
        if (cancelled) {
          return;
        }
        setProfiles(view.profiles);
        setRoot(view.root);
        setActive((current) => current ?? view.profiles[0]?.name ?? null);
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
  }, [reloadToken, version]);

  useEffect(() => {
    const timer = setTimeout(() => setDebouncedQuery(query), 250);
    return () => clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    if (!active) {
      return;
    }
    let cancelled = false;
    (async () => {
      setLoading(true);
      try {
        const full = await hermesList(active);
        const hasFilter =
          debouncedQuery.trim().length > 0 || category !== null || tag !== null;
        const narrowed = hasFilter
          ? await hermesList(active, {
              query: debouncedQuery.trim() || undefined,
              category: category !== null && category !== UNCATEGORIZED ? category : undefined,
              uncategorized: category === UNCATEGORIZED || undefined,
              tags: tag !== null && tag !== NO_TAG ? [tag] : undefined,
              noTag: tag === NO_TAG || undefined,
            })
          : full;
        if (cancelled) {
          return;
        }
        setBase(full);
        setFiltered(narrowed);
        setError(null);
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
  }, [active, debouncedQuery, category, tag, reloadToken, version]);

  useEffect(() => {
    setSelected([]);
  }, [active, debouncedQuery, category, tag]);

  // 档案下拉：点外部或 Escape 关闭。
  useEffect(() => {
    if (!pickerOpen) {
      return;
    }
    const close = (event: MouseEvent) => {
      if (pickerRef.current && !pickerRef.current.contains(event.target as Node)) {
        setPickerOpen(false);
      }
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setPickerOpen(false);
      }
    };
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", onKey);
    };
  }, [pickerOpen]);

  const skills = useMemo(() => {
    const all = filtered?.skills ?? [];
    if (enabledFilter === "all") {
      return all;
    }
    return all.filter((skill) =>
      enabledFilter === "enabled" ? skill.enabled : !skill.enabled,
    );
  }, [filtered, enabledFilter]);

  const tagCandidates = useMemo(() => filtered?.tag_candidates ?? [], [filtered]);

  useEffect(() => {
    if (tag !== null && tag !== NO_TAG && !tagCandidates.includes(tag)) {
      setTag(null);
    }
  }, [tag, tagCandidates]);

  const railCategories = useMemo(() => {
    const counts = new Map<string, number>();
    let uncategorized = 0;
    for (const skill of base?.skills ?? []) {
      if (skill.category === undefined || skill.category === null) {
        uncategorized += 1;
      } else {
        counts.set(skill.category, (counts.get(skill.category) ?? 0) + 1);
      }
    }
    return {
      named: [...counts.entries()].sort((left, right) => (left[0] < right[0] ? -1 : 1)),
      uncategorized,
    };
  }, [base]);

  const activeProfile = profiles.find((profile) => profile.name === active) ?? null;
  const isDefault = activeProfile?.kind === "default";

  const filterActive =
    debouncedQuery.trim().length > 0 || category !== null || tag !== null || enabledFilter !== "all";

  function clearFilters() {
    setQuery("");
    setDebouncedQuery("");
    setCategory(null);
    setTag(null);
    setEnabledFilter("all");
  }

  function toggleSelect(name: string) {
    setSelected((current) =>
      current.includes(name) ? current.filter((item) => item !== name) : [...current, name],
    );
  }

  function openDetail(skill: HermesSkillView) {
    if (multi) {
      toggleSelect(skill.name);
      return;
    }
    if (skill.library_skill) {
      setDetail(skill.library_skill);
    } else {
      setAdoptTarget(skill);
    }
  }

  const selectedSkills = skills.filter((skill) => selected.includes(skill.name));
  const allVisibleSelected = skills.length > 0 && selectedSkills.length === skills.length;
  const adoptableSelected = selectedSkills.filter((skill) => skill.needs_adoption);
  const toggleableSelected = selectedSkills.filter((skill) => !skill.needs_adoption);

  async function runToggle(names: string[], enabled: boolean) {
    if (!active || names.length === 0) {
      return;
    }
    try {
      const result = enabled
        ? await hermesEnable(active, names)
        : await hermesDisable(active, names);
      const failed = result.items.filter((item) => !item.ok);
      const parts = [
        `已${enabled ? "启用" : "禁用"} ${result.changed} 项`,
      ];
      if (failed.length > 0) {
        parts.push(
          `${failed.length} 项未完成：${failed
            .map((item) => `${item.name}（${item.message ?? "未知原因"}）`)
            .join("；")}`,
        );
      }
      setNotice(parts.join("；"));
      setError(null);
      setSelected([]);
      setVersion((value) => value + 1);
    } catch (err) {
      setError(String(err));
    }
  }

  async function confirmAdopt(names: string[]) {
    if (!active || names.length === 0) {
      return;
    }
    setAdoptTarget(null);
    try {
      const result = await hermesAdopt(active, names);
      const failed = result.items.filter((item) => !item.ok);
      const parts = [`已收编 ${result.adopted} 项到龙虾库并部署 default`];
      if (failed.length > 0) {
        parts.push(
          `${failed.length} 项未完成：${failed
            .map((item) => `${item.name}（${item.message ?? "未知原因"}）`)
            .join("；")}`,
        );
      }
      setNotice(parts.join("；"));
      setError(null);
      setSelected([]);
      setVersion((value) => value + 1);
      void onLibrarySaved?.();
    } catch (err) {
      setError(String(err));
    }
  }

  if (profiles.length === 0 && !loading) {
    return (
      <div className="rounded-xl border border-line-soft bg-panel p-6 text-[13px] text-muted">
        <FolderTree className="mb-2 size-5 text-faint" strokeWidth={1.75} />
        本机没有发现 Hermes 档案（根：{root || "~/.hermes"}）。档案发现只读本机事实；
        不提供档案创建。
      </div>
    );
  }

  return (
    <div>
      <div className="mb-3.5 flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-2.5">
          {/* 档案选择 */}
          <div className="relative" ref={pickerRef}>
            <button
              type="button"
              onClick={() => setPickerOpen((open) => !open)}
              title="选择档案"
              className="flex h-10 w-[228px] items-center gap-2.5 rounded-lg border border-line bg-panel px-3 text-[13px] text-ink-2"
            >
              <span className="grid size-[18px] shrink-0 place-items-center rounded-[5px] bg-accent-strong text-[10px] font-semibold text-white">
                {(activeProfile?.name ?? "?").slice(0, 1).toUpperCase()}
              </span>
              <span className="min-w-0 flex-1 truncate text-left">
                {activeProfile?.name ?? "选择档案"}
              </span>
              <ChevronDown className="size-4 shrink-0 text-faint" strokeWidth={1.75} />
            </button>
            {pickerOpen && (
              <div className="absolute top-[46px] left-0 z-40 w-[300px] rounded-xl border border-line-soft bg-panel p-1.5 shadow-card-hover">
                {profiles.map((profile) => (
                  <button
                    key={profile.name}
                    type="button"
                    onClick={() => {
                      setActive(profile.name);
                      setPickerOpen(false);
                    }}
                    className={`flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left ${
                      profile.name === active ? "bg-selected" : "hover:bg-hover"
                    }`}
                  >
                    <span className="grid size-[18px] shrink-0 place-items-center rounded-[5px] bg-accent-strong text-[10px] font-semibold text-white">
                      {profile.name.slice(0, 1).toUpperCase()}
                    </span>
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-[13px] text-ink-2">{profile.name}</span>
                      <span className="block truncate text-[11px] text-faint" title={profile.path}>
                        {profile.path}
                      </span>
                    </span>
                    <span className="shrink-0 text-[11px] text-faint">
                      {profile.enabled_count} 个已启用
                    </span>
                    {profile.name === active && (
                      <Check className="size-3.5 shrink-0 text-accent" strokeWidth={2} />
                    )}
                  </button>
                ))}
                <p className="px-2.5 py-1.5 text-[11px] text-faint">
                  本机已发现的档案，不管理档案生命周期。
                </p>
              </div>
            )}
          </div>

          <label className="flex h-10 w-[240px] items-center gap-2.5 rounded-lg border border-line bg-panel px-3 text-faint">
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
              { value: "enabled", label: "已启用" },
              { value: "disabled", label: "未启用" },
            ]}
            value={enabledFilter}
            onChange={(value) => setEnabledFilter(value as EnabledFilter)}
          />
        </div>
        <div className="flex items-center gap-2.5">
          <Button compact onClick={() => setVersion((value) => value + 1)}>
            <RefreshCw className="size-3.5" strokeWidth={1.75} />
            刷新
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

      <p className="mb-3.5 flex items-center gap-1.5 text-[12px] text-faint">
        <MoreHorizontal className="size-3.5" strokeWidth={1.75} />
        其他档案从 default 候选池显式启用，新增到 default 的技能不会自动启用到其他档案。
      </p>

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
      {filtered?.config_error && (
        <div className="mb-3.5">
          <Notice tone="warn">
            档案 `config.yaml` 读取失败，启用状态不作有效结论：{filtered.config_error}
          </Notice>
        </div>
      )}

      {multi && (
        <div className="mb-3.5 flex flex-wrap items-center gap-2.5 rounded-lg border border-line bg-panel p-2.5">
          <label className="flex items-center gap-1.5 text-[13px]">
            <input
              type="checkbox"
              className="selectbox"
              ref={(element) => {
                if (element) {
                  element.indeterminate = selected.length > 0 && !allVisibleSelected;
                }
              }}
              checked={allVisibleSelected}
              onChange={(event) =>
                setSelected(event.target.checked ? skills.map((skill) => skill.name) : [])
              }
            />
            全选当前结果
          </label>
          <span className="text-[13px] text-muted">已选择 {selected.length}</span>
          <Button
            compact
            disabled={isDefault || toggleableSelected.length === 0}
            title={isDefault ? "default 档案的技能由部署记录表达，请用技能库部署/撤销" : undefined}
            onClick={() => void runToggle(toggleableSelected.map((skill) => skill.name), true)}
          >
            批量启用
          </Button>
          <Button
            compact
            disabled={isDefault || toggleableSelected.length === 0}
            onClick={() => void runToggle(toggleableSelected.map((skill) => skill.name), false)}
          >
            批量禁用
          </Button>
          <Button
            compact
            disabled={adoptableSelected.length === 0}
            onClick={() => void confirmAdopt(adoptableSelected.map((skill) => skill.name))}
          >
            收编到龙虾库
          </Button>
        </div>
      )}

      <div className="flex items-start gap-5">
        <aside className="w-[152px] shrink-0">
          <button
            type="button"
            onClick={() => setCategory(null)}
            className={`mb-0.5 flex h-[35px] w-full items-center gap-1.5 rounded-md px-2.5 text-left text-[14px] ${
              category === null ? "bg-selected text-ink" : "text-ink-3 hover:bg-hover"
            }`}
          >
            全部分类
            <span className="grid h-[18px] min-w-[20px] place-items-center rounded-full bg-subtle px-1.5 text-[11px] text-muted">
              {base?.total ?? 0}
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
                category === UNCATEGORIZED ? "bg-selected text-ink" : "text-ink-3 hover:bg-hover"
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
              ? "grid min-w-0 flex-1 grid-cols-2 gap-3 min-[1301px]:grid-cols-3"
              : "grid min-w-0 flex-1 grid-cols-1 gap-3"
          }
        >
          {skills.map((skill) => {
            const unsupported = unsupportedOs(skill.oses, os);
            const isSelected = selected.includes(skill.name);
            const toggleDisabled = isDefault || skill.needs_adoption || !skill.in_default_pool;
            return (
              <article
                key={skill.name}
                className={`group min-w-0 overflow-hidden rounded-xl border border-line bg-panel shadow-card transition-shadow hover:shadow-card-hover ${
                  unsupported ? "opacity-[0.43]" : ""
                } ${
                  multi && isSelected ? "border-accent shadow-[0_0_0_1px_rgba(5,150,105,0.33)]" : ""
                } ${
                  layout === "list"
                    ? "grid grid-cols-[340px_1fr_260px] items-center"
                    : "flex flex-col"
                }`}
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
                      onChange={() => toggleSelect(skill.name)}
                    />
                  ) : (
                    <span
                      title={
                        skill.needs_adoption
                          ? "档案独有技能（未受管）"
                          : "库内统一内容（在技能库维护）"
                      }
                      className={`size-2 shrink-0 rounded-full ${
                        skill.needs_adoption
                          ? "bg-subtle shadow-[0_0_0_3px_rgba(0,0,0,0.03)]"
                          : "bg-accent-light shadow-[0_0_0_3px_rgba(5,150,105,0.05)]"
                      }`}
                    />
                  )}
                  <h3 className="flex min-w-0 flex-1 items-baseline gap-1.5">
                    <button
                      type="button"
                      title={skill.name}
                      onClick={() => openDetail(skill)}
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
                  <Toggle
                    enabled={skill.enabled}
                    disabled={toggleDisabled}
                    onToggle={() => void runToggle([skill.name], !skill.enabled)}
                  />
                </div>

                <div
                  role="presentation"
                  onClick={() => openDetail(skill)}
                  className={`min-w-0 ${
                    layout === "list"
                      ? "cursor-pointer px-2.5 py-2.5"
                      : "cursor-pointer px-3.5 pt-1 pb-3"
                  }`}
                >
                  <p className="truncate text-[13px] leading-[19px] text-faint">
                    {skill.description ?? ""}
                  </p>
                  <div className="mt-2.5 flex flex-wrap items-center gap-1.5">
                    {skill.tags.map((item) => (
                      <TagChip key={item} tag={item} />
                    ))}
                    {skill.needs_adoption && (
                      <Badge tone="blue" title="档案独有技能，可收编到龙虾库">
                        档案独有
                      </Badge>
                    )}
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
                    {skill.warning && (
                      <Badge tone="amber" title={skill.warning}>
                        支持范围已收窄
                      </Badge>
                    )}
                    {skill.blocked && (
                      <Badge tone="amber" title={skill.blocked}>
                        禁止新增启用
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
                  className={`flex min-h-[43px] items-center gap-2.5 px-3.5 py-2.5 ${
                    layout === "list" ? "" : "mt-auto border-t border-line-faint"
                  }`}
                >
                  <span className="flex max-w-[170px] items-center gap-1 truncate rounded-full border border-line-soft bg-subtle px-2 text-[11px] leading-[18px] text-muted">
                    <FolderTree className="size-3 shrink-0" strokeWidth={1.75} />
                    <span className="truncate">{skill.category ?? "未分类"}</span>
                  </span>
                  <span className="ml-auto text-[12px] text-muted">
                    {skill.enabled ? "已启用" : "未启用"}
                  </span>
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

      {detail && (
        <HermesSkillDetail
          reference={detail}
          reloadToken={reloadToken + version}
          onClose={() => setDetail(null)}
          onOpenInLibrary={(reference) => {
            setDetail(null);
            onOpenInLibrary(reference);
          }}
        />
      )}

      {adoptTarget && (
        <ConfirmDialog
          title="收编档案独有技能"
          subtitle="技能进入龙虾库并部署到 default 档案；原档案启用状态保持不变。"
          message={`将从档案「${active}」收编 ${adoptTarget.name} 到龙虾库，并部署到 default 档案。同域同名已存在时会被阻止。`}
          confirmLabel="收编"
          onCancel={() => setAdoptTarget(null)}
          onConfirm={() => void confirmAdopt([adoptTarget.name])}
        />
      )}
    </div>
  );
}
