import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { Bot, Check, ChevronDown, Search, SendHorizontal, Settings } from "lucide-react";

import { Button, Dialog, Notice, buttonClass } from "./ui";
import { MarkdownView } from "./MarkdownView";
import { getTransport } from "../services/transport";

export interface ChatViewRef {
  page?: string;
  domain?: string;
  selection?: {
    refs?: string[];
    ref?: string;
    presetId?: string;
    agent?: string;
    tool?: string;
  };
}

export interface ChatPanelProps {
  open: boolean;
  view: ChatViewRef;
  onClose: () => void;
}

interface ChatMessage {
  role: string;
  text: string;
  taskId?: string | null;
}

interface AgentRuntime {
  cliId?: string | null;
  displayName?: string;
  defaultCliId?: string;
  binaryPath?: string | null;
  ready?: boolean;
  installed?: boolean;
  installedByApp?: boolean;
  userSelected?: boolean;
  needsInstall?: boolean;
  needsLogin?: boolean;
  loginCommand?: string;
  loginHint?: string;
  installHint?: string;
  error?: string | null;
  message?: string;
}

interface AgentModel {
  id: string;
  name: string;
}

interface AgentModels {
  currentModelId?: string | null;
  models?: AgentModel[];
}

interface ToolStep {
  id: string;
  title: string;
  status: string;
}

function toolStatusLabel(status: string): { text: string; tone: "done" | "failed" | "running" } {
  const normalized = status.toLowerCase();
  if (normalized === "completed" || normalized === "done" || normalized === "success") {
    return { text: "已完成", tone: "done" };
  }
  if (normalized === "failed" || normalized === "error" || normalized === "rejected") {
    return { text: "失败", tone: "failed" };
  }
  return { text: "进行中", tone: "running" };
}

const LOGIN_ACK_KEY = "sm-cli-login-ack";
const HIDDEN_MODELS_KEY = "sm-hidden-chat-models";

const QUICK_PROMPTS = ["检查上游更新", "有哪些译文待更新", "查看当前部署状态"];

function loadHiddenModels(): string[] {
  try {
    const raw = localStorage.getItem(HIDDEN_MODELS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === "string") : [];
  } catch {
    return [];
  }
}

