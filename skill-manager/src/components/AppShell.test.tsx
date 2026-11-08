import { fireEvent, render, screen } from "@testing-library/react";
import { Settings } from "lucide-react";
import { describe, expect, it, vi } from "vitest";

import { AppShell } from "./AppShell";

const baseProps = {
  groups: [],
  settingsItem: { key: "settings", label: "设置", icon: Settings },
  activeKey: "library-code",
  onSelect: () => {},
  title: "技能库",
  children: null,
};

describe("AppShell 更新提示", () => {
  it("有新版本时在左下角常驻版本号与立即更新", () => {
    const onInstall = vi.fn();
    render(<AppShell {...baseProps} update={{ version: "0.1.3", onInstall }} />);

    expect(screen.getByText(/发现新版本/)).toBeInTheDocument();
    expect(screen.getByText("0.1.3")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "立即更新" }));
    expect(onInstall).toHaveBeenCalledTimes(1);
  });

  it("没有新版本时不渲染更新提示", () => {
    render(<AppShell {...baseProps} />);

    expect(screen.queryByText(/发现新版本/)).toBeNull();
    expect(screen.queryByRole("button", { name: "立即更新" })).toBeNull();
  });
});
