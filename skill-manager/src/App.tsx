import { useCallback, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Bot, FileText, FolderTree, Layers, MessageSquare, RefreshCw, Settings, Share2, Wrench } from "lucide-react";

import { AppShell, type NavGroup, type NavItem } from "./components/AppShell";
import { SidebarPresets } from "./components/SidebarPresets";
import { LibraryPage } from "./pages/LibraryPage";
import { AgentPage } from "./pages/AgentPage";
import { HermesPage } from "./pages/HermesPage";
import { PresetPage } from "./pages/PresetPage";
import { ToolCenterPage } from "./pages/ToolCenterPage";
import { SettingsPage } from "./pages/SettingsPage";
import { LoginPage } from "./pages/LoginPage";
import { InstructionEditor } from "./components/InstructionEditor";
import { FastTooltip } from "./components/FastTooltip";
import { AppUpdateDialog } from "./components/AppUpdateDialog";
import { DeviceBar } from "./components/DeviceBar";
import { PlanConfirmHost } from "./components/PlanConfirmHost";
import { ChatPanel, type ChatViewRef } from "./components/ChatPanel";
import { Button, Notice } from "./components/ui";
import { checkForAppUpdate, type AppUpdate } from "./lib/appUpdate";
import { useAppearance } from "./lib/appearance";
import { useLibrarySaveFlow } from "./lib/librarySave";
import {
  agentList,
  currentVault,
  currentSchedule,
  onScheduleFinished,
  environmentOs,
  isDesktop,
  onVaultChanged,
  openVault,
  settingsGet,
  syncPull,
  type AgentView,
  type Domain,
  type EnvironmentOs,
  type SettingsView,
  type ScheduleReportView,
  type VaultInfo,
} from "./services/client";

/**
 * 页面标识。技能库按管理域区分；Agent 页面按管理域或单个 Agent 区分。
 */
type ViewKey = string;

/** 静态页面标题。 */
const VIEWS: Record<string, { title: string; pending?: string }> = {
  "library-code": { title: "技能库" },
  "library-lobster": {
    title: "龙虾技能库",
  },
  hermes: { title: "Hermes 档案" },
  tools: { title: "工具中心" },
  presets: { title: "预设" },
  "agents-code": {
    title: "全部编码 Agents",
  },
  "agents-lobster": {
    title: "全部龙虾 Agent",
  },
  "agent:shared": { title: "共享 Agent" },
  settings: { title: "设置" },
};

const settingsItem: NavItem = { key: "settings", label: "设置", icon: Settings };

/** 共享目录是编码 Agent 的部署目标，入口与单个 Agent 同级并始终显示（未设置路径也不隐藏）。 */
const SHARED_VIEW_KEY = "agent:shared";

