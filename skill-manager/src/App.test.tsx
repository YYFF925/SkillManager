import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { App } from "./App";
import { checkNow, syncPull, syncPush, type PullReportView } from "./services/client";

vi.mock("./lib/appearance", () => ({ useAppearance: vi.fn() }));
vi.mock("./pages/LibraryPage", () => ({
  LibraryPage: ({ reloadToken }: { reloadToken: number }) => <div data-testid="library-refresh">{reloadToken}</div>,
}));
vi.mock("./pages/PresetPage", () => ({
  PresetPage: ({ reloadToken }: { reloadToken: number }) => <div data-testid="preset-refresh">{reloadToken}</div>,
}));
vi.mock("./services/client", async (importOriginal) => ({
  ...await importOriginal<typeof import("./services/client")>(),
  isDesktop: () => true,
  currentVault: vi.fn(async () => ({ root: "/tmp/test-vault", remote: "test", has_remote: true })),
  agentList: vi.fn(async () => []),
  environmentOs: vi.fn(async () => "macos"),
  settingsGet: vi.fn(async () => ({ theme: "system", font_scale: "default", auto_check_minutes: 60, git_auto_push: true })),
  onVaultChanged: vi.fn(async () => () => {}),
  onScheduleFinished: vi.fn(async () => () => {}),
  currentSchedule: vi.fn(async () => null),
  listPresets: vi.fn(async () => []),
  syncPull: vi.fn(),
  syncPush: vi.fn(),
  checkNow: vi.fn(),
}));

const pulled: PullReportView = {
  outcome: "pulled", connected: true, remote: "test", conflicts: [], deployments: [],
  deployment_errors: [], instructions_changed: false, instruction_targets: [], errors: [],
  message: "已拉取最新技能库",
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(syncPull).mockReset().mockResolvedValue(pulled);
});

it("预设页可从指令左侧拉取，等待期间能切页且不会重复触发，完成后刷新当前页面", async () => {
  let finish!: (report: PullReportView) => void;
  vi.mocked(syncPull).mockReturnValue(new Promise((resolve) => { finish = resolve; }));
  render(<App />);
  const presetDomain = await screen.findByRole("group", { name: "预设所属库" });
  fireEvent.click(within(presetDomain).getByRole("button", { name: "编码库" }));
  expect(screen.getByTestId("preset-refresh")).toHaveTextContent("0");
  const pull = screen.getByRole("button", { name: "检查并拉取 SkillManagerVault" });
  const instruction = screen.getByRole("button", { name: "Agent 指令" });
  expect(pull.compareDocumentPosition(instruction) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  fireEvent.click(pull);
  expect(pull).toBeDisabled();
  expect(pull).toHaveAttribute("aria-busy", "true");
  expect(instruction).toBeEnabled();
  fireEvent.click(pull);
  expect(syncPull).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole("button", { name: "技能库" }));
  expect(screen.getByTestId("library-refresh")).toHaveTextContent("0");
  await act(async () => { finish(pulled); });
  expect(await screen.findByText("已拉取最新技能库")).toBeInTheDocument();
  expect(screen.getByTestId("library-refresh")).toHaveTextContent("1");
  expect(pull).toBeEnabled();
  fireEvent.click(within(screen.getByRole("group", { name: "预设所属库" })).getByRole("button", { name: "编码库" }));
  expect(screen.getByTestId("preset-refresh")).toHaveTextContent("1");
  expect(syncPush).not.toHaveBeenCalled();
  expect(checkNow).not.toHaveBeenCalled();
});

it.each([
  { outcome: "conflict" as const, errors: [], message: "库同步存在冲突，已暂停" },
  { outcome: "disconnected" as const, errors: [], message: "远端不可用（未连接）" },
  { outcome: "pulled" as const, errors: [{ message: "部署副本被修改，已暂停" }], message: "库已拉取，已有部署刷新部分失败" },
])("拉取返回 $outcome 时如实显示结果，并恢复更新按钮", async (report) => {
  vi.mocked(syncPull).mockResolvedValue({ ...pulled, ...report });
  render(<App />);
  const pull = await screen.findByRole("button", { name: "检查并拉取 SkillManagerVault" });
  fireEvent.click(pull);
  expect(await screen.findByText(report.message)).toBeInTheDocument();
  await waitFor(() => expect(pull).toBeEnabled());
  expect(syncPush).not.toHaveBeenCalled();
});

it("拉取异常时显示原因，允许用户再次点击", async () => {
  vi.mocked(syncPull).mockRejectedValueOnce(new Error("连接超时"));
  render(<App />);
  const pull = await screen.findByRole("button", { name: "检查并拉取 SkillManagerVault" });
  fireEvent.click(pull);
  expect(await screen.findByText(/拉取 SkillManagerVault 失败：.*连接超时/)).toBeInTheDocument();
  expect(pull).toBeEnabled();
  fireEvent.click(pull);
  expect(await screen.findByText("已拉取最新技能库")).toBeInTheDocument();
  expect(syncPull).toHaveBeenCalledTimes(2);
});
