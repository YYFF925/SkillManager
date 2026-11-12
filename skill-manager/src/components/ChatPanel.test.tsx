import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { ChatPanel } from "./ChatPanel";
import { getTransport, resetTransportForTests } from "../services/transport";

vi.mock("../services/transport", () => {
  const invoke = vi.fn();
  return {
    getTransport: () => ({ invoke, subscribe: async () => () => undefined }),
    resetTransportForTests: () => {
      invoke.mockReset();
    },
  };
});

function transportInvoke() {
  return getTransport().invoke as unknown as ReturnType<typeof vi.fn>;
}

beforeEach(() => {
  sessionStorage.clear();
  resetTransportForTests();
  transportInvoke().mockImplementation(async (command: string) => {
    if (command === "agent_runtime_get") {
      return {
        cliId: "opencode",
        displayName: "OpenCode",
        defaultCliId: "opencode",
        ready: false,
        installed: false,
        needsInstall: true,
        needsLogin: false,
        loginCommand: "opencode auth login",
        exclusiveVendors: false,
      };
    }
    if (command === "agent_list_messages" || command === "agent_set_context") {
      return { messages: [] };
    }
    if (command === "agent_runtime_install") {
      return {
        cliId: "opencode",
        displayName: "OpenCode",
        ready: false,
        installed: true,
        installedByApp: true,
        needsInstall: false,
        needsLogin: true,
        loginCommand: "C:\\\\opencode.cmd auth login",
      };
    }
    if (command === "agent_runtime_login") {
      return { ok: true, message: "已打开 OpenCode 登录。", loginCommand: "opencode auth login" };
    }
    return {};
  });
});

afterEach(() => {
  sessionStorage.clear();
});

it("does not flash login before runtime status arrives", async () => {
  let finish: (value: unknown) => void = () => undefined;
  const pending = new Promise((resolve) => {
    finish = resolve;
  });
  transportInvoke().mockImplementation(async (command: string) => {
    if (command === "agent_runtime_get") {
      return pending;
    }
    return { messages: [] };
  });
  render(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  expect(screen.getByText("正在检查管理 CLI…")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "打开登录" })).toBeNull();
  expect(screen.queryByRole("button", { name: "我已完成登录" })).toBeNull();
  expect(screen.queryByRole("button", { name: "安装 OpenCode" })).toBeNull();
  finish({
    cliId: "opencode",
    displayName: "OpenCode",
    ready: false,
    installed: false,
    needsInstall: true,
    needsLogin: false,
    loginCommand: "opencode auth login",
  });
  expect(await screen.findByRole("button", { name: "安装 OpenCode" })).toBeEnabled();
  expect(screen.queryByRole("button", { name: "打开登录" })).toBeNull();
  expect(screen.queryByText("正在检查管理 CLI…")).toBeNull();
});

it("shows in-product OpenCode install instead of treating a missing CLI as a skip", async () => {
  render(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  expect(await screen.findByRole("button", { name: "安装 OpenCode" })).toBeEnabled();
  expect(screen.getByText(/钉死这一家/)).toBeTruthy();
  expect(screen.getByRole("button", { name: "发送" })).toBeDisabled();
});

it("ends the CLI check with a real error when agent_runtime_get fails", async () => {
  transportInvoke().mockImplementation(async (command: string) => {
    if (command === "agent_runtime_get") {
      throw new Error("agent_runtime_get超时（15s），请检查本机 Host 是否在线。");
    }
    return { messages: [] };
  });
  render(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  expect(await screen.findByText(/agent_runtime_get超时/)).toBeTruthy();
  expect(screen.queryByText("正在检查管理 CLI…")).toBeNull();
  expect(screen.getByRole("button", { name: "发送" })).toBeDisabled();
  expect(screen.getByPlaceholderText(/完善中文注释/)).toBeDisabled();
});

it("enables send after a ready runtime check", async () => {
  transportInvoke().mockImplementation(async (command: string) => {
    if (command === "agent_runtime_get") {
      return {
        cliId: "opencode",
        displayName: "OpenCode",
        ready: true,
        installed: true,
        installedByApp: true,
        userSelected: false,
        needsInstall: false,
        needsLogin: false,
        acpArgs: ["--print-logs", "acp"],
      };
    }
    return { messages: [] };
  });
  render(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  const box = await screen.findByPlaceholderText(/完善中文注释/);
  await waitFor(() => expect(box).toBeEnabled());
  expect(screen.queryByText("正在检查管理 CLI…")).toBeNull();
  expect(screen.queryByRole("button", { name: "打开登录" })).toBeNull();
  expect(screen.queryByRole("button", { name: "我已完成登录" })).toBeNull();
  fireEvent.change(box, { target: { value: "完善中文注释" } });
  expect(screen.getByRole("button", { name: "发送" })).toBeEnabled();
});

it("shows a handshake failure once in the transcript, not also as a notice", async () => {
  const failure =
    "无法与 C:\\\\opencode.exe 完成 ACP 握手（已试 --print-logs acp）。管理 CLI 握手超时（12s 看门狗已 taskkill，exit 1）";
  transportInvoke().mockImplementation(async (command: string) => {
    if (command === "agent_runtime_get") {
      return {
        cliId: "opencode",
        displayName: "OpenCode",
        ready: true,
        installed: true,
        needsInstall: false,
        needsLogin: false,
      };
    }
    if (command === "agent_post_message") {
      return { status: "failed", message: failure, taskId: "t1" };
    }
    return { messages: [] };
  });
  render(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  const box = await screen.findByPlaceholderText(/完善中文注释/);
  await waitFor(() => expect(box).toBeEnabled());
  fireEvent.change(box, { target: { value: "完善中文注释" } });
  fireEvent.click(screen.getByRole("button", { name: "发送" }));
  expect(await screen.findByText(failure)).toBeTruthy();
  expect(screen.queryAllByText(failure)).toHaveLength(1);
});

it("walks install then login before enabling send", async () => {
  render(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  fireEvent.click(await screen.findByRole("button", { name: "安装 OpenCode" }));
  expect(await screen.findByRole("button", { name: "打开登录" })).toBeEnabled();
  fireEvent.click(screen.getByRole("button", { name: "打开登录" }));
  await waitFor(() => expect(transportInvoke()).toHaveBeenCalledWith("agent_runtime_login"));
  fireEvent.click(screen.getByRole("button", { name: "我已完成登录" }));
  await waitFor(() => {
    transportInvoke().mockImplementation(async (command: string) => {
      if (command === "agent_runtime_get") {
        return {
          cliId: "opencode",
          displayName: "OpenCode",
          ready: true,
          installed: true,
          installedByApp: true,
          needsInstall: false,
          needsLogin: false,
        };
      }
      return { messages: [] };
    });
  });
});
