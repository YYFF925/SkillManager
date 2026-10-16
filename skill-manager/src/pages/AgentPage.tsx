/**
 * Agent 页面：本机 Agent 分组、重新检测、启停与拖动排序、目录行内修改，
 * 以及单个目标（内置 Agent 目录或共享目录）的本地技能列表与删除
 * （受管/仅本地/本地有变更）。
 *
 * 检测只读本机存在事实；启停、顺序与目录只写本机状态，不改动已有部署。
 * 共享目录是本机部署目标，不是可登记的 Agent 或第二个管理库。
 * US2 的收编入口在同页后续追加，本期不实现。
 */
import { useCallback, useEffect, useState } from "react";
import {
  Check,
  ChevronDown,
  ChevronRight,
  Copy,
  GripVertical,
  Layers,
  Pencil,
  RefreshCw,
  RotateCcw,
  Trash2,
  Upload,
  X,
} from "lucide-react";

import {
  agentDeleteLocal,
  agentDetect,
  agentDisable,
  agentEnable,
  agentLocalSkills,
  agentReorder,
  agentResetDir,
  agentSetDir,
  copySkill,
  sharedDirGet,
  type AgentDirKind,
  type AgentView,
  type Domain,
  type LocalSkillView,
} from "../services/client";
import { AdoptDialog } from "../components/AdoptDialog";
import { CopyToDomainDialog } from "../components/SkillDialogs";
import { Badge, Button, ConfirmDialog, Notice, Segmented } from "../components/ui";

/** 共享目录目标标识（与 core 的 `shared` 取值一致）；它是编码 Agent 的部署目标。 */
const SHARED_TARGET = "shared";

/** 目标展示名：内置 Agent 用其名称，共享目录用固定标签。 */
function targetLabel(target: string, agent: AgentView | undefined): string {
  return agent?.display_name ?? (target === SHARED_TARGET ? "共享 Agent" : target);
}

function agentIconSrc(agent: AgentView): string | null {
  return agent.icon ? `/${agent.icon}` : null;
}

/** 开关（原型 .toggle）。 */
function Toggle({
  checked,
  onChange,
  title,
}: {
  checked: boolean;
  onChange: () => void;
  title: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      title={title}
      onClick={onChange}
      className={`relative h-[22px] w-[38px] shrink-0 rounded-full transition-colors ${
        checked ? "bg-accent" : "bg-line"
      }`}
    >
      <span
        className={`absolute top-[2px] size-[18px] rounded-full bg-white shadow-sm transition-all ${
          checked ? "left-[18px]" : "left-[2px]"
        }`}
      />
    </button>
  );
}

