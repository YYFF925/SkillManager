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

function viewLabel(view: ChatViewRef): string {
  return view.selection?.ref || view.selection?.refs?.[0] || view.page || "当前界面";
}

/** 对话只上报页面与选中引用，正文由 Host 组装。 */
export function ChatPanel({ open, view, onClose }: ChatPanelProps) {
  const [sessionId] = useState(() => sessionStorage.getItem("sm-agent-session") ?? crypto.randomUUID());
  const [text, setText] = useState("");
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [taskId, setTaskId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

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
    // viewKey 用于稳定依赖，避免每次渲染都重报同一引用。
  }, [open, sessionId, viewKey, view]);

  async function send() {
    if (!text.trim()) return;
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
        setError(message || "对话未能执行");
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
        {messages.map((message, index) => (
          <div key={`${message.role}-${index}`} className="mb-2">
            <div className="text-[11px] text-faint">{message.role}</div>
            <p className="whitespace-pre-wrap text-[13px] text-ink-2">{message.text}</p>
          </div>
        ))}
        {error && <Notice tone="danger">{error}</Notice>}
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
          onChange={(e) => setText(e.target.value)}
        />
        <div className="mt-2">
          <Button kind="primary" compact disabled={busy || !text.trim()} onClick={() => void send()}>
            {busy ? "发送中…" : "发送"}
          </Button>
        </div>
      </div>
    </aside>
  );
}
