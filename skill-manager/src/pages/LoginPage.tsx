import { useState } from "react";

import { Button, Notice } from "../components/ui";

export interface LoginPageProps {
  onLoggedIn: (deviceId?: string | null) => void;
}

/** 管理站登录：未登录不加载技能数据。 */
export function LoginPage({ onLoggedIn }: LoginPageProps) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [otp, setOtp] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit() {
    setBusy(true);
    setError(null);
    try {
      const response = await fetch("/auth/login", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ username, password, otp }),
      });
      const body = await response.json().catch(() => ({}));
      if (!response.ok) {
        throw new Error(
          typeof body === "object" && body && "error" in body
            ? String((body as { error: unknown }).error)
            : "登录失败",
        );
      }
      const deviceId =
        typeof body === "object" && body && "deviceId" in body
          ? String((body as { deviceId: unknown }).deviceId ?? "")
          : "";
      onLoggedIn(deviceId || null);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="login-screen grid h-full place-items-center bg-app px-6">
      <div className="login-card w-[420px] rounded-xl border border-line-soft bg-panel p-7">
        <div className="mb-4 flex items-center gap-2.5">
          <img src="/assets/app-logo.png" alt="" className="size-7" />
          <span className="text-[17px] font-semibold">Skill Manager</span>
        </div>
        <h1 className="text-[17px] font-semibold">登录管理站</h1>
        <p className="mt-1 text-[12px] text-muted">
          一次登录后切换设备。未登录不会加载任何技能数据。
        </p>
        <label className="mt-5 block text-[12px] text-muted">
          用户名
          <input
            className="mt-1.5 h-10 w-full rounded-lg border border-line bg-panel px-3 text-[13px]"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            autoComplete="username"
          />
        </label>
        <label className="mt-3 block text-[12px] text-muted">
          密码
          <input
            type="password"
            className="mt-1.5 h-10 w-full rounded-lg border border-line bg-panel px-3 text-[13px]"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="current-password"
          />
        </label>
        <label className="mt-3 block text-[12px] text-muted">
          一次性密码或恢复码
          <input
            className="mt-1.5 h-10 w-full rounded-lg border border-line bg-panel px-3 text-[13px]"
            value={otp}
            onChange={(e) => setOtp(e.target.value)}
            autoComplete="one-time-code"
          />
        </label>
        <div className="mt-5">
          <Button kind="primary" disabled={busy || !username || !password || !otp} onClick={() => void submit()}>
            {busy ? "正在登录…" : "登录"}
          </Button>
        </div>
        {error && (
          <div className="mt-4">
            <Notice tone="danger">{error}</Notice>
          </div>
        )}
      </div>
    </div>
  );
}
