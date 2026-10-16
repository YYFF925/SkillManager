import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DeployDialog } from "./DeployDialog";
import { PresetDeployDialog } from "./PresetDeployDialog";
import { QuickDeploy } from "./QuickDeploy";
import { deployStatus, deployTo, deployUndo, getSkill, type AgentView, type DeploymentStatusView } from "../services/client";

vi.mock("../services/client", () => ({ deployStatus: vi.fn(), deployTo: vi.fn(), deployUndo: vi.fn(), getSkill: vi.fn() }));
const agent = (key: string, extra = {}): AgentView => ({ key, display_name: key, domain: "code", detected: true, enabled: true, order: 0, skills_dir: `/tmp/${key}/skills`, skills_dir_custom: false, skills_dir_default: `/tmp/${key}/skills`, project_rel_dir: null, project_rel_dir_default: null, instructions_target: "/tmp/AGENTS.md", icon: null, invert_in_dark: false, ...extra });
const agents = [agent("codex"), agent("claude"), agent("disabled", { enabled: false }), agent("undetected", { detected: false })];
const record = (skill: string, target: string): DeploymentStatusView => ({ deployment_blocks: {}, skill, target: `agent:${target}`, target_path: `/tmp/${target}/${skill}`, status: "ok", source_hash: null, deployed_hash: null, synced_at: null, library_exists: true });
const result = (target: string, refs: string[], action: string) => ({ items: refs.map((skill) => ({ deployment_blocks: {}, skill, target: `agent:${target}`, path: `/tmp/${target}/${skill}`, action, status: "ok" as const })), errors: [] });

const skillDetail = (reference: string, blocks: Record<string, string> = {}): Awaited<ReturnType<typeof getSkill>> => ({
  skill: { domain: "code", name: reference.split('/')[1], deployment_blocks: blocks, category: null, comment: null, tags: [], description: null, agents: null, oses: null, has_translation: false, translation_stale: false, vault_rel_dir: reference, path: `/tmp/${reference}`, entry_file: 'SKILL.md', source: null, manager_error: null, presets: [] },
  content: { original: '正文', translation: null, has_translation: false, translation_stale: false, default_lang: 'orig' },
  vault_path: '/tmp', files: [], deployments: [], deployments_note: '',
});

beforeEach(() => {
  vi.mocked(getSkill).mockImplementation(async (reference) => skillDetail(reference));
  vi.mocked(deployStatus).mockResolvedValue([record("code/alpha", "codex")]);
  vi.mocked(deployTo).mockImplementation(async (target, refs) => result(target, refs, "deployed"));
  vi.mocked(deployUndo).mockImplementation(async (target, refs) => result(target, refs, "removed"));
});
afterEach(() => { cleanup(); vi.resetAllMocks(); });
async function renderDialog(props = {}) {
  render(<DeployDialog domain="code" references={["code/alpha"]} agents={agents} onClose={() => { }} onDone={() => { }} {...props} />);
  await waitFor(() => expect(screen.queryByText("正在读取部署状态…")).toBeNull());
}

