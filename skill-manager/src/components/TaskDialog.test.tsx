import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { TaskDialog } from "./TaskDialog";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("TaskDialog 复制", () => {
  it("复制前按当前要求重新生成，不依赖先点刷新", async () => {
    const generate = vi.fn(async (requirement: string) =>
      requirement ? `用户要求：${requirement}` : "用户要求：未填写",
    );
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });

    render(
      <TaskDialog
        title="安装 / 登记工具"
        requirementLabel="工具名称与来源 / 安装要求"
        generate={generate}
        onClose={() => {}}
      />,
    );

    await waitFor(() => expect(screen.getByText("用户要求：未填写")).toBeTruthy());
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "https://github.com/Infisical/cli 安装这个 cli 并登记" },
    });
    expect(screen.getByText("用户要求：未填写")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "复制任务说明" }));

    await waitFor(() =>
      expect(writeText).toHaveBeenCalledWith(
        "用户要求：https://github.com/Infisical/cli 安装这个 cli 并登记",
      ),
    );
    expect(generate).toHaveBeenLastCalledWith(
      "https://github.com/Infisical/cli 安装这个 cli 并登记",
    );
    expect(screen.getByText("用户要求：https://github.com/Infisical/cli 安装这个 cli 并登记")).toBeTruthy();
  });
});
