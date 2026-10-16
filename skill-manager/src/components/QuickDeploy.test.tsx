import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { QuickDeploy } from "./QuickDeploy";
import type { AgentView } from "../services/client";

vi.mock("../services/client", () => ({
  deployStatus: vi.fn(async () => []),
  deployTo: vi.fn(),
  deployUndo: vi.fn(),
}));

const codex: AgentView = {
  key: "codex",
  display_name: "Codex",
  domain: "code",
  detected: true,
  enabled: true,
  order: 0,
  skills_dir: "/tmp/codex/skills",
  skills_dir_custom: false,
  skills_dir_default: "/tmp/codex/skills",
  project_rel_dir: null,
  project_rel_dir_default: null,
  instructions_target: "/tmp/codex/AGENTS.md",
  icon: null,
  invert_in_dark: false,
};

describe("QuickDeploy", () => {
  it("编码库快捷部署包含共享 Agent，并以即时提示显示名称", () => {
    render(
      <QuickDeploy
        reference="code/shared-credentials"
        domain="code"
        agents={[codex]}
        records={[]}
        onChange={() => {}}
        onError={() => {}}
      />,
    );

    expect(screen.getByRole("button", { name: "部署到 共享 Agent" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "部署到 Codex" })).toBeTruthy();
    expect(screen.getByRole("tooltip", { name: "共享 Agent" })).toBeTruthy();
  });
});
