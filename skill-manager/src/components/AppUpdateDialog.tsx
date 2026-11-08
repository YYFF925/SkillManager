import { useState } from "react";

import type { AppUpdate } from "../lib/appUpdate";
import { Button, Dialog, Notice } from "./ui";

/** 发现应用新版本后的确认与安装进度。 */
export function AppUpdateDialog({
  update,
  onDismiss,
}: {
  update: AppUpdate;
  onDismiss: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<string | null>(null);

  async function install() {
    setBusy(true);
    setError(null);
    setProgress("正在下载…");
    try {
      await update.install((percent) => {
        setProgress(percent == null ? "正在下载…" : `正在下载 ${percent}%`);
      });
    } catch (err) {
      setError(String(err));
      setBusy(false);
      setProgress(null);
    }
  }

  return (
    <Dialog
      title="发现新版本"
      subtitle={`当前 ${update.currentVersion} → ${update.version}`}
      width="w-[480px]"
      onClose={busy ? () => {} : onDismiss}
      footer={
        <>
          <span className="text-[12px] text-faint">{progress}</span>
          <div className="flex gap-2.5">
            <Button onClick={onDismiss} disabled={busy}>
              稍后
            </Button>
            <Button kind="primary" onClick={() => void install()} disabled={busy}>
              {busy ? "正在更新…" : "立即更新"}
            </Button>
          </div>
        </>
      }
    >
      {error && (
        <div className="mb-3">
          <Notice tone="danger">{error}</Notice>
        </div>
      )}
      <p className="text-[13px] leading-[21px] text-ink-3 whitespace-pre-wrap">
        {update.notes || "有新版本可用。"}
      </p>
    </Dialog>
  );
}
