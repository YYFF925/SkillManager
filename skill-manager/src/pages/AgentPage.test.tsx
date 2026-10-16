/**
 * Agent 页面（T105/T106/T111）：跨目标部署清单，以及单个目标（共享 Agent 页）复用的本地技能列表视图。
 *
 * 验证直接读取自动定位的共享目录，无需前往设置；按受管／仅本地／本地有变更三态渲染并提供删除入口；
 * 「全部 Agents」渲染部署记录，不再出现 Agent 管理入口（FR-057）。
 */
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { AgentPage } from "./AgentPage";
import type { AgentView, DeploymentStatusView, LocalSkillView } from "../services/client";

/** 受管（已同步）。 */
const managed: LocalSkillView = {
  name: "agent-skill-deployment",
  path: "/tmp/home/.agents/skills/agent-skill-deployment",
  managed: true,
  status: "managed",
  target: "shared",
  library_skill: "code/agent-skill-deployment",
  library_exists: true,
  comment: "技能部署",
};

/** 受管但本地有变更：按撤销与暂停规则处理。 */
const paused: LocalSkillView = {
  name: "spec-kit-cli",
  path: "/tmp/home/.agents/skills/spec-kit-cli",
  managed: true,
  status: "paused_local_change",
  target: "shared",
  library_skill: "code/spec-kit-cli",
  library_exists: true,
};

/** 未受管：直接删除实际文件。 */
const localOnly: LocalSkillView = {
  name: "local-only",
  path: "/tmp/home/.agents/skills/local-only",
  managed: false,
  status: "local",
  target: "shared",
  library_exists: false,
};

/** 自动识别的共享目录内容。 */
const skillsFixture = {
  agent: "shared",
  skills_dir: "/tmp/home/.agents/skills",
  skills: [managed, paused, localOnly],
  managed: 2,
  total: 3,
};

/** 本机内置 Agent（用于部署清单的目标展示名）。 */
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

/** 跨目标部署记录：同一技能部署到两个目标，其一本地有变更。 */
const deployRecords: DeploymentStatusView[] = [
  {
    skill: "code/agent-skill-deployment",
    target: "agent:codex",
    target_path: "/tmp/codex/skills/agent-skill-deployment",
    status: "ok",
    source_hash: "a",
    deployed_hash: "a",
    synced_at: null,
    library_exists: true,
  },
  {
    skill: "code/agent-skill-deployment",
    target: "shared",
    target_path: "/tmp/home/.agents/skills/agent-skill-deployment",
    status: "paused_local_change",
    source_hash: "a",
    deployed_hash: "b",
    synced_at: null,
    library_exists: true,
  },
];

vi.mock("../services/client", () => ({
  agentLocalSkills: vi.fn(async () => skillsFixture),
  agentReadLocal: vi.fn(async (_target: string, name: string, file?: string) => ({
    name,
    path: `/tmp/home/.agents/skills/${name}`,
    file: file ?? "SKILL.md",
    text: file ? "辅助文件内容" : "本机副本正文",
    size: 18,
    files: ["SKILL.md", "references/usage.md"],
  })),
  adoptSkills: vi.fn(async () => ({ adopted: 1, items: [], errors: [] })),
  agentDeleteLocal: vi.fn(async () => ({ removed: [], errors: [] })),
  copySkill: vi.fn(),
  // 部署清单与「从库部署」；默认无记录，用例按需覆盖。
  deployStatus: vi.fn(async () => []),
  listSkills: vi.fn(async () => ({
    skills: [],
    total: 0,
    tag_candidates: [],
    categories: [],
  })),
  deployTo: vi.fn(),
  deployUndo: vi.fn(),
}));

import { agentDeleteLocal, agentLocalSkills, agentReadLocal, deployStatus } from "../services/client";

const noop = () => {};

afterEach(() => {
  vi.clearAllMocks();
  vi.mocked(agentLocalSkills).mockResolvedValue(skillsFixture);
  vi.mocked(deployStatus).mockResolvedValue([]);
});

function renderShared(onMeta = vi.fn()) {
  return render(
    <AgentPage
      domain="code"
      selectedTarget="shared"
      agents={[]}
      onMeta={onMeta}
      onOpenInLibrary={noop}
      onNotice={noop}
    />,
  );
}

