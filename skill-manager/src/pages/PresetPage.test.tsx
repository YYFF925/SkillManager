import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { PresetPage } from "./PresetPage";
import { deletePreset, deployTo, listPresets, type PresetView } from "../services/client";

vi.mock("../services/client", async (importOriginal) => ({
  ...await importOriginal<typeof import("../services/client")>(),
  listPresets: vi.fn(async () => []),
  listSkills: vi.fn(async () => ({ skills: [], total: 0, categories: [], tag_candidates: [] })),
  deployStatus: vi.fn(async () => []),
  deletePreset: vi.fn(),
  createPreset: vi.fn(async () => ({ domain: "code", id: "test", name: "测试", members: [], missing_members: [], order: 0, icon: null })),
  deployTo: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(listPresets).mockResolvedValue([]);
});

const common: PresetView = {
  domain: "code", id: "common", name: "通用", members: [], missing_members: [], order: 0,
};
const work: PresetView = { ...common, id: "work", name: "工作", order: 1 };

it("进入预设直接显示首项成员页，页内列表可切换预设", async () => {
  vi.mocked(listPresets).mockResolvedValue([common, work]);
  render(<PresetPage os="macos" agents={[]} reloadToken={0} onCount={vi.fn()} onTitle={vi.fn()} />);
  expect(await screen.findByRole("heading", { name: "通用" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "组合部署…" })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "查看成员" })).not.toBeInTheDocument();
  expect(screen.queryByPlaceholderText("搜索预设名称…")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "工作" }));
  expect(screen.getByRole("heading", { name: "工作" })).toBeInTheDocument();
});

it("切换至空管理域仍保留页内列表、新建与域切换入口", async () => {
  vi.mocked(listPresets).mockImplementation(async (domain) => domain === "code" ? [common] : []);
  render(<PresetPage os="macos" agents={[]} reloadToken={0} onCount={vi.fn()} onTitle={vi.fn()} />);
  await screen.findByRole("heading", { name: "通用" });
  fireEvent.click(screen.getByRole("button", { name: "龙虾库" }));
  expect(await screen.findByText("该管理域还没有预设，请从左侧新建预设。")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "新建预设" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "编码库" }));
  expect(await screen.findByRole("heading", { name: "通用" })).toBeInTheDocument();
});

it("删除当前预设后直接显示剩余首项", async () => {
  vi.mocked(listPresets).mockResolvedValue([common, work]);
  vi.mocked(deletePreset).mockImplementation(async () => {
    vi.mocked(listPresets).mockResolvedValue([work]);
    return common;
  });
  render(<PresetPage os="macos" agents={[]} reloadToken={0} onCount={vi.fn()} onTitle={vi.fn()} />);
  await screen.findByRole("heading", { name: "通用" });
  fireEvent.click(screen.getByRole("button", { name: "删除预设 通用" }));
  fireEvent.click(screen.getByRole("button", { name: "删除" }));
  expect(await screen.findByRole("heading", { name: "工作" })).toBeInTheDocument();
  expect(deletePreset).toHaveBeenCalledWith("code", "common");
});

it("保存预设定义进入已有上传确认流程，创建本身不首次部署", async () => {
  const saved = vi.fn(async () => {});
  render(<PresetPage os="macos" agents={[]} reloadToken={0} onCount={vi.fn()} onTitle={vi.fn()} onLibrarySaved={saved} />);
  fireEvent.click((await screen.findAllByRole("button", { name: "新建预设" }))[0]);
  fireEvent.change(screen.getByPlaceholderText("例如：Rust 依赖检查"), { target: { value: "测试" } });
  fireEvent.click(screen.getByRole("button", { name: "创建" }));
  await waitFor(() => expect(saved).toHaveBeenCalledTimes(1));
  expect(deployTo).not.toHaveBeenCalled();
});
