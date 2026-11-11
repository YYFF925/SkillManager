import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { App } from "./App";
import { currentVault, openVault } from "./services/client";

vi.mock("./lib/appearance", () => ({ useAppearance: vi.fn() }));
vi.mock("./pages/LibraryPage", () => ({
  LibraryPage: () => <div data-testid="library-page">技能库已打开</div>,
}));
vi.mock("./services/client", async (importOriginal) => ({
  ...await importOriginal<typeof import("./services/client")>(),
  isDesktop: () => false,
  currentVault: vi.fn(async () => null),
  openVault: vi.fn(async () => ({
    root: "/home/user/SkillManagerVault",
    remote: "test",
    has_remote: true,
  })),
  agentList: vi.fn(async () => []),
  environmentOs: vi.fn(async () => "linux"),
  settingsGet: vi.fn(async () => ({
    theme: "system",
    font_scale: "default",
    auto_check_minutes: 60,
    git_auto_push: true,
  })),
  onVaultChanged: vi.fn(async () => () => {}),
  onScheduleFinished: vi.fn(async () => () => {}),
  currentSchedule: vi.fn(async () => null),
  listPresets: vi.fn(async () => []),
}));

let devices: Array<Record<string, unknown>> = [];

function jsonOk(body: unknown, status = 200) {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  };
}

beforeEach(() => {
  devices = [];
  vi.mocked(currentVault).mockReset().mockResolvedValue(null);
  vi.mocked(openVault).mockReset().mockResolvedValue({
    root: "/home/user/SkillManagerVault",
    remote: "test",
    has_remote: true,
  });
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      const method = (init?.method ?? "GET").toUpperCase();
      if (url.endsWith("/auth/session") || url === "/auth/session") {
        return jsonOk({});
      }
      if (url === "/devices" || url.endsWith("/devices")) {
        return jsonOk({ devices });
      }
      if (url === "/devices/pairing-codes" && method === "POST") {
        return jsonOk({ code: "ab12cd", expiresAt: "10 分钟内" });
      }
      if ((url === "/current" || url.endsWith("/current")) && method === "PUT") {
        return jsonOk({ ok: true, deviceId: "dev-1" });
      }
      return jsonOk({ error: url }, 404);
    }),
  );
});

afterEach(() => {
  vi.unstubAllGlobals();
});

it("lets a logged-in admin generate a pairing code before any device or vault exists", async () => {
  render(<App />);
  expect(await screen.findByRole("button", { name: "生成对接码" })).toBeEnabled();
  expect(screen.getByText("请先对接并选择当前设备，再接入技能库。")).toBeInTheDocument();
  const attach = screen.getByRole("button", { name: "请先选择当前设备" });
  expect(attach).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "生成对接码" }));
  expect(await screen.findByText("ab12cd")).toBeInTheDocument();
  expect(currentVault).not.toHaveBeenCalled();
  expect(openVault).not.toHaveBeenCalled();
  expect(screen.queryByText(/尚未选择当前设备/)).toBeNull();
});

it("attaches a vault path only after the paired device is selected", async () => {
  render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: "生成对接码" }));
  expect(await screen.findByText("ab12cd")).toBeInTheDocument();
  devices = [{ deviceId: "dev-1", displayName: "书房 Host", online: true, revoked: false }];
  await waitFor(
    () => expect(screen.getByRole("option", { name: "书房 Host" })).toBeInTheDocument(),
    { timeout: 4000 },
  );
  fireEvent.change(screen.getByRole("combobox"), { target: { value: "dev-1" } });
  const path = await screen.findByPlaceholderText("/home/user/SkillManagerVault");
  expect(path).toBeEnabled();
  fireEvent.change(path, { target: { value: "/home/user/SkillManagerVault" } });
  const attach = await screen.findByRole("button", { name: "接入该路径" });
  expect(attach).toBeEnabled();
  fireEvent.click(attach);
  await waitFor(() => expect(openVault).toHaveBeenCalledWith("/home/user/SkillManagerVault"));
  expect(await screen.findByTestId("library-page")).toBeInTheDocument();
  expect(screen.queryByText("尚未选择当前设备")).toBeNull();
});