describe("AgentPage · 共享 Agent", () => {
  it("直接读取自动识别的共享目标，无需前往设置", async () => {
    const onMeta = vi.fn();
    renderShared(onMeta);
    await waitFor(() => expect(agentLocalSkills).toHaveBeenCalledWith("shared"));
    await waitFor(() =>
      expect(onMeta).toHaveBeenLastCalledWith({
        count: 3,
        subtitle: "/tmp/home/.agents/skills · 2 个受管理",
      }),
    );
    expect(screen.queryByRole("button", { name: "前往设置" })).toBeNull();
  });

  it("按三态渲染共享技能，并提供删除入口", async () => {
    const onMeta = vi.fn();
    renderShared(onMeta);

    await waitFor(() => expect(screen.getByText("agent-skill-deployment")).toBeTruthy());
    expect(agentLocalSkills).toHaveBeenCalledWith("shared");
    await waitFor(() =>
      expect(onMeta).toHaveBeenLastCalledWith({
        count: 3,
        subtitle: "/tmp/home/.agents/skills · 2 个受管理",
      }),
    );

    // 三态在卡片上分别表达；筛选控件用的是“仅本地/有变更”，故按卡片断言。
    const managedCard = screen.getByText("agent-skill-deployment").closest("article")!;
    const pausedCard = screen.getByText("spec-kit-cli").closest("article")!;
    const localCard = screen.getByText("local-only").closest("article")!;
    expect(within(managedCard).getByText("已同步")).toBeTruthy();
    expect(within(pausedCard).getByText("本地有变更")).toBeTruthy();
    expect(within(localCard).getByText("仅本地")).toBeTruthy();
    expect(within(managedCard).getByText("受管 · code/agent-skill-deployment")).toBeTruthy();

    expect(screen.getByTitle("收编 local-only 到技能库并接管原部署")).toBeTruthy();

    // 未受管项提供删除入口，删除按目标传参。
    fireEvent.click(screen.getByTitle("删除本机副本 local-only"));
    await waitFor(() =>
      expect(screen.getByText(/受管理项按撤销规则处理/)).toBeTruthy(),
    );
    fireEvent.click(screen.getByRole("button", { name: "删除" }));
    await waitFor(() => expect(agentDeleteLocal).toHaveBeenCalledWith("shared", ["local-only"]));
    expect(agentReadLocal).not.toHaveBeenCalled();
  });

  it("目录为空时保持空白，不显示空状态或底部说明", async () => {
    vi.mocked(agentLocalSkills).mockResolvedValue({
      agent: "shared",
      skills_dir: "/tmp/home/.agents/skills",
      skills: [],
      managed: 0,
      total: 0,
    });
    const onMeta = vi.fn();
    renderShared(onMeta);

    await waitFor(() =>
      expect(onMeta).toHaveBeenLastCalledWith({
        count: 0,
        subtitle: "/tmp/home/.agents/skills · 0 个受管理",
      }),
    );
    expect(screen.queryByText(/没有技能|没有符合筛选条件|仅本地：/)).toBeNull();
    expect(screen.queryByRole("article")).toBeNull();
  });

  it("点击卡片阅读本机正文与文件，勾选框不打开详情", async () => {
    renderShared();
    const title = await screen.findByText("local-only");
    const card = title.closest("article")!;
    fireEvent.click(screen.getByRole("button", { name: "多选" }));
    fireEvent.click(within(card).getByRole("checkbox"));
    expect(agentReadLocal).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "取消多选" }));
    fireEvent.click(card);
    await screen.findByText("本机副本正文");
    expect(agentReadLocal).toHaveBeenCalledWith("shared", "local-only", undefined);
    fireEvent.click(screen.getByRole("button", { name: "文件" }));
    fireEvent.click(screen.getByRole("button", { name: "references/usage.md" }));
    await screen.findByText("辅助文件内容");
    fireEvent.click(screen.getByText("关闭", { selector: "button" }));
    fireEvent.click(screen.getByTitle("收编 local-only 到技能库并接管原部署"));
    await screen.findByText("收编本机技能");
    expect(screen.queryByText("本机副本正文")).toBeNull();
  });
});

describe("AgentPage · 全部编码 Agents（跨目标部署清单）", () => {
  it("按技能 × 目标渲染部署记录，且不再出现 Agent 管理入口", async () => {
    vi.mocked(deployStatus).mockResolvedValue(deployRecords);
    render(
      <AgentPage
        domain="code"
        selectedTarget={null}
        agents={[codex]}
        onMeta={noop}
        onOpenInLibrary={noop}
        onNotice={noop}
      />,
    );

    // 同一技能的两个目标都出现，目标名与路径、库内引用、状态都在卡片上。
    await waitFor(() => expect(screen.getAllByText("agent-skill-deployment")).toHaveLength(2));
    expect(deployStatus).toHaveBeenCalled();
    expect(screen.getByText("目标：Codex")).toBeTruthy();
    expect(screen.getByText("目标：共享目录")).toBeTruthy();
    expect(screen.getAllByText("库内引用 code/agent-skill-deployment")).toHaveLength(2);
    expect(screen.getByText("/tmp/codex/skills/agent-skill-deployment")).toBeTruthy();
    expect(screen.getByText("已同步")).toBeTruthy();
    expect(screen.getByText("本地有变更")).toBeTruthy();

    // Agent 管理（检测/启停/排序/目录编辑）已迁到设置页，本页不再出现。
    expect(screen.queryByText("重新检测")).toBeNull();
    expect(screen.queryByText(/其他支持的 Agent/)).toBeNull();
    expect(screen.queryByTitle("修改技能目录")).toBeNull();
    expect(screen.queryByRole("switch")).toBeNull();
  });

  it("按状态筛选部署记录（已部署 / 有变更）", async () => {
    vi.mocked(deployStatus).mockResolvedValue(deployRecords);
    render(
      <AgentPage
        domain="code"
        selectedTarget={null}
        agents={[codex]}
        onMeta={noop}
        onOpenInLibrary={noop}
        onNotice={noop}
      />,
    );
    await waitFor(() => expect(screen.getAllByText("agent-skill-deployment")).toHaveLength(2));

    fireEvent.click(screen.getByRole("button", { name: "有变更" }));
    expect(screen.getAllByText("agent-skill-deployment")).toHaveLength(1);
    expect(screen.getByText("目标：共享目录")).toBeTruthy();
    expect(screen.queryByText("目标：Codex")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "已部署" }));
    expect(screen.getByText("目标：Codex")).toBeTruthy();
    expect(screen.queryByText("目标：共享目录")).toBeNull();
  });
});
