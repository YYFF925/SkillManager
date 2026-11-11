import { useState } from "react";

import { Button, Notice } from "../components/ui";

export interface LoginPageProps {
  onLoggedIn: (deviceId?: string | null) => void;
}

interface RebindPayload {
  totpSecret: string;
  otpauth: string;
  qr: string;
  recoveryCodes: string[];
}

/** 管理站登录：未登录不加载技能数据。恢复码登录后须重新绑定 TOTP。 */
export function LoginPage({ onLoggedIn }: LoginPageProps) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [otp, setOtp] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [rebind, setRebind] = useState<RebindPayload | null>(null);
  const [rebindOtp, setRebindOtp] = useState("");

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
      if (body && typeof body === "object" && (body as { mustRebindTotp?: boolean }).mustRebindTotp) {
        const payload = body as Record<string, unknown>;
        setRebind({
          totpSecret: String(payload.totpSecret ?? ""),
          otpauth: String(payload.otpauth ?? ""),
          qr: String(payload.qr ?? ""),
          recoveryCodes: Array.isArray(payload.recoveryCodes)
            ? payload.recoveryCodes.map((item) => String(item))
            : [],
        });
        setOtp("");
        return;
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

  async function confirmRebind() {
    setBusy(true);
    setError(null);
    try {
      const response = await fetch("/auth/totp/rebind", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ otp: rebindOtp }),
      });
      const body = await response.json().catch(() => ({}));
      if (!response.ok) {
        throw new Error(
          typeof body === "object" && body && "error" in body
            ? String((body as { error: unknown }).error)
            : "绑定失败",
        );
      }
      onLoggedIn(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  if (rebind) {
    return (
      <div className="login-screen grid h-full place-items-center bg-app px-6">
        <div className="login-card w-[420px] rounded-xl border border-line-soft bg-panel p-7">
          <h1 className="text-[17px] font-semibold">重新绑定一次性密码</h1>
          <p className="mt-1 text-[12px] text-muted">
            恢复码已使用。请把新密钥加入验证器，并用新的一次性密码完成绑定。
          </p>
          {rebind.qr && (
            <pre className="mt-3 overflow-auto text-[10px] leading-none text-ink-2" aria-label="TOTP 二维码">
              {rebind.qr}
            </pre>
          )}
          <p className="mt-3 text-[12px] text-muted">
            密钥 <code className="rounded bg-app px-1 text-[12px] text-ink-2">{rebind.totpSecret}</code>
          </p>
          {rebind.recoveryCodes.length > 0 && (
            <div className="mt-3">
              <p className="text-[12px] text-muted">新的恢复码（只显示一次）</p>
              <ul className="mt-1 list-disc pl-4 text-[12px] text-ink-2">
                {rebind.recoveryCodes.map((code) => (
                  <li key={code}>
                    <code>{code}</code>
                  </li>
                ))}
              </ul>
            </div>
          )}
          <label className="mt-4 block text-[12px] text-muted">
            新的一次性密码
            <input
              className="mt-1.5 h-10 w-full rounded-lg border border-line bg-panel px-3 text-[13px]"
              value={rebindOtp}
              onChange={(e) => setRebindOtp(e.target.value)}
              autoComplete="one-time-code"
            />
          </label>
          <div className="mt-5">
            <Button kind="primary" disabled={busy || !rebindOtp} onClick={() => void confirmRebind()}>
              {busy ? "正在绑定…" : "完成绑定"}
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
