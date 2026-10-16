/**
 * 部署弹窗“不受支持时阻止新增部署”的行为：
 * DeployDialog（技能多选）与 PresetDeployDialog（预设组合）。
 *
 * 只渲染弹窗本身，`../services/client` 全部 mock 且不应被调用；目标选项由
 * `../lib/targets` 的纯函数与传入的本机 Agent 视图决定，不依赖真实 HOME。
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { DeployDialog } from "./DeployDialog";
import { PresetDeployDialog } from "./PresetDeployDialog";
import {
  deployPreset,
  deployTo,
  deployUndo,
  deployUndoPreset,
  type AgentView,
  type PresetView,
} from "../services/client";

vi.mock("../services/client", () => ({
  deployTo: vi.fn(),
  deployUndo: vi.fn(),
  deployPreset: vi.fn(),
  deployUndoPreset: vi.fn(),
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

const REASON = "该技能的支持声明不含当前系统（macos），无法新增部署。";

const agents: AgentView[] = [
  {
    key: "claude",
    display_name: "Claude Code",
    domain: "code",
    detected: true,
    enabled: true,
    order: 0,
    skills_dir: "/tmp/home/.claude/skills",
    skills_dir_custom: false,
    skills_dir_default: "/tmp/home/.claude/skills",
    project_rel_dir: null,
    project_rel_dir_default: null,
    instructions_target: "/tmp/home/.claude/CLAUDE.md",
    icon: null,
    invert_in_dark: false,
  },
];

const preset: PresetView = {
  domain: "code",
  id: "alpha-pack",
  name: "Alpha 组合",
  icon: null,
  order: 0,
  members: [
    { name: "alpha", missing: false },
    { name: "beta", missing: false },
  ],
  missing_members: [],
};

/**
 * 页脚主操作按钮。
 *
 * 不能按 `getByRole("button", { name: "部署" })` 定位：分段控件的「部署」选项与主按钮同名，
 * 会同时命中两个元素；这里由与主按钮同级的「关闭」按钮取下一个兄弟元素。
 */
function primaryButton(): HTMLButtonElement {
  const close = screen.getByText("关闭").closest("button");
  const primary = close?.nextElementSibling;
  if (!(primary instanceof HTMLButtonElement)) {
    throw new Error("未找到页脚主操作按钮");
  }
  return primary;
}

function renderDeployDialog(unsupportedReason?: string | null) {
  return render(
    <DeployDialog
      domain="code"
      references={["code/alpha"]}
      agents={agents}
      sharedDir="/tmp/shared-skills"
      unsupportedReason={unsupportedReason}
      onSharedDirChange={async () => {}}
      onClose={() => {}}
      onDone={() => {}}
    />,
  );
}

describe("DeployDialog", () => {
  it("部署模式下选中项全部不受支持：展示原因并禁用部署按钮", () => {
    renderDeployDialog(REASON);
    // 已选目标，使禁用只可能来自“不支持声明”，而不是 targets.length === 0。
    fireEvent.click(screen.getByRole("button", { name: "共享目录" }));

    expect(screen.getByText(REASON)).toBeTruthy();
    expect(primaryButton()).toBeDisabled();
    expect(primaryButton()).toHaveAttribute("title", REASON);
    expect(deployTo).not.toHaveBeenCalled();
    expect(deployUndo).not.toHaveBeenCalled();
  });

  it("切到撤销模式：提示消失且部署按钮恢复可用", () => {
    renderDeployDialog(REASON);
    // 已选目标，排除 targets.length === 0 的禁用，只让“不支持声明”决定按钮状态。
    fireEvent.click(screen.getByRole("button", { name: "共享目录" }));
    expect(primaryButton()).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "撤销" }));

    expect(screen.queryByText(REASON)).toBeNull();
    expect(primaryButton()).not.toBeDisabled();
  });

  it("未给不支持原因且已选中目标时按钮可点", () => {
    renderDeployDialog(null);

    fireEvent.click(screen.getByRole("button", { name: "共享目录" }));

    expect(primaryButton()).not.toBeDisabled();
  });
});

describe("PresetDeployDialog", () => {
  function renderDialog(unsupportedReason?: string | null) {
    return render(
      <PresetDeployDialog
        domain="code"
        preset={preset}
        agents={agents}
        sharedMembers={[]}
        unsupportedReason={unsupportedReason}
        onClose={() => {}}
        onDone={() => {}}
      />,
    );
  }

  it("部署模式下成员全部不受支持：展示原因并禁用叠加部署按钮", () => {
    renderDialog(REASON);
    // 已选目标，使禁用只可能来自“不支持声明”，而不是 targets.length === 0。
    fireEvent.click(screen.getByRole("button", { name: "共享目录" }));

    expect(screen.getByText(REASON)).toBeTruthy();
    const primary = screen.getByRole("button", { name: "叠加部署" });
    expect(primary).toBeDisabled();
    expect(primary).toHaveAttribute("title", REASON);
    expect(deployPreset).not.toHaveBeenCalled();
    expect(deployUndoPreset).not.toHaveBeenCalled();
  });

  it("切到撤销模式：提示消失且叠加部署按钮恢复可用", () => {
    renderDialog(REASON);
    // 已选目标，排除 targets.length === 0 的禁用，只让“不支持声明”决定按钮状态。
    fireEvent.click(screen.getByRole("button", { name: "共享目录" }));
    expect(primaryButton()).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "撤销" }));

    expect(screen.queryByText(REASON)).toBeNull();
    expect(primaryButton()).not.toBeDisabled();
  });

  it("未给不支持原因且已选中目标时按钮可点", () => {
    renderDialog(null);

    fireEvent.click(screen.getByRole("button", { name: "共享目录" }));

    expect(primaryButton()).not.toBeDisabled();
  });
});
