/**
 * Agent 指令编辑器（T072）：全局与逐 Agent 专属指令的单编辑区、单“保存”。
 *
 * 左列「全局指令」固定第一项并默认选中，下面是本机已启用且已检测的 Agent；同一时刻只有一个
 * 编辑区，切换工具保留未保存草稿；底部只有一个「保存」按钮，保存后保持打开。
 *
 * 保存走 core 的 `instruction set`：写源并按“全局在前、专属在后”覆盖本机已启用目标。
 * 拼接规则与分发范围由 core 决定，前端不复制；不加分发勾选、不加第二个分发按钮。
 * 指令路径与技能路径独立（由 Agents 的本机指令目标决定）。不扩展 Hermes 档案指令。
 */
import { useEffect, useMemo, useState } from "react";

import { Dialog, Button, Notice } from "./ui";
import {
  instructionGet,
  instructionSet,
  type AgentView,
  type InstructionTargetResult,
} from "../services/client";

const GLOBAL_SCOPE = "global";

export interface InstructionEditorProps {
  agents: AgentView[];
  /** 关闭编辑器（保存不会触发它）。 */
  onClose: () => void;
  /** 指令源保存成功后触发（用于按已确认策略上传）。 */
  onSaved: () => void;
}

/** 左列可编辑的作用域：全局 + 本机已启用且已检测的 Agent。 */
function instructionScopes(agents: AgentView[]): AgentView[] {
  return agents
    .filter((agent) => agent.detected && agent.enabled)
    .sort((a, b) => a.order - b.order || a.key.localeCompare(b.key));
}

