/**
 * Agent 页面：本机目标的**跨目标部署清单**（「全部编码 Agents」/「全部龙虾 Agent」，FR-057），
 * 以及单个目标（内置 Agent 目录或共享目录）的本地技能列表与删除
 * （受管/仅本地/本地有变更）。
 *
 * Agent 检测、启停、排序与目录配置在设置页（FR-041），本页不再承担 Agent 管理。
 * 共享目录是本机部署目标，不是可登记的 Agent 或第二个管理库。
 */
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  CheckSquare,
  Copy,
  LayoutGrid,
  List,
  Plus,
  RefreshCw,
  Search,
  Square,
  Trash2,
  Upload,
} from "lucide-react";

import {
  agentDeleteLocal,
  agentLocalSkills,
  agentReadLocal,
  copySkill,
  deployStatus,
  listSkills,
  type AgentView,
  type DeploymentStatusView,
  type Domain,
  type LocalSkillView,
  type SkillSummary,
} from "../services/client";
import { AdoptDialog } from "../components/AdoptDialog";
import { LocalSkillDetail } from "../components/LocalSkillDetail";
import { CopyToDomainDialog } from "../components/SkillDialogs";
import { DeployDialog } from "../components/DeployDialog";
import { QuickDeploy } from "../components/QuickDeploy";
import { AgentPresetPills } from "../components/AgentPresetPills";
// 部署记录的目标展示名（`agent:<key>`/`shared`/`hermes:default`）复用库内既有映射。
import { targetLabel as deploymentTargetName } from "../lib/targets";
import { Badge, Button, ConfirmDialog, Dialog, Notice, Segmented } from "../components/ui";

/** 共享目录目标标识（与 core 的 `shared` 取值一致）；它是编码 Agent 的部署目标。 */
const SHARED_TARGET = "shared";

/** 单个目标的展示名：内置 Agent 用其名称，共享目录用固定标签。 */
function targetLabel(target: string, agent: AgentView | undefined): string {
  return agent?.display_name ?? (target === SHARED_TARGET ? "共享 Agent" : target);
}

/** 库内引用 `domain/name` 的技能名部分。 */
function skillName(reference: string): string {
  const slash = reference.indexOf("/");
  return slash === -1 ? reference : reference.slice(slash + 1);
}

export interface AgentPageProps {
  domain: Domain;
  /** 选中的单个目标（内置 Agent key 或共享目录 `shared`）；为 null 时显示该管理域的跨目标部署清单。 */
  selectedTarget: string | null;
  agents: AgentView[];
  /** 页头元信息：当前列表计数与路径副标题。 */
  onMeta: (meta: { count: number; subtitle: string }) => void;
  onNotice: (message: string) => void;
  onOpenInLibrary: (reference: string) => void;
  /** 保存库内修改后的统一流程（先刷新已有部署，再按已确认策略上传）。 */
  onLibrarySaved?: () => Promise<void>;
}

export function AgentPage({
  domain,
  selectedTarget,
  agents,
  onMeta,
  onNotice,
  onOpenInLibrary,
  onLibrarySaved,
}: AgentPageProps) {
  if (selectedTarget !== null) {
    const agent = agents.find((item) => item.key === selectedTarget);
    return (
      <LocalSkillsView
        target={selectedTarget}
        agent={agent}
        agents={agents}
        onMeta={onMeta}
        onNotice={onNotice}
        onOpenInLibrary={onOpenInLibrary}
        onLibrarySaved={onLibrarySaved}
      />
    );
  }

  return (
    <DeploymentList domain={domain} agents={agents} onMeta={onMeta} onNotice={onNotice} />
  );
}

