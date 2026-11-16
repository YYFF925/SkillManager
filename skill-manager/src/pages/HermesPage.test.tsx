import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { HermesPage } from "./HermesPage";

vi.mock("../services/client", () => ({
  isDesktop: () => true,
  hermesProfiles: vi.fn(async () => ({
    root: "/tmp/home/.hermes",
    profiles: [
      {
        name: "default",
        display_name: null,
        kind: "default",
        path: "/tmp/home/.hermes",
        skills_dir: "/tmp/home/.hermes/skills",
        enabled_count: 1,
      },
      {
        name: "blank",
        display_name: null,
        kind: "named",
        path: "/tmp/home/.hermes/profiles/blank",
        skills_dir: "/tmp/home/.hermes/profiles/blank/skills",
        enabled_count: 1,
      },
      {
        name: "u7ed8-u753b-u52a9-u624b",
        display_name: "绘画助手",
        kind: "named",
        path: "/tmp/home/.hermes/profiles/u7ed8-u753b-u52a9-u624b",
        skills_dir: "/tmp/home/.hermes/profiles/u7ed8-u753b-u52a9-u624b/skills",
        enabled_count: 0,
      },
    ],
  })),
  hermesList: vi.fn(async () => ({
    profile: "blank",
    kind: "named",
    path: "/tmp/home/.hermes/profiles/blank",
    skills_dir: "/tmp/home/.hermes/profiles/blank/skills",
    root: "/tmp/home/.hermes",
    default_skills_dir: "/tmp/home/.hermes/skills",
    skills: [
      {
        name: "alpha",
        domain: "lobster",
        library_skill: "lobster/alpha",
        in_library: true,
        in_default_pool: true,
        enabled: true,
        needs_adoption: false,
        tags: ["ops"],
        agents: null,
        oses: null,
        has_translation: false,
        translation_stale: false,
      },
    ],
    total: 1,
    tag_candidates: ["ops"],
    categories: [],
  })),
  hermesEnable: vi.fn(),
  hermesDisable: vi.fn(),
  hermesEnablePreset: vi.fn(),
  hermesDisablePreset: vi.fn(),
  hermesAdopt: vi.fn(),
  hermesRead: vi.fn(async () => ({ name: "local-only", path: "/tmp/profile/skills/local-only", file: "SKILL.md", text: "# 独有正文", size: 20, files: ["SKILL.md"] })),
  hermesIdentityGet: vi.fn(async () => ({ scope: "global", source: "", composed: "" })),
  hermesIdentitySet: vi.fn(async () => []),
  listPresets: vi.fn(async () => []),
  deployStatus: vi.fn(async () => []),
  renameCategory: vi.fn(async () => ({ domain: "lobster", from: "a", to: "b", affected: 0 })),
}));

import {
  hermesList,
  hermesAdopt,
  hermesRead,
  hermesEnablePreset,
  hermesDisablePreset,
  listPresets,
} from "../services/client";

afterEach(() => {
  vi.clearAllMocks();

});

