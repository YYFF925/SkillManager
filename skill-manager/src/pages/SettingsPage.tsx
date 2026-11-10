/**
 * 设置页（T055/T083/T087/T110）：按原型分区——支持的 Agent → 全局配置 →
 * 自己的 SkillManagerVault 同步 → 后台检查与上游更新 → 诊断与日志。
 *
 * 只放本期已接入的真实设置；主题/字号/频率取值由 core 校验并持久化，前端不复制业务判断。
 * 诊断与容量都来自 core 的真实结果，不由前端估算。
 * Agent 检测区块（FR-041）由 AgentSettings 组件承担，与原型一致地放在本页首个区块。
 */
import { useCallback, useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";

import { ArchiveDialog } from "../components/ArchiveDialog";
import { Segmented, Notice, Badge, Button } from "../components/ui";
import { checkForAppUpdate, currentAppVersion, type AppUpdate } from "../lib/appUpdate";
import { AgentSettings } from "../components/AgentSettings";
import { SyncStatus } from "../components/SyncStatus";
import { DiagnosticsPanel } from "../components/DiagnosticsPanel";
import { CapacityReport } from "../components/CapacityReport";
import { skillHasChange } from "../components/UpstreamDialog";
import {
  setAutoCheckMinutes,
  setFontScale,
  setTheme,
  sharedDirGet,
  checkNow as runCheck,
  type ScheduleReportView,
  type AgentView,
  type FontScale,
  type SettingsView,
  type ThemeSetting,
  type VaultInfo,
} from "../services/client";

export interface SettingsPageProps {
  vault: VaultInfo;
  schedule: ScheduleReportView | null;
  /** 最近一次上传失败的可读原因；无失败为 null。 */
  uploadFailure: string | null;
  /** 显式重试最近一次已同意的上传。 */
  onRetryUpload: () => Promise<void>;
  /** 本机设置（由 App 统一读取并应用外观）。 */
  settings: SettingsView | null;
  onSettingsChange: (settings: SettingsView) => void;
  /** 全量内置 Agent 名单（检测/启停/排序/目录由本页首区块维护）。 */
  agents: AgentView[];
  onAgentsChanged: (agents: AgentView[]) => void;
  /** 点击 Agent 名称前往该目标的本地技能视图。 */
  onOpenTarget: (target: string) => void;
  /** 手动检查到新版本后交给整页共享的更新提示处理。 */
  onAppUpdateFound: (update: AppUpdate) => void;
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

/** 配置行（原型 .settings-panel 的一行）：左标签，右控件或只读值。 */
function SettingRow({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 border-b border-line-faint py-3 first:pt-0 last:border-b-0 last:pb-0">
      <div className="min-w-0">
        <p className="text-[13px] text-ink-2">{label}</p>
      </div>
      <div className="flex shrink-0 items-center gap-2.5">{children}</div>
    </div>
  );
}

/** 只读展示值（原型 .path-pill）：纯展示，不渲染可编辑控件。 */
function ReadOnlyValue({ children, mono = false }: { children: React.ReactNode; mono?: boolean }) {
  return (
    <span
      className={`inline-flex max-w-[320px] items-center truncate rounded-lg border border-line bg-panel px-2.5 py-1 text-[12px] text-ink-3 ${
        mono ? "font-mono" : ""
      }`}
    >
      {children}
    </span>
  );
}

export function SettingsPage({
  vault,
  uploadFailure,
  onRetryUpload,
  settings,
  onSettingsChange,
  agents,
  onAgentsChanged,
  onOpenTarget,
  onAppUpdateFound,
  reloadToken,
  schedule,
}: SettingsPageProps) {
  const [sharedDir, setSharedDir] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [syncToken, setSyncToken] = useState(0);
  const [checking, setChecking] = useState(false);
  const [appVersion, setAppVersion] = useState("…");
  const [checkingAppUpdate, setCheckingAppUpdate] = useState(false);
  const [archiveOpen, setArchiveOpen] = useState(false);
  const [manualSchedule, setManualSchedule] = useState<ScheduleReportView | null>(null);
  const latestSchedule = manualSchedule ?? schedule;
  useEffect(() => setManualSchedule(null), [schedule]);

  useEffect(() => {
    void sharedDirGet()
      .then(setSharedDir)
      .catch((err) => setError(String(err)));
  }, [reloadToken]);

  useEffect(() => {
    void currentAppVersion().then(setAppVersion);
  }, []);

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

  /**
   * 立即检查与启动、定时、托盘共用 core 统一调度；分别展示三项结果。
   */
  async function checkNow() {
    setChecking(true);
    setError(null);
    try {
      const report = await runCheck();
      setManualSchedule(report);
      setSyncToken((value) => value + 1);
      setNotice("检查已结束，三项结果如下。");
    } catch (err) {
      setError(String(err));
    } finally {
      setChecking(false);
    }
  }

  async function retryUpload() {
    await onRetryUpload();
    setSyncToken((value) => value + 1);
  }

  async function checkAppUpdate() {
    setCheckingAppUpdate(true);
    setError(null);
    try {
      const found = await checkForAppUpdate();
      if (found) {
        onAppUpdateFound(found);
        setNotice(`发现新版本 ${found.version}。`);
      } else {
        setNotice("当前已是最新版本。");
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setCheckingAppUpdate(false);
    }
  }

  const upstreamChanged = (latestSchedule?.upstream_report?.skills ?? []).filter(skillHasChange).length;
  const upstreamCandidates = latestSchedule?.upstream_report?.candidates.length ?? 0;

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col gap-4 overflow-y-auto overscroll-contain">
      {error && <Notice tone="danger">{error}</Notice>}

      <AgentSettings
        agents={agents}
        onAgentsChanged={onAgentsChanged}
        onNotice={(message) => {
          setNotice(message);
          setError(null);
        }}
        onSelectTarget={onOpenTarget}
      />

      <Panel
        title="全局配置"
      >
        <div className="flex flex-col">
          <SettingRow
            label="SkillManagerVault 目录"
          >
            <ReadOnlyValue mono>{vault.root}</ReadOnlyValue>
          </SettingRow>

          <SettingRow label="技能归档">
            <Button compact onClick={() => setArchiveOpen(true)}>
              查看归档
            </Button>
          </SettingRow>

          <SettingRow
            label="共享技能目录"
          >
            <ReadOnlyValue mono>{sharedDir ?? "读取中…"}</ReadOnlyValue>
          </SettingRow>

          <SettingRow
            label="部署方式"
          >
            <ReadOnlyValue>文件复制</ReadOnlyValue>
          </SettingRow>

          <SettingRow label="主题" >
            <Segmented
              options={THEME_OPTIONS}
              value={settings?.theme ?? "system"}
              onChange={(value) => void changeTheme(value)}
              className={busy ? "opacity-60" : ""}
            />
          </SettingRow>

          <SettingRow label="文字大小" >
            <Segmented
              options={FONT_OPTIONS}
              value={settings?.font_scale ?? "default"}
              onChange={(value) => void changeFont(value)}
              className={busy ? "opacity-60" : ""}
            />
          </SettingRow>

          <SettingRow
            label="语言"
          >
            <ReadOnlyValue>简体中文</ReadOnlyValue>
          </SettingRow>

          <SettingRow
            label="关闭行为"
          >
            <ReadOnlyValue>托盘模式</ReadOnlyValue>
          </SettingRow>

          <SettingRow
            label="软件版本"
          >
            <ReadOnlyValue mono>{appVersion}</ReadOnlyValue>
            <Button compact disabled={checkingAppUpdate} onClick={() => void checkAppUpdate()}>
              {checkingAppUpdate ? "正在检查…" : "检查更新"}
            </Button>
          </SettingRow>

          <SettingRow
            label="托盘／菜单栏图标"
          >
            <ReadOnlyValue>固定开启</ReadOnlyValue>
          </SettingRow>
        </div>
      </Panel>

      <Panel
        title="自己的 SkillManagerVault 同步"
      >
        <div className="flex flex-col gap-4">
          <SyncStatus
            reloadToken={syncToken}
            failure={uploadFailure}
            onRetryUpload={retryUpload}
          />

          <div className="border-t border-line-faint pt-3.5">
            <p className="text-[13px] text-ink-2">自动上传确认</p>

            <div className="mt-2 flex items-center gap-2.5">
              {settings?.git_auto_push ? (
                <Badge tone="green">已确认</Badge>
              ) : (
                <Badge tone="gray">尚未确认</Badge>
              )}
              <span className="text-[13px] text-ink-3">
                {settings?.git_auto_push
                  ? "界面保存后会自动提交并推送本机 SkillManagerVault。"
                  : "尚未确认；下次保存时询问一次是否以后自动上传。"}
              </span>
            </div>
            <p className="mt-2 text-[12px] text-faint">
              只影响界面保存这一条路径；外部文件改动、CLI 与同步已有部署不会触发上传。
            </p>
          </div>
        </div>
      </Panel>

      <Panel
        title="后台检查与上游更新"
      >
        <div className="flex flex-col">
          <SettingRow
            label="检查频率"
          >
            <Segmented
              options={FREQUENCY_OPTIONS}
              value={settings ? String(settings.auto_check_minutes) : "60"}
              onChange={(value) => void changeFrequency(value)}
              className={busy ? "opacity-60" : ""}
            />
          </SettingRow>

          <SettingRow
            label="立即检查"
          >
            <Button compact disabled={checking || checking} onClick={() => void checkNow()}>
              <RefreshCw className="size-3.5" strokeWidth={1.75} />
              {checking ? "正在检查…" : "立即检查"}
            </Button>
          </SettingRow>
        </div>

        <p className="mt-2 text-[12px] text-faint">
          {settings?.auto_check_minutes === 0
            ? "当前已关闭定时检查；仍可用「立即检查」。"
            : "定时检查已开启。"}
        </p>
        {latestSchedule && <div className="mt-3 space-y-2">
          {([['上游检查', latestSchedule.upstream], ['库同步', latestSchedule.pull], ['工具检查', latestSchedule.tool_check]] as const).map(([label, result]) => (
            <Notice key={label} tone={result.status === 'error' ? 'danger' : 'info'}>
              <span className="font-medium">{label}：</span><span className="whitespace-pre-wrap">{result.detail ?? '尚无结果'}</span>
            </Notice>
          ))}
        </div>}
        {latestSchedule?.upstream_report && (
          <p className="mt-1 text-[12px] text-faint">
            上次上游检查：{upstreamChanged} 个技能有变化，{upstreamCandidates} 个新增候选（只展示，未应用）。
          </p>
        )}
      </Panel>

      <Panel
        title="诊断与日志"
      >
        <div className="flex flex-col gap-4">
          <div>
            <p className="text-[13px] text-ink-2">技能库容量</p>

            <div className="mt-2.5">
              <CapacityReport reloadToken={reloadToken} />
            </div>
          </div>

          <div className="border-t border-line-faint pt-3.5">
            <p className="text-[13px] text-ink-2">诊断与运行日志</p>

            <div className="mt-2.5">
              <DiagnosticsPanel reloadToken={reloadToken} />
            </div>
          </div>
        </div>
      </Panel>

      {notice && <Notice>{notice}</Notice>}

      {archiveOpen && (
        <ArchiveDialog
          onClose={() => setArchiveOpen(false)}
          onChanged={(message) => {
            setNotice(message);
            setError(null);
          }}
        />
      )}
    </div>
  );
}
