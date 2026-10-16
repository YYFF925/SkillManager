import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { LibraryPage } from "./LibraryPage";
import { listSkills } from "../services/client";

vi.mock("../services/client", async (importOriginal) => ({
  ...await importOriginal<typeof import("../services/client")>(),
  listSkills: vi.fn(async () => ({ skills: [], total: 0, tag_candidates: [], categories: [] })),
  listPresets: vi.fn(async () => [{ domain: "code", id: "daily", name: "日常维护", order: 0, members: [] }]),
  deployStatus: vi.fn(async () => [{ skill: "code/alpha", target: "agent:codex" }]),
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