/** 对话只上报页面与选中引用，正文由 Host 组装。 */
export function ChatPanel({ open, view, onClose: _onClose }: ChatPanelProps) {
  const [sessionId, setSessionId] = useState(() => sessionStorage.getItem("sm-agent-session") ?? crypto.randomUUID());
  const bodyRef = useRef<HTMLDivElement | null>(null);
  const composerRef = useRef<HTMLTextAreaElement | null>(null);
  const [text, setText] = useState("");
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [taskId, setTaskId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [runtime, setRuntime] = useState<AgentRuntime | null>(null);
  const [runtimeChecked, setRuntimeChecked] = useState(false);
  const [setupBusy, setSetupBusy] = useState<"install" | "login" | null>(null);
  const [models, setModels] = useState<AgentModel[]>([]);
  const [currentModelId, setCurrentModelId] = useState<string | null>(null);
  const [modelBusy, setModelBusy] = useState(false);
  const [modelStatus, setModelStatus] = useState<"idle" | "loading" | "empty" | "error">("idle");
  const [modelError, setModelError] = useState<string | null>(null);
  const [modelMenuOpen, setModelMenuOpen] = useState(false);
  const [modelEditorOpen, setModelEditorOpen] = useState(false);
  const [modelQuery, setModelQuery] = useState("");
  const [hiddenModelIds, setHiddenModelIds] = useState<string[]>(loadHiddenModels);
  const [thoughtText, setThoughtText] = useState("");
  const [toolSteps, setToolSteps] = useState<ToolStep[]>([]);
  const [streamingReply, setStreamingReply] = useState("");
  const [activityExpanded, setActivityExpanded] = useState(true);
  const modelWrapRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    sessionStorage.setItem("sm-agent-session", sessionId);
  }, [sessionId]);

  useEffect(() => {
    if (!open) return;
    let disposed = false;
    let stopActivity: (() => void) | undefined;
    let stopChunk: (() => void) | undefined;
    const transport = getTransport();
    void transport
      .subscribe("agent_activity", (raw) => {
        if (disposed || !raw || typeof raw !== "object") return;
        const payload = raw as Record<string, unknown>;
        if (payload.sessionId !== sessionId) return;
        if (payload.kind === "thought" && typeof payload.text === "string" && payload.text) {
          setThoughtText((prev) => prev + payload.text);
        } else if (payload.kind === "tool") {
          const id = String(payload.toolCallId || payload.title || "");
          if (!id) return;
          const title = String(payload.title || payload.toolKind || "工具调用");
          const status = String(payload.status || "in_progress");
          setToolSteps((prev) => {
            const idx = prev.findIndex((item) => item.id === id);
            if (idx >= 0) {
              const next = [...prev];
              next[idx] = { id, title: title || next[idx].title, status };
              return next;
            }
            return [...prev, { id, title, status }];
          });
        }
      })
      .then((unsub) => {
        if (disposed) unsub();
        else stopActivity = unsub;
      })
      .catch(() => undefined);
    void transport
      .subscribe("agent_message_chunk", (raw) => {
        if (disposed || !raw || typeof raw !== "object") return;
        const payload = raw as Record<string, unknown>;
        if (payload.sessionId !== sessionId) return;
        if (typeof payload.text === "string" && payload.text) {
          setStreamingReply((prev) => prev + payload.text);
        }
      })
      .then((unsub) => {
        if (disposed) unsub();
        else stopChunk = unsub;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      stopActivity?.();
      stopChunk?.();
    };
  }, [open, sessionId]);

  const viewKey = JSON.stringify(view);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    const transport = getTransport();
    setRuntimeChecked(false);
    void transport.invoke("agent_set_context", { sessionId, view }).catch(() => undefined);
    void transport
      .invoke<{ messages?: Array<{ role: string; text: string; taskId?: string | null }> }>(
        "agent_list_messages",
        { sessionId },
      )
      .then((result) => {
        if (cancelled) return;
        const rows = result.messages ?? [];
        setMessages(
          rows.map((row) => ({
            role: row.role,
            text: row.text,
            taskId: row.taskId,
          })),
        );
      })
      .catch(() => undefined);
    void transport
      .invoke<AgentRuntime>("agent_runtime_get")
      .then((result) => {
        if (cancelled) return;
        setRuntime(result);
        setError(null);
        setRuntimeChecked(true);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setRuntime(null);
        setRuntimeChecked(true);
        setError(err instanceof Error ? err.message : String(err));
      });
    return () => {
      cancelled = true;
    };
  }, [open, sessionId, viewKey, view]);

  const runtimeLoaded = runtimeChecked;
  const needsInstall = !!runtime?.needsInstall;
  const needsLogin = !!runtime?.needsLogin;
  const name = runtime?.displayName || "OpenCode";
  const canSend = runtimeLoaded && !!runtime?.ready && !needsInstall && !needsLogin;
  const showSetup = runtimeLoaded && (needsInstall || needsLogin);

  useEffect(() => {
    if (!open || !canSend) {
      if (!open || needsInstall || needsLogin) {
        setModels([]);
        setCurrentModelId(null);
        setModelStatus("idle");
        setModelError(null);
      }
      return;
    }
    let cancelled = false;
    setModelStatus("loading");
    setModelError(null);
    void getTransport()
      .invoke<AgentModels>("agent_models_get", { sessionId })
      .then((result) => {
        if (cancelled) return;
        const rows = (result.models ?? []).filter((item) => item.id);
        setModels(rows);
        setCurrentModelId(result.currentModelId ?? rows[0]?.id ?? null);
        setModelStatus(rows.length > 0 ? "idle" : "empty");
      })
      .catch((err) => {
        if (cancelled) return;
        setModels([]);
        setModelStatus("error");
        setModelError(err instanceof Error ? err.message : String(err));
      });
    return () => {
      cancelled = true;
    };
  }, [open, canSend, sessionId, needsInstall, needsLogin]);

  async function refreshRuntime() {
    try {
      const next = await getTransport().invoke<AgentRuntime>("agent_runtime_get");
      setRuntime(next);
      setError(null);
      setRuntimeChecked(true);
      return next;
    } catch (err) {
      setRuntimeChecked(true);
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    }
  }

  async function installCli() {
    setSetupBusy("install");
    setError(null);
    try {
      const next = await getTransport().invoke<AgentRuntime>("agent_runtime_install");
      setRuntime({ ...next, needsLogin: true, ready: false });
      sessionStorage.removeItem(LOGIN_ACK_KEY);
    } catch (err) {
      setError(String(err));
    } finally {
      setSetupBusy(null);
    }
  }

  async function loginCli() {
    setSetupBusy("login");
    setError(null);
    try {
      const next = await getTransport().invoke<AgentRuntime>("agent_runtime_login");
      setRuntime((prev) => ({
        ...(prev ?? {}),
        ...next,
        needsLogin: true,
        ready: false,
      }));
    } catch (err) {
      setError(String(err));
    } finally {
      setSetupBusy(null);
    }
  }

  function ackLogin() {
    sessionStorage.setItem(LOGIN_ACK_KEY, "1");
    void refreshRuntime();
  }

  function composerKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== "Enter" || event.shiftKey) return;
    if (event.nativeEvent.isComposing || event.keyCode === 229) return;
    event.preventDefault();
    void send();
  }

  async function changeModel(modelId: string) {
    setModelMenuOpen(false);
    if (!modelId || modelId === currentModelId) return;
    setModelBusy(true);
    setError(null);
    try {
      const result = await getTransport().invoke<AgentModels>("agent_set_model", {
        sessionId,
        modelId,
      });
      const rows = (result.models ?? []).filter((item) => item.id);
      if (rows.length) setModels(rows);
      setCurrentModelId(result.currentModelId ?? modelId);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setModelBusy(false);
    }
  }

  function startNewConversation() {
    if (taskId) {
      void getTransport()
        .invoke("agent_cancel_task", { taskId, sessionId })
        .catch(() => undefined);
    }
    const nextSessionId = crypto.randomUUID();
    setTaskId(null);
    setMessages([]);
    setError(null);
    setBusy(false);
    setText("");
    setThoughtText("");
    setToolSteps([]);
    setStreamingReply("");
    setActivityExpanded(true);
    setModelMenuOpen(false);
    setSessionId(nextSessionId);
    requestAnimationFrame(() => {
      composerRef.current?.focus();
    });
  }

  async function send() {
    const userText = text.trim();
    if (!userText) return;
    if (userText.toLowerCase() === "/new") {
      startNewConversation();
      return;
    }
    if (!canSend) return;
    setBusy(true);
    setError(null);
    setText("");
    setThoughtText("");
    setToolSteps([]);
    setStreamingReply("");
    setActivityExpanded(true);
    setMessages((prev) => [...prev, { role: "user", text: userText }]);
    try {
      const result = await getTransport().invoke<Record<string, unknown>>("agent_post_message", {
        sessionId,
        text: userText,
      });
      const status = String(result.status ?? "");
      const message = String(result.message ?? result.error ?? "");
      const nextTask = typeof result.taskId === "string" ? result.taskId : null;
      if (result.needsInstall || result.needsLogin) {
        setRuntime((prev) => ({
          ...(prev ?? {}),
          needsInstall: Boolean(result.needsInstall),
          needsLogin: Boolean(result.needsLogin),
          ready: false,
          loginCommand: typeof result.loginCommand === "string" ? result.loginCommand : prev?.loginCommand,
        }));
        if (result.needsLogin) {
          sessionStorage.removeItem(LOGIN_ACK_KEY);
        }
      }
      if (status === "cancelled") {
        setTaskId(null);
        setMessages((prev) => [
          ...prev,
          { role: "assistant", text: message || "任务已取消，不会标成成功。", taskId: nextTask },
        ]);
        return;
      }
      if (status === "failed") {
        setTaskId(null);
        setMessages((prev) => [
          ...prev,
          { role: "assistant", text: message || "对话未能执行。", taskId: nextTask },
        ]);
        return;
      }
      setTaskId(status === "running" ? nextTask : null);
      if (message) {
        setMessages((prev) => [
          ...prev,
          { role: "assistant", text: message, taskId: nextTask },
        ]);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    if (!open || !taskId) return;
    let cancelled = false;
    const tick = () => {
      void getTransport()
        .invoke<{ messages?: ChatMessage[] }>("agent_list_messages", { sessionId })
        .then((result) => {
          if (cancelled) return;
          const rows = result.messages ?? [];
          setMessages(
            rows.map((row) => ({
              role: row.role,
              text: row.text,
              taskId: row.taskId,
            })),
          );
          const forTask = rows.filter((row) => row.taskId === taskId);
          const finished = forTask.some(
            (row) =>
              row.role === "system" ||
              (row.role === "assistant" &&
                forTask.some((item) => item.role === "user")),
          );
          if (finished) setTaskId(null);
        })
        .catch(() => undefined);
    };
    tick();
    const timer = window.setInterval(tick, 800);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [open, sessionId, taskId]);

  async function cancel() {
    if (!taskId) return;
    const cancelledId = taskId;
    try {
      const result = await getTransport().invoke<Record<string, unknown>>("agent_cancel_task", {
        taskId: cancelledId,
        sessionId,
      });
      const message = String(result.message ?? "任务已取消，不会标成成功。");
      setMessages((prev) => [...prev, { role: "system", text: message, taskId: cancelledId }]);
    } catch (err) {
      setError(String(err));
    } finally {
      setTaskId(null);
    }
  }

  function applyQuickPrompt(prompt: string) {
    setText(prompt);
    requestAnimationFrame(() => {
      const el = composerRef.current;
      if (!el) return;
      el.focus();
      el.setSelectionRange(prompt.length, prompt.length);
    });
  }

  useEffect(() => {
    if (!modelMenuOpen) return;
    function handleOutside(event: MouseEvent) {
      if (modelWrapRef.current && !modelWrapRef.current.contains(event.target as Node)) {
        setModelMenuOpen(false);
      }
    }
    function handleEscape(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape") {
        setModelMenuOpen(false);
      }
    }
    document.addEventListener("mousedown", handleOutside);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handleOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [modelMenuOpen]);

  function toggleModelVisible(modelId: string) {
    setHiddenModelIds((prev) => {
      const next = prev.includes(modelId)
        ? prev.filter((id) => id !== modelId)
        : [...prev, modelId];
      try {
        localStorage.setItem(HIDDEN_MODELS_KEY, JSON.stringify(next));
      } catch {
        // 忽略存储受限场景
      }
      return next;
    });
  }

  function resetHiddenModels() {
    setHiddenModelIds([]);
    try {
      localStorage.removeItem(HIDDEN_MODELS_KEY);
    } catch {
      // 忽略存储受限场景
    }
  }

  useEffect(() => {
    if (!open || !bodyRef.current) return;
    bodyRef.current.scrollTop = bodyRef.current.scrollHeight;
  }, [open, messages.length, taskId]);

  const hiddenSet = useMemo(() => new Set(hiddenModelIds), [hiddenModelIds]);
  const visibleModels = useMemo(
    () => models.filter((item) => !hiddenSet.has(item.id)),
    [models, hiddenSet],
  );
  const selectAllRef = useRef<HTMLInputElement | null>(null);
  const filteredEditorModels = useMemo(() => {
    const q = modelQuery.trim().toLowerCase();
    return q
      ? models.filter((item) => `${item.name} ${item.id}`.toLowerCase().includes(q))
      : models;
  }, [models, modelQuery]);
  const allFilteredEnabled =
    filteredEditorModels.length > 0 &&
    filteredEditorModels.every((item) => !hiddenSet.has(item.id));
  const someFilteredEnabled =
    filteredEditorModels.some((item) => !hiddenSet.has(item.id)) && !allFilteredEnabled;

  useEffect(() => {
    if (selectAllRef.current) {
      selectAllRef.current.indeterminate = someFilteredEnabled;
    }
  }, [someFilteredEnabled, modelEditorOpen]);

  function toggleAllVisibleModels() {
    if (filteredEditorModels.length === 0) return;
    const targetIds = new Set(filteredEditorModels.map((item) => item.id));
    setHiddenModelIds((prev) => {
      const next = allFilteredEnabled
        ? [...new Set([...prev, ...targetIds])]
        : prev.filter((id) => !targetIds.has(id));
      try {
        localStorage.setItem(HIDDEN_MODELS_KEY, JSON.stringify(next));
      } catch {
        // 忽略存储受限场景
      }
      return next;
    });
  }

  const groupedEditorModels = useMemo(() => {
    const groups = new Map<string, AgentModel[]>();
    for (const item of filteredEditorModels) {
      const slash = item.id.indexOf("/");
      const group = slash > 0 ? item.id.slice(0, slash) : "其他";
      const list = groups.get(group);
      if (list) list.push(item);
      else groups.set(group, [item]);
    }
    return [...groups.entries()];
  }, [filteredEditorModels]);

  if (!open) return null;

  const loginCommand = runtime?.loginCommand || "opencode auth login";
  const activeModel =
    visibleModels.find((item) => item.id === currentModelId) ?? visibleModels[0] ?? null;

  return (
    <aside className="chat-panel flex h-full w-[380px] shrink-0 flex-col border-l border-line-soft">
      <div ref={bodyRef} className="chat-panel-body">
        {!runtimeLoaded && (
          <p className="text-[12px] text-muted">正在检查管理 CLI…</p>
        )}
        {showSetup && (
          <div className="chat-cli-setup">
            <h3>先接入管理 CLI</h3>
            <p>
              内置对话使用本机默认管理 CLI「{name}」。安装并完成登录后会钉死这一家，不再扫描本机其它命令行。
            </p>
            <ol>
              <li>安装 OpenCode 到本机 Host 目录（不打进 Skill Manager 安装包）。</li>
              <li>打开登录，在本机浏览器完成授权。密钥留在 OpenCode，不会写入 Host 或管理站。</li>
              <li>完成后即可发送。</li>
            </ol>
            <code>{needsInstall ? "host agent-cli --install" : loginCommand}</code>
            <div className="chat-cli-setup-actions">
              {needsInstall && (
                <Button
                  kind="primary"
                  compact
                  disabled={setupBusy !== null}
                  onClick={() => void installCli()}
                >
                  {setupBusy === "install" ? "正在安装…" : "安装 OpenCode"}
                </Button>
              )}
              {!needsInstall && needsLogin && (
                <>
                  <Button
                    kind="primary"
                    compact
                    disabled={setupBusy !== null}
                    onClick={() => void loginCli()}
                  >
                    {setupBusy === "login" ? "正在打开…" : "打开登录"}
                  </Button>
                  <Button compact disabled={setupBusy !== null} onClick={ackLogin}>
                    我已完成登录
                  </Button>
                </>
              )}
            </div>
          </div>
        )}
        {messages.map((message, index) => {
          if (message.role === "user") {
            return (
              <div key={`user-${index}`} className="chat-msg chat-msg-user">
                <div className="chat-msg-bubble">{message.text}</div>
              </div>
            );
          }
          if (message.role === "system") {
            return (
              <div key={`system-${index}`} className="chat-msg chat-msg-system">
                <div className="chat-msg-system-pill">{message.text}</div>
              </div>
            );
          }
          return (
            <div key={`assistant-${index}`} className="chat-msg chat-msg-assistant">
              <div className="chat-msg-head">
                <span className="chat-msg-avatar" aria-hidden="true">
                  <Bot size={11} strokeWidth={1.8} />
                </span>
                <span className="chat-msg-role">管理助手</span>
              </div>
              <div className="chat-msg-card">
                <MarkdownView text={message.text} className="chat-markdown" />
              </div>
            </div>
          );
        })}
        {error && <Notice tone="danger">{error}</Notice>}
        {messages.length === 0 && !showSetup && runtimeLoaded && (
          <div className="chat-empty">
            <div className="chat-empty-icon" aria-hidden="true">
              <Bot size={17} strokeWidth={1.7} />
            </div>
            <h3>有什么可以帮你管理？</h3>
            <p>只上报当前界面引用。全局清单与详情由 Host 组装。</p>
            {canSend && (
              <div className="chat-quick-prompts">
                {QUICK_PROMPTS.map((prompt) => (
                  <button
                    key={prompt}
                    type="button"
                    className="chat-quick-btn"
                    onClick={() => applyQuickPrompt(prompt)}
                  >
                    {prompt}
                  </button>
                ))}
              </div>
            )}
          </div>
        )}
        {(thoughtText || toolSteps.length > 0) && (
          <div className="chat-activity-card">
            <button
              type="button"
              className="chat-activity-toggle"
              aria-expanded={activityExpanded}
              onClick={() => setActivityExpanded((prev) => !prev)}
            >
              <span>
                思考与执行过程
                {toolSteps.length > 0 ? `（${toolSteps.length} 步）` : ""}
              </span>
              <ChevronDown
                size={13}
                strokeWidth={1.8}
                style={{ transform: activityExpanded ? "rotate(180deg)" : undefined }}
              />
            </button>
            {activityExpanded && (
              <div className="chat-activity-body">
                {toolSteps.length > 0 && (
                  <div className="chat-activity-tools">
                    {toolSteps.map((step) => {
                      const badge = toolStatusLabel(step.status);
                      return (
                        <div key={step.id} className="chat-activity-tool">
                          <span className="chat-activity-tool-name">{step.title}</span>
                          <span className={`chat-activity-tool-status ${badge.tone}`}>
                            {badge.text}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                )}
                {thoughtText && <pre className="chat-activity-thought">{thoughtText}</pre>}
              </div>
            )}
          </div>
        )}
        {taskId && streamingReply && (
          <div className="chat-msg chat-msg-assistant">
            <div className="chat-msg-head">
              <span className="chat-msg-avatar" aria-hidden="true">
                <Bot size={11} strokeWidth={1.8} />
              </span>
              <span className="chat-msg-role">管理助手</span>
            </div>
            <div className="chat-msg-card">
              <MarkdownView text={streamingReply} className="chat-markdown" />
            </div>
          </div>
        )}
        {taskId && (
          <div className="chat-running-bar">
            <span className="chat-running-state">
              <span className="chat-pulse-dot" aria-hidden="true" />
              <span>正在处理请求…</span>
            </span>
            <Button compact onClick={() => void cancel()}>
              取消进行中任务
            </Button>
          </div>
        )}
      </div>

      <div className="chat-panel-foot">
        {canSend && /^\/[a-z]*$/i.test(text.trim()) && "/new".startsWith(text.trim().toLowerCase()) && (
          <div className="chat-slash-menu" role="listbox" aria-label="快捷命令">
            <button
              type="button"
              role="option"
              aria-selected={text.trim().toLowerCase() === "/new"}
              className="chat-slash-item"
              onClick={startNewConversation}
            >
              <span className="chat-slash-cmd">/new</span>
              <span className="chat-slash-desc">开始新的对话</span>
            </button>
          </div>
        )}
        <div className="chat-composer">
          <textarea
            ref={composerRef}
            placeholder="例如：完善中文注释 / 有哪些译文待更新"
            value={text}
            disabled={!canSend}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={composerKeyDown}
          />
          <div className="chat-composer-foot">
            <div className="chat-composer-tools">
              {canSend && (
                <div ref={modelWrapRef} className={`chat-model ${modelMenuOpen ? "open" : ""}`}>
                  {models.length > 0 ? (
                    <>
                      {modelMenuOpen && (
                        <div className="chat-model-menu" role="listbox" aria-label="模型列表">
                          <div className="chat-model-menu-title">模型</div>
                          <div className="chat-model-menu-list">
                            {visibleModels.length > 0 ? (
                              visibleModels.map((model) => {
                                const selected = activeModel?.id === model.id;
                                return (
                                  <button
                                    key={model.id}
                                    type="button"
                                    role="option"
                                    aria-selected={selected}
                                    className={`chat-model-option ${selected ? "on" : ""}`}
                                    onClick={() => void changeModel(model.id)}
                                  >
                                    <span className="chat-model-option-name">{model.name || model.id}</span>
                                    {selected && (
                                      <Check size={14} strokeWidth={2} className="chat-model-option-check" />
                                    )}
                                  </button>
                                );
                              })
                            ) : (
                              <div className="chat-model-empty">已隐藏全部模型</div>
                            )}
                          </div>
                          <div className="chat-model-menu-foot">
                            <button
                              type="button"
                              className="chat-model-edit-btn"
                              onClick={() => {
                                setModelMenuOpen(false);
                                setModelQuery("");
                                setModelEditorOpen(true);
                              }}
                            >
                              <Settings size={13} strokeWidth={1.8} />
                              <span>编辑模型…</span>
                            </button>
                          </div>
                        </div>
                      )}
                      <button
                        type="button"
                        role="combobox"
                        className="chat-model-trigger"
                        disabled={busy || modelBusy}
                        aria-label="当前模型"
                        aria-haspopup="listbox"
                        aria-expanded={modelMenuOpen}
                        onClick={() => setModelMenuOpen((prev) => !prev)}
                      >
                        <span>{activeModel ? activeModel.name || activeModel.id : "选择模型"}</span>
                        <ChevronDown size={13} strokeWidth={1.8} />
                      </button>
                    </>
                  ) : (
                    <span className="chat-model-status" aria-label="当前模型">
                      {modelStatus === "error"
                        ? modelError || "无法列出"
                        : modelStatus === "empty"
                          ? "暂无列表"
                          : "正在获取…"}
                    </span>
                  )}
                </div>
              )}
            </div>
            <button
              type="button"
              className={`${buttonClass("primary", true)} chat-send-btn`}
              disabled={busy || !canSend || !text.trim()}
              onClick={() => void send()}
            >
              <SendHorizontal size={12} strokeWidth={1.8} />
              <span>{busy ? "发送中…" : "发送"}</span>
            </button>
          </div>
        </div>
      </div>

      {modelEditorOpen && (
        <Dialog
          title="编辑模型"
          subtitle="开启的模型会显示在模型列表中。"
          width="w-[420px]"
          onClose={() => setModelEditorOpen(false)}
          footer={
            <>
              <Button compact onClick={resetHiddenModels} disabled={hiddenModelIds.length === 0}>
                恢复默认
              </Button>
              <Button kind="primary" compact onClick={() => setModelEditorOpen(false)}>
                完成
              </Button>
            </>
          }
        >
          <label className="chat-model-editor-search">
            <Search size={14} strokeWidth={1.8} />
            <input
              type="text"
              placeholder="搜索模型"
              value={modelQuery}
              onChange={(event) => setModelQuery(event.target.value)}
            />
          </label>
          {filteredEditorModels.length > 0 && (
            <div className="chat-model-editor-selectall">
              <label>
                <input
                  ref={selectAllRef}
                  type="checkbox"
                  checked={allFilteredEnabled}
                  onChange={toggleAllVisibleModels}
                />
                <span>全选</span>
              </label>
              <span>
                已启用 {filteredEditorModels.filter((item) => !hiddenSet.has(item.id)).length} /{" "}
                {filteredEditorModels.length}
              </span>
            </div>
          )}
          <div className="chat-model-editor-list">
            {groupedEditorModels.length > 0 ? (
              groupedEditorModels.map(([group, rows]) => (
                <div key={group}>
                  <div className="chat-model-editor-group">{group}</div>
                  {rows.map((model) => {
                    const enabled = !hiddenSet.has(model.id);
                    const label = model.name || model.id;
                    return (
                      <div key={model.id} className="chat-model-editor-row">
                        <span className="chat-model-editor-name" title={model.id}>
                          {label}
                        </span>
                        <button
                          type="button"
                          role="switch"
                          aria-checked={enabled}
                          aria-label={`启用 ${label}`}
                          className={`chat-model-switch ${enabled ? "on" : ""}`}
                          onClick={() => toggleModelVisible(model.id)}
                        >
                          <span />
                        </button>
                      </div>
                    );
                  })}
                </div>
              ))
            ) : (
              <div className="chat-model-empty">没有匹配的模型</div>
            )}
          </div>
        </Dialog>
      )}
    </aside>
  );
}