export function InstructionEditor({ agents, onClose, onSaved }: InstructionEditorProps) {
  const scopes = useMemo(() => instructionScopes(agents), [agents]);
  const [selected, setSelected] = useState<string>(GLOBAL_SCOPE);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [composed, setComposed] = useState<Record<string, string>>({});
  const [loaded, setLoaded] = useState<Record<string, boolean>>({});
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [results, setResults] = useState<InstructionTargetResult[] | null>(null);
  const [query, setQuery] = useState("");

  const nameOf = useMemo(() => {
    const map = new Map(agents.map((agent) => [agent.key, agent.display_name]));
    return (key: string) => map.get(key) ?? key;
  }, [agents]);

  // 首次选中某作用域时读取源内容与合成预览；已有草稿不被覆盖。
  useEffect(() => {
    if (loaded[selected]) {
      return;
    }
    let cancelled = false;
    setLoading(true);
    void instructionGet(selected)
      .then((view) => {
        if (cancelled) {
          return;
        }
        setDrafts((prev) => (prev[selected] === undefined ? { ...prev, [selected]: view.source } : prev));
        setComposed((prev) => ({ ...prev, [selected]: view.composed }));
        setLoaded((prev) => ({ ...prev, [selected]: true }));
        setError(null);
      })
      .catch((err) => {
        if (!cancelled) {
          setError(String(err));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [selected, loaded]);

  const visibleScopes = scopes.filter(
    (agent) => query.trim().length === 0 || agent.display_name.toLowerCase().includes(query.toLowerCase()),
  );

  const value = drafts[selected] ?? "";
  const isAgentScope = selected !== GLOBAL_SCOPE;
  const preview = composed[selected];

  async function save() {
    setBusy(true);
    setError(null);
    setResults(null);
    try {
      const outcome = await instructionSet(selected, value);
      setResults(outcome);
      // 保存后刷新该作用域的源与合成预览；其它作用域的草稿保留，下次选中时更新预览。
      const view = await instructionGet(selected);
      setDrafts((prev) => ({ ...prev, [selected]: view.source }));
      setComposed((prev) => ({ ...prev, [selected]: view.composed }));
      setLoaded({ [selected]: true });
      onSaved();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  const failed = (results ?? []).filter((item) => !item.ok);

  return (
    <Dialog
      title="Agent 指令文件"
      width="w-[880px]"
      onClose={onClose}
      footer={
        <Button kind="primary" disabled={busy || loading} onClick={() => void save()}>
          {busy ? "正在保存…" : "保存"}
        </Button>
      }
    >
      <div className="flex gap-4">
        <div className="flex w-[200px] shrink-0 flex-col gap-2">
          <input
            className="field-input h-9 text-[13px]"
            value={query}
            placeholder="搜索 Agent"
            onChange={(event) => setQuery(event.target.value)}
          />
          <div className="flex flex-col gap-0.5">
            <button
              type="button"
              onClick={() => setSelected(GLOBAL_SCOPE)}
              className={`flex h-9 items-center rounded-md px-2.5 text-left text-[13px] ${
                selected === GLOBAL_SCOPE ? "bg-selected font-medium text-ink" : "text-ink-3 hover:bg-hover"
              }`}
            >
              全局指令
            </button>
            {visibleScopes.map((agent) => (
              <button
                key={agent.key}
                type="button"
                title={agent.key}
                onClick={() => setSelected(agent.key)}
                className={`flex h-9 items-center gap-2 rounded-md px-2.5 text-left text-[13px] ${
                  selected === agent.key ? "bg-selected font-medium text-ink" : "text-ink-3 hover:bg-hover"
                }`}
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
                <span className="min-w-0 flex-1 truncate">{agent.display_name}</span>
              </button>
            ))}
            {visibleScopes.length === 0 && query.trim().length > 0 && (
              <p className="px-2.5 py-1 text-[12px] text-faint">没有匹配的 Agent。</p>
            )}
          </div>
        </div>

        <div className="flex min-w-0 flex-1 flex-col gap-2.5">
          <p className="text-[12px] font-medium text-muted">
            {selected === GLOBAL_SCOPE ? "全局指令" : `专属指令 — ${nameOf(selected)}`}
          </p>

          {loading ? (
            <div className="grid h-[300px] place-items-center rounded-lg border border-line-soft text-[13px] text-muted">
              正在读取指令源…
            </div>
          ) : (
            <textarea
              className="field-input h-[300px] resize-none font-mono text-[12px]"
              value={value}
              placeholder={isAgentScope ? "该 Agent 的专属指令（留空表示仅使用全局指令）" : "全局指令（对所有已启用目标生效）"}
              onChange={(event) =>
                setDrafts((prev) => ({ ...prev, [selected]: event.target.value }))
              }
            />
          )}

          {isAgentScope && value.trim() === "" && (
            <p className="text-[12px] text-faint">
              {preview && preview.trim().length > 0
                ? "该 Agent 暂无专属内容，将仅使用全局指令。"
                : "全局与专属内容均为空。"}
            </p>
          )}

          {error && <Notice tone="danger">{error}</Notice>}

          {results && (
            <div className="flex flex-col gap-1">
              <p className="text-[12px] text-muted">
                源已保存；按本机已启用目标分发结果：
              </p>
              {results.length === 0 ? (
                <p className="text-[12px] text-faint">本机暂无可分发的已启用目标。</p>
              ) : (
                <ul className="flex flex-col gap-0.5 text-[12px]">
                  {results.map((item) => (
                    <li key={item.agent} className={item.ok ? "text-ink-3" : "text-danger"}>
                      {item.ok ? "已刷新" : "失败"} {nameOf(item.agent)} →{" "}
                      <span className="font-mono text-[11px]">{item.path}</span>
                      {!item.ok && item.error ? `（${item.error}）` : ""}
                    </li>
                  ))}
                </ul>
              )}
              {failed.length > 0 && (
                <Notice tone="warn">
                  源已保存，但 {failed.length} 个目标刷新失败，未冒充成功；请按上表逐项处理。
                </Notice>
              )}
            </div>
          )}

        </div>
      </div>
    </Dialog>
  );
}
