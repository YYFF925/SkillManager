/**
 * 设置页（T110/T112）：按原型分区的区块顺序、块内只读展示行，以及既有行为的接线。
 *
 * 只验证界面与 core 输出之间的接线：区块顺序按 FR-056；只读行（部署方式/语言/关闭行为）
 * 不得渲染成可编辑控件；共享目录未设置时如实显示并保留行内修改入口。
 * Agent 检测区块来自 AgentSettings（FR-041），本页只断言它作为首个区块并带摘要与三个动作。
 */
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { SettingsPage } from "./SettingsPage";
import type { AgentView, SettingsView, VaultInfo } from "../services/client";

const vault: VaultInfo = { root: "/tmp/vault", remote: "https://example.invalid/vault.git", has_remote: true };

const settingsFixture: SettingsView = {
  theme: "system",
  font_scale: "default",
  auto_check_minutes: 60,
  git_auto_push: false,
};

/** 已检测且已启用的 Agent。 */
const codex: AgentView = {
  key: "codex",
  display_name: "Codex",
  domain: "code",
  detected: true,
  enabled: true,
  order: 0,
  skills_dir: "/tmp/codex/skills",
  skills_dir_default: "/tmp/codex/skills",
  skills_dir_custom: false,
  project_rel_dir: null,
  project_rel_dir_default: null,
  instructions_target: "/tmp/codex/AGENTS.md",
  icon: null,
  invert_in_dark: false,
};

/** 未检测到的内置 Agent：出现在「其他支持的 Agent」折叠区计数中。 */
const hermes: AgentView = {
  ...codex,
  key: "hermes",
  display_name: "Hermes Agent",
  domain: "lobster",
  detected: false,
  enabled: false,
  order: 1,
};

vi.mock("../services/client", () => ({
  // 共享目录未设置时为 null；用例按需覆盖。
  sharedDirGet: vi.fn(async () => null),
  sharedDirSet: vi.fn(async () => "/tmp/shared-skills"),
  setTheme: vi.fn(async () => settingsFixture),
  setFontScale: vi.fn(async () => settingsFixture),
  setAutoCheckMinutes: vi.fn(async () => settingsFixture),
  syncStatus: vi.fn(async () => ({
    vault: "/tmp/vault",
    remote: "https://example.invalid/vault.git",
    connected: true,
    branch: "main",
    uncommitted: [],
    dirty: false,
    conflict: false,
    conflicts: [],
    last_sync_at: null,
    auto_push: false,
  })),
  syncConflicts: vi.fn(async () => ({ has_conflict: false, conflicts: [], status: [], message: "" })),
  capacityReport: vi.fn(async () => ({
    vault: "/tmp/vault",
    total_size: 0,
    file_count: 0,
    skill_total_size: 0,
    skill_count: 0,
    skills: [],
    note: "统计排除 .git",
  })),
  diagnosticsReport: vi.fn(async () => ({
    generated_at: "2026-10-16T00:00:00Z",
    version: "0.1.0",
    os: "macos",
    arch: "arm64",
    vault: "/tmp/vault",
    state_dir: "/tmp/state",
    log_path: "/tmp/state/logs/app.log",
    crash_path: "/tmp/state/crash.log",
    last_check_at: null,
    last_sync_at: null,
    auto_check_minutes: 60,
    theme: "system",
    font_scale: "default",
    recent_logs: [],
    crashes: [],
    notes: [],
  })),
  diagnosticsExport: vi.fn(),
  // Agent 检测区块：默认名单与用例一致。
  agentDetect: vi.fn(async () => [codex, hermes]),
  agentEnable: vi.fn(),
  agentDisable: vi.fn(),
  agentReorder: vi.fn(),
  agentSetDir: vi.fn(),
  agentResetDir: vi.fn(),
  // 「立即检查」复用库页的上游检查与库同步两个既有函数。
  checkUpstream: vi.fn(async () => ({ repos: [], skills: [], candidates: [] })),
  onFetchProgress: vi.fn(async () => () => {}),
  syncPull: vi.fn(async () => ({
    outcome: "up_to_date",
    connected: true,
    remote: "https://example.invalid/vault.git",
    conflicts: [],
    deployments: [],
    deployment_errors: [],
    instructions_changed: false,
    instruction_targets: [],
    message: "与远端一致，无需拉取。",
  })),
}));

import { checkUpstream, sharedDirGet, syncPull } from "../services/client";

afterEach(() => {
  vi.clearAllMocks();
  vi.mocked(sharedDirGet).mockResolvedValue(null);
});