/** 本机目标的技能列表：内置 Agent 目录与共享目录共用同一套筛选与删除。 */
function LocalSkillsView({
  target,
  agent,
  agents,
  onMeta,
  onNotice,
  onOpenInLibrary,
  onLibrarySaved,
}: {
  target: string;
  agent: AgentView | undefined;
  agents: AgentView[];
  onMeta: (meta: { count: number; subtitle: string }) => void;
  onNotice: (message: string) => void;
  onOpenInLibrary: (reference: string) => void;
  onLibrarySaved?: () => Promise<void>;
}) {
  const [skills, setSkills] = useState<LocalSkillView[]>([]);
  const [records, setRecords] = useState<DeploymentStatusView[]>([]);
  const [skillsDir, setSkillsDir] = useState("");
  const [filter, setFilter] = useState<"all" | "managed" | "local" | "changed">("all");
  const [query, setQuery] = useState("");
  const [layout, setLayout] = useState<"grid" | "list">("grid");
  const [multi, setMulti] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const [deleteTarget, setDeleteTarget] = useState<string[] | null>(null);
  const [adoptTargets, setAdoptTargets] = useState<LocalSkillView[] | null>(null);
  const [copySource, setCopySource] = useState<LocalSkillView | null>(null);
  const [detailSkill, setDetailSkill] = useState<LocalSkillView | null>(null);
  const [localBusy, setLocalBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [version, setVersion] = useState(0);
  const [picking, setPicking] = useState(false);
  const [deployTargets, setDeployTargets] = useState<string[] | null>(null);

  const label = targetLabel(target, agent);
  const agentDomain: Domain = agent?.domain ?? "code";

  const reload = useCallback(async () => {
    try {
      const [view, deployments] = await Promise.all([agentLocalSkills(target), deployStatus()]);
      setSkills(view.skills);
      setRecords(deployments);
      setSkillsDir(view.skills_dir);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, [target]);

  useEffect(() => {
    void reload();
  }, [reload, version]);

  useEffect(() => {
    setSelected([]);
  }, [target, filter, query, multi]);

  const visible = skills.filter((skill) => {
    const q = query.trim().toLowerCase();
    if (
      q !== "" &&
      ![skill.name, skill.comment ?? "", skill.library_skill ?? "", skill.path]
        .join(" ")
        .toLowerCase()
        .includes(q)
    ) {
      return false;
    }
    if (filter === "all") return true;
    if (filter === "managed") return skill.managed;
    if (filter === "local") return !skill.managed;
    return skill.status === "paused_local_change";
  });
  const managedCount = skills.filter((skill) => skill.managed).length;
  const unmanagedSelected = skills.filter((skill) => !skill.managed && selected.includes(skill.name));

  useEffect(() => {
    onMeta({
      count: visible.length,
      subtitle: `${skillsDir || "读取中…"} · ${managedCount} 个受管理`,
    });
  }, [managedCount, onMeta, skillsDir, visible.length]);

  function toggleSelect(name: string) {
    setSelected((current) =>
      current.includes(name) ? current.filter((item) => item !== name) : [...current, name],
    );
  }

  async function copyToLibrary(skill: LocalSkillView, to: Domain, category?: string) {
    if (!skill.library_skill) {
      return;
    }
    setLocalBusy(true);
    setError(null);
    try {
      const created = await copySkill(skill.library_skill, to, {
        category,
        uncategorized: category === undefined,
      });
      onNotice(
        `已复制 ${skill.library_skill} 到${to === "code" ? "编码" : "龙虾"}库：${to}/${created.name}（不部署，两份独立维护）。`,
      );
      await onLibrarySaved?.();
    } catch (err) {
      setError(String(err));
    } finally {
      setCopySource(null);
      setLocalBusy(false);
    }
  }

  async function confirmDelete() {
    const names = deleteTarget ?? [];
    setDeleteTarget(null);
    if (names.length === 0) {
      return;
    }
    setLocalBusy(true);
    setError(null);
    try {
      const result = await agentDeleteLocal(target, names);
      const parts = [`已移除本机副本 ${result.removed.length} 项`];
      if (result.errors.length > 0) {
        parts.push(
          `${result.errors.length} 项未完成：${result.errors
            .map((item) => `${item.skill ?? "-"}（${item.message}）`)
            .join("；")}`,
        );
      }
      onNotice(`${parts.join("；")}。`);
      setVersion((value) => value + 1);
    } catch (err) {
      setError(String(err));
    } finally {
      setLocalBusy(false);
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <AgentPresetPills
        domain={agentDomain}
        target={target}
        agents={agents}
        reloadToken={version}
        onNotice={onNotice}
        onChanged={() => setVersion((value) => value + 1)}
      />
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-2.5">
          <label className="flex h-10 w-[240px] items-center gap-2.5 rounded-lg border border-line bg-panel px-3 text-faint">
            <Search className="size-4 shrink-0" strokeWidth={1.75} />
            <input
              value={query}
              placeholder="搜索名称、注释或路径"
              onChange={(event) => setQuery(event.target.value)}
              className="w-full border-0 bg-transparent text-[13px] text-ink-3 outline-0 placeholder:text-faint"
            />
          </label>
          <Segmented
            options={[
              { value: "all", label: "全部" },
              { value: "managed", label: "受管理" },
              { value: "local", label: "仅本地" },
              { value: "changed", label: "有变更" },
            ]}
            value={filter}
            onChange={(value) => setFilter(value)}
          />
        </div>
        <div className="flex flex-wrap items-center gap-2.5">
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
          {target !== SHARED_TARGET && (
            <Button kind="primary" compact onClick={() => setPicking(true)}>
              <Plus className="size-3.5" strokeWidth={1.75} />
              从库部署
            </Button>
          )}
        </div>
      </div>

      {error && <Notice tone="danger">{error}</Notice>}

      {multi && (
        <div className="flex flex-wrap items-center gap-2.5 rounded-lg border border-line bg-panel p-2.5">
          <Button
            compact
            disabled={visible.every((skill) => selected.includes(skill.name))}
            onClick={() => setSelected(visible.map((skill) => skill.name))}
          >
            全选
          </Button>
          <Button compact disabled={selected.length === 0} onClick={() => setSelected([])}>
            全不选
          </Button>
          <span className="text-[13px] text-muted">已选择 {selected.length}</span>
          <Button
            compact
            disabled={unmanagedSelected.length === 0}
            title={`纳入管理 ${unmanagedSelected.length} 个技能`}
            aria-label={`纳入管理 ${unmanagedSelected.length} 个技能`}
            onClick={() => setAdoptTargets(unmanagedSelected)}
          >
            <Upload className="size-3.5" strokeWidth={1.75} />
          </Button>
          <Button compact disabled={localBusy || selected.length === 0} onClick={() => setDeleteTarget(selected)}>
            <Trash2 className="size-3.5" strokeWidth={1.75} />
            删除本机副本
          </Button>
        </div>
      )}

      {visible.length > 0 && (
        <div className={layout === "grid" ? "grid grid-cols-3 gap-3" : "grid grid-cols-1 gap-3"}>
          {visible.map((skill) => {
            const isSelected = selected.includes(skill.name);
            const statusText =
              skill.status === "managed"
                ? "已同步"
                : skill.status === "paused_local_change"
                  ? "本地有变更"
                  : "仅本地";
            const tone =
              skill.status === "managed"
                ? "green"
                : skill.status === "paused_local_change"
                  ? "amber"
                  : "gray";
            return (
              <article
                key={skill.name}
                onClick={() => (multi ? toggleSelect(skill.name) : setDetailSkill(skill))}
                className={`cursor-pointer rounded-xl border border-line bg-panel p-3.5 shadow-card hover:shadow-card-hover ${
                  isSelected ? "border-accent" : ""
                }`}
              >
                <div className="flex items-center gap-2.5">
                  {multi ? (
                    <input
                      type="checkbox"
                      className="selectbox shrink-0"
                      checked={isSelected}
                      onClick={(event) => event.stopPropagation()}
                      onChange={() => toggleSelect(skill.name)}
                    />
                  ) : (
                    <span
                      className={`size-2 shrink-0 rounded-full ${
                        skill.managed ? "bg-accent-light" : "bg-faint"
                      }`}
                    />
                  )}
                  <h3 className="min-w-0 flex-1 truncate text-[14px] font-semibold" title={skill.name}>
                    <button type="button" className="max-w-full truncate text-left" onClick={() => setDetailSkill(skill)}>
                      {skill.name}
                    </button>
                    {skill.comment && (
                      <span className="ml-1 text-[11px] font-normal text-faint">
                        （{skill.comment}）
                      </span>
                    )}
                  </h3>
                  {!skill.managed && (
                    <button
                      type="button"
                      title={`收编 ${skill.name} 到技能库并接管原部署`}
                      aria-label={`纳入管理 ${skill.name}`}
                      disabled={localBusy}
                      onClick={(event) => { event.stopPropagation(); setAdoptTargets([skill]); }}
                      className="grid size-6 shrink-0 place-items-center rounded-[5px] text-accent hover:bg-hover disabled:opacity-40"
                    >
                      <Upload className="size-3.5" strokeWidth={1.75} />
                    </button>
                  )}
                  {skill.managed && skill.library_skill && (
                    <button
                      type="button"
                      title={`复制 ${skill.library_skill} 到另一库`}
                      disabled={localBusy}
                      onClick={(event) => { event.stopPropagation(); setCopySource(skill); }}
                      className="grid size-6 shrink-0 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-accent disabled:opacity-40"
                    >
                      <Copy className="size-3.5" strokeWidth={1.75} />
                    </button>
                  )}
                  <button
                    type="button"
                    title={`删除本机副本 ${skill.name}`}
                    disabled={localBusy}
                    onClick={(event) => { event.stopPropagation(); setDeleteTarget([skill.name]); }}
                    className="grid size-6 shrink-0 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-danger disabled:opacity-40"
                  >
                    <Trash2 className="size-3.5" strokeWidth={1.75} />
                  </button>
                </div>
                <div className="mt-2 flex flex-wrap items-center gap-1.5">
                  <Badge tone={tone}>{statusText}</Badge>
                  {skill.managed && skill.library_skill && (
                    <span className="text-[11px] text-faint">受管 · {skill.library_skill}</span>
                  )}
                  {skill.managed && !skill.library_exists && (
                    <Badge tone="amber">库内条目已删除</Badge>
                  )}
                </div>
                <p className="mt-2 truncate font-mono text-[11px] text-faint" title={skill.path}>
                  {skill.path}
                </p>
                <div className="mt-2 flex">
                  <QuickDeploy deploymentBlocks={skill.deployment_blocks} reference={skill.library_exists ? skill.library_skill : null} domain={agentDomain}
                    agents={agents} records={records} onChange={reload} onError={setError} />
                </div>
              </article>
            );
          })}
        </div>
      )}

      {detailSkill && (
        <LocalSkillDetail
          readFile={(file) => agentReadLocal(target, detailSkill.name, file)}
          skill={detailSkill}
          onClose={() => setDetailSkill(null)}
          onAdopt={() => { setAdoptTargets([detailSkill]); setDetailSkill(null); }}
          onOpenInLibrary={(reference) => { setDetailSkill(null); onOpenInLibrary(reference); }}
        />
      )}

      {deleteTarget && (
        <ConfirmDialog
          title="删除本机技能副本"
          subtitle="只影响该技能目录，不改变 SkillManagerVault 中的技能。"
          message={`将删除 ${deleteTarget.length} 项：${deleteTarget.join("、")}。受管理项按撤销规则处理；有本地变更的副本会暂停并保留。`}
          confirmLabel="删除"
          danger
          onCancel={() => setDeleteTarget(null)}
          onConfirm={() => void confirmDelete()}
        />
      )}

      {adoptTargets && (
        <AdoptDialog
          agentKey={target}
          agentLabel={label}
          domain={agentDomain}
          skills={adoptTargets}
          agents={agents}
          onClose={() => setAdoptTargets(null)}
          onDone={(message) => {
            setAdoptTargets(null);
            onNotice(message);
            setSelected([]);
            setVersion((value) => value + 1);
            void onLibrarySaved?.();
          }}
        />
      )}

      {copySource && copySource.library_skill && (
        <CopyToDomainDialog
          reference={copySource.library_skill}
          from={agentDomain}
          onCancel={() => setCopySource(null)}
          onConfirm={(to, category) => void copyToLibrary(copySource, to, category)}
        />
      )}

      {picking && (
        <SkillPickerDialog
          domain={agentDomain}
          onCancel={() => setPicking(false)}
          onConfirm={(references) => {
            setPicking(false);
            setDeployTargets(references);
          }}
        />
      )}

      {deployTargets && (
        <DeployDialog
          domain={agentDomain}
          references={deployTargets}
          agents={agents}
          onClose={() => {
            setDeployTargets(null);
            setVersion((value) => value + 1);
          }}
          onDone={(message) => {
            setDeployTargets(null);
            onNotice(message);
            setError(null);
            setVersion((value) => value + 1);
          }}
        />
      )}
    </div>
  );
}

/** 部署清单筛选：全部 / 已部署（记录正常）/ 有变更（副本被外部修改而暂停）。 */
type DeploymentFilter = "all" | "deployed" | "changed";

/**
 * 跨目标部署清单（FR-057）：「全部编码 Agents」/「全部龙虾 Agent」的页面内容。
 *
 * 数据与库页「文件与部署」同一份本机部署记录（`deploy_status`，跨目标的受管部署），
 * 按「技能 × 目标」列出；前端只做展示、筛选与发起部署，准入与合成仍由 core 决定。
 * 本批不做原型的「仅本地」筛选：那要对每个目标目录逐一扫描，成本高，只做记录侧筛选。
 */
function DeploymentList({
  domain,
  agents,
  onMeta,
  onNotice,
}: {
  domain: Domain;
  agents: AgentView[];
  onMeta: (meta: { count: number; subtitle: string }) => void;
  onNotice: (message: string) => void;
}) {
  const [records, setRecords] = useState<DeploymentStatusView[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<DeploymentFilter>("all");
  const [version, setVersion] = useState(0);
  const [picking, setPicking] = useState(false);
  const [deployTargets, setDeployTargets] = useState<string[] | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const list = await deployStatus();
        if (!cancelled) {
          setRecords(list);
          setError(null);
        }
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
  }, [version]);


  const mine = useMemo(
    () => records.filter((record) => record.skill.startsWith(`${domain}/`)),
    [records, domain],
  );

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return mine.filter((record) => {
      if (filter === "deployed" && record.status !== "ok") {
        return false;
      }
      if (filter === "changed" && record.status !== "paused_local_change") {
        return false;
      }
      if (q === "") {
        return true;
      }
      return [
        record.skill,
        skillName(record.skill),
        deploymentTargetName(record.target, agents),
        record.target_path,
      ]
        .join(" ")
        .toLowerCase()
        .includes(q);
    });
  }, [mine, filter, query, agents]);

  useEffect(() => {
    onMeta({
      count: visible.length,
      subtitle: "",
    });
  }, [onMeta, visible.length]);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-3">
          <div className="relative">
            <Search
              className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-faint"
              strokeWidth={1.75}
            />
            <input
              className="field-input h-9 w-[240px] pl-8 text-[12px]"
              placeholder="搜索技能、目标或路径"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </div>
          <Segmented
            options={[
              { value: "all", label: "全部" },
              { value: "deployed", label: "已部署" },
              { value: "changed", label: "有变更" },
            ]}
            value={filter}
            onChange={(value) => setFilter(value)}
          />
        </div>
        <div className="flex items-center gap-2.5">
          <span className="text-[12px] text-faint">本机部署记录 {mine.length} 项</span>
          <Button compact onClick={() => setVersion((value) => value + 1)}>
            <RefreshCw className="size-3.5" strokeWidth={1.75} />
            刷新
          </Button>
          <Button kind="primary" compact onClick={() => setPicking(true)}>
            <Plus className="size-3.5" strokeWidth={1.75} />
            从库部署
          </Button>
        </div>
      </div>

      {error && <Notice tone="danger">{error}</Notice>}

      {loading ? (
        <p className="text-[13px] text-muted">正在读取本机部署记录…</p>
      ) : visible.length === 0 ? (
        <p className="rounded-lg border border-line-soft bg-panel p-4 text-[13px] text-muted">
          {mine.length === 0
            ? "本机还没有该管理域的部署记录；可用「从库部署」把库内技能部署到本机目标。"
            : "没有符合筛选条件的部署记录。"}
        </p>
      ) : (
        <div className="grid grid-cols-3 gap-3">
          {visible.map((record) => {
            const paused = record.status === "paused_local_change";
            return (
              <article
                key={`${record.skill}:${record.target}`}
                className="rounded-xl border border-line bg-panel p-3.5 shadow-card"
              >
                <div className="flex items-center gap-2.5">
                  <span
                    className={`size-2 shrink-0 rounded-full ${paused ? "bg-warn" : "bg-accent-light"}`}
                  />
                  <h3
                    className="min-w-0 flex-1 truncate text-[14px] font-semibold"
                    title={record.skill}
                  >
                    {skillName(record.skill)}
                  </h3>
                  <Badge tone={paused ? "amber" : "green"}>
                    {paused ? "本地有变更" : "已同步"}
                  </Badge>
                </div>
                <div className="mt-2 flex flex-wrap items-center gap-1.5 text-[11px] text-faint">
                  <span>目标：{deploymentTargetName(record.target, agents)}</span>
                  <span className="font-mono">库内引用 {record.skill}</span>
                  {!record.library_exists && <Badge tone="amber">库内条目已删除，副本待处置</Badge>}
                  {record.warning && (
                    <Badge tone="amber" title={record.warning}>
                      支持范围收窄
                    </Badge>
                  )}
                </div>
                <p
                  className="mt-2 truncate font-mono text-[11px] text-faint"
                  title={record.target_path}
                >
                  {record.target_path}
                </p>
                <div className="mt-2 flex">
                  <QuickDeploy deploymentBlocks={record.deployment_blocks} reference={record.library_exists ? record.skill : null} domain={domain}
                    agents={agents} records={records} onChange={async () => setRecords(await deployStatus())} onError={setError} />
                </div>
              </article>
            );
          })}
        </div>
      )}

      {picking && (
        <SkillPickerDialog
          domain={domain}
          onCancel={() => setPicking(false)}
          onConfirm={(references) => {
            setPicking(false);
            setDeployTargets(references);
          }}
        />
      )}

      {deployTargets && (
        <DeployDialog
          domain={domain}
          references={deployTargets}
          agents={agents}
          onClose={() => { setDeployTargets(null); setVersion((value) => value + 1); }}
          onDone={(message) => {
            setDeployTargets(null);
            onNotice(message);
            setError(null);
            setVersion((value) => value + 1);
          }}
        />
      )}
    </div>
  );
}

