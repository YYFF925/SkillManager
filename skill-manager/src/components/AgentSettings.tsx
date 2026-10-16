/**
 * 支持的 Agent 区块（T109）：检测/重新检测、启停、拖动排序、技能目录与项目相对路径行内修改。
 *
 * 从 Agent 页原样搬到设置页（FR-041）：检测只读本机存在事实；启停、顺序与目录只写本机状态，
 * 不改动已有部署。组件内不含管理域过滤——设置页展示全量内置名单（已检测数/内置总数）。
 */
import { useEffect, useState } from "react";
import {
  Check,
  ChevronDown,
  ChevronRight,
  GripVertical,
  Layers,
  Pencil,
  RefreshCw,
  RotateCcw,
  X,
} from "lucide-react";

import {
  agentDetect,
  agentDisable,
  agentEnable,
  agentReorder,
  agentResetDir,
  agentSetDir,
  type AgentDirKind,
  type AgentView,
} from "../services/client";
import { Notice } from "./ui";

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

export interface AgentSettingsProps {
  /** 全量内置 Agent 名单（含两个管理域）；检测与顺序事实由 core 给出。 */
  agents: AgentView[];
  onAgentsChanged: (agents: AgentView[]) => void;
  onNotice: (message: string) => void;
  /** 点击 Agent 名称前往该目标的本地技能视图。 */
  onSelectTarget: (target: string) => void;
}

export function AgentSettings({
  agents,
  onAgentsChanged,
  onNotice,
  onSelectTarget,
}: AgentSettingsProps) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [othersOpen, setOthersOpen] = useState(false);
  const [dragKey, setDragKey] = useState<string | null>(null);

  const sorted = [...agents].sort((a, b) => a.order - b.order || a.key.localeCompare(b.key));
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

  return (
    <section className="rounded-xl border border-line-soft bg-panel p-5 shadow-card">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-[15px] font-semibold">
          支持的 Agent（{detected.length}/{agents.length}）
        </h2>
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

      <div className="mt-1.5 text-[13px] text-muted">
        已检测 <b className="text-ink">{detected.length}</b>
        <span className="mx-2 text-faint">·</span>已启用 <b className="text-ink">{enabled}</b>
      </div>

      {error && (
        <div className="mt-3">
          <Notice tone="danger">{error}</Notice>
        </div>
      )}

      <div className="mt-3.5">
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

      <div className="mt-3">
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
    </section>
  );
}

function kindLabel(kind: AgentDirKind): string {
  return kind === "skills" ? "技能目录" : kind === "project-rel" ? "项目相对路径" : "指令目标";
}

function agentIconSrc(agent: AgentView): string | null {
  return agent.icon ? `/${agent.icon}` : null;
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
          isCustom={agent.project_rel_dir !== agent.project_rel_dir_default}
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
