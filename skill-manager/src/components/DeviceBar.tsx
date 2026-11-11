import { useCallback, useEffect, useState } from "react";

import { Button, Notice } from "./ui";

export interface StationDevice {
  deviceId: string;
  displayName: string;
  online: boolean;
  revoked: boolean;
}

export interface DeviceBarProps {
  currentId: string | null;
  onChange: (deviceId: string) => void;
}

interface PairingCode {
  code: string;
  expiresAt: string;
}

async function readJson(response: Response): Promise<Record<string, unknown>> {
  return (await response.json().catch(() => ({}))) as Record<string, unknown>;
}

/** 仅管理站远程壳显示。桌面不得渲染。 */
export function DeviceBar({ currentId, onChange }: DeviceBarProps) {
  const [devices, setDevices] = useState<StationDevice[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [pairing, setPairing] = useState<PairingCode | null>(null);
  const [busy, setBusy] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draftName, setDraftName] = useState("");

  const reload = useCallback(async () => {
    const res = await fetch("/devices", { credentials: "include" });
    const body = await readJson(res);
    if (!res.ok) throw new Error(String(body.error ?? "无法读取设备列表"));
    const list = ((body.devices as unknown[]) ?? body ?? []) as Record<string, unknown>[];
    const mapped = (Array.isArray(list) ? list : []).map((row) => ({
      deviceId: String(row.deviceId ?? row.device_id ?? ""),
      displayName: String(row.displayName ?? row.display_name ?? row.deviceId ?? row.device_id ?? ""),
      online: Boolean(row.online),
      revoked: Boolean(row.revoked),
    }));
    setDevices(mapped.filter((device) => device.deviceId));
  }, []);

  useEffect(() => {
    let cancelled = false;
    void reload().catch((err) => {
      if (!cancelled) setError(String(err));
    });
    return () => {
      cancelled = true;
    };
  }, [currentId, reload]);

  const current = devices.find((d) => d.deviceId === currentId);

  async function issueCode() {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/devices/pairing-codes", { method: "POST", credentials: "include" });
      const body = await readJson(res);
      if (!res.ok) throw new Error(String(body.error ?? "无法生成对接码"));
      setPairing({
        code: String(body.code ?? ""),
        expiresAt: String(body.expiresAt ?? body.expires_at ?? ""),
      });
      await reload();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function rename(id: string, name: string) {
    const displayName = name.trim();
    if (!displayName) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/devices/${encodeURIComponent(id)}`, {
        method: "PATCH",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ displayName }),
      });
      const body = await readJson(res);
      if (!res.ok) throw new Error(String(body.error ?? "无法改名"));
      setEditingId(null);
      await reload();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function revoke(id: string) {
    if (!window.confirm("解除对接后该设备须重新用对接码接入。确定？")) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/devices/${encodeURIComponent(id)}/revoke`, {
        method: "POST",
        credentials: "include",
      });
      const body = await readJson(res);
      if (!res.ok) throw new Error(String(body.error ?? "无法解除对接"));
      await reload();
      if (currentId === id) onChange("");
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="device-bar mx-2 mb-2 rounded-md border border-line-soft bg-panel px-2 py-1.5">
      <div className="device-bar-row flex items-center gap-2">
        <span className="text-[11px] text-muted">当前设备</span>
        <select
          className="h-[28px] min-w-0 flex-1 rounded-md border border-line bg-panel px-1.5 text-[12px]"
          value={currentId ?? ""}
          onChange={(e) => onChange(e.target.value)}
        >
          <option value="">选择设备</option>
          {devices.map((device) => (
            <option key={device.deviceId} value={device.deviceId} disabled={device.revoked || !device.online}>
              {device.displayName}
              {device.online ? "" : "（离线）"}
            </option>
          ))}
        </select>
      </div>
      <div className="device-bar-actions mt-1.5 flex flex-wrap gap-1.5">
        <Button compact disabled={busy} onClick={() => void issueCode()}>
          生成对接码
        </Button>
      </div>
      {pairing?.code && (
        <p className="device-pair-code mt-1.5 text-[11px] text-ink-2">
          对接码 <code className="rounded bg-app px-1">{pairing.code}</code>
          {pairing.expiresAt ? `（${pairing.expiresAt} 前有效）` : "（10 分钟内有效）"}
        </p>
      )}
      {devices.length > 0 && (
        <ul className="device-list mt-1.5 space-y-1">
          {devices.map((device) => (
            <li key={device.deviceId} className="device-item flex items-center gap-1.5 text-[11px]">
              {editingId === device.deviceId ? (
                <>
                  <input
                    className="h-6 min-w-0 flex-1 rounded border border-line bg-panel px-1"
                    value={draftName}
                    onChange={(e) => setDraftName(e.target.value)}
                    aria-label="设备名称"
                  />
                  <button type="button" className="text-accent" disabled={busy} onClick={() => void rename(device.deviceId, draftName)}>
                    保存
                  </button>
                  <button type="button" className="text-muted" onClick={() => setEditingId(null)}>
                    取消
                  </button>
                </>
              ) : (
                <>
                  <span className="min-w-0 flex-1 truncate">
                    {device.displayName}
                    {device.online ? "" : "（离线）"}
                    {device.revoked ? "（已解除）" : ""}
                  </span>
                  <button
                    type="button"
                    className="text-muted"
                    disabled={busy}
                    onClick={() => {
                      setEditingId(device.deviceId);
                      setDraftName(device.displayName);
                    }}
                  >
                    改名
                  </button>
                  <button
                    type="button"
                    className="text-danger"
                    disabled={busy || device.revoked}
                    onClick={() => void revoke(device.deviceId)}
                  >
                    解除对接
                  </button>
                </>
              )}
            </li>
          ))}
        </ul>
      )}
      {current && !current.online && (
        <div className="mt-1.5">
          <Notice tone="warn">该设备离线，不能操作，也不会用缓存技能顶上。</Notice>
        </div>
      )}
      {!currentId && devices.length > 0 && (
        <p className="mt-1 text-[11px] text-muted">先选择一台在线设备，再管理它的技能库。</p>
      )}
      {error && <p className="mt-1 text-[11px] text-danger">{error}</p>}
    </div>
  );
}
