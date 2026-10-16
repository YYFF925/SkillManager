/**
 * 设置页（T055/T083/T087）：主题与字号、共同检查频率、自动上传确认、诊断与容量、库路径。
 *
 * 只放本期已接入的真实设置；主题/字号/频率取值由 core 校验并持久化，前端不复制业务判断。
 * 诊断与容量都来自 core 的真实结果，不由前端估算。
 */
import { useCallback, useEffect, useState } from "react";
import { Check, Pencil, X } from "lucide-react";

import { Segmented, Notice, Badge, Button } from "../components/ui";
import { SyncStatus } from "../components/SyncStatus";
import { DiagnosticsPanel } from "../components/DiagnosticsPanel";
import { CapacityReport } from "../components/CapacityReport";
import {
  setAutoCheckMinutes,
  setFontScale,
  setTheme,
  sharedDirGet,
  sharedDirSet,
  type FontScale,
  type SettingsView,
  type ThemeSetting,
  type VaultInfo,
} from "../services/client";

export interface SettingsPageProps {
  vault: VaultInfo;
  /** 最近一次上传失败的可读原因；无失败为 null。 */
  uploadFailure: string | null;
  /** 显式重试最近一次已同意的上传。 */
  onRetryUpload: () => Promise<void>;
  /** 本机设置（由 App 统一读取并应用外观）。 */
  settings: SettingsView | null;
  onSettingsChange: (settings: SettingsView) => void;
  reloadToken: number;
}

const FREQUENCY_OPTIONS = [
  { value: "0", label: "关闭" },
  { value: "30", label: "30 分钟" },
  { value: "60", label: "1 小时" },
  { value: "360", label: "6 小时" },
  { value: "1440", label: "24 小时" },
];

const THEME_OPTIONS: { value: ThemeSetting; label: string }[] = [
  { value: "light", label: "浅色" },
  { value: "dark", label: "深色" },
  { value: "system", label: "跟随系统" },
];

const FONT_OPTIONS: { value: FontScale; label: string }[] = [
  { value: "small", label: "小" },
  { value: "default", label: "默认" },
  { value: "large", label: "大" },
  { value: "xlarge", label: "特大" },
];

function Panel({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-xl border border-line-soft bg-panel p-5 shadow-card">
      <h2 className="text-[15px] font-semibold">{title}</h2>
      {description && <p className="mt-1 text-[12px] text-muted">{description}</p>}
      <div className="mt-3.5">{children}</div>
    </section>
  );
}

