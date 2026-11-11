/**
 * 业务入口：桌面走 Tauri invoke，管理站网页走 HTTP。
 * 命令名与现有 Tauri command 一致。
 */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type InvokeArgs = Record<string, unknown>;

export interface Transport {
  invoke<T>(command: string, args?: InvokeArgs): Promise<T>;
  subscribe(event: string, handler: (payload: unknown) => void): Promise<() => void>;
  confirmPlan?(planId: string): Promise<void>;
}

export function isDesktopShell(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

const SESSION_KEY = "sm-host-session";

async function hostSession(): Promise<string> {
  const existing = sessionStorage.getItem(SESSION_KEY);
  if (existing) return existing;
  const response = await fetch("/current/session", { method: "POST", credentials: "include" });
  const body = (await response.json().catch(() => ({}))) as { sessionId?: string };
  const id = body.sessionId ?? crypto.randomUUID();
  sessionStorage.setItem(SESSION_KEY, id);
  return id;
}

class TauriTransport implements Transport {
  invoke<T>(command: string, args: InvokeArgs = {}): Promise<T> {
    return invoke<T>(command, args);
  }
  async subscribe(event: string, handler: (payload: unknown) => void): Promise<() => void> {
    return listen(event, (e) => handler(e.payload));
  }
}

export class NeedsConfirmationError extends Error {
  plan: { planId: string; action?: string; preview?: unknown };
  constructor(plan: { planId: string; action?: string; preview?: unknown }) {
    super("需要确认后才能继续");
    this.name = "NeedsConfirmationError";
    this.plan = plan;
  }
}

class HttpTransport implements Transport {
  async invoke<T>(command: string, args: InvokeArgs = {}): Promise<T> {
    const sessionId = await hostSession();
    const response = await fetch(`/current/invoke/${command}`, {
      method: "POST",
      credentials: "include",
      headers: {
        "Content-Type": "application/json",
        "x-skill-manager-session": sessionId,
        "x-skill-manager-source": "gui",
      },
      body: JSON.stringify({ ...args, sessionId }),
    });
    const body = await response.json().catch(() => null);
    if (response.status === 409 && body && typeof body === "object" && "planId" in body) {
      const plan = body as { planId: string; action?: string; preview?: unknown };
      const ok = window.confirm(
        plan.action === "skill.delete"
          ? "确认删除这些技能？确认前不会改库。"
          : "确认覆盖本机已改动的部署副本？确认前不会改库。",
      );
      if (!ok) {
        throw new NeedsConfirmationError(plan);
      }
      await this.confirmPlan(plan.planId);
      return this.invoke<T>(command, { ...args, planId: plan.planId, sessionId });
    }
    if (!response.ok) {
      const message =
        body && typeof body === "object" && "error" in body
          ? String((body as { error: unknown }).error)
          : `请求失败（${response.status}）`;
      throw new Error(message);
    }
    return body as T;
  }

  async subscribe(event: string, handler: (payload: unknown) => void): Promise<() => void> {
    const source = new EventSource("/current/events", { withCredentials: true });
    const listener = (e: MessageEvent) => {
      try {
        handler(JSON.parse(e.data));
      } catch {
        handler(e.data);
      }
    };
    source.addEventListener(event, listener);
    source.onmessage = (e) => {
      if (!event) handler(e.data);
    };
    return () => {
      source.removeEventListener(event, listener);
      source.close();
    };
  }

  async confirmPlan(planId: string): Promise<void> {
    const sessionId = await hostSession();
    const response = await fetch(`/current/plans/${planId}/confirm`, {
      method: "POST",
      credentials: "include",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ sessionId }),
    });
    if (!response.ok) {
      const body = await response.json().catch(() => ({}));
      throw new Error(
        typeof body === "object" && body && "error" in body
          ? String((body as { error: unknown }).error)
          : "确认计划失败",
      );
    }
  }
}

let cached: Transport | null = null;

export function getTransport(): Transport {
  if (!cached) {
    cached = isDesktopShell() ? new TauriTransport() : new HttpTransport();
  }
  return cached;
}
