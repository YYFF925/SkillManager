/**
 * 业务入口：桌面走 Tauri invoke，管理站网页走 HTTP。
 * 命令名与现有 Tauri command 一致。
 */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { confirmHostPlan, isHostPlan, type HostPlan } from "./planConfirm";

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
  if (isDesktopShell()) {
    const wrapped = await invoke<HostInvokeResult>("host_invoke", {
      command: "__session__",
      args: {},
    });
    const id =
      wrapped && typeof wrapped === "object" && wrapped.data && typeof wrapped.data === "object"
        ? String((wrapped.data as { sessionId?: string }).sessionId ?? crypto.randomUUID())
        : crypto.randomUUID();
    sessionStorage.setItem(SESSION_KEY, id);
    return id;
  }
  const response = await fetch("/current/session", { method: "POST", credentials: "include" });
  const body = (await response.json().catch(() => ({}))) as { sessionId?: string };
  const id = body.sessionId ?? crypto.randomUUID();
  sessionStorage.setItem(SESSION_KEY, id);
  return id;
}

export class NeedsConfirmationError extends Error {
  plan: HostPlan;
  constructor(plan: HostPlan) {
    super("需要确认后才能继续");
    this.name = "NeedsConfirmationError";
    this.plan = plan;
  }
}

interface HostInvokeResult {
  ok?: boolean;
  status?: number;
  data?: unknown;
}

function errorMessage(body: unknown, fallback: string): string {
  if (body && typeof body === "object" && "error" in body) {
    return String((body as { error: unknown }).error);
  }
  return fallback;
}

async function handlePlan<T>(
  plan: HostPlan,
  retry: () => Promise<T>,
  confirm: (planId: string) => Promise<void>,
): Promise<T> {
  const ok = await confirmHostPlan(plan);
  if (!ok) {
    throw new NeedsConfirmationError(plan);
  }
  await confirm(plan.planId);
  return retry();
}

class TauriTransport implements Transport {
  async invoke<T>(command: string, args: InvokeArgs = {}): Promise<T> {
    const sessionId = await hostSession();
    const wrapped = await invoke<HostInvokeResult>("host_invoke", {
      command,
      args,
      sessionId,
    });
    const status = wrapped?.status ?? (wrapped?.ok === false ? 400 : 200);
    const data = (wrapped && "data" in wrapped ? wrapped.data : wrapped) as T;
    if (status === 409 && isHostPlan(data)) {
      return handlePlan(
        data,
        () => this.invoke<T>(command, { ...args, planId: data.planId, sessionId }),
        (planId) => this.confirmPlan(planId),
      );
    }
    if (wrapped && wrapped.ok === false) {
      throw new Error(errorMessage(data, `请求失败（${status}）`));
    }
    return data;
  }

  async subscribe(event: string, handler: (payload: unknown) => void): Promise<() => void> {
    return listen(event, (e) => handler(e.payload));
  }

  async confirmPlan(planId: string): Promise<void> {
    const sessionId = await hostSession();
    const wrapped = await invoke<HostInvokeResult>("host_invoke", {
      command: "plans_confirm",
      args: { planId, sessionId },
      sessionId,
    });
    if (wrapped && wrapped.ok === false) {
      throw new Error(errorMessage(wrapped.data, "确认计划失败"));
    }
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
    if (response.status === 409 && isHostPlan(body)) {
      return handlePlan(
        body,
        () => this.invoke<T>(command, { ...args, planId: body.planId, sessionId }),
        (planId) => this.confirmPlan(planId),
      );
    }
    if (!response.ok) {
      throw new Error(errorMessage(body, `请求失败（${response.status}）`));
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
      throw new Error(errorMessage(body, "确认计划失败"));
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

export function resetTransportForTests(): void {
  cached = null;
}
