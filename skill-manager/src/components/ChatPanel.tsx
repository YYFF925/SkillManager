import { useEffect, useState } from "react";

import { Button, Notice } from "./ui";
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

const LOGIN_ACK_KEY = "sm-cli-login-ack";

function viewLabel(view: ChatViewRef): string {
  return view.selection?.ref || view.selection?.refs?.[0] || view.page || "当前界面";
}

function readLoginAck(): boolean {
  return sessionStorage.getItem(LOGIN_ACK_KEY) === "1";
}

/** 对话只上报页面与选中引用，正文由 Host 组装。 */
export function ChatPanel({ open, view, onClose }: ChatPanelProps) {
  const [sessionId] = useState(() => sessionStorage.getItem("sm-agent-session") ?? crypto.randomUUID());
  const [text, setText] = useState("");
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [taskId, setTaskId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [runtime, setRuntime] = useState<AgentRuntime | null>(null);
  const [setupBusy, setSetupBusy] = useState<"install" | "login" | null>(null);
  const [loginAck, setLoginAck] = useState(readLoginAck);

  useEffect(() => {
    sessionStorage.setItem("sm-agent-session", sessionId);
  }, [sessionId]);

  const viewKey = JSON.stringify(view);

  useEffect(() => {
    if (!open) return;
    const transport = getTransport();
    void transport.invoke("agent_set_context", { sessionId, view }).catch(() => undefined);
    void transport
      .invoke<{ messages?: Array<{ role: string; text: string; taskId?: string | null }> }>(
        "agent_list_messages",
        { sessionId },
      )
      .then((result) => {
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
      .then((result) => setRuntime(result))
      .catch(() => undefined);
  }, [open, sessionId, viewKey, view]);

  const runtimeLoaded = runtime !== null;
  const needsInstall = !!runtime?.needsInstall;
  const needsLogin = !!runtime?.needsLogin;
  const installed = !!runtime?.installed || !!runtime?.binaryPath;
  const name = runtime?.displayName || "OpenCode";
  const mustAckLogin = installed && !runtime?.userSelected && runtime?.installedByApp && !loginAck;
  const canSend = runtimeLoaded && !!runtime?.ready && !needsInstall && !needsLogin && !mustAckLogin;
  const showSetup =
    runtimeLoaded && (needsInstall || needsLogin || mustAckLogin);

  async function refreshRuntime() {
    const next = await getTransport().invoke<AgentRuntime>("agent_runtime_get");
    setRuntime(next);
    return next;
  }

  async function installCli() {
    setSetupBusy("install");
    setError(null);
    try {
      const next = await getTransport().invoke<AgentRuntime>("agent_runtime_install");
      setRuntime({ ...next, needsLogin: true, ready: false });
      setLoginAck(false);
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
    setLoginAck(true);
    void refreshRuntime();
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
          setLoginAck(false);
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

  if (!open) return null;

  const loginCommand = runtime?.loginCommand || "opencode auth login";

  return (
    <aside className="chat-panel flex h-full w-[340px] shrink-0 flex-col border-l border-line-soft bg-panel">
      <div className="flex items-center justify-between border-b border-line-soft px-3 py-2">
        <div>
          <h2 className="text-[13px] font-semibold">管理对话</h2>
          <p className="text-[11px] text-muted">{viewLabel(view)}</p>
        </div>
        <button type="button" className="text-[12px] text-muted" onClick={onClose}>
          关闭
        </button>
      </div>
      <div className="min-h-0 flex-1 overflow-auto px-3 py-2">
        {!runtimeLoaded && (
          <p className="text-[12px] text-muted">正在检查管理 CLI…</p>
        )}
        {showSetup && (
          <div className="chat-cli-setup notice">
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
              {!needsInstall && (needsLogin || mustAckLogin) && (
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
        {messages.map((message, index) => (
          <div key={`${message.role}-${index}`} className="mb-2">
            <div className="text-[11px] text-faint">{message.role}</div>
            <p className="whitespace-pre-wrap text-[13px] text-ink-2">{message.text}</p>
          </div>
        ))}
        {error && <Notice tone="danger">{error}</Notice>}
        {messages.length === 0 && !showSetup && (
          <p className="text-[12px] text-muted">只上报当前界面引用。全局清单与详情由 Host 组装。</p>
        )}
      </div>
      {taskId && (
        <div className="border-t border-line-soft px-3 py-2">
          <Button compact onClick={() => void cancel()}>
            取消进行中任务
          </Button>
        </div>
      )}
      <div className="border-t border-line-soft p-3">
        <textarea
          className="h-[72px] w-full resize-none rounded-lg border border-line bg-app px-2 py-1.5 text-[13px]"
          placeholder="例如：完善中文注释 / 有哪些译文待更新"
          value={text}
          disabled={!canSend}
          onChange={(e) => setText(e.target.value)}
        />
        <div className="mt-2">
          <Button kind="primary" compact disabled={busy || !canSend || !text.trim()} onClick={() => void send()}>
            {busy ? "发送中…" : "发送"}
          </Button>
        </div>
      </div>
    </aside>
  );
}