/** 从库部署的技能选择：只选技能，目标在既有部署弹窗里选。 */
function SkillPickerDialog({
  domain,
  onCancel,
  onConfirm,
}: {
  domain: Domain;
  onCancel: () => void;
  onConfirm: (references: string[]) => void;
}) {
  const [skills, setSkills] = useState<SkillSummary[] | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void listSkills({ domain })
      .then((result) => {
        if (!cancelled) {
          setSkills(result.skills);
          setError(null);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(String(err));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [domain]);

  const visible = (skills ?? []).filter((skill) => {
    const q = query.trim().toLowerCase();
    if (q === "") {
      return true;
    }
    return [skill.name, skill.comment ?? "", ...skill.tags].join(" ").toLowerCase().includes(q);
  });

  return (
    <Dialog
      title={`从库部署 · 选择技能（${domain === "code" ? "编码库" : "龙虾库"}）`}
      width="w-[640px]"
      onClose={onCancel}
      footer={
        <>
          <span className="text-[12px] text-faint">已选 {selected.length} 个技能</span>
          <div className="flex gap-2.5">
            <Button onClick={onCancel}>取消</Button>
            <Button
              kind="primary"
              disabled={selected.length === 0}
              onClick={() => onConfirm(selected)}
            >
              下一步：选择目标
            </Button>
          </div>
        </>
      }
    >
      <div className="flex flex-col gap-3">
        {error && <Notice tone="danger">{error}</Notice>}

        <input
          className="field-input"
          placeholder="搜索名称、中文注释或标签"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />

        {skills === null && error === null ? (
          <p className="text-[13px] text-muted">正在读取库内技能…</p>
        ) : visible.length === 0 ? (
          <p className="rounded-lg border border-line-soft p-4 text-[13px] text-muted">
            {skills?.length === 0 ? "该管理域还没有技能。" : "没有符合搜索条件的技能。"}
          </p>
        ) : (
          <ul className="flex max-h-[45vh] flex-col gap-1.5 overflow-auto">
            {visible.map((skill) => {
              const reference = `${skill.domain}/${skill.name}`;
              const on = selected.includes(reference);
              return (
                <li key={reference}>
                  <label
                    className={`flex cursor-pointer items-start gap-2.5 rounded-[10px] border p-2.5 ${
                      on ? "border-accent-line bg-accent-soft" : "border-line"
                    }`}
                  >
                    <input
                      type="checkbox"
                      className="selectbox mt-0.5"
                      checked={on}
                      onChange={() =>
                        setSelected((current) =>
                          current.includes(reference)
                            ? current.filter((item) => item !== reference)
                            : [...current, reference],
                        )
                      }
                    />
                    <span className="min-w-0">
                      <span className="block truncate text-[13px] font-medium" title={skill.name}>
                        {skill.name}
                      </span>
                      {skill.comment && (
                        <span className="block truncate text-[12px] text-muted">{skill.comment}</span>
                      )}
                      <span className="block font-mono text-[11px] text-faint">{reference}</span>
                    </span>
                  </label>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </Dialog>
  );
}

