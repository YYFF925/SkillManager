/**
 * 侧栏预设区：编码库/龙虾库紧凑切换、拖拽排序、小火箭安装到 Agent。
 * 对应原型 `.preset-section`；预设页本身不再重复左侧列表。
 */
import { useEffect, useMemo, useRef, useState } from "react";
import { ChevronDown, GripVertical, Pencil, Plus, Rocket, Trash2 } from "lucide-react";

import {
  createPreset,
  deletePreset,
  deployPreset,
  listPresets,
  renamePreset,
  reorderPresets,
  setPresetIcon,
  type AgentView,
  type Domain,
  type PresetView,
} from "../services/client";
import { PRESET_ICON_OPTIONS, presetIconOption } from "../lib/presetIcons";
import { Button, ConfirmDialog, Dialog, Field, Notice } from "./ui";

export interface SidebarPresetsProps {
  agents: AgentView[];
  domain: Domain;
  selectedId: string | null;
  active: boolean;
  reloadToken: number;
  onDomainChange: (domain: Domain) => void;
  onSelect: (presetId: string) => void;
  onCreateNavigate: (presetId: string) => void;
  onNotice: (message: string) => void;
  onChanged: () => void;
}

function deployTargets(agents: AgentView[], domain: Domain): AgentView[] {
  return agents
    .filter((agent) => agent.domain === domain && agent.detected && agent.enabled)
    .sort((a, b) => a.order - b.order || a.key.localeCompare(b.key));
}