describe("部署选择与保存", () => {
  it("载入实际状态，只列检测且启用的 Agent，取消预演与模式切换", async () => {
    await renderDialog();
    expect(screen.getByRole("button", { name: "codex" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "claude" })).toHaveAttribute("aria-pressed", "false");
    expect(screen.queryByText("disabled")).toBeNull();
    expect(screen.queryByText("undetected")).toBeNull();
    expect(screen.queryByText(/预演|其他支持项|按管理域|撤销保留/)).toBeNull();
    expect(screen.queryByRole("button", { name: "撤销" })).toBeNull();
    expect(screen.getByRole("button", { name: "保存" })).toBeDisabled();
  });
  it("保存前无副作用，保存一次同时部署与撤销，只处理变化项", async () => {
    const done = vi.fn();
    await renderDialog({ onDone: done });
    fireEvent.click(screen.getByRole("button", { name: "codex" }));
    fireEvent.click(screen.getByRole("button", { name: "claude" }));
    expect(deployTo).not.toHaveBeenCalled();
    expect(deployUndo).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(done).toHaveBeenCalled());
    expect(deployTo).toHaveBeenCalledExactlyOnceWith("claude", ["code/alpha"]);
    expect(deployUndo).toHaveBeenCalledExactlyOnceWith("codex", ["code/alpha"]);
  });
  it("多技能部分部署保持原状态，选择补齐时只新增缺失技能", async () => {
    await renderDialog({ references: ["code/alpha", "code/beta"] });
    expect(screen.getByRole("button", { name: "codex 1/2" })).toHaveAttribute("aria-pressed", "mixed");
    expect(screen.getByRole("button", { name: "保存" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "codex 1/2" }));
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(deployTo).toHaveBeenCalledExactlyOnceWith("codex", ["code/beta"]));
    expect(deployUndo).not.toHaveBeenCalled();
  });
  it("不受支持时阻止新增但允许撤销已有部署", async () => {
    await renderDialog({ unsupportedReason: "不支持当前系统" });
    fireEvent.click(screen.getByRole("button", { name: "claude" }));
    expect(screen.getByRole("button", { name: "保存" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "claude" }));
    fireEvent.click(screen.getByRole("button", { name: "codex" }));
    expect(screen.getByRole("button", { name: "保存" })).not.toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(deployUndo).toHaveBeenCalled());
    expect(deployTo).not.toHaveBeenCalled();
  });
  it("部分失败留在弹窗显示原因，重试不重复已成功项", async () => {
    const done = vi.fn();
    vi.mocked(deployTo).mockResolvedValueOnce({ items: [], errors: [{ skill: "code/alpha", target: "agent:claude", message: "部署暂停" }] });
    await renderDialog({ onDone: done });
    fireEvent.click(screen.getByRole("button", { name: "codex" }));
    fireEvent.click(screen.getByRole("button", { name: "claude" }));
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await screen.findByText(/部署暂停/);
    expect(done).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(done).toHaveBeenCalled());
    expect(deployUndo).toHaveBeenCalledTimes(1);
    expect(deployTo).toHaveBeenCalledTimes(2);
  });
  it("部署记录读取失败时禁止保存", async () => {
    vi.mocked(deployStatus).mockRejectedValue(new Error("读取失败"));
    await renderDialog();
    expect(screen.getByText(/读取失败/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "保存" })).toBeDisabled();
  });
  it("预设使用同一保存交互，撤销时保留跨预设影响提示", async () => {
    render(<PresetDeployDialog domain="code" preset={{ domain: "code", id: "pack", name: "组合", icon: null, order: 0, members: [{ name: "alpha", missing: false }], missing_members: [] }} agents={agents} sharedMembers={["alpha"]} onClose={() => { }} onDone={() => { }} />);
    await waitFor(() => expect(screen.getByRole("button", { name: "codex" })).toHaveAttribute("aria-pressed", "true"));
    fireEvent.click(screen.getByRole("button", { name: "codex" }));
    expect(screen.getByText(/其他预设成员/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(deployUndo).toHaveBeenCalledExactlyOnceWith("codex", ["code/alpha"]));
  });
});

describe("支持声明", () => {
  it("禁止新增不支持目标，但已有部署仍可撤销", async () => {
    vi.mocked(getSkill).mockImplementation(async (reference) => skillDetail(reference, { 'agent:claude': '仅支持 codex', 'agent:codex': '已收窄' }));
    await renderDialog();
    expect(screen.getByRole('button', { name: 'claude' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'codex' })).not.toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'codex' }));
    fireEvent.click(screen.getByRole('button', { name: '保存' }));
    await waitFor(() => expect(deployUndo).toHaveBeenCalled());
    expect(deployTo).not.toHaveBeenCalled();
  });
});

describe("卡片快捷部署", () => {
  it("图标按记录亮显，点击立即撤销且不打开卡片", async () => {
    const change = vi.fn(), open = vi.fn();
    render(<article onClick={open}><QuickDeploy deploymentBlocks={{}} reference="code/alpha" domain="code" agents={agents} records={[record("code/alpha", "codex")]} onChange={change} onError={vi.fn()} /></article>);
    expect(screen.getByRole("button", { name: "撤销部署 codex" })).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(screen.getByRole("button", { name: "撤销部署 codex" }));
    await waitFor(() => expect(change).toHaveBeenCalled());
    expect(deployUndo).toHaveBeenCalledExactlyOnceWith("codex", ["code/alpha"]);
    expect(open).not.toHaveBeenCalled();
  });
  it("灰暗图标点击部署，失败如实报告", async () => {
    const error = vi.fn();
    vi.mocked(deployTo).mockResolvedValue({ items: [], errors: [{ message: "权限不足" }] });
    render(<QuickDeploy deploymentBlocks={{}} reference="code/alpha" domain="code" agents={agents} records={[]} onChange={vi.fn()} onError={error} />);
    fireEvent.click(screen.getByRole("button", { name: "部署到 claude" }));
    await waitFor(() => expect(error).toHaveBeenCalledWith("权限不足"));
    expect(deployTo).toHaveBeenCalledExactlyOnceWith("claude", ["code/alpha"]);
  });
  it("未受管卡片不能直接部署，需先纳入管理", () => {
    render(<QuickDeploy deploymentBlocks={{}} domain="code" agents={agents} records={[]} onChange={vi.fn()} onError={vi.fn()} />);
    expect(screen.getByRole("button", { name: "codex · 先纳入管理" })).toBeDisabled();
  });
});
