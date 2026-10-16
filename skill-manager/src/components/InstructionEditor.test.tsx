import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { InstructionEditor } from "./InstructionEditor";
import type { AgentView } from "../services/client";

vi.mock("../services/client", () => ({
  instructionGet: vi.fn(async (scope: string) => ({
    scope,
    source: scope === "global" ? "GLOBAL" : "",
    composed: scope === "global" ? "GLOBAL" : "GLOBAL",
  })),
  instructionSet: vi.fn(async () => [
    { agent: "cursor", path: "/tmp/cursor/AGENTS.md", ok: true, error: null },
  ]),
}));

const agents: AgentView[] = [
  {
    key: "cursor",
    display_name: "Cursor",
    domain: "code",
    detected: true,
    enabled: true,
    order: 1,
    skills_dir: "/tmp/cursor/skills",
    skills_dir_custom: false,
    skills_dir_default: "/tmp/cursor/skills",
    project_rel_dir: null,
    project_rel_dir_default: null,
    instructions_target: "/tmp/cursor/AGENTS.md",
    icon: null,
    invert_in_dark: false,
  },
];

afterEach(() => {
  vi.clearAllMocks();
});

function textarea(container: HTMLElement): HTMLTextAreaElement {
  const node = container.querySelector("textarea");
  if (!node) {
    throw new Error("textarea not found");
  }
  return node as HTMLTextAreaElement;
}

describe("InstructionEditor", () => {
  it("keeps a single editor, retains drafts across tools, and stays open after saving", async () => {
    const { container } = render(
      <InstructionEditor agents={agents} onClose={() => {}} onSaved={() => {}} />,
    );

    await waitFor(() => expect(textarea(container).value).toBe("GLOBAL"));

    // 编辑全局草稿。
    fireEvent.change(textarea(container), { target: { value: "GLOBAL EDIT" } });

    // 切换到 Cursor：只有一名编辑区，显示该 Agent 的源（空）。
    fireEvent.click(screen.getByRole("button", { name: /Cursor/ }));
    await waitFor(() => expect(textarea(container).value).toBe(""));
    expect(screen.getByText("该 Agent 暂无专属内容，将仅使用全局指令。")).toBeTruthy();

    // 切回全局：未保存草稿保留。
    fireEvent.click(screen.getByRole("button", { name: "全局指令" }));
    await waitFor(() => expect(textarea(container).value).toBe("GLOBAL EDIT"));

    // 单个“保存”按钮；保存后弹窗保持打开并逐项显示目标结果。
    const saveButtons = screen.getAllByRole("button", { name: "保存" });
    expect(saveButtons).toHaveLength(1);
    fireEvent.click(saveButtons[0]);

    await waitFor(() => expect(container.textContent).toContain("已刷新 Cursor"));
    expect(screen.getByText("Agent 指令文件")).toBeTruthy();
  });
});