function renderSettings() {
  return render(
    <SettingsPage
      vault={vault}
      uploadFailure={null}
      onRetryUpload={async () => {}}
      settings={settingsFixture}
      onSettingsChange={() => {}}
      agents={[codex, hermes]}
      onAgentsChanged={() => {}}
      onOpenTarget={() => {}}
      reloadToken={0}
    />,
  );
}

/** 页面上的顶层区块（每个 Panel 与 Agent 区块各是一个 section）。 */
function blocks(): HTMLElement[] {
  return Array.from(document.querySelectorAll("section"));
}

describe("SettingsPage · 区块顺序", () => {
  it("首个区块是「支持的 Agent」，含摘要与三个动作，后续区块按原型顺序排列", async () => {
    renderSettings();
    await waitFor(() =>
      expect(screen.getByRole("heading", { name: "支持的 Agent（1/2）" })).toBeTruthy(),
    );

    const sections = blocks();
    // 首个区块：标题（已检测数/内置总数）、摘要（已检测/已启用）与三个动作都在同一区块内。
    const first = sections[0];
    expect(first.textContent).toContain("支持的 Agent（1/2）");
    expect(first.textContent).toContain("已检测 1");
    expect(first.textContent).toContain("已启用 1");
    const scope = within(first);
    expect(scope.getByText("已检测到的 Agent 1")).toBeTruthy();
    expect(scope.getByRole("button", { name: "全部启用" })).toBeTruthy();
    expect(scope.getByRole("button", { name: "全部禁用" })).toBeTruthy();
    expect(scope.getByRole("button", { name: "重新检测" })).toBeTruthy();
    expect(scope.getByText("其他支持的 Agent（1）")).toBeTruthy();

    // 其余区块按 FR-056 顺序（代码块之外不再有 Agent 管理入口）。
    expect(sections[1].textContent).toContain("全局配置");
    expect(sections[2].textContent).toContain("自己的 SkillVault 同步");
    expect(sections[3].textContent).toContain("后台检查与上游更新");
    expect(sections[4].textContent).toContain("诊断与日志");
    expect(sections.length).toBe(5);
  });
});

describe("SettingsPage · 只读展示行", () => {
  it("部署方式/语言/关闭行为是纯展示，所在行没有任何可编辑控件", async () => {
    renderSettings();
    const global = await screen.findByRole("heading", { name: "全局配置" });
    const panel = global.closest("section")!;

    // 值以文本呈现。
    expect(within(panel).getByText("文件复制")).toBeTruthy();
    expect(within(panel).getByText("简体中文")).toBeTruthy();
    expect(within(panel).getByText("托盘模式")).toBeTruthy();

    // 每个只读行内：有标签与说明，且没有输入框、切换开关或分段按钮。
    for (const [label, value] of [
      ["部署方式", "文件复制"],
      ["语言", "简体中文"],
      ["关闭行为", "托盘模式"],
    ]) {
      const row = within(panel).getByText(value).closest("div")!.parentElement!;
      expect(within(row).getByText(label)).toBeTruthy();
      expect(within(row).queryByRole("textbox")).toBeNull();
      expect(within(row).queryByRole("switch")).toBeNull();
      expect(within(row).queryByRole("checkbox")).toBeNull();
      expect(within(row).queryByRole("button")).toBeNull();
    }
  });

  it("共享目录未设置时如实显示，并保留行内修改入口（既有行为不回归）", async () => {
    renderSettings();
    const global = await screen.findByRole("heading", { name: "全局配置" });
    const panel = global.closest("section")!;

    await waitFor(() => expect(within(panel).getByText("未设置")).toBeTruthy());
    // 只读展示，不猜默认路径：未设置时不渲染输入框。
    expect(within(panel).queryByRole("textbox")).toBeNull();

    fireEvent.click(within(panel).getByRole("button", { name: /修改/ }));
    const input = within(panel).getByPlaceholderText("绝对路径");
    expect((input as HTMLInputElement).value).toBe("");
  });
});

describe("SettingsPage · 立即检查", () => {
  it("复用上游检查与库同步两个既有函数，不引入组合命令", async () => {
    renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: /立即检查/ }));

    await waitFor(() => expect(syncPull).toHaveBeenCalledTimes(1));
    // 上游检查仍是库页「检查全部」用的同一调用（全库登记来源，一次覆盖两库）。
    expect(checkUpstream).toHaveBeenCalledTimes(1);
    expect(vi.mocked(checkUpstream).mock.calls[0][0]).toEqual({ all: true });
    await waitFor(() => expect(screen.getByText(/已检查上游变化并同步自己的库/)).toBeTruthy());
  });
});
