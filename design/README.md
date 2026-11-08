# 前端设计层

本目录是 Skill Manager 的设计层入口，职责对应 OpenDesign 的「看设计 → 改设计 → 把产品改到和设计一样」。

它不是产品功能，也不引入 OpenDesign 桌面应用或另一套视觉体系。画布仍是已有的可操作 HTML 原型；本层把它升级为强制工作流。

## 三步（同一轮做完）

1. **看设计**：打开 [gallery.html](gallery.html) 或 [可操作原型](../specs/001-skill-management-prototype/ui-prototype/index.html)。
2. **改设计**：改 [prototype.css](../specs/001-skill-management-prototype/ui-prototype/prototype.css) 和原型 `index.html`（token 变了再改 [DESIGN.md](DESIGN.md)）。改完重导受影响效果图。
3. **落到产品**：按 [DESIGN.md](DESIGN.md) 对照表改对应 React，与原型所有可见细节一致。用户要改界面时不要改完原型就停；对照表在技能 `.agents/skills/frontend-design/SKILL.md`。

Agent 按 [.agents/skills/frontend-design/SKILL.md](../.agents/skills/frontend-design/SKILL.md) 执行。

## 真值分工

| 东西 | 职责 |
|---|---|
| [DESIGN.md](DESIGN.md) | 机器可读的设计系统：气质、token、组件、Don'ts |
| [可操作原型](../specs/001-skill-management-prototype/ui-prototype/index.html) | 可点击的设计画布与交互真值 |
| [prototype.css](../specs/001-skill-management-prototype/ui-prototype/prototype.css) | 原型（也是产品要对齐的）视觉样式 |
| [效果图](../specs/001-skill-management-prototype/ui-prototype/renders/) | 1720×1040 静态验收图 |
| [视觉说明](../specs/001-skill-management-prototype/visual-reference.md) | 沿革、旧版对照、历史尺寸 |
| [002 规格](../specs/002-skill-management/spec.md) | 正式业务行为；与原型演示冲突时业务以 002 为准，然后改原型视觉去对齐新行为 |

产品 CSS 变量在 `skill-manager/src/styles.css`。改 token 时同步 DESIGN.md、`prototype.css` `:root` 与产品变量，不要只改一处。页面／弹窗对照表在技能里：改哪一块原型，就改哪一个 React 文件。

## 重导效果图

```text
node specs/001-skill-management-prototype/ui-prototype/render-prototype.cjs
```

可传效果图名只导部分，例如 `01-code-library`。脚本只写 `ui-prototype/renders/`。

## 当前基线

2026-11-08 原型已追到正在运行的 v0.1.3 主界面 chrome（技能库／预设／设置／Hermes／工具中心）。之后改界面：先改原型 CSS/HTML，同一轮按对照表改 React。