/** 首次接入：远程须先对接并选择当前设备，再接入已存在的 SkillManagerVault（FR-002）。 */
function VaultWelcome({
  error,
  busy,
  remote,
  path,
  deviceId,
  onPathChange,
  onDeviceChange,
  onChoose,
}: {
  error: string | null;
  busy: boolean;
  remote: boolean;
  path: string;
  deviceId: string | null;
  onPathChange: (path: string) => void;
  onDeviceChange: (deviceId: string) => void;
  onChoose: () => void;
}) {
  const canAttach = !remote || Boolean(deviceId);
  return (
    <div
      className={
        remote
          ? "login-screen grid h-full place-items-center bg-app px-6"
          : "grid h-full place-items-center bg-app px-6"
      }
    >
      <div
        className={`login-card ${remote ? "first-connect-card" : ""} w-[520px] rounded-xl border border-line-soft bg-panel p-7`}
      >
        <div className="mb-4 flex items-center gap-2.5">
          <img src="/assets/app-logo.png" alt="" className="size-7" />
          <span className="text-[17px] font-semibold">Skill Manager</span>
        </div>
        <h1 className={remote ? "text-[17px] font-semibold" : "text-[15px] font-semibold"}>
          接入你的 SkillManagerVault
        </h1>
        {remote ? (
          <>
            <p className="muted mt-1.5 text-[13px] leading-[21px] text-muted">
              先对接一台 Host，再接入该机上已存在的技能库。没有设备时无法打开库路径。
            </p>
            <ol className="first-connect-steps">
              <li>生成一次性对接码。</li>
              <li>
                在该机执行{" "}
                <code>host pair --station &lt;管理站地址&gt; --code &lt;对接码&gt;</code>。
              </li>
              <li>选择当前设备后，填写该 Host 上已存在的技能库路径。</li>
            </ol>
            <DeviceBar placement="welcome" currentId={deviceId} onChange={onDeviceChange} />
            {!deviceId ? (
              <p className="first-connect-hint">请先对接并选择当前设备，再接入技能库。</p>
            ) : null}
          </>
        ) : (
          <p className="mt-2 text-[13px] leading-[21px] text-muted">
            选择一个已存在的本地技能库目录；它需要是 Git 仓库并已配置远端。
            编码技能库（code/）与龙虾技能库（lobster/）在同一个库内分别管理，
            技能内容与管理资料都留在这个目录里。
          </p>
        )}
        {remote ? (
          <label className="mt-5 block text-[12px] text-muted">
            Host 上的技能库路径
            <input
              className="mt-1.5 h-10 w-full rounded-lg border border-line bg-panel px-3 text-[13px]"
              value={path}
              disabled={!canAttach}
              onChange={(event) => onPathChange(event.target.value)}
              placeholder="/home/user/SkillManagerVault"
            />
          </label>
        ) : null}
        <div className="first-connect-actions">
          <Button
            kind="primary"
            disabled={busy || !canAttach || (remote && !path.trim())}
            onClick={onChoose}
          >
            {busy
              ? "正在接入…"
              : remote
                ? canAttach
                  ? "接入该路径"
                  : "请先选择当前设备"
                : "选择技能库目录"}
          </Button>
        </div>
        <p className="muted mt-1.5 text-[12px] text-muted">
          不提供克隆或创建向导；库的创建与远端配置请在 Git 中完成。
        </p>
        {error && (
          <div className="mt-4">
            <Notice tone="danger">{error}</Notice>
          </div>
        )}
      </div>
    </div>
  );
}

/** 侧栏中某管理域下已检测且启用的 Agent（按本机顺序）。 */
function sidebarAgents(agents: AgentView[], domain: Domain): AgentView[] {
  return agents
    .filter((agent) => agent.domain === domain && agent.detected && agent.enabled)
    .sort((a, b) => a.order - b.order || a.key.localeCompare(b.key));
}

