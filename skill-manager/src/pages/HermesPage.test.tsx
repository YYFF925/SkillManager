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
  hermesAdopt: vi.fn(),
  hermesRead: vi.fn(async () => ({ name: "local-only", path: "/tmp/profile/skills/local-only", file: "SKILL.md", text: "# 独有正文", size: 20, files: ["SKILL.md"] })),
  deployStatus: vi.fn(async () => []),
  renameCategory: vi.fn(async () => ({ domain: "lobster", from: "a", to: "b", affected: 0 })),
}));

import { hermesList, hermesAdopt, hermesRead } from "../services/client";

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
  });
});
