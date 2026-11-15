import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { ChatPanel } from "./ChatPanel";
import { getTransport, resetTransportForTests } from "../services/transport";

const listeners = new Map<string, Set<(payload: unknown) => void>>();

vi.mock("../services/transport", () => {
  const invoke = vi.fn();
  return {
    getTransport: () => ({
      invoke,
      subscribe: async (event: string, handler: (payload: unknown) => void) => {
        const set = listeners.get(event) ?? new Set();
        set.add(handler);
        listeners.set(event, set);
        return () => {
          set.delete(handler);
        };
      },
    }),
    resetTransportForTests: () => {
      invoke.mockReset();
      listeners.clear();
    },
  };
});

function emitTransportEvent(event: string, payload: unknown) {
  for (const handler of listeners.get(event) ?? []) {
    handler(payload);
  }
}

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
    if (command === "agent_models_get" || command === "agent_set_model") {
      return { currentModelId: null, models: [] };
    }
    return {};
  });
});

afterEach(() => {
  sessionStorage.clear();
  localStorage.clear();
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

it("sends on Enter and inserts a newline on Shift+Enter", async () => {
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
      return { status: "running", taskId: "t-enter", message: "" };
    }
    if (command === "agent_models_get") {
      return { currentModelId: null, models: [] };
    }
    return { messages: [] };
  });
  render(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  const box = await screen.findByPlaceholderText(/完善中文注释/);
  await waitFor(() => expect(box).toBeEnabled());
  fireEvent.change(box, { target: { value: "完善中文注释" } });
  fireEvent.keyDown(box, { key: "Enter", shiftKey: true });
  expect(transportInvoke()).not.toHaveBeenCalledWith(
    "agent_post_message",
    expect.anything(),
  );
  fireEvent.keyDown(box, { key: "Enter", keyCode: 229, isComposing: true });
  expect(transportInvoke()).not.toHaveBeenCalledWith(
    "agent_post_message",
    expect.anything(),
  );
  fireEvent.keyDown(box, { key: "Enter" });
  await waitFor(() =>
    expect(transportInvoke()).toHaveBeenCalledWith("agent_post_message", {
      sessionId: expect.any(String),
      text: "完善中文注释",
    }),
  );
});

it("lists OpenCode models and applies a selection", async () => {
  transportInvoke().mockImplementation(async (command: string, args?: Record<string, unknown>) => {
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
    if (command === "agent_models_get") {
      return {
        currentModelId: "opencode/gpt-5",
        models: [
          { id: "opencode/gpt-5", name: "GPT-5" },
          { id: "opencode/claude-sonnet-4", name: "Claude Sonnet 4" },
        ],
      };
    }
    if (command === "agent_set_model") {
      return {
        currentModelId: args?.modelId,
        models: [
          { id: "opencode/gpt-5", name: "GPT-5" },
          { id: "opencode/claude-sonnet-4", name: "Claude Sonnet 4" },
        ],
      };
    }
    return { messages: [] };
  });
  render(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  const trigger = await screen.findByRole("combobox", { name: "当前模型" });
  expect(trigger).toHaveTextContent("GPT-5");
  fireEvent.click(trigger);
  const option = await screen.findByRole("option", { name: "Claude Sonnet 4" });
  fireEvent.click(option);
  await waitFor(() =>
    expect(transportInvoke()).toHaveBeenCalledWith("agent_set_model", {
      sessionId: expect.any(String),
      modelId: "opencode/claude-sonnet-4",
    }),
  );
});

it("keeps the model control visible while the list is loading or empty", async () => {
  let finish: (value: unknown) => void = () => undefined;
  const pending = new Promise((resolve) => {
    finish = resolve;
  });
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
    if (command === "agent_models_get") {
      return pending;
    }
    return { messages: [] };
  });
  render(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  await waitFor(() => expect(screen.getByPlaceholderText(/完善中文注释/)).toBeEnabled());
  expect(screen.getByText("正在获取…")).toBeTruthy();
  expect(screen.queryByRole("combobox", { name: "当前模型" })).toBeNull();
  finish({ currentModelId: null, models: [] });
  expect(await screen.findByText("暂无列表")).toBeTruthy();
});

it("shows a model-list error instead of hiding the control", async () => {
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
    if (command === "agent_models_get") {
      throw new Error("agent_models_get超时（30s），请检查本机 Host 是否在线。");
    }
    return { messages: [] };
  });
  render(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  expect(await screen.findByText(/agent_models_get超时/)).toBeTruthy();
  expect(screen.getByLabelText("当前模型")).toBeTruthy();
  expect(screen.queryByRole("combobox", { name: "当前模型" })).toBeNull();
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

it("renders assistant markdown and fills quick prompts into the composer", async () => {
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
    if (command === "agent_list_messages") {
      return {
        messages: [
          { role: "assistant", text: "**状态**\n- 已检查 `skill_update`" },
        ],
      };
    }
    return { messages: [] };
  });
  const { rerender } = render(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  expect(await screen.findByText("状态")).toBeTruthy();
  expect(screen.getByText("skill_update").tagName).toBe("CODE");

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
    return { messages: [] };
  });
  rerender(<ChatPanel open={false} view={{ page: "code" }} onClose={() => undefined} />);
  rerender(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  const quick = await screen.findByRole("button", { name: "检查上游更新" });
  fireEvent.click(quick);
  expect((screen.getByPlaceholderText(/完善中文注释/) as HTMLTextAreaElement).value).toBe("检查上游更新");
});

