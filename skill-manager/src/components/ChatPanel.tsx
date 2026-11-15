import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { Bot, SendHorizontal, X } from "lucide-react";

import { Button, Notice, buttonClass } from "./ui";
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

const LOGIN_ACK_KEY = "sm-cli-login-ack";

function viewLabel(view: ChatViewRef): string {
  return view.selection?.ref || view.selection?.refs?.[0] || view.page || "当前界面";
}

const QUICK_PROMPTS = ["检查上游更新", "有哪些译文待更新", "查看当前部署状态"];

/** 对话只上报页面与选中引用，正文由 Host 组装。 */
export function ChatPanel({ open, view, onClose }: ChatPanelProps) {
  const [sessionId] = useState(() => sessionStorage.getItem("sm-agent-session") ?? crypto.randomUUID());
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

  useEffect(() => {
    sessionStorage.setItem("sm-agent-session", sessionId);
  }, [sessionId]);

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

  async function send() {
    if (!text.trim() || !canSend) return;
    setBusy(true);
    setError(null);
    const userText = text.trim();
    setText("");
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
    if (!open || !bodyRef.current) return;
    bodyRef.current.scrollTop = bodyRef.current.scrollHeight;
  }, [open, messages.length, taskId]);

  if (!open) return null;

  const loginCommand = runtime?.loginCommand || "opencode auth login";
  const focusLabel = viewLabel(view);

  return (
    <aside className="chat-panel flex h-full w-[380px] shrink-0 flex-col border-l border-line-soft">
      <div className="chat-panel-head">
        <div className="chat-panel-title">
          <span className="chat-panel-badge" aria-hidden="true">
            <Bot size={14} strokeWidth={1.7} />
          </span>
          <div className="chat-panel-title-text">
            <h2>管理对话</h2>
            <span className="chat-context-pill" title={focusLabel}>
              {focusLabel}
            </span>
          </div>
        </div>
        <div className="chat-panel-head-actions">
          <button
            type="button"
            className="chat-close-btn"
            aria-label="关闭"
            title="关闭"
            onClick={onClose}
          >
            <X size={14} strokeWidth={1.8} />
          </button>
        </div>
      </div>

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
                <label className="chat-model">
                  {models.length > 0 ? (
                    <select
                      value={currentModelId ?? models[0]?.id ?? ""}
                      disabled={busy || modelBusy}
                      aria-label="当前模型"
                      onChange={(event) => void changeModel(event.target.value)}
                    >
                      {models.map((model) => (
                        <option key={model.id} value={model.id}>
                          {model.name || model.id}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <span className="chat-model-status" aria-label="当前模型">
                      {modelStatus === "error"
                        ? modelError || "无法列出"
                        : modelStatus === "empty"
                          ? "暂无列表"
                          : "正在获取…"}
                    </span>
                  )}
                </label>
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
    </aside>
  );
}