describe("HermesPage", () => {
  it("独有技能可先只读查看，不触发收编", async () => {
    const view = await hermesList("blank");
    vi.mocked(hermesList).mockResolvedValueOnce({ ...view, skills: [{ ...view.skills[0], deployment_blocks: {}, name: "local-only", library_skill: undefined, in_library: false, needs_adoption: true, path: "/tmp/profile/skills/local-only" }] });
    render(<HermesPage agents={[]} os="macos" reloadToken={0} onCount={vi.fn()} onOpenInLibrary={vi.fn()} />);
    fireEvent.click(await screen.findByRole("button", { name: "local-only" }));
    expect(await screen.findByRole("heading", { name: "独有正文" })).toBeTruthy();
    expect(hermesRead).toHaveBeenCalled();
    expect(hermesAdopt).not.toHaveBeenCalled();
  });
  it("renders discovered profiles and the profile skill list", async () => {
    render(
      <HermesPage
        agents={[]}
        os="macos"
        reloadToken={0}
        onCount={() => {}}
        onOpenInLibrary={() => {}}
      />,
    );
    await waitFor(() => expect(screen.getByText("default")).toBeTruthy());
    await waitFor(() => expect(screen.getByText("alpha")).toBeTruthy());
    fireEvent.click(screen.getByTitle("选择档案"));
    expect(screen.getByText("绘画助手")).toBeTruthy();
    expect(screen.queryByText("u7ed8-u753b-u52a9-u624b")).toBeNull();
    expect(screen.queryByText("本机已发现的档案，不管理档案生命周期。")).toBeNull();
    expect(screen.getByText("其他档案从 default 候选池显式启用，新增到 default 的技能不会自动启用到其他档案。")).toBeTruthy();
    expect(screen.getByRole("button", { name: "新建分类" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "SOUL" })).toBeTruthy();
  });

  it("具名档案在标签上方展示龙虾预设并支持一键启用与禁用，default 不展示", async () => {
    vi.mocked(listPresets).mockResolvedValue([
      {
        domain: "lobster",
        id: "ops",
        name: "运维必备",
        icon: "wrench",
        order: 0,
        members: [
          { name: "alpha", missing: false },
          { name: "beta", missing: false },
        ],
        missing_members: [],
      },
    ]);
    vi.mocked(hermesEnablePreset).mockResolvedValue({
      profile: "blank",
      preset_id: "ops",
      preset_name: "运维必备",
      enabled: true,
      installed: 1,
      changed: 2,
      items: [
        { name: "alpha", ok: true, enabled: true },
        { name: "beta", ok: true, enabled: true },
      ],
    });
    vi.mocked(hermesDisablePreset).mockResolvedValue({
      profile: "blank",
      preset_id: "ops",
      preset_name: "运维必备",
      enabled: false,
      installed: 0,
      changed: 2,
      items: [
        { name: "alpha", ok: true, enabled: false },
        { name: "beta", ok: true, enabled: false },
      ],
    });

    render(
      <HermesPage
        agents={[]}
        os="macos"
        reloadToken={0}
        onCount={() => {}}
        onOpenInLibrary={() => {}}
      />,
    );

    // 默认选中第一个档案 default，不展示预设条。
    await waitFor(() => expect(screen.getByText("alpha")).toBeTruthy());
    expect(screen.queryByRole("button", { name: /运维必备/ })).toBeNull();

    // 切换到具名档案 blank：显示预设条（1/2 部分启用状态）。
    fireEvent.click(screen.getByTitle("选择档案"));
    fireEvent.click(screen.getByText("blank"));
    const presetBtn = await screen.findByRole("button", { name: /运维必备/ });
    expect(presetBtn).toBeTruthy();
    expect(presetBtn.textContent).toContain("1/2");

    // 预设条位于标签栏上方。
    const tagLabel = screen.getByText("标签");
    expect(
      Boolean(presetBtn.compareDocumentPosition(tagLabel) & Node.DOCUMENT_POSITION_FOLLOWING),
    ).toBe(true);

    // 点击部分启用预设：自动安装缺失项并启用。
    fireEvent.click(presetBtn);
    await waitFor(() => expect(hermesEnablePreset).toHaveBeenCalledWith("blank", "ops"));
    expect(
      await screen.findByText(/已为「blank」启用预设「运维必备」（自动安装 1 项到 default，当前档案启用 2 项）/),
    ).toBeTruthy();

    // 若当前档案已全部启用该预设，再次点击只禁用当前档案。
    const baseView = await hermesList("blank");
    vi.mocked(hermesList).mockResolvedValue({
      ...baseView,
      skills: [
        baseView.skills[0],
        { ...baseView.skills[0], name: "beta", library_skill: "lobster/beta", enabled: true },
      ],
    });
    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    await waitFor(() =>
      expect(screen.getByTitle("仅在当前档案禁用该预设")).toBeTruthy(),
    );
    fireEvent.click(screen.getByTitle("仅在当前档案禁用该预设"));
    await waitFor(() => expect(hermesDisablePreset).toHaveBeenCalledWith("blank", "ops"));
    expect(
      await screen.findByText(/已在「blank」禁用预设「运维必备」（2 项，default 与其他档案不受影响）/),
    ).toBeTruthy();
  });
});
