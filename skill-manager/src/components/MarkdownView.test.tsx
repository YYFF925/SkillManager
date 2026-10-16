import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { MarkdownView } from "./MarkdownView";

describe("MarkdownView", () => {
  it("将 frontmatter、标题和代码块渲染为结构化正文", () => {
    render(
      <MarkdownView
        text={`---
name: shared-credentials
description: 凭据管理
---

# Agent 共享凭据

第一次使用时读 [CLI 与配置](references/cli.md)。

\`\`\`text
python credentials.py
\`\`\`
`}
      />,
    );

    expect(screen.getByText("name")).toBeTruthy();
    expect(screen.getByText("shared-credentials")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Agent 共享凭据" })).toBeTruthy();
    expect(screen.getByText("python credentials.py")).toBeTruthy();
    expect(screen.getByRole("link", { name: "CLI 与配置" })).toBeTruthy();
    expect(screen.queryByText("---")).toBeNull();
    expect(screen.queryByText("# Agent 共享凭据")).toBeNull();
  });
});