/** 目录行：显示 + 行内编辑 + 保存/取消 + 恢复默认。 */
function PathLine({
  label,
  value,
  fallback,
  isCustom,
  busy,
  onSave,
  onReset,
  allowReset,
}: {
  label: string;
  value: string | null;
  fallback?: string | null;
  isCustom: boolean;
  busy: boolean;
  onSave: (next: string) => void;
  onReset: () => void;
  /** 项目相对路径允许留空（等于清除）；技能目录不允许。 */
  allowReset: boolean;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value ?? "");
  const display = value ?? fallback ?? "未设置";

  useEffect(() => {
    if (!editing) {
      setDraft(value ?? "");
    }
  }, [value, editing]);

  if (editing) {
    return (
      <div className="flex items-center gap-1.5">
        <input
          autoFocus
          className="field-input h-8 flex-1 text-[12px]"
          value={draft}
          placeholder={fallback ? `默认：${fallback}` : "绝对路径"}
          onChange={(event) => setDraft(event.target.value)}
        />
        <button
          type="button"
          title="保存"
          disabled={busy}
          onClick={() => {
            onSave(draft);
            setEditing(false);
          }}
          className="grid size-6 place-items-center rounded-[5px] text-accent hover:bg-hover disabled:opacity-40"
        >
          <Check className="size-3.5" strokeWidth={2} />
        </button>
        <button
          type="button"
          title="取消"
          onClick={() => {
            setDraft(value ?? "");
            setEditing(false);
          }}
          className="grid size-6 place-items-center rounded-[5px] text-faint hover:bg-hover"
        >
          <X className="size-3.5" strokeWidth={2} />
        </button>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-1.5">
      <span
        title={display}
        className={`min-w-0 flex-1 truncate text-[12px] ${isCustom ? "text-ink-3" : "text-muted"}`}
      >
        {label}：{display}
        {!isCustom && fallback ? "（默认）" : ""}
      </span>
      <button
        type="button"
        title={`修改${label}`}
        onClick={() => setEditing(true)}
        className="grid size-6 shrink-0 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-accent"
      >
        <Pencil className="size-3.5" strokeWidth={1.75} />
      </button>
      {isCustom && allowReset && (
        <button
          type="button"
          title="恢复默认"
          disabled={busy}
          onClick={onReset}
          className="grid size-6 shrink-0 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-accent disabled:opacity-40"
        >
          <RotateCcw className="size-3.5" strokeWidth={1.75} />
        </button>
      )}
    </div>
  );
}

export interface AgentPageProps {
  domain: Domain;
  /** 选中的单个目标（内置 Agent key 或共享目录 `shared`）；为 null 时显示该管理域的 Agent 管理视图。 */
  selectedTarget: string | null;
  agents: AgentView[];
  onSelectTarget: (target: string | null) => void;
  onAgentsChanged: (agents: AgentView[]) => void;
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
  onAgentsChanged,
  onNotice,
  onOpenSettings,
  onLibrarySaved,
}: AgentPageProps) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [othersOpen, setOthersOpen] = useState(false);
  const [dragKey, setDragKey] = useState<string | null>(null);

  const domainAgents = agents.filter((agent) => agent.domain === domain);
  const sorted = [...domainAgents].sort((a, b) => a.order - b.order || a.key.localeCompare(b.key));
  const detected = sorted.filter((agent) => agent.detected);
  const others = sorted.filter((agent) => !agent.detected);
  const enabled = detected.filter((agent) => agent.enabled).length;

  async function run(action: () => Promise<AgentView[]>, message: string) {
    setBusy(true);
    setError(null);
    try {
      const next = await action();
      onAgentsChanged(next);
      onNotice(message);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function toggle(agent: AgentView) {
    setBusy(true);
    setError(null);
    try {
      const updated = agent.enabled
        ? await agentDisable(agent.key)
        : await agentEnable(agent.key);
      onAgentsChanged(agents.map((item) => (item.key === updated.key ? updated : item)));
      onNotice(`${updated.display_name} 已${updated.enabled ? "启用" : "停用"}（本机配置）。`);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function updateDir(key: string, kind: AgentDirKind, path: string) {
    setBusy(true);
    setError(null);
    try {
      const updated = await agentSetDir(key, kind, path);
      onAgentsChanged(agents.map((item) => (item.key === updated.key ? updated : item)));
      onNotice(`${updated.display_name} 的${kindLabel(kind)}已更新（本机配置）。`);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function resetDir(key: string, kind: AgentDirKind) {
    setBusy(true);
    setError(null);
    try {
      const updated = await agentResetDir(key, kind);
      onAgentsChanged(agents.map((item) => (item.key === updated.key ? updated : item)));
      onNotice(`${updated.display_name} 的${kindLabel(kind)}已恢复默认。`);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  function dropOn(targetKey: string) {
    if (!dragKey || dragKey === targetKey) {
      return;
    }
    const ordered = detected.map((agent) => agent.key);
    const from = ordered.indexOf(dragKey);
    const to = ordered.indexOf(targetKey);
    if (from === -1 || to === -1) {
      return;
    }
    ordered.splice(to, 0, ...ordered.splice(from, 1));
    void run(() => agentReorder(ordered), `已调整显示顺序（本机配置）。`);
  }

  if (selectedTarget !== null) {
    const agent = agents.find((item) => item.key === selectedTarget);
    return (
      <LocalSkillsView
        target={selectedTarget}
        agent={agent}
        agents={agents}
        busy={busy}
        onBack={() => onSelectTarget(null)}
        onOpenSettings={onOpenSettings}
        onNotice={onNotice}
        onLibrarySaved={onLibrarySaved}
      />
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="text-[13px] text-muted">
          已检测 <b className="text-ink">{detected.length}</b>
          <span className="mx-2 text-faint">·</span>
          已启用 <b className="text-ink">{enabled}</b>
        </div>
        <div className="flex items-center gap-3 text-[13px]">
          <button
            type="button"
            disabled={busy || detected.length === 0}
            onClick={() => {
              const pending = detected.filter((agent) => !agent.enabled);
              void run(
                () =>
                  pending.length === 0
                    ? Promise.resolve(agents)
                    : Promise.all(pending.map((agent) => agentEnable(agent.key))),
                "已启用全部已检测的 Agent。",
              );
            }}
            className="text-accent hover:underline disabled:opacity-40"
          >
            全部启用
          </button>
          <button
            type="button"
            disabled={busy || detected.length === 0}
            onClick={() => {
              const pending = detected.filter((agent) => agent.enabled);
              void run(
                () =>
                  pending.length === 0
                    ? Promise.resolve(agents)
                    : Promise.all(pending.map((agent) => agentDisable(agent.key))),
                "已停用全部已检测的 Agent。",
              );
            }}
            className="text-accent hover:underline disabled:opacity-40"
          >
            全部禁用
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() =>
              void run(() => agentDetect(), "已按本机目录重新检测（检测不启动 Agent、不部署技能）。")
            }
            className="flex items-center gap-1.5 text-accent hover:underline disabled:opacity-40"
          >
            <RefreshCw className="size-3.5" strokeWidth={1.75} />
            重新检测
          </button>
        </div>
      </div>

      {error && <Notice tone="danger">{error}</Notice>}

      <div>
        <p className="mb-2 text-[12px] text-faint">已检测到的 Agent {detected.length}</p>
        {detected.length === 0 ? (
          <p className="rounded-lg border border-line-soft bg-panel p-4 text-[13px] text-muted">
            本机尚未检测到 Agent；可在安装 Agent 后重新检测。
          </p>
        ) : (
          <div className="grid grid-cols-3 gap-2">
            {detected.map((agent) => (
              <AgentCard
                key={agent.key}
                agent={agent}
                busy={busy}
                draggable
                dragging={dragKey === agent.key}
                onDragStart={() => setDragKey(agent.key)}
                onDragOver={(event) => event.preventDefault()}
                onDrop={() => dropOn(agent.key)}
                onToggle={() => void toggle(agent)}
                onSelect={() => onSelectTarget(agent.key)}
                onSaveDir={(kind, path) => void updateDir(agent.key, kind, path)}
                onResetDir={(kind) => void resetDir(agent.key, kind)}
              />
            ))}
          </div>
        )}
      </div>

      <div>
        <button
          type="button"
          onClick={() => setOthersOpen((value) => !value)}
          className="flex items-center gap-1.5 text-[12px] text-muted hover:text-ink-2"
        >
          {othersOpen ? (
            <ChevronDown className="size-3.5" strokeWidth={2} />
          ) : (
            <ChevronRight className="size-3.5" strokeWidth={2} />
          )}
          其他支持的 Agent（{others.length}）
        </button>
        {othersOpen && (
          <div className="mt-2 grid grid-cols-3 gap-2">
            {others.map((agent) => (
              <AgentCard
                key={agent.key}
                agent={agent}
                busy={busy}
                dragging={false}
                onDragStart={() => {}}
                onDragOver={() => {}}
                onDrop={() => {}}
                onToggle={() => void toggle(agent)}
                onSelect={() => onSelectTarget(agent.key)}
                onSaveDir={(kind, path) => void updateDir(agent.key, kind, path)}
                onResetDir={(kind) => void resetDir(agent.key, kind)}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function kindLabel(kind: AgentDirKind): string {
  return kind === "skills" ? "技能目录" : kind === "project-rel" ? "项目相对路径" : "指令目标";
}

function AgentCard({
  agent,
  busy,
  draggable = false,
  dragging,
  onDragStart,
  onDragOver,
  onDrop,
  onToggle,
  onSelect,
  onSaveDir,
  onResetDir,
}: {
  agent: AgentView;
  busy: boolean;
  draggable?: boolean;
  dragging: boolean;
  onDragStart: () => void;
  onDragOver: (event: React.DragEvent<HTMLDivElement>) => void;
  onDrop: () => void;
  onToggle: () => void;
  onSelect: () => void;
  onSaveDir: (kind: AgentDirKind, path: string) => void;
  onResetDir: (kind: AgentDirKind) => void;
}) {
  const icon = agentIconSrc(agent);
  return (
    <div
      draggable={draggable}
      onDragStart={onDragStart}
      onDragOver={onDragOver}
      onDrop={onDrop}
      className={`min-h-[94px] rounded-xl border border-line bg-panel px-3.5 py-3 shadow-card ${
        dragging ? "opacity-60" : ""
      }`}
    >
      <div className="flex items-center gap-2.5">
        {draggable && (
          <GripVertical className="size-3.5 shrink-0 cursor-grab text-faint" strokeWidth={1.75} />
        )}
        {icon ? (
          <img
            src={icon}
            alt=""
            className={`size-4 shrink-0 object-contain ${agent.invert_in_dark ? "dark:invert" : ""}`}
          />
        ) : (
          <Layers className="size-4 shrink-0 text-muted" strokeWidth={1.75} />
        )}
        <button
          type="button"
          title={agent.key}
          onClick={onSelect}
          className="min-w-0 flex-1 truncate text-left text-[14px] font-semibold hover:text-accent"
        >
          {agent.display_name}
        </button>
        <Toggle
          checked={agent.enabled}
          onChange={onToggle}
          title={`${agent.enabled ? "停用" : "启用"} ${agent.display_name}`}
        />
      </div>

      <div className="mt-2.5 flex flex-col gap-1.5">
        <PathLine
          label="技能目录"
          value={agent.skills_dir}
          fallback={agent.skills_dir_default}
          isCustom={agent.skills_dir_custom}
          busy={busy}
          allowReset
          onSave={(next) => onSaveDir("skills", next)}
          onReset={() => onResetDir("skills")}
        />
        <PathLine
          label="项目路径"
          value={agent.project_rel_dir}
          fallback={agent.project_rel_dir_default}
          isCustom={
            agent.project_rel_dir !== agent.project_rel_dir_default
          }
          busy={busy}
          allowReset
          onSave={(next) => onSaveDir("project-rel", next)}
          onReset={() => onResetDir("project-rel")}
        />
        <p className="text-[11px] text-faint">
          检测目录：{agent.detected ? "存在" : "不存在"} · 目标 {`agent:${agent.key}`}
        </p>
      </div>
    </div>
  );
}

/** 本机目标的技能列表：内置 Agent 目录与共享目录共用同一套筛选与删除。 */
function LocalSkillsView({
  target,
  agent,
  agents,
  busy,
  onBack,
  onOpenSettings,
  onNotice,
  onLibrarySaved,
}: {
  target: string;
  agent: AgentView | undefined;
  agents: AgentView[];
  busy: boolean;
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
                      disabled={busy || localBusy}
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
                      disabled={busy || localBusy}
                      onClick={() => setCopySource(skill)}
                      className="grid size-6 shrink-0 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-accent disabled:opacity-40"
                    >
                      <Copy className="size-3.5" strokeWidth={1.75} />
                    </button>
                  )}
                  <button
                    type="button"
                    title={`删除本机副本 ${skill.name}`}
                    disabled={busy || localBusy}
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
