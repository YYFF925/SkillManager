/**
 * 设置页（T055/T083/T087/T110）：按原型分区——支持的 Agent → 全局配置 →
 * 自己的 SkillVault 同步 → 后台检查与上游更新 → 诊断与日志。
 *
 * 只放本期已接入的真实设置；主题/字号/频率取值由 core 校验并持久化，前端不复制业务判断。
 * 诊断与容量都来自 core 的真实结果，不由前端估算。
 * Agent 检测区块（FR-041）由 AgentSettings 组件承担，与原型一致地放在本页首个区块。
 */
import { useCallback, useEffect, useState } from "react";
import { Check, Pencil, RefreshCw, X } from "lucide-react";

import { Segmented, Notice, Badge, Button } from "../components/ui";
import { AgentSettings } from "../components/AgentSettings";
import { SyncStatus } from "../components/SyncStatus";
import { DiagnosticsPanel } from "../components/DiagnosticsPanel";
import { CapacityReport } from "../components/CapacityReport";
import { skillHasChange } from "../components/UpstreamDialog";
import { useUpstream } from "../lib/upstream";
import {
  setAutoCheckMinutes,
  setFontScale,
  setTheme,
  sharedDirGet,
  sharedDirSet,
  syncPull,
  type AgentView,
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
  /** 全量内置 Agent 名单（检测/启停/排序/目录由本页首区块维护）。 */
  agents: AgentView[];
  onAgentsChanged: (agents: AgentView[]) => void;
  /** 点击 Agent 名称前往该目标的本地技能视图。 */
  onOpenTarget: (target: string) => void;
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

/** 配置行（原型 .settings-panel 的一行）：左标签与说明，右控件或只读值。 */
function SettingRow({
  label,
  description,
  children,
}: {
  label: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 border-b border-line-faint py-3 first:pt-0 last:border-b-0 last:pb-0">
      <div className="min-w-0">
        <p className="text-[13px] text-ink-2">{label}</p>
        <p className="mt-1 text-[12px] text-muted">{description}</p>
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
  reloadToken,
}: SettingsPageProps) {
  const [sharedDir, setSharedDir] = useState<string | null>(null);
  const [editingShared, setEditingShared] = useState(false);
  const [sharedDraft, setSharedDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [syncToken, setSyncToken] = useState(0);
  const [checking, setChecking] = useState(false);
  const upstream = useUpstream();

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

  /**
   * 立即检查：复用库页两条既有能力——上游检查（库页工具栏「检查全部」同一函数）与库同步
   * （库页「同步」同一函数）。上游更新不自动应用，只展示变化。
   */
  async function checkNow() {
    setChecking(true);
    setError(null);
    try {
      await upstream.checkAll();
      const report = await syncPull();
      setNotice(`已检查上游变化并同步自己的库；上游更新仍需你确认。${report.message}`);
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

  const upstreamChanged = (upstream.report?.skills ?? []).filter(skillHasChange).length;
  const upstreamCandidates = upstream.report?.candidates.length ?? 0;

  return (
    <div className="flex flex-col gap-4">
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
        description="本机运行配置；路径变更不搬动已有文件，部署方式、语言与关闭行为是当前固定取值。"
      >
        <div className="flex flex-col">
          <SettingRow
            label="SkillVault 目录"
            description="所有技能、管理资料、指令源与预设的统一维护目录。"
          >
            <ReadOnlyValue mono>{vault.root}</ReadOnlyValue>
          </SettingRow>

          <SettingRow
            label="共享技能目录"
            description="共享目录是编码 Agent 的部署目标；留空表示清除。"
          >
            {editingShared ? (
              <div className="flex items-center gap-2">
                <input
                  autoFocus
                  className="field-input h-9 w-[240px] text-[12px]"
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
                <ReadOnlyValue mono>{sharedDir ?? "未设置"}</ReadOnlyValue>
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
          </SettingRow>

          <SettingRow
            label="部署方式"
            description="根据目标合成覆盖层和中文注释后生成部署副本。"
          >
            <ReadOnlyValue>文件复制</ReadOnlyValue>
          </SettingRow>

          <SettingRow label="主题" description="主题默认跟随系统；系统深色变化时界面实时切换。">
            <Segmented
              options={THEME_OPTIONS}
              value={settings?.theme ?? "system"}
              onChange={(value) => void changeTheme(value)}
              className={busy ? "opacity-60" : ""}
            />
          </SettingRow>

          <SettingRow label="文字大小" description="调整界面与阅读的基础字号，保存在本机。">
            <Segmented
              options={FONT_OPTIONS}
              value={settings?.font_scale ?? "default"}
              onChange={(value) => void changeFont(value)}
              className={busy ? "opacity-60" : ""}
            />
          </SettingRow>

          <SettingRow
            label="语言"
            description="当前界面使用简体中文；技能正文仍可切换中文与原文。"
          >
            <ReadOnlyValue>简体中文</ReadOnlyValue>
          </SettingRow>

          <SettingRow
            label="关闭行为"
            description="关闭窗口后留在后台；托盘右键「退出」才结束运行。"
          >
            <ReadOnlyValue>托盘模式</ReadOnlyValue>
          </SettingRow>

          <SettingRow
            label="托盘／菜单栏图标"
            description="显示快捷打开、检查更新及退出入口；托盘行为固定，不由设置开关改变。"
          >
            <ReadOnlyValue>固定开启</ReadOnlyValue>
          </SettingRow>
        </div>
      </Panel>

      <Panel
        title="自己的 SkillVault 同步"
        description="库内容、管理资料、预设与指令源随库同步；部署记录、档案启用与 Agent 配置留本机。"
      >
        <div className="flex flex-col gap-4">
          <SyncStatus
            reloadToken={syncToken}
            failure={uploadFailure}
            onRetryUpload={retryUpload}
          />

          <div className="border-t border-line-faint pt-3.5">
            <p className="text-[13px] text-ink-2">自动上传确认</p>
            <p className="mt-1 text-[12px] text-muted">
              首次在界面保存库内修改时询问一次，确认后沿用该设置，不再逐次询问。
            </p>
            <div className="mt-2 flex items-center gap-2.5">
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
          </div>
        </div>
      </Panel>

      <Panel
        title="后台检查与上游更新"
        description="上游检查、库同步与工具版本检查共用同一周期；普通关窗只收起到托盘，定时检查继续运行。"
      >
        <div className="flex flex-col">
          <SettingRow
            label="检查频率"
            description="后台检查自己的库和上游仓库；上游更新仍由你确认（默认 1 小时）。"
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
            description="检查上游变化并同步自己的库，不自动应用上游更新。"
          >
            <Button compact disabled={checking || upstream.busy} onClick={() => void checkNow()}>
              <RefreshCw className="size-3.5" strokeWidth={1.75} />
              {checking ? "正在检查…" : "立即检查"}
            </Button>
          </SettingRow>
        </div>

        <p className="mt-2 text-[12px] text-faint">
          {settings?.auto_check_minutes === 0
            ? "当前已关闭定时检查；仍可用「立即检查」。"
            : "到达周期后执行一次上游检查与库拉取；结果分别提示。"}
        </p>
        {upstream.report && (
          <p className="mt-1 text-[12px] text-faint">
            上次上游检查：{upstreamChanged} 个技能有变化，{upstreamCandidates} 个新增候选（只展示，未应用）。
          </p>
        )}
      </Panel>

      <Panel
        title="诊断与日志"
        description="技能库容量、诊断信息与运行日志；都由你主动获取，不会自动上传。"
      >
        <div className="flex flex-col gap-4">
          <div>
            <p className="text-[13px] text-ink-2">技能库容量</p>
            <p className="mt-1 text-[12px] text-muted">
              按当前库文件统计（排除 .git）；只展示占用与较大技能，供决定是否清理。
            </p>
            <div className="mt-2.5">
              <CapacityReport reloadToken={reloadToken} />
            </div>
          </div>

          <div className="border-t border-line-faint pt-3.5">
            <p className="text-[13px] text-ink-2">诊断与运行日志</p>
            <p className="mt-1 text-[12px] text-muted">
              版本、平台、有关路径、最近操作与错误、崩溃记录。
            </p>
            <div className="mt-2.5">
              <DiagnosticsPanel reloadToken={reloadToken} />
            </div>
          </div>
        </div>
      </Panel>

      {notice && <Notice>{notice}</Notice>}
    </div>
  );
}