export function App() {
  const [vault, setVault] = useState<VaultInfo | null>(null);
  const [agents, setAgents] = useState<AgentView[]>([]);
  const [os, setOs] = useState<EnvironmentOs | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [pulling, setPulling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [activeKey, setActiveKey] = useState<ViewKey>("library-code");
  const [reloadToken, setReloadToken] = useState(0);
  const [count, setCount] = useState<number | null>(null);
  const [subtitle, setSubtitle] = useState<string | null>(null);
  const [titleOverride, setTitleOverride] = useState<string | null>(null);
  /** 从 Hermes 详情“前往技能库编辑”时，技能库要直接打开的技能引用。 */
  const [libraryFocus, setLibraryFocus] = useState<string | null>(null);
  /** 侧栏预设区当前管理域与选中项。 */
  const [presetDomain, setPresetDomain] = useState<Domain>("code");
  const [presetId, setPresetId] = useState<string | null>(null);
  const [presetReload, setPresetReload] = useState(0);
  /** 指令编辑器是否打开。 */
  const [instructionOpen, setInstructionOpen] = useState(false);
  /** 本机设置（主题/字号等）；由本页统一读取并应用到 `<html>`。 */
  const [settings, setSettings] = useState<SettingsView | null>(null);
  const [schedule, setSchedule] = useState<ScheduleReportView | null>(null);
  const [appUpdate, setAppUpdate] = useState<AppUpdate | null>(null);
  const [updateOpen, setUpdateOpen] = useState(false);
  const [updateAutoInstall, setUpdateAutoInstall] = useState(false);
  const remote = !isDesktop();
  const [stationReady, setStationReady] = useState(!remote);
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [deviceOnline, setDeviceOnline] = useState(true);
  const [chatOpen, setChatOpen] = useState(false);
  const [chatSelection, setChatSelection] = useState<string[]>([]);
  const [vaultPathDraft, setVaultPathDraft] = useState("");

  useEffect(() => {
    if (remote && (!stationReady || !deviceId)) {
      return;
    }
    let cancelled = false;
    let off: (() => void) | undefined;
    const accept = (report: ScheduleReportView) => {
      if (cancelled) return;
      setSchedule(report);
      setReloadToken((value) => value + 1);
    };
    void onScheduleFinished(accept).then(async (stop) => {
      if (cancelled) { stop(); return; }
      off = stop;
      const report = await currentSchedule();
      if (report) accept(report);
    }).catch((err) => { if (!cancelled) setError(String(err)); });
    return () => { cancelled = true; off?.(); };
  }, [remote, stationReady, deviceId]);

  // 主题与字号是本项目新增能力：默认跟随系统，不继承旧版 dark 默认。
  useAppearance(settings?.theme ?? null, settings?.font_scale ?? null);

  // GUI 保存库内修改后的统一流程：先刷新已有部署，再按已确认策略上传（T054/T055）。
  const saveFlow = useLibrarySaveFlow({
    onNotice: setNotice,
    onError: setError,
  });
  const setAgentMeta = useCallback(
    (meta: { count: number; subtitle: string }) => {
      setCount(meta.count);
      setSubtitle(meta.subtitle);
    },
    [],
  );

  useEffect(() => {
    if (remote) return;
    let cancelled = false;
    void checkForAppUpdate({ silent: true }).then((update) => {
      if (!cancelled && update) {
        setAppUpdate(update);
        setUpdateOpen(true);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [remote]);

  useEffect(() => {
    if (!remote) {
      setStationReady(true);
      return;
    }
    let cancelled = false;
    void fetch("/auth/session", { credentials: "include" })
      .then(async (res) => {
        if (!res.ok) return;
        const body = await res.json().catch(() => ({}));
        if (cancelled) return;
        if (body && typeof body === "object" && (body as { mustRebindTotp?: boolean }).mustRebindTotp) {
          return;
        }
        setDeviceId(typeof body.deviceId === "string" ? body.deviceId : null);
        setStationReady(true);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [remote]);

  useEffect(() => {
    if (remote && !deviceId) {
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const loaded = await settingsGet();
        if (!cancelled) {
          setSettings(loaded);
        }
      } catch {
        // 尚未接入库时读取设置失败不影响进入欢迎页。
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [remote, deviceId]);

  useEffect(() => {
    if (remote && !stationReady) {
      setLoading(false);
      return;
    }
    if (remote && !deviceId) {
      setLoading(false);
      setVault(null);
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const info = await currentVault();
        if (cancelled) {
          return;
        }
        setVault(info);
        if (info) {
          const [list, currentOs] = await Promise.all([agentList(), environmentOs()]);
          if (cancelled) {
            return;
          }
          setAgents(list);
          setOs(currentOs);
        }
      } catch (err) {
        if (!cancelled) {
          const message = String(err);
          if (remote && message.includes("尚未选择当前设备")) {
            setVault(null);
          } else {
            setError(message);
          }
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [remote, stationReady, deviceId]);

  // 库内外部改动（GUI 自身写入也会触发）：重新拉取界面，不重启、不提交或部署。
  useEffect(() => {
    if (!vault) {
      return;
    }
    let stop: (() => void) | null = null;
    let cancelled = false;
    void onVaultChanged(() => setReloadToken((value) => value + 1)).then((off) => {
      if (cancelled) {
        off();
      } else {
        stop = off;
      }
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, [vault]);

  const chooseVault = useCallback(async () => {
    if (remote && !deviceId) {
      setError("请先对接并选择当前设备，再接入技能库。");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      let selected: string | null = vaultPathDraft.trim() || null;
      if (!remote) {
        const picked = await open({
          directory: true,
          multiple: false,
          title: "选择已有的 SkillManagerVault 目录",
        });
        selected = typeof picked === "string" ? picked : null;
      }
      if (!selected) {
        return;
      }
      const info = await openVault(selected);
      const [list, currentOs] = await Promise.all([agentList(), environmentOs()]);
      setVault(info);
      setAgents(list);
      setOs(currentOs);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }, [remote, vaultPathDraft, deviceId]);

  const switchDevice = useCallback(async (nextId: string) => {
    setError(null);
    setVault(null);
    setAgents([]);
    setDeviceOnline(true);
    if (!nextId) {
      setDeviceId(null);
      return;
    }
    try {
      const response = await fetch("/current", {
        method: "PUT",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ deviceId: nextId }),
      });
      const body = await response.json().catch(() => ({}));
      if (!response.ok) {
        setDeviceOnline(false);
        throw new Error(
          typeof body === "object" && body && "error" in body
            ? String((body as { error: unknown }).error)
            : "无法切换设备",
        );
      }
      setDeviceId(nextId);
    } catch (err) {
      setDeviceOnline(false);
      setError(String(err));
    }
  }, []);

  async function pullVault() {
    setPulling(true);
    setNotice(null);
    setError(null);
    try {
      const report = await syncPull();
      setReloadToken((value) => value + 1);
      if ((report.outcome === "pulled" || report.outcome === "up_to_date") && !report.errors.length) {
        setNotice(report.message);
      } else {
        setError(report.message);
      }
    } catch (err) {
      setError(`拉取 SkillManagerVault 失败：${String(err)}`);
    } finally {
      setPulling(false);
    }
  }

  const groups: NavGroup[] = useMemo(() => {
    const agentItems = (domain: Domain, allKey: string, allLabel: string): NavItem[] => [
      { key: allKey, label: allLabel, icon: Bot },
      ...(domain === "code"
        ? [{ key: SHARED_VIEW_KEY, label: "共享 Agent", icon: Share2 }]
        : []),
      ...sidebarAgents(agents, domain).map((agent) => ({
        key: `agent:${agent.key}`,
        label: agent.display_name,
        icon: Bot,
        iconSrc: agent.icon ? `/${agent.icon}` : undefined,
        invertInDark: agent.invert_in_dark,
      })),
    ];
    return [
      {
        items: [
          { key: "library-code", label: "技能库", icon: Layers },
          { key: "library-lobster", label: "龙虾技能库", icon: Layers },
          { key: "hermes", label: "Hermes 档案", icon: Bot },
          { key: "tools", label: "工具中心", icon: Wrench },
        ],
      },
      {
        title: "编码 Agent",
        items: agentItems("code", "agents-code", "全部 Agents"),
      },
      {
        title: "龙虾 Agent",
        items: agentItems("lobster", "agents-lobster", "全部龙虾 Agent"),
      },
    ];
  }, [agents]);

  if (remote && !stationReady) {
    return <LoginPage onLoggedIn={(id) => { setDeviceId(id ?? null); setStationReady(true); }} />;
  }

  if (loading) {
    return (
      <div className="grid h-full place-items-center bg-app text-[13px] text-muted">
        正在接入 SkillManagerVault…
      </div>
    );
  }

  if (!vault) {
    return (
      <VaultWelcome
        error={error}
        busy={busy}
        remote={remote}
        path={vaultPathDraft}
        deviceId={deviceId}
        onPathChange={setVaultPathDraft}
        onDeviceChange={(id) => void switchDevice(id)}
        onChoose={() => void chooseVault()}
      />
    );
  }

  const isLibrary = activeKey === "library-code" || activeKey === "library-lobster";
  const selectedTarget = activeKey.startsWith("agent:") ? activeKey.slice("agent:".length) : null;
  const selectedAgent = agents.find((agent) => agent.key === selectedTarget);
  const isAgentView =
    activeKey === "agents-code" || activeKey === "agents-lobster" || selectedTarget !== null;
  const agentDomain: Domain =
    activeKey === "agents-lobster" || selectedAgent?.domain === "lobster" ? "lobster" : "code";

  const view = selectedAgent
    ? { title: selectedAgent.display_name }
    : (VIEWS[activeKey] ?? { title: activeKey, pending: "后续里程碑" });
  const pageTitle = titleOverride ?? view.title;
  const chatView: ChatViewRef = {
    page: activeKey,
    domain: isLibrary
      ? activeKey === "library-lobster"
        ? "lobster"
        : "code"
      : activeKey === "presets"
        ? presetDomain
        : agentDomain,
    selection: {
      ...(chatSelection.length ? { refs: chatSelection, ref: chatSelection[0] } : {}),
      ...(activeKey === "presets" && presetId ? { presetId } : {}),
      ...(selectedTarget ? { agent: selectedTarget } : {}),
    },
  };

  return (
    <>
    <PlanConfirmHost />
    <AppShell
      groups={groups}
      settingsItem={settingsItem}
      activeKey={activeKey}
      onSelect={(key) => {
        setActiveKey(key);
        setCount(null);
        setSubtitle(null);
        setTitleOverride(null);
        setNotice(null);
      }}
      title={pageTitle}
      subtitle={subtitle ?? undefined}
      count={count ?? undefined}
      deviceBar={
        remote ? (
          <DeviceBar currentId={deviceId} onChange={(id) => void switchDevice(id)} />
        ) : undefined
      }
      chat={
        <ChatPanel open={chatOpen} view={chatView} onClose={() => setChatOpen(false)} />
      }
      update={
        !remote && appUpdate
          ? {
              version: appUpdate.version,
              onInstall: () => {
                setUpdateAutoInstall(true);
                setUpdateOpen(true);
              },
            }
          : undefined
      }
      headerExtra={
        <>
          {notice && <span className="text-[12px] text-muted">{notice}</span>}
          <FastTooltip label={pulling ? "正在拉取 SkillManagerVault…" : "检查并拉取 SkillManagerVault"}>
            <button
              type="button"
              aria-label="检查并拉取 SkillManagerVault"
              aria-busy={pulling}
              disabled={pulling}
              onClick={() => void pullVault()}
              className="grid size-8 place-items-center rounded-lg border border-line bg-panel text-muted transition-colors hover:bg-hover hover:text-accent disabled:opacity-60"
            >
              <RefreshCw className={`size-4 ${pulling ? "animate-spin" : ""}`} strokeWidth={1.75} />
            </button>
          </FastTooltip>
          <button
            type="button"
            title="Agent 指令"
            aria-label="Agent 指令"
            onClick={() => setInstructionOpen(true)}
            className="grid size-8 place-items-center rounded-lg border border-line bg-panel text-muted transition-colors hover:bg-hover hover:text-accent"
          >
            <FileText className="size-4" strokeWidth={1.75} />
          </button>
          <button
            type="button"
            title="管理对话"
            aria-label="管理对话"
            onClick={() => setChatOpen((open) => !open)}
            className="grid size-8 place-items-center rounded-lg border border-line bg-panel text-muted transition-colors hover:bg-hover hover:text-accent"
          >
            <MessageSquare className="size-4" strokeWidth={1.75} />
          </button>
        </>
      }
      betweenNav={
        <SidebarPresets
          agents={agents}
          domain={presetDomain}
          selectedId={presetId}
          active={activeKey === "presets"}
          reloadToken={reloadToken + presetReload}
          onDomainChange={(domain) => {
            setPresetDomain(domain);
            setActiveKey("presets");
            setCount(null);
            setSubtitle(null);
            setTitleOverride(null);
          }}
          onSelect={(id) => {
            setPresetId(id);
            setActiveKey("presets");
            setCount(null);
            setSubtitle(null);
            setTitleOverride(null);
          }}
          onCreateNavigate={(id) => {
            setPresetId(id);
            setActiveKey("presets");
            setPresetReload((value) => value + 1);
            setCount(null);
            setSubtitle(null);
            setTitleOverride(null);
          }}
          onNotice={(message) => {
            setNotice(message);
            setError(null);
          }}
          onChanged={() => {
            setPresetReload((value) => value + 1);
            setReloadToken((value) => value + 1);
            void saveFlow.afterSave();
          }}
        />
      }
      vaultStatus={{ label: "SkillManagerVault", detail: vault.root, ok: true }}
    >
      {error && <div className="mb-3"><Notice tone="danger">{error}</Notice></div>}
      {remote && !deviceOnline && (
        <div className="mb-3">
          <Notice tone="warn">当前设备离线，不能操作，也不会用缓存技能顶上。</Notice>
        </div>
      )}
      {isLibrary ? (
        <LibraryPage
          key={activeKey}
          scheduledReport={schedule?.upstream_report}
          domain={activeKey === "library-code" ? "code" : "lobster"}
          os={os}
          agents={agents}
          reloadToken={reloadToken}
          onCount={setCount}
          onLibrarySaved={saveFlow.afterSave}
          focusReference={libraryFocus}
          onFocusConsumed={() => setLibraryFocus(null)}
          onSelectionChange={setChatSelection}
        />
      ) : activeKey === "hermes" ? (
        <HermesPage
          agents={agents}
          os={os}
          reloadToken={reloadToken}
          onCount={setCount}
          onLibrarySaved={saveFlow.afterSave}
          onOpenInLibrary={(reference) => {
            setLibraryFocus(reference);
            setActiveKey("library-lobster");
            setCount(null);
            setSubtitle(null);
            setTitleOverride(null);
            setNotice(null);
          }}
        />
      ) : activeKey === "tools" ? (
        <ToolCenterPage
          reloadToken={reloadToken}
          onCount={setCount}
          onLibrarySaved={saveFlow.afterSave}
        />
      ) : activeKey === "presets" ? (
        <PresetPage
          os={os}
          agents={agents}
          reloadToken={reloadToken + presetReload}
          domain={presetDomain}
          selectedId={presetId}
          onSelectedIdChange={setPresetId}
          onCount={setCount}
          onTitle={setTitleOverride}
          onLibrarySaved={saveFlow.afterSave}
        />
      ) : isAgentView ? (
        <AgentPage
          domain={agentDomain}
          selectedTarget={selectedTarget}
          agents={agents}
          onMeta={setAgentMeta}
          onOpenInLibrary={(reference) => {
            setLibraryFocus(reference);
            setActiveKey(reference.startsWith("lobster/") ? "library-lobster" : "library-code");
            setCount(null);
            setSubtitle(null);
            setTitleOverride(null);
            setNotice(null);
          }}
          onLibrarySaved={saveFlow.afterSave}
          onNotice={(message) => {
            setNotice(message);
            setError(null);
          }}
        />
      ) : activeKey === "settings" ? (
        <SettingsPage
          schedule={schedule}
          vault={vault}
          uploadFailure={saveFlow.failure}
          onRetryUpload={saveFlow.retry}
          settings={settings}
          onSettingsChange={setSettings}
          agents={agents}
          onAgentsChanged={setAgents}
          onOpenTarget={(target) => {
            setActiveKey(`agent:${target}`);
            setCount(null);
            setSubtitle(null);
            setTitleOverride(null);
            setNotice(null);
          }}
          onAppUpdateFound={(update) => {
            if (remote) return;
            setAppUpdate(update);
            setUpdateOpen(true);
          }}
          reloadToken={reloadToken}
          remote={remote}
        />
      ) : (
        <div className="flex items-center gap-2 rounded-xl border border-line-soft bg-panel p-6 text-[13px] text-muted">
          <FolderTree className="size-4 shrink-0" strokeWidth={1.75} />
          {view.title}尚未实现，计划在 {view.pending ?? "后续里程碑"} 提供。
        </div>
      )}
    </AppShell>
    {instructionOpen && (
      <InstructionEditor
        agents={agents}
        onClose={() => setInstructionOpen(false)}
        onSaved={() => void saveFlow.afterSave()}
      />
    )}
    {saveFlow.dialog}
    {appUpdate && updateOpen && !remote && (
      <AppUpdateDialog
        update={appUpdate}
        autoInstall={updateAutoInstall}
        onDismiss={() => {
          setUpdateOpen(false);
          setUpdateAutoInstall(false);
        }}
      />
    )}
    </>
  );
}