export function SidebarPresets({
  agents,
  domain,
  selectedId,
  active,
  reloadToken,
  onDomainChange,
  onSelect,
  onCreateNavigate,
  onNotice,
  onChanged,
}: SidebarPresetsProps) {
  const [presets, setPresets] = useState<PresetView[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [dragId, setDragId] = useState<string | null>(null);
  const [rocketFor, setRocketFor] = useState<string | null>(null);
  const [rocketPos, setRocketPos] = useState<{ left: number; top: number } | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [editTarget, setEditTarget] = useState<PresetView | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<PresetView | null>(null);
  const [name, setName] = useState("");
  const [icon, setIcon] = useState<string | null>(PRESET_ICON_OPTIONS[0]?.key ?? null);
  const [busy, setBusy] = useState(false);
  const rocketRef = useRef<HTMLDivElement>(null);

  const targets = useMemo(() => deployTargets(agents, domain), [agents, domain]);

  async function refresh() {
    try {
      const list = await listPresets(domain);
      setPresets(list);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }

  useEffect(() => {
    void refresh();
  }, [domain, reloadToken]);

  useEffect(() => {
    if (!rocketFor) {
      return;
    }
    function onPointer(event: MouseEvent) {
      if (!rocketRef.current?.contains(event.target as Node)) {
        const trigger = (event.target as HTMLElement | null)?.closest?.("[data-preset-rocket]");
        if (!trigger) {
          setRocketFor(null);
        }
      }
    }
    document.addEventListener("mousedown", onPointer);
    return () => document.removeEventListener("mousedown", onPointer);
  }, [rocketFor]);

  async function reorder(targetId: string) {
    if (!dragId || dragId === targetId) {
      setDragId(null);
      return;
    }
    const from = presets.findIndex((item) => item.id === dragId);
    const to = presets.findIndex((item) => item.id === targetId);
    if (from < 0 || to < 0) {
      setDragId(null);
      return;
    }
    const next = [...presets];
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    setPresets(next);
    setDragId(null);
    try {
      const updated = await reorderPresets(
        domain,
        next.map((entry) => entry.id),
      );
      setPresets(updated);
      onChanged();
    } catch (err) {
      setError(String(err));
      void refresh();
    }
  }

  async function install(presetId: string, agentKey: string) {
    setRocketFor(null);
    setBusy(true);
    try {
      await deployPreset(domain, presetId, `agent:${agentKey}`);
      onNotice(`已按预设安装到 ${targets.find((item) => item.key === agentKey)?.display_name ?? agentKey}`);
      onChanged();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function submitCreate() {
    const trimmed = name.trim();
    if (!trimmed) {
      return;
    }
    setBusy(true);
    try {
      const created = await createPreset(domain, trimmed, icon ?? undefined);
      setCreateOpen(false);
      setName("");
      await refresh();
      onCreateNavigate(created.id);
      onChanged();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function submitEdit() {
    if (!editTarget) {
      return;
    }
    const trimmed = name.trim();
    if (!trimmed) {
      return;
    }
    setBusy(true);
    try {
      if (trimmed !== editTarget.name) {
        await renamePreset(domain, editTarget.id, trimmed);
      }
      if ((icon ?? null) !== (editTarget.icon ?? null)) {
        await setPresetIcon(domain, editTarget.id, icon);
      }
      setEditTarget(null);
      await refresh();
      onChanged();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function confirmDelete() {
    if (!deleteTarget) {
      return;
    }
    const target = deleteTarget;
    setDeleteTarget(null);
    try {
      await deletePreset(domain, target.id);
      await refresh();
      onChanged();
      onNotice(`已删除预设「${target.name}」`);
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="mt-2 flex flex-col border-t border-line-soft pt-2">
      <div className="mb-2 flex items-center gap-2 px-1">
        <div className="flex items-center gap-1 text-[12px] font-medium text-muted">
          <ChevronDown className="size-3 text-faint" strokeWidth={2} />
          预设
        </div>
        <div
          role="group"
          aria-label="预设所属库"
          className="ml-auto flex w-max items-center rounded-lg border border-line bg-panel p-0.5"
        >
          {(
            [
              ["code", "编码库"],
              ["lobster", "龙虾库"],
            ] as const
          ).map(([value, label]) => (
            <button
              key={value}
              type="button"
              onClick={() => onDomainChange(value)}
              className={`h-[22px] rounded-md px-2 text-[12px] leading-[22px] ${
                domain === value ? "bg-selected font-medium text-ink-2" : "text-muted hover:text-ink-2"
              }`}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {error && (
        <div className="mb-1.5 px-1">
          <Notice tone="danger">{error}</Notice>
        </div>
      )}

      <div className="flex flex-col gap-0.5">
        {presets.length === 0 && (
          <p className="px-2 py-1 text-[12px] text-faint">该库还没有预设</p>
        )}
        {presets.map((item) => {
          const option = presetIconOption(item.icon);
          const Icon = option.icon;
          const selected = active && item.id === selectedId;
          return (
            <div
              key={item.id}
              className={`group relative flex items-center rounded-md ${
                selected ? "bg-selected" : "hover:bg-hover"
              }`}
              onDragOver={(event) => event.preventDefault()}
              onDrop={() => void reorder(item.id)}
            >
              <button
                type="button"
                draggable
                title="按住拖动排序"
                aria-label={`拖动排序 ${item.name}`}
                onDragStart={() => setDragId(item.id)}
                onDragEnd={() => setDragId(null)}
                className="grid h-7 w-4 shrink-0 cursor-grab place-items-center text-[#c4c4cc] active:cursor-grabbing group-hover:text-muted"
              >
                <GripVertical className="size-3.5" strokeWidth={1.75} />
              </button>
              <button
                type="button"
                title={`${item.name}（${item.id}）`}
                onClick={() => onSelect(item.id)}
                className={`flex h-[35px] min-w-0 flex-1 items-center gap-2 truncate pr-1 text-left text-[14px] ${
                  selected ? "font-medium text-ink" : "text-ink-2"
                }`}
              >
                <Icon className="size-4 shrink-0" strokeWidth={1.75} style={{ color: option.color }} />
                <span className="min-w-0 truncate">{item.name}</span>
              </button>
              <div className="flex shrink-0 items-center opacity-0 group-hover:opacity-100 group-focus-within:opacity-100">
                <button
                  type="button"
                  title="重命名 / 图标"
                  aria-label={`编辑预设 ${item.name}`}
                  onClick={() => {
                    setEditTarget(item);
                    setName(item.name);
                    setIcon(item.icon ?? PRESET_ICON_OPTIONS[0]?.key ?? null);
                  }}
                  className="grid size-5 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-accent"
                >
                  <Pencil className="size-3.5" strokeWidth={1.75} />
                </button>
                <button
                  type="button"
                  data-preset-rocket={item.id}
                  title="安装到 Agent"
                  aria-label={`安装预设 ${item.name} 到 Agent`}
                  disabled={busy || targets.length === 0}
                  onClick={(event) => {
                    const rect = event.currentTarget.getBoundingClientRect();
                    const next = rocketFor === item.id ? null : item.id;
                    setRocketFor(next);
                    if (next) {
                      const left = Math.max(8, Math.min(window.innerWidth - 204, rect.left));
                      const top =
                        rect.bottom + 6 + 180 > window.innerHeight
                          ? Math.max(8, rect.top - 180 - 6)
                          : rect.bottom + 6;
                      setRocketPos({ left, top });
                    }
                  }}
                  className="grid size-5 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-accent disabled:opacity-40"
                >
                  <Rocket className="size-3.5" strokeWidth={1.75} />
                </button>
                <button
                  type="button"
                  title="删除预设"
                  aria-label={`删除预设 ${item.name}`}
                  onClick={() => setDeleteTarget(item)}
                  className="grid size-5 place-items-center rounded-[5px] text-faint hover:bg-hover hover:text-danger"
                >
                  <Trash2 className="size-3.5" strokeWidth={1.75} />
                </button>
              </div>
            </div>
          );
        })}
      </div>

      <button
        type="button"
        onClick={() => {
          setName("");
          setIcon(PRESET_ICON_OPTIONS[0]?.key ?? null);
          setCreateOpen(true);
        }}
        className="mt-1 flex items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-[13px] text-muted hover:bg-hover"
      >
        <Plus className="size-4 shrink-0" strokeWidth={1.75} />
        新建预设
      </button>

      {rocketFor && rocketPos && (
        <div
          ref={rocketRef}
          role="menu"
          className="fixed z-50 w-[196px] rounded-lg border border-line bg-panel p-1 shadow-[0_8px_24px_rgba(24,24,27,0.12)]"
          style={{ left: rocketPos.left, top: rocketPos.top }}
        >
          <div className="px-2 pt-1 pb-0.5 text-[11px] text-faint">安装到 Agent</div>
          {targets.map((agent) => (
            <button
              key={agent.key}
              type="button"
              role="menuitem"
              onClick={() => void install(rocketFor, agent.key)}
              className="flex h-8 w-full items-center gap-2 rounded-md px-2 text-left text-[13px] text-ink-3 hover:bg-hover"
            >
              {agent.icon ? (
                <img
                  src={`/${agent.icon}`}
                  alt=""
                  className={`size-4 shrink-0 object-contain ${agent.invert_in_dark ? "dark:invert" : ""}`}
                />
              ) : (
                <span className="size-4 shrink-0" />
              )}
              <span className="truncate">{agent.display_name}</span>
            </button>
          ))}
          {targets.length === 0 && (
            <p className="px-2 py-1.5 text-[12px] text-faint">暂无可用 Agent</p>
          )}
        </div>
      )}

      {createOpen && (
        <Dialog
          title="新建预设"
          width="w-[480px]"
          onClose={() => setCreateOpen(false)}
          footer={
            <>
              <span />
              <div className="flex gap-2.5">
                <Button onClick={() => setCreateOpen(false)}>取消</Button>
                <Button kind="primary" disabled={busy || !name.trim()} onClick={() => void submitCreate()}>
                  创建
                </Button>
              </div>
            </>
          }
        >
          <div className="flex flex-col gap-3">
            <Field label="名称">
              <input
                className="field-input"
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder="例如：日常维护"
              />
            </Field>
            <Field label="图标">
              <div className="grid max-w-[350px] grid-cols-8 gap-1.5">
                {PRESET_ICON_OPTIONS.map((option) => {
                  const Icon = option.icon;
                  const on = option.key === icon;
                  return (
                    <button
                      key={option.key}
                      type="button"
                      title={option.label}
                      aria-label={option.label}
                      onClick={() => setIcon(option.key)}
                      className={`flex h-9 items-center justify-center rounded-lg border ${
                        on
                          ? "border-accent-line bg-accent-soft text-accent"
                          : "border-line bg-panel text-muted hover:bg-hover"
                      }`}
                    >
                      <Icon className="size-[15px]" strokeWidth={1.75} style={{ color: option.color }} />
                    </button>
                  );
                })}
              </div>
            </Field>
          </div>
        </Dialog>
      )}

      {editTarget && (
        <Dialog
          title="编辑预设"
          width="w-[480px]"
          onClose={() => setEditTarget(null)}
          footer={
            <>
              <span />
              <div className="flex gap-2.5">
                <Button onClick={() => setEditTarget(null)}>取消</Button>
                <Button kind="primary" disabled={busy || !name.trim()} onClick={() => void submitEdit()}>
                  保存
                </Button>
              </div>
            </>
          }
        >
          <div className="flex flex-col gap-3">
            <Field label="名称">
              <input
                className="field-input"
                value={name}
                onChange={(event) => setName(event.target.value)}
              />
            </Field>
            <Field label="图标">
              <div className="grid max-w-[350px] grid-cols-8 gap-1.5">
                {PRESET_ICON_OPTIONS.map((option) => {
                  const Icon = option.icon;
                  const on = option.key === icon;
                  return (
                    <button
                      key={option.key}
                      type="button"
                      title={option.label}
                      aria-label={option.label}
                      onClick={() => setIcon(option.key)}
                      className={`flex h-9 items-center justify-center rounded-lg border ${
                        on
                          ? "border-accent-line bg-accent-soft text-accent"
                          : "border-line bg-panel text-muted hover:bg-hover"
                      }`}
                    >
                      <Icon className="size-[15px]" strokeWidth={1.75} style={{ color: option.color }} />
                    </button>
                  );
                })}
              </div>
            </Field>
          </div>
        </Dialog>
      )}

      {deleteTarget && (
        <ConfirmDialog
          title="删除预设"
          danger
          confirmLabel="删除"
          message={`删除预设「${deleteTarget.name}」只移除组合定义，不会撤销已部署的技能。`}
          onCancel={() => setDeleteTarget(null)}
          onConfirm={() => void confirmDelete()}
        />
      )}
    </div>
  );
}