export function SettingsPage({
  vault,
  uploadFailure,
  onRetryUpload,
  settings,
  onSettingsChange,
  reloadToken,
}: SettingsPageProps) {
  const [sharedDir, setSharedDir] = useState<string | null>(null);
  const [editingShared, setEditingShared] = useState(false);
  const [sharedDraft, setSharedDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [syncToken, setSyncToken] = useState(0);

  useEffect(() => {
    void sharedDirGet()
      .then(setSharedDir)
      .catch(() => setSharedDir(null));
  }, [reloadToken]);

  const changeTheme = useCallback(
    async (value: string) => {
      setBusy(true);
      setError(null);
      try {
        const next = await setTheme(value as ThemeSetting);
        onSettingsChange(next);
        setNotice(
          value === "system"
            ? "主题已设为跟随系统；系统外观变化会实时反映。"
            : "主题已保存到本机设置。",
        );
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [onSettingsChange],
  );

  const changeFont = useCallback(
    async (value: string) => {
      setBusy(true);
      setError(null);
      try {
        const next = await setFontScale(value as FontScale);
        onSettingsChange(next);
        setNotice("字号已保存到本机设置。");
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [onSettingsChange],
  );

  const changeFrequency = useCallback(
    async (value: string) => {
      const minutes = Number(value);
      setBusy(true);
      setError(null);
      try {
        const next = await setAutoCheckMinutes(minutes);
        onSettingsChange(next);
        setNotice(
          minutes === 0
            ? "已关闭定时检查；仍可手动检查上游与同步库。"
            : `共同检查频率已设为 ${FREQUENCY_OPTIONS.find((o) => o.value === value)?.label ?? value}；上游检查、库同步与工具检查共用该周期。`,
        );
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [onSettingsChange],
  );

  const saveSharedDir = useCallback(
    async (path: string) => {
      setBusy(true);
      setError(null);
      try {
        const saved = await sharedDirSet(path.trim() === "" ? null : path);
        setSharedDir(saved);
        setNotice(saved === null ? "已清除共享目录路径。" : `共享目录已设为 ${saved}。`);
        setEditingShared(false);
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [],
  );

  async function retryUpload() {
    await onRetryUpload();
    setSyncToken((value) => value + 1);
  }

  return (
    <div className="flex flex-col gap-4">
      {error && <Notice tone="danger">{error}</Notice>}

      <Panel title="外观" description="主题默认跟随系统；字号作用于整个界面，均保存在本机。">
        <div className="flex flex-col gap-3">
          <div className="flex items-center gap-3">
            <span className="w-[52px] shrink-0 text-[12px] text-faint">主题</span>
            <Segmented
              options={THEME_OPTIONS}
              value={settings?.theme ?? "system"}
              onChange={(value) => void changeTheme(value)}
              className={busy ? "opacity-60" : ""}
            />
            <span className="text-[12px] text-faint">
              {settings?.theme === "system"
                ? "跟随系统：系统深色变化时界面实时切换。"
                : "已固定主题，不随系统变化。"}
            </span>
          </div>
          <div className="flex items-center gap-3">
            <span className="w-[52px] shrink-0 text-[12px] text-faint">字号</span>
            <Segmented
              options={FONT_OPTIONS}
              value={settings?.font_scale ?? "default"}
              onChange={(value) => void changeFont(value)}
              className={busy ? "opacity-60" : ""}
            />
          </div>
        </div>
      </Panel>

      <Panel
        title="共同检查频率"
        description="上游检查、库同步与工具版本检查三项共用同一周期；默认 1 小时。"
      >
        <Segmented
          options={FREQUENCY_OPTIONS}
          value={settings ? String(settings.auto_check_minutes) : "60"}
          onChange={(value) => void changeFrequency(value)}
          className={busy ? "opacity-60" : ""}
        />
        <p className="mt-2 text-[12px] text-faint">
          {settings?.auto_check_minutes === 0
            ? "当前已关闭定时检查。"
            : "到达周期后执行一次上游检查与库拉取；结果分别提示。普通关窗只收起到托盘，定时检查继续运行。"}
        </p>
      </Panel>

      <Panel
        title="自动上传确认"
        description="首次在界面保存库内修改时询问一次，确认后沿用该设置，不再逐次询问。"
      >
        <div className="flex items-center gap-2.5">
          {settings?.git_auto_push ? (
            <Badge tone="green">已确认</Badge>
          ) : (
            <Badge tone="gray">尚未确认</Badge>
          )}
          <span className="text-[13px] text-ink-3">
            {settings?.git_auto_push
              ? "界面保存后会自动提交并推送本机 SkillVault。"
              : "尚未确认；下次保存时询问一次是否以后自动上传。"}
          </span>
        </div>
        <p className="mt-2 text-[12px] text-faint">
          只影响界面保存这一条路径；外部文件改动、CLI 与同步已有部署不会触发上传。
        </p>
      </Panel>

      <Panel title="SkillVault 目录" description="所有技能、管理资料、指令源与预设的统一维护目录。">
        <p className="break-all font-mono text-[12px] text-ink-3">{vault.root}</p>
      </Panel>

      <Panel title="共享技能目录" description="共享目录是编码 Agent 的部署目标；留空表示清除。">
        {editingShared ? (
          <div className="flex items-center gap-2">
            <input
              autoFocus
              className="field-input h-9 flex-1 text-[12px]"
              value={sharedDraft}
              placeholder="绝对路径"
              onChange={(event) => setSharedDraft(event.target.value)}
            />
            <button
              type="button"
              title="保存"
              disabled={busy}
              onClick={() => void saveSharedDir(sharedDraft)}
              className="grid size-7 place-items-center rounded-[5px] text-accent hover:bg-hover disabled:opacity-40"
            >
              <Check className="size-4" strokeWidth={2} />
            </button>
            <button
              type="button"
              title="取消"
              onClick={() => setEditingShared(false)}
              className="grid size-7 place-items-center rounded-[5px] text-faint hover:bg-hover"
            >
              <X className="size-4" strokeWidth={2} />
            </button>
          </div>
        ) : (
          <div className="flex items-center gap-2">
            <span className="min-w-0 flex-1 break-all font-mono text-[12px] text-ink-3">
              {sharedDir ?? "未设置"}
            </span>
            <Button
              compact
              onClick={() => {
                setSharedDraft(sharedDir ?? "");
                setEditingShared(true);
              }}
            >
              <Pencil className="size-3.5" strokeWidth={1.75} />
              修改
            </Button>
          </div>
        )}
      </Panel>

      <Panel title="同步状态" description="连接、库变化与上游变化；冲突只提示，由 Agent 检查后继续。">
        <SyncStatus
          reloadToken={syncToken}
          failure={uploadFailure}
          onRetryUpload={retryUpload}
        />
      </Panel>

      <Panel
        title="技能库容量"
        description="按当前库文件统计（排除 .git）；只展示占用与较大技能，供决定是否清理。"
      >
        <CapacityReport reloadToken={reloadToken} />
      </Panel>

      <Panel
        title="诊断与运行日志"
        description="版本、平台、有关路径、最近操作与错误、崩溃记录；由你主动获取，不会自动上传。"
      >
        <DiagnosticsPanel reloadToken={reloadToken} />
      </Panel>

      {notice && <Notice>{notice}</Notice>}
    </div>
  );
}