it("edits visible models and hides disabled ones from the model list", async () => {
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
    if (command === "agent_models_get") {
      return {
        currentModelId: "opencode/gpt-5",
        models: [
          { id: "opencode/gpt-5", name: "GPT-5" },
          { id: "opencode/claude-sonnet-4", name: "Claude Sonnet 4" },
        ],
      };
    }
    return { messages: [] };
  });
  render(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  const trigger = await screen.findByRole("combobox", { name: "当前模型" });
  fireEvent.click(trigger);
  fireEvent.click(await screen.findByRole("button", { name: "编辑模型…" }));
  const toggle = await screen.findByRole("switch", { name: "启用 Claude Sonnet 4" });
  expect(toggle).toHaveAttribute("aria-checked", "true");
  fireEvent.click(toggle);
  expect(toggle).toHaveAttribute("aria-checked", "false");

  const selectAll = screen.getByRole("checkbox", { name: "全选" });
  fireEvent.click(selectAll);
  expect(toggle).toHaveAttribute("aria-checked", "true");
  expect(screen.getByRole("switch", { name: "启用 GPT-5" })).toHaveAttribute("aria-checked", "true");

  fireEvent.click(selectAll);
  expect(toggle).toHaveAttribute("aria-checked", "false");
  expect(screen.getByRole("switch", { name: "启用 GPT-5" })).toHaveAttribute("aria-checked", "false");

  fireEvent.click(screen.getByRole("switch", { name: "启用 GPT-5" }));
  expect(screen.getByRole("switch", { name: "启用 GPT-5" })).toHaveAttribute("aria-checked", "true");
  expect(toggle).toHaveAttribute("aria-checked", "false");
  fireEvent.click(screen.getByRole("button", { name: "完成" }));

  fireEvent.click(screen.getByRole("combobox", { name: "当前模型" }));
  expect(screen.getByRole("option", { name: "GPT-5" })).toBeTruthy();
  expect(screen.queryByRole("option", { name: "Claude Sonnet 4" })).toBeNull();
});

it("supports /new slash command to start a new conversation", async () => {
  const posted: string[] = [];
  const listedSessions: string[] = [];
  transportInvoke().mockImplementation(async (command: string, args?: unknown) => {
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
    if (command === "agent_models_get") {
      return { currentModelId: "opencode/gpt-5", models: [{ id: "opencode/gpt-5", name: "GPT-5" }] };
    }
    if (command === "agent_list_messages") {
      const sid = String((args as { sessionId?: string })?.sessionId ?? "");
      listedSessions.push(sid);
      if (listedSessions.length === 1) {
        return { messages: [{ role: "assistant", text: "上一轮对话内容" }] };
      }
      return { messages: [] };
    }
    if (command === "agent_post_message") {
      posted.push(String((args as { text?: string })?.text ?? ""));
      return { status: "done", message: "ok" };
    }
    return {};
  });

  render(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  expect(await screen.findByText("上一轮对话内容")).toBeTruthy();
  const initialSession = sessionStorage.getItem("sm-agent-session");

  const input = screen.getByPlaceholderText(/完善中文注释/);
  fireEvent.change(input, { target: { value: "/" } });
  expect(screen.getByRole("listbox", { name: "快捷命令" })).toBeTruthy();
  expect(screen.getByRole("option", { name: /\/new/ })).toBeTruthy();

  fireEvent.change(input, { target: { value: "/new" } });
  fireEvent.keyDown(input, { key: "Enter" });

  await waitFor(() => {
    expect(screen.queryByText("上一轮对话内容")).toBeNull();
  });
  expect(sessionStorage.getItem("sm-agent-session")).not.toBe(initialSession);
  expect(posted).toEqual([]);
  expect((input as HTMLTextAreaElement).value).toBe("");
});

it("displays streaming thinking and tool execution progress", async () => {
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
    if (command === "agent_models_get") {
      return { currentModelId: "opencode/gpt-5", models: [{ id: "opencode/gpt-5", name: "GPT-5" }] };
    }
    return { messages: [] };
  });

  render(<ChatPanel open view={{ page: "code" }} onClose={() => undefined} />);
  await screen.findByRole("combobox", { name: "当前模型" });
  const sid = sessionStorage.getItem("sm-agent-session");

  await act(async () => {
    emitTransportEvent("agent_activity", {
      sessionId: sid,
      kind: "thought",
      text: "先检查上游状态",
    });
    emitTransportEvent("agent_activity", {
      sessionId: sid,
      kind: "tool",
      toolCallId: "call_1",
      title: "skill_check",
      status: "in_progress",
    });
  });

  expect(screen.getByText("思考与执行过程（1 步）")).toBeTruthy();
  expect(screen.getByText("skill_check")).toBeTruthy();
  expect(screen.getByText("进行中")).toBeTruthy();
  expect(screen.getByText("先检查上游状态")).toBeTruthy();

  await act(async () => {
    emitTransportEvent("agent_activity", {
      sessionId: sid,
      kind: "tool",
      toolCallId: "call_1",
      title: "skill_check",
      status: "completed",
    });
  });
  expect(screen.getByText("已完成")).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: /思考与执行过程/ }));
  expect(screen.queryByText("先检查上游状态")).toBeNull();
});
