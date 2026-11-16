import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { PresetPage } from "./PresetPage";
import { loadCategoryRail } from "../lib/categoryRail";
import { addPresetMembers, listPresets, listSkills, type PresetView } from "../services/client";

vi.mock("../services/client", async (importOriginal) => ({
  ...await importOriginal<typeof import("../services/client")>(),
  listPresets: vi.fn(async () => []),
  listSkills: vi.fn(async () => ({ skills: [], total: 0, categories: [], tag_candidates: [] })),
  deployStatus: vi.fn(async () => []),
  addPresetMembers: vi.fn(),
  deployTo: vi.fn(),
  renameCategory: vi.fn(async () => ({ domain: "code", from: "a", to: "b", affected: 0 })),
}));

beforeEach(() => {
  localStorage.clear();
  vi.clearAllMocks();
  vi.mocked(listPresets).mockResolvedValue([]);
});

const common: PresetView = {
  domain: "code",
  id: "common",
  name: "通用",
  members: [],
  missing_members: [],
  order: 0,
};
const work: PresetView = { ...common, id: "work", name: "工作", order: 1 };

it("shows the selected preset member page without an in-page preset rail", async () => {
  vi.mocked(listPresets).mockResolvedValue([common, work]);
  const onSelectedIdChange = vi.fn();
  render(
    <PresetPage
      os="macos"
      agents={[]}
      reloadToken={0}
      domain="code"
      selectedId="common"
      onSelectedIdChange={onSelectedIdChange}
      onCount={vi.fn()}
      onTitle={vi.fn()}
    />,
  );
  expect(await screen.findByRole("heading", { name: "通用" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "组合部署…" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "新建分类" })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "查看成员" })).not.toBeInTheDocument();
  expect(screen.queryByPlaceholderText("搜索预设名称…")).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "龙虾库" })).not.toBeInTheDocument();
});

it("shows empty guidance when the selected domain has no presets", async () => {
  vi.mocked(listPresets).mockResolvedValue([]);
  render(
    <PresetPage
      os="macos"
      agents={[]}
      reloadToken={0}
      domain="lobster"
      selectedId={null}
      onSelectedIdChange={vi.fn()}
      onCount={vi.fn()}
      onTitle={vi.fn()}
    />,
  );
  expect(await screen.findByText("该库还没有预设，请从左侧新建。")).toBeInTheDocument();
});

it("keeps member browsing wired to the controlled selected preset", async () => {
  vi.mocked(listPresets).mockResolvedValue([common, work]);
  const { rerender } = render(
    <PresetPage
      os="macos"
      agents={[]}
      reloadToken={0}
      domain="code"
      selectedId="common"
      onSelectedIdChange={vi.fn()}
      onCount={vi.fn()}
      onTitle={vi.fn()}
    />,
  );
  await screen.findByRole("heading", { name: "通用" });
  rerender(
    <PresetPage
      os="macos"
      agents={[]}
      reloadToken={1}
      domain="code"
      selectedId="work"
      onSelectedIdChange={vi.fn()}
      onCount={vi.fn()}
      onTitle={vi.fn()}
    />,
  );
  expect(await screen.findByRole("heading", { name: "工作" })).toBeInTheDocument();
});

it("does not deploy when only browsing members", async () => {
  vi.mocked(listPresets).mockResolvedValue([common]);
  render(
    <PresetPage
      os="macos"
      agents={[]}
      reloadToken={0}
      domain="code"
      selectedId="common"
      onSelectedIdChange={vi.fn()}
      onCount={vi.fn()}
      onTitle={vi.fn()}
      onLibrarySaved={vi.fn(async () => {})}
    />,
  );
  await screen.findByRole("heading", { name: "通用" });
  expect(addPresetMembers).not.toHaveBeenCalled();
  await waitFor(() => expect(listPresets).toHaveBeenCalled());
});

it("does not leak lobster categories into code storage when switching preset domain", async () => {
  const lobsterPreset: PresetView = { ...common, domain: "lobster", id: "lobster-common", name: "龙虾通用" };
  vi.mocked(listPresets).mockImplementation(async (domain) =>
    domain === "lobster" ? [lobsterPreset] : [common],
  );
  vi.mocked(listSkills).mockImplementation(async (query) => {
    if (query?.domain === "lobster") {
      return {
        skills: [
          {
            domain: "lobster",
            name: "comfy-helper",
            category: "AI绘画",
            comment: null,
            tags: [],
            description: null,
            agents: null,
            oses: null,
            has_translation: false,
            translation_stale: false,
            vault_rel_dir: "Lobster/AI绘画/comfy-helper",
            path: "/tmp/Lobster/AI绘画/comfy-helper",
            entry_file: "SKILL.md",
            source: null,
            manager_error: null,
            presets: [],
            deployment_blocks: {},
          },
        ],
        total: 1,
        categories: ["AI绘画"],
        tag_candidates: [],
      };
    }
    return {
      skills: [
        {
          domain: "code",
          name: "infisical-cli",
          category: "Infisical",
          comment: null,
          tags: [],
          description: null,
          agents: null,
          oses: null,
          has_translation: false,
          translation_stale: false,
          vault_rel_dir: "Code/Infisical/infisical-cli",
          path: "/tmp/Code/Infisical/infisical-cli",
          entry_file: "SKILL.md",
          source: null,
          manager_error: null,
          presets: [],
          deployment_blocks: {},
        },
      ],
      total: 1,
      categories: ["Infisical"],
      tag_candidates: [],
    };
  });

  const { rerender } = render(
    <PresetPage
      os="macos"
      agents={[]}
      reloadToken={0}
      domain="lobster"
      selectedId="lobster-common"
      onSelectedIdChange={vi.fn()}
      onCount={vi.fn()}
      onTitle={vi.fn()}
    />,
  );
  expect(await screen.findByText("AI绘画")).toBeInTheDocument();

  rerender(
    <PresetPage
      os="macos"
      agents={[]}
      reloadToken={0}
      domain="code"
      selectedId="common"
      onSelectedIdChange={vi.fn()}
      onCount={vi.fn()}
      onTitle={vi.fn()}
    />,
  );
  expect(await screen.findByText("Infisical")).toBeInTheDocument();
  expect(screen.queryByText("AI绘画")).not.toBeInTheDocument();
  expect(loadCategoryRail("code")).toEqual({ order: ["Infisical"], extras: [] });
  expect(loadCategoryRail("lobster")).toEqual({ order: ["AI绘画"], extras: [] });
});
