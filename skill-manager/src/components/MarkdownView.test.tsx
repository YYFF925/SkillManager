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

  it("将 Markdown 表格渲染为结构化 table 而不是拼成一行段落", () => {
    render(
      <MarkdownView
        text={`译文待更新共 2 项：
| 引用 | 说明 |
|---|---|
| \`code/ego-browser\` | 原文 2.2.0 / 译文 2.1.0 |
| \`lobster/comfy\` | ComfyUI 多媒体创作 |
`}
      />,
    );

    expect(screen.getByRole("table")).toBeTruthy();
    expect(screen.getByRole("columnheader", { name: "引用" })).toBeTruthy();
    expect(screen.getByRole("columnheader", { name: "说明" })).toBeTruthy();
    expect(screen.getByText("code/ego-browser")).toBeTruthy();
    expect(screen.getByText("原文 2.2.0 / 译文 2.1.0")).toBeTruthy();
    expect(screen.queryByText(/\|---\|---\|/)).toBeNull();
  });

  it("支持嵌套列表、任务复选框、删除线、图片、自动链接、硬换行且不误伤下划线标识符", () => {
    const { container } = render(
      <MarkdownView
        text={`第一行  
第二行

- 一级项
  - 二级项
  1. 有序子项

- [ ] 待处理
- [x] 已完成

~~旧写法~~ ***重要提示*** 调用 skill_update_repo 查看 https://example.com/docs。

![流程图](https://example.com/flow.png)
`}
      />,
    );

    expect(container.querySelectorAll("br").length).toBe(1);
    expect(screen.getByText("二级项").closest("ul")?.parentElement?.tagName).toBe("LI");
    expect(screen.getByText("有序子项").closest("ol")?.parentElement?.tagName).toBe("LI");

    const checkboxes = screen.getAllByRole("checkbox") as HTMLInputElement[];
    expect(checkboxes).toHaveLength(2);
    expect(checkboxes[0].checked).toBe(false);
    expect(checkboxes[1].checked).toBe(true);

    expect(container.querySelector("del")?.textContent).toBe("旧写法");
    expect(container.querySelector("strong > em")?.textContent).toBe("重要提示");
    // 下划线标识符保持原样，不被误解析成 <em>
    expect(container.querySelector("em")?.textContent).toBe("重要提示");
    expect(screen.getByText(/skill_update_repo/)).toBeTruthy();

    expect(screen.getByRole("link", { name: "https://example.com/docs" })).toBeTruthy();
    const img = screen.getByRole("img", { name: "流程图" }) as HTMLImageElement;
    expect(img.getAttribute("src")).toBe("https://example.com/flow.png");
  });
});
