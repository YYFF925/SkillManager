import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { LibraryPage } from "./LibraryPage";
import { listSkills, renameCategory, type SkillSummary } from "../services/client";

vi.mock("../services/client", async (importOriginal) => ({
  ...await importOriginal<typeof import("../services/client")>(),
  listSkills: vi.fn(async () => ({ skills: [], total: 0, tag_candidates: [], categories: [] })),
  listPresets: vi.fn(async () => [{ domain: "code", id: "daily", name: "日常维护", order: 0, members: [] }]),
  deployStatus: vi.fn(async () => [{ skill: "code/alpha", target: "agent:codex" }]),
  renameCategory: vi.fn(async () => ({ domain: "code", from: "家庭运维", to: "脚本", affected: 1 })),
}));

it("combines actual deployment, preset and text filters through the existing query", async () => {
  render(<LibraryPage domain="code" os="macos" agents={[]} reloadToken={0} onCount={vi.fn()} />);
  await screen.findByRole("option", { name: "日常维护" });
  await screen.findByRole("option", { name: "agent:codex" });
  fireEvent.change(screen.getByLabelText("指定部署目标"), { target: { value: "agent:codex" } });
  fireEvent.change(screen.getByLabelText("所属预设"), { target: { value: "daily" } });
  fireEvent.change(screen.getByPlaceholderText("搜索名称、中文注释、正文或标签…"), { target: { value: "alpha" } });
  await waitFor(() => expect(listSkills).toHaveBeenLastCalledWith(expect.objectContaining({
    domain: "code", deployedTo: "agent:codex", preset: "daily", query: "alpha",
  })));
  fireEvent.click(screen.getByRole("button", { name: "清除筛选" }));
  await waitFor(() => expect(listSkills).toHaveBeenLastCalledWith({ domain: "code" }));
});
it("puts skills unsupported on the current OS after supported ones", async () => {
  vi.mocked(listSkills).mockResolvedValue({
    skills: [
      {
        deployment_blocks: {},
        domain: "code",
        name: "linux-only",
        category: "家庭运维",
        comment: "不支持当前系统",
        tags: [],
        description: "仅 Linux",
        agents: null,
        oses: ["linux"],
        has_translation: false,
        translation_stale: false,
        vault_rel_dir: "code/家庭运维/linux-only",
        path: "/tmp/linux-only",
        entry_file: "SKILL.md",
        source: null,
        manager_error: null,
        presets: [],
      },
      {
        deployment_blocks: {},
        domain: "code",
        name: "macos-ready",
        category: "家庭运维",
        comment: "自建",
        tags: [],
        description: "当前系统可用",
        agents: null,
        oses: null,
        has_translation: false,
        translation_stale: false,
        vault_rel_dir: "code/家庭运维/macos-ready",
        path: "/tmp/macos-ready",
        entry_file: "SKILL.md",
        source: null,
        manager_error: null,
        presets: [],
      },
    ],
    total: 2,
    tag_candidates: [],
    categories: ["家庭运维"],
  });
  render(<LibraryPage domain="code" os="macos" agents={[]} reloadToken={0} onCount={vi.fn()} />);
  const linux = await screen.findByRole("button", { name: "linux-only" });
  const macos = screen.getByRole("button", { name: "macos-ready" });
  expect(macos.compareDocumentPosition(linux) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
});

function summary(name: string, category: string | null): SkillSummary {
  return {
    deployment_blocks: {},
    domain: "code",
    name,
    category,
    comment: null,
    tags: [],
    description: name,
    agents: null,
    oses: null,
    has_translation: false,
    translation_stale: false,
    vault_rel_dir: category ? `code/${category}/${name}` : `code/${name}`,
    path: `/tmp/${name}`,
    entry_file: "SKILL.md",
    source: null,
    manager_error: null,
    presets: [],
  };
}

it("packs unselected tag filter chips without extra button padding", async () => {
  vi.mocked(listSkills).mockResolvedValue({
    skills: [],
    total: 0,
    tag_candidates: ["API", "Fork 维护"],
    categories: [],
  });
  render(<LibraryPage domain="code" os="macos" agents={[]} reloadToken={0} onCount={vi.fn()} />);
  const api = await screen.findByRole("button", { name: "API" });
  expect(api.className).toContain("px-0");
  expect(api.className).not.toContain("px-2");
  expect(api.querySelector("span")?.textContent).toBe("API");

  fireEvent.click(api);
  await waitFor(() => expect(listSkills).toHaveBeenLastCalledWith(expect.objectContaining({
    domain: "code",
    tags: ["API"],
  })));
  const selected = screen.getByRole("button", { name: "API" });
  expect(selected.className).toContain("px-2");
  expect(selected.className).toContain("bg-accent");
  expect(selected.querySelector("span")).toBeNull();
});

it("shows a rename control on named categories like presets", async () => {
  vi.mocked(listSkills).mockResolvedValue({
    skills: [summary("alpha", "家庭运维"), summary("beta", null)],
    total: 2,
    tag_candidates: [],
    categories: ["家庭运维"],
  });
  render(<LibraryPage domain="code" os="macos" agents={[]} reloadToken={0} onCount={vi.fn()} />);
  expect(await screen.findByRole("button", { name: "重命名分类 家庭运维" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "重命名分类 未分类" })).toBeNull();
  expect(screen.queryByRole("button", { name: "重命名分类 全部分类" })).toBeNull();

  fireEvent.click(screen.getByRole("button", { name: "重命名分类 家庭运维" }));
  const input = await screen.findByDisplayValue("家庭运维");
  fireEvent.change(input, { target: { value: "脚本" } });
  fireEvent.click(screen.getByRole("button", { name: "保存" }));
  await waitFor(() => expect(renameCategory).toHaveBeenCalledWith("code", "家庭运维", "脚本"));
});
