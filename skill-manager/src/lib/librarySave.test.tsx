import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { useLibrarySaveFlow } from "./librarySave";

vi.mock("../services/client", () => ({
  deploySync: vi.fn(async () => ({ items: [], errors: [] })),
  settingsGet: vi.fn(async () => ({ theme: "system", font_scale: "default", auto_check_minutes: 60, git_auto_push: true })),
  setGitAutoPush: vi.fn(async () => ({ theme: "system", font_scale: "default", auto_check_minutes: 60, git_auto_push: true })),
  syncPush: vi.fn(async () => ({ committed: true, pushed: true, remote: "r", message: "已提交并推送工作区全部改动" })),
}));

import { deploySync, setGitAutoPush, settingsGet, syncPush } from "../services/client";

const mocked = {
  deploySync: vi.mocked(deploySync),
  settingsGet: vi.mocked(settingsGet),
  setGitAutoPush: vi.mocked(setGitAutoPush),
  syncPush: vi.mocked(syncPush),
};

afterEach(() => {
  vi.clearAllMocks();
  mocked.deploySync.mockResolvedValue({ items: [], errors: [] });
  mocked.settingsGet.mockResolvedValue({ theme: "system", font_scale: "default", auto_check_minutes: 60, git_auto_push: true });
  mocked.syncPush.mockResolvedValue({
    committed: true,
    pushed: true,
    remote: "r",
    message: "已提交并推送工作区全部改动",
  } as never);
});

function Harness() {
  const flow = useLibrarySaveFlow({ onNotice: () => {}, onError: () => {} });
  const [saved, setSaved] = useState(false);
  return (
    <>
      <button onClick={async () => { await flow.afterSave(); setSaved(true); }}>保存</button>
      {saved && <span>保存完成</span>}
      <span data-testid="failure">{flow.failure ?? ""}</span>
      {flow.dialog}
    </>
  );
}

describe("useLibrarySaveFlow", () => {
  it("保存完成不等待后台部署刷新，刷新结束后仍按顺序上传", async () => {
    let finish!: (report: Awaited<ReturnType<typeof deploySync>>) => void;
    mocked.deploySync.mockReturnValue(new Promise((resolve) => { finish = resolve; }));
    render(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    expect(await screen.findByText("保存完成")).toBeInTheDocument();
    expect(mocked.syncPush).not.toHaveBeenCalled();
    finish({ items: [], errors: [] });
    await waitFor(() => expect(mocked.syncPush).toHaveBeenCalledTimes(1));
  });

  it("慢上传期间保存已完成，稍后的失败仍可见且不自动重试", async () => {
    let fail!: (reason: Error) => void;
    mocked.syncPush.mockReturnValue(new Promise((_, reject) => { fail = reject; }));
    render(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(mocked.syncPush).toHaveBeenCalledTimes(1));
    expect(screen.getByText("保存完成")).toBeInTheDocument();
    fail(new Error("连接超时"));
    await waitFor(() => expect(screen.getByTestId("failure")).toHaveTextContent("连接超时"));
    expect(mocked.syncPush).toHaveBeenCalledTimes(1);
  });

  it("refreshes deployments first, then uploads when already confirmed", async () => {
    render(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => expect(mocked.syncPush).toHaveBeenCalledTimes(1));
    expect(mocked.deploySync).toHaveBeenCalledTimes(1);
    expect(mocked.deploySync.mock.invocationCallOrder[0]).toBeLessThan(
      mocked.syncPush.mock.invocationCallOrder[0],
    );
    // 已确认时不弹确认框。
    expect(screen.queryByText("保存后自动上传本机 SkillManagerVault？")).toBeNull();
  });

  it("asks once, records the confirmation, then uploads", async () => {
    mocked.settingsGet.mockResolvedValue({ theme: "system", font_scale: "default", auto_check_minutes: 60, git_auto_push: false });
    render(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => expect(screen.getByText("保存后自动上传本机 SkillManagerVault？")).toBeTruthy());
    expect(mocked.syncPush).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "确认并上传" }));
    await waitFor(() => expect(mocked.syncPush).toHaveBeenCalledTimes(1));
    expect(mocked.setGitAutoPush).toHaveBeenCalledWith(true);
  });

  it("does not upload when the confirmation is declined", async () => {
    mocked.settingsGet.mockResolvedValue({ theme: "system", font_scale: "default", auto_check_minutes: 60, git_auto_push: false });
    render(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => expect(screen.getByText("保存后自动上传本机 SkillManagerVault？")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    await waitFor(() => expect(screen.queryByText("保存后自动上传本机 SkillManagerVault？")).toBeNull());
    expect(mocked.syncPush).not.toHaveBeenCalled();
    expect(mocked.setGitAutoPush).not.toHaveBeenCalled();
  });

  it("keeps a readable reason when the upload fails", async () => {
    mocked.syncPush.mockRejectedValue(new Error("远端不可用（未连接）"));
    render(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() =>
      expect(screen.getByTestId("failure").textContent).toContain("远端不可用"),
    );
  });
});
