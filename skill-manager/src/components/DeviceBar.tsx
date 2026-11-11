import { useEffect, useState } from "react";

import { Notice } from "./ui";

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

/** 仅管理站远程壳显示。桌面不得渲染。 */
export function DeviceBar({ currentId, onChange }: DeviceBarProps) {
  const [devices, setDevices] = useState<StationDevice[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void fetch("/devices", { credentials: "include" })
      .then(async (res) => {
        const body = await res.json();
        if (!res.ok) throw new Error(body?.error ?? "无法读取设备列表");
        const list = (body.devices ?? body ?? []).map((row: Record<string, unknown>) => ({
          deviceId: String(row.deviceId ?? row.device_id ?? ""),
          displayName: String(row.displayName ?? row.display_name ?? row.deviceId ?? row.device_id ?? ""),
          online: Boolean(row.online),
          revoked: Boolean(row.revoked),
        }));
        if (!cancelled) setDevices(list.filter((device: StationDevice) => device.deviceId));
      })
      .catch((err) => {
        if (!cancelled) setError(String(err));
      });
    return () => {
      cancelled = true;
    };
  }, [currentId]);

  const current = devices.find((d) => d.deviceId === currentId);

  return (
    <div className="device-bar mx-2 mb-2 rounded-md border border-line-soft bg-panel px-2 py-1.5">
      <div className="flex items-center gap-2">
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
      {current && !current.online && (
        <div className="mt-1.5">
          <Notice tone="warn">该设备离线，不能操作，也不会用缓存技能顶上。</Notice>
        </div>
      )}
      {!currentId && devices.length > 0 && (
        <p className="mt-1 text-[11px] text-muted">先选择一台在线设备，再管理它的技能库。</p>
      )}
      {error && (
        <p className="mt-1 text-[11px] text-danger">{error}</p>
      )}
    </div>
  );
}
