/**
 * Agent 页面：本机目标的**跨目标部署清单**（「全部编码 Agents」/「全部龙虾 Agent」，FR-057），
 * 以及单个目标（内置 Agent 目录或共享目录）的本地技能列表与删除
 * （受管/仅本地/本地有变更）。
 *
 * Agent 检测、启停、排序与目录配置在设置页（FR-041），本页不再承担 Agent 管理。
 * 共享目录是本机部署目标，不是可登记的 Agent 或第二个管理库。
 */
import { useCallback, useEffect, useMemo, useState } from "react";
import { Copy, Plus, RefreshCw, Search, Trash2, Upload } from "lucide-react";

import {
  agentDeleteLocal,
  agentLocalSkills,
  copySkill,
  deployStatus,
  listSkills,
  sharedDirGet,
  sharedDirSet,
  type AgentView,
  type DeploymentStatusView,
  type Domain,
  type LocalSkillView,
  type SkillSummary,
} from "../services/client";
import { AdoptDialog } from "../components/AdoptDialog";
import { CopyToDomainDialog } from "../components/SkillDialogs";
import { DeployDialog } from "../components/DeployDialog";
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
  onSelectTarget: (target: string | null) => void;
  onNotice: (message: string) => void;
  /** 前往设置页（共享目录未设置时提供入口，不代为猜测路径）。 */
  onOpenSettings: () => void;
  /** 保存库内修改后的统一流程（先刷新已有部署，再按已确认策略上传）。 */
  onLibrarySaved?: () => Promise<void>;
}

export function AgentPage({
  domain,
  selectedTarget,
  agents,
  onSelectTarget,
  onNotice,
  onOpenSettings,
  onLibrarySaved,
}: AgentPageProps) {
  if (selectedTarget !== null) {
    const agent = agents.find((item) => item.key === selectedTarget);
    return (
      <LocalSkillsView
        target={selectedTarget}
        agent={agent}
        agents={agents}
        onBack={() => onSelectTarget(null)}
        onOpenSettings={onOpenSettings}
        onNotice={onNotice}
        onLibrarySaved={onLibrarySaved}
      />
    );
  }

  return (
    <DeploymentList domain={domain} agents={agents} onNotice={onNotice} />
  );
}

