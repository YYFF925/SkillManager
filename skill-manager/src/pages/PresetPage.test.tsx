import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { PresetPage } from "./PresetPage";
import { deployTo } from "../services/client";

vi.mock("../services/client", async (importOriginal) => ({
  ...await importOriginal<typeof import("../services/client")>(),
  listPresets: vi.fn(async () => []),
  listSkills: vi.fn(async () => ({ skills: [], total: 0, categories: [], tag_candidates: [] })),
  deployStatus: vi.fn(async () => []),
  createPreset: vi.fn(async () => ({ domain: "code", id: "test", name: "测试", members: [], missing_members: [], order: 0, icon: null })),
  deployTo: vi.fn(),
}));

it("保存预设定义进入已有上传确认流程，创建本身不首次部署", async () => {
  const saved = vi.fn(async () => {});
  render(<PresetPage os="macos" agents={[]} reloadToken={0} onCount={vi.fn()} onTitle={vi.fn()} onLibrarySaved={saved} />);
  fireEvent.click((await screen.findAllByRole("button", { name: "新建预设" }))[0]);
  fireEvent.change(screen.getByPlaceholderText("例如：Rust 依赖检查"), { target: { value: "测试" } });
  fireEvent.click(screen.getByRole("button", { name: "创建" }));
  await waitFor(() => expect(saved).toHaveBeenCalledTimes(1));
  expect(deployTo).not.toHaveBeenCalled();
});
