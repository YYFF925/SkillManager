import { fireEvent, render, screen, waitFor } from "@testing-library/react";
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
  return (
    <>
      <button onClick={() => void flow.afterSave()}>保存</button>
      <span data-testid="failure">{flow.failure ?? ""}</span>
      {flow.dialog}
    </>
  );
}

describe("useLibrarySaveFlow", () => {
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