/** 本机目标的技能列表：内置 Agent 目录与共享目录共用同一套筛选与删除。 */
function LocalSkillsView({
  target,
  agent,
  agents,
  onBack,
  onOpenSettings,
  onNotice,
  onLibrarySaved,
}: {
  target: string;
  agent: AgentView | undefined;
  agents: AgentView[];
  onBack: () => void;
  onOpenSettings: () => void;
  onNotice: (message: string) => void;
  onLibrarySaved?: () => Promise<void>;
}) {
  const [skills, setSkills] = useState<LocalSkillView[]>([]);
  const [skillsDir, setSkillsDir] = useState("");
  const [dirUnset, setDirUnset] = useState(false);
  const [filter, setFilter] = useState<"all" | "managed" | "local" | "changed">("all");
  const [selected, setSelected] = useState<string[]>([]);
  const [deleteTarget, setDeleteTarget] = useState<string[] | null>(null);
  const [adoptTargets, setAdoptTargets] = useState<LocalSkillView[] | null>(null);
  const [copySource, setCopySource] = useState<LocalSkillView | null>(null);
  const [localBusy, setLocalBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [version, setVersion] = useState(0);

  /** 共享目录是编码 Agent 的部署目标，不提供收编（它是部署产物，不是来源）。 */
  const isShared = target === SHARED_TARGET;
  const canAdopt = !isShared;
  const label = targetLabel(target, agent);
  const agentDomain: Domain = agent?.domain ?? "code";

  const reload = useCallback(async () => {
    try {
      // 未设置共享目录路径时如实提示：不猜默认路径，也不显示成“没有技能”。
      if (isShared && (await sharedDirGet()) === null) {
        setSkills([]);
        setSkillsDir("");
        setDirUnset(true);
        setError(null);
        return;
      }
      setDirUnset(false);
      const view = await agentLocalSkills(target);
      setSkills(view.skills);
      setSkillsDir(view.skills_dir);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, [target, isShared]);

  useEffect(() => {
    void reload();
  }, [reload, version]);

  useEffect(() => {
    setSelected([]);
  }, [target, filter]);

  const visible = skills.filter((skill) => {
    if (filter === "all") return true;
    if (filter === "managed") return skill.managed;
    if (filter === "local") return !skill.managed;
    return skill.status === "paused_local_change";
  });
  const managedCount = skills.filter((skill) => skill.managed).length;
  const unmanagedSelected = skills.filter((skill) => !skill.managed && selected.includes(skill.name));

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
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2.5">
          <button
            type="button"
            onClick={onBack}
            className="text-[13px] text-accent hover:underline"
          >
            全部 Agents
          </button>
          <span className="text-faint">/</span>
          <span className="text-[14px] font-semibold">{label}</span>
          <span className="text-[12px] text-faint">
            {dirUnset ? "未设置" : `${skillsDir || "未设置"} · ${managedCount} 个受管理`}
          </span>
        </div>
        {!dirUnset && (
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
        )}
      </div>

      {error && <Notice tone="danger">{error}</Notice>}

      {!dirUnset && selected.length > 0 && (
        <div className="flex flex-wrap items-center gap-2.5 rounded-lg border border-line bg-panel p-2.5">
          <span className="text-[13px] text-muted">已选择 {selected.length}</span>
          <Button compact onClick={() => setSelected([])}>
            取消选择
          </Button>
          {canAdopt && (
            <Button
              compact
              disabled={unmanagedSelected.length === 0}
              title={
                unmanagedSelected.length === 0
                  ? "收编只针对未受管的“仅本地”技能"
                  : undefined
              }
              onClick={() => setAdoptTargets(unmanagedSelected)}
            >
              <Upload className="size-3.5" strokeWidth={1.75} />
              收编仅本地{unmanagedSelected.length > 0 ? `（${unmanagedSelected.length}）` : ""}
            </Button>
          )}
          <Button compact disabled={localBusy} onClick={() => setDeleteTarget(selected)}>
            <Trash2 className="size-3.5" strokeWidth={1.75} />
            删除本机副本
          </Button>
        </div>
      )}

      {dirUnset ? (
        <div className="rounded-lg border border-line-soft bg-panel p-4 text-[13px] text-muted">
          <p>
            尚未设置共享目录路径（共享目录是编码 Agent 的部署目标），因此无法列出技能。
          </p>
          <button
            type="button"
            onClick={onOpenSettings}
            className="mt-2 text-accent hover:underline"
          >
            前往设置
          </button>
        </div>
      ) : visible.length === 0 ? (
        <p className="rounded-lg border border-line-soft bg-panel p-4 text-[13px] text-muted">
          {skills.length === 0
            ? isShared
              ? "共享技能目录下没有技能（尚未部署）。"
              : "该 Agent 技能目录下没有技能（未安装或尚未部署）。"
            : "没有符合筛选条件的技能。"}
        </p>
      ) : (
        <div className="grid grid-cols-3 gap-3">
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
                className={`rounded-xl border border-line bg-panel p-3.5 shadow-card ${
                  isSelected ? "border-accent" : ""
                }`}
              >
                <div className="flex items-center gap-2.5">
                  <input
                    type="checkbox"
                    className="selectbox shrink-0"
                    checked={isSelected}
                    onChange={() =>
                      setSelected((current) =>
                        current.includes(skill.name)
                          ? current.filter((item) => item !== skill.name)
                          : [...current, skill.name],
                      )
                    }
                  />
                  <span
                    className={`size-2 shrink-0 rounded-full ${
                      skill.managed ? "bg-accent-light" : "bg-faint"
                    }`}
                  />
                  <h3 className="min-w-0 flex-1 truncate text-[14px] font-semibold" title={skill.name}>
                    {skill.name}
                    {skill.comment && (
                      <span className="ml-1 text-[11px] font-normal text-faint">
                        （{skill.comment}）
                      </span>
                    )}
                  </h3>
                  {!skill.managed && canAdopt && (
                    <button
                      type="button"
                      title={`收编 ${skill.name} 到技能库并接管原部署`}
                      disabled={localBusy}
                      onClick={() => setAdoptTargets([skill])}
                      className="grid size-6 shrink-0 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-accent disabled:opacity-40"
                    >
                      <Upload className="size-3.5" strokeWidth={1.75} />
                    </button>
                  )}
                  {skill.managed && skill.library_skill && (
                    <button
                      type="button"
                      title={`复制 ${skill.library_skill} 到另一库`}
                      disabled={localBusy}
                      onClick={() => setCopySource(skill)}
                      className="grid size-6 shrink-0 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-accent disabled:opacity-40"
                    >
                      <Copy className="size-3.5" strokeWidth={1.75} />
                    </button>
                  )}
                  <button
                    type="button"
                    title={`删除本机副本 ${skill.name}`}
                    disabled={localBusy}
                    onClick={() => setDeleteTarget([skill.name])}
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
              </article>
            );
          })}
        </div>
      )}

      {!dirUnset && (
        <p className="text-[12px] text-faint">
          仅本地：直接删除实际文件，不改 SkillVault；受管理：按撤销部署规则处理，删除本机副本不会删库。
        </p>
      )}

      {deleteTarget && (
        <ConfirmDialog
          title="删除本机技能副本"
          subtitle="只影响该技能目录，不改变 SkillVault 中的技能。"
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
  onNotice,
}: {
  domain: Domain;
  agents: AgentView[];
  onNotice: (message: string) => void;
}) {
  const [records, setRecords] = useState<DeploymentStatusView[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<DeploymentFilter>("all");
  const [sharedDir, setSharedDir] = useState<string | null>(null);
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

  useEffect(() => {
    void sharedDirGet()
      .then(setSharedDir)
      .catch(() => setSharedDir(null));
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

  async function changeSharedDir(path: string | null) {
    try {
      const saved = await sharedDirSet(path);
      setSharedDir(saved);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }

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
              </article>
            );
          })}
        </div>
      )}

      <p className="text-[12px] text-faint">
        同一个技能可部署到多个目标；本清单只列本机受管部署记录（未受管的「仅本地」技能需在对应目标页查看）。
      </p>

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
          sharedDir={sharedDir}
          onSharedDirChange={changeSharedDir}
          onClose={() => setDeployTargets(null)}
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
      subtitle="选择库内技能后，在部署弹窗里选择目标；部署生成合成副本，不改动库内技能。"
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

