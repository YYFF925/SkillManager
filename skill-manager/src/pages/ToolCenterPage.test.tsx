/**
 * 工具中心界面：未安装条目保留并如实表达状态、配套 Skill 候选入库入口与选择流程（T080/T081）。
 *
 * 只验证界面与 core 输出之间的接线：不重复业务判断，也不断言 core 内部规则。
 */
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ToolCenterPage } from "./ToolCenterPage";
import type { ToolEntry } from "../services/client";

const browserUse: ToolEntry = {
  slug: "browser-use",
  name: "browser-use",
  summary: "通过命令行提供浏览器自动化，供 Agent 按任务调用。",
  tags: ["页面交互", "自动化"],
  category: "浏览器",
  dir: "/tmp/vault/tools/browser-use",
  has_guide: true,
  has_check_command: true,
  skills: [
    {
      domain: "code",
      name: "browser-use",
      repo: "https://example.invalid/bu.git",
      subpath: "browser-use",
      in_vault: false,
      vault_path: null,
      comment: null,
      tags: [],
      category: null,
      source_version: null,
    },
  ],
  install: {
    version: "0.12.0",
    method: "uv 工具环境",
    path: "~/.local/bin/browser-use",
    config_path: "~/.config/browser-use",
  },
  check: {
    status: "checked",
    latest_version: "0.13.0",
    checked_at: "2026-10-15T00:00:00Z",
    output: "检查成功",
  },
  comparison: "updatable",
  comparison_reason: null,
};

/** 未安装且未登记检查命令：保留显示，不派生“本机最新/可更新”。 */
const ripgrep: ToolEntry = {
  slug: "ripgrep",
  name: "ripgrep",
  summary: "快速搜索本地文件内容。",
  tags: ["cli"],
  category: "命令行工具",
  dir: "/tmp/vault/tools/ripgrep",
  has_guide: false,
  has_check_command: false,
  skills: [],
  install: null,
  check: {
    status: "no_command",
    latest_version: null,
    checked_at: null,
    output: "未登记检查命令",
  },
  comparison: null,
  comparison_reason: null,
};

vi.mock("../services/client", () => ({
  toolList: vi.fn(async () => [browserUse, ripgrep]),
  toolGet: vi.fn(async () => ({
    ...browserUse,
    intro: "browser-use 是用于浏览器任务的自动化工具。",
    check_command: "sh ./check-latest.sh",
    guide: "# 维护指南\n用途与调用入口：browser-use\n",
  })),
  toolCheck: vi.fn(async () => ({ checked: 2, tools: [] })),
  toolTask: vi.fn(async () => "任务说明"),
  toolSkillCandidates: vi.fn(async () => ({
    slug: "browser-use",
    name: "browser-use",
    tool: {
      installed: true,
      installed_version: "0.12.0",
      status: "checked",
      latest_version: "0.13.0",
      comparison: "updatable",
      comparison_reason: null,
    },
    candidates: [
      {
        domain: "code",
        name: "browser-use",
        repo: "https://example.invalid/bu.git",
        subpath: "browser-use",
        in_vault: false,
        vault_path: null,
        description: null,
        comment: null,
        tags: [],
        category: null,
        source_version: null,
      },
    ],
  })),
  previewRepo: vi.fn(async () => ({ candidates: [{ name: "browser-use", subpath: "browser-use", description: "浏览器操作", comment: null, tags: [] }] })),
  onFetchProgress: vi.fn(async () => () => {}),
  cancelFetch: vi.fn(async () => {}),
  importSkills: vi.fn(async () => ({
    items: [{ subpath: "browser-use", skill: "code", name: "browser-use", category: null, version: "abc", path: "/tmp/x" }],
    errors: [],
  })),
  setSkill: vi.fn(async () => ({})),
}));

import { importSkills, previewRepo, toolSkillCandidates } from "../services/client";

afterEach(() => {
  vi.clearAllMocks();
});

describe("ToolCenterPage", () => {
  it("keeps uninstalled tools visible and expresses each check state separately", async () => {
    render(
      <ToolCenterPage reloadToken={0} onCount={() => {}} />,
    );

    await waitFor(() => expect(screen.getByText("browser-use")).toBeTruthy());

    // 未安装条目保留显示，并以“未安装”表达；未登记命令不冒充“未检查”。
    expect(screen.getByText("ripgrep")).toBeTruthy();
    expect(screen.getAllByText("未安装").length).toBeGreaterThan(0);
    expect(screen.getByText("未登记检查命令")).toBeTruthy();
    // 已安装 + 上游更新 → 有更新；不显示“未安装”之外的伪造结论。
    expect(screen.getAllByText("有更新").length).toBeGreaterThan(0);
    expect(screen.getByText("有 Skill · 待入库 1")).toBeTruthy();
    expect(screen.getAllByText("待补维护指南").length).toBeGreaterThan(0);
  });

  it("opens the companion-skill intake dialog and imports only after explicit selection", async () => {
    render(<ToolCenterPage reloadToken={0} onCount={() => {}} />);
    await waitFor(() => expect(screen.getByText("browser-use")).toBeTruthy());

    // 打开工具详情 → 概览里的“选择并入库…”。
    fireEvent.click(screen.getByText("browser-use"));
    await waitFor(() => expect(toolSkillCandidates).toBeTruthy());
    const intakeButton = await screen.findByRole("button", { name: /选择并入库/ });
    fireEvent.click(intakeButton);

    // 单候选也列出原名、来源与已选数量；未勾选任何候选时不能确认。
    await waitFor(() => expect(screen.getByText(/配套 Skill 入库 · browser-use/)).toBeTruthy());
    expect(screen.getByText(/已选 0 \/ 1/)).toBeTruthy();
    const confirm = screen.getByRole("button", { name: "确认入库" });
    expect(confirm).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "预览来源资料" }));
    await screen.findByText("浏览器操作");
    expect(importSkills).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("checkbox"));
    fireEvent.click(confirm);

    // 复用 US2 预览 + 入库流程；不自动部署（本模块没有部署调用）。
    await waitFor(() => expect(previewRepo).toHaveBeenCalledTimes(1));
    expect(previewRepo).toHaveBeenCalledWith("https://example.invalid/bu.git", undefined, expect.any(String));
    await waitFor(() => expect(importSkills).toHaveBeenCalledTimes(1));
    const [repo, domain, subpaths] = vi.mocked(importSkills).mock.calls[0];
    expect(repo).toBe("https://example.invalid/bu.git");
    expect(domain).toBe("lobster");
    expect(subpaths).toEqual(["browser-use"]);
  });
});
