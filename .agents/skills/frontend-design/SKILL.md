---
name: frontend-design
description: 前端设计层｜在 design/ 看与改可操作原型，再把正式 React 界面改到与原型所有可见细节一致。改 GUI 外观、布局或交互时使用；纯 CLI、数据模型或无界面修复不用。
---

# 前端设计层

仓库内的 OpenDesign 式闭环：先改原型，再按对照表改 React。画布是已有 HTML 原型，不是新应用。

入口：[design/README.md](../../../design/README.md)。token、Don'ts、对照表：[design/DESIGN.md](../../../design/DESIGN.md)。
样式在 [ui-prototype/prototype.css](../../../specs/001-skill-management-prototype/ui-prototype/prototype.css)，HTML 在 [ui-prototype/index.html](../../../specs/001-skill-management-prototype/ui-prototype/index.html)。

## 何时用

- 用户要看界面、改界面、让产品长得和设计一样。
- 任何会改变 `skill-manager/src/` 可见外观或交互的工作。

不用：纯 CLI／Rust 业务、数据格式、不可见修复。若产品只是没跟上已有原型，直接把产品改到与当前原型一致，不必先改设计。

## 一次做完

用户要改界面时：**同一轮**改原型 CSS/HTML，重导受影响效果图，并按下面对照表改对应 React。不要按页面停下来汇报「这一页对齐了吗」。不要改完原型就等确认，除非用户只说「先看设计、先别改产品」。

模拟数据、本机路径、技能名、数量不算差异。用户能看出的布局、控件、文案位置、间距、图标、弹窗结构必须一致。

## 流程

1. **改设计**  
   只改 `prototype.css`（外观）和 `index.html`（结构／文案／交互）。token／组件规则变了再改 `DESIGN.md`。  
   改完：`node specs/001-skill-management-prototype/ui-prototype/render-prototype.cjs`（可传 `01-code-library` 这类名字）。

2. **落到产品**  
   打开对照表里的 React 文件，把原型里的 DOM 结构、class、文案、间距抄过去。禁止实现时另做一套布局。  
   共用控件已经在 `skill-manager/src/components/ui.tsx`，对原型 `.btn` / `.seg` / `.badge` / `.notice` / `.dialog`。改这些时同时改 `prototype.css` 与 `ui.tsx`／`styles.css`。  
   token 与 `DESIGN.md`、`prototype.css` `:root`、`skill-manager/src/styles.css` 一起改。

3. **业务冲突**  
   正式行为以 `specs/002-skill-management/spec.md` 为准。原型演示与正式规则不一致时，按正式规则改原型视觉并记录差异，再落地。

## 对照表

| 画面 | 原型 | React |
|---|---|---|
| 外壳／侧栏／页头 | `sidebar` `head` `nav` | `components/AppShell.tsx` |
| 技能库／龙虾库 | `library` `libraryCard` `rail` `chips` `batchBar` | `pages/LibraryPage.tsx` |
| Hermes 档案 | `profiles` | `pages/HermesPage.tsx` |
| 工具中心 | `toolCenter` `toolCard` | `pages/ToolCenterPage.tsx` |
| 工具详情 | `toolDetailPage` `toolGuideHTML` | `components/ToolDetail.tsx` |
| 预设 | `presetPage` `presetDetail` | `pages/PresetPage.tsx` |
| Agent 列表／详情 | `agents` `deployRecordCard` `localCard` | `pages/AgentPage.tsx` |
| 设置 | `settings` `settingsAgentCard` | `pages/SettingsPage.tsx`、`AgentSettings.tsx`、`SyncStatus.tsx`、`DiagnosticsPanel.tsx`、`CapacityReport.tsx` |
| 按钮／分段／徽标／提示／弹窗外壳 | `.btn` `.seg` `.badge` `.notice` `.overlay` `.dialog` | `components/ui.tsx` |
| 技能详情 | `skillDialog` | `components/SkillDetail.tsx` |
| Hermes 详情 | `profileSkillDialog` | `components/HermesSkillDetail.tsx` |
| 本地技能 | `localDialog` | `components/LocalSkillDetail.tsx` |
| 部署 | `deployDialog` | `components/DeployDialog.tsx` |
| 入库 | `intakeDialog` | `components/ImportDialog.tsx` |
| 收编 | `adoptDialog` | `components/AdoptDialog.tsx` |
| 上游更新 | `updatesDialog` `singleSkillUpdateDialog` | `components/UpstreamDialog.tsx` |
| Agent 指令 | `drawInstructions` | `components/InstructionEditor.tsx` |
| 标签／分类 | `addSkillTagsDialog` `tagsDialog` `categoryDialog` | `components/SkillDialogs.tsx`、`ManagerEditor.tsx` |
| 维护／工具任务 | `maintenanceDialog` `toolTaskDialog` `toolRegisterDialog` | `components/TaskDialog.tsx` |
| 工具配套 Skill | `toolSkillIntakeDialog` | `components/ToolSkillCandidates.tsx` |
| 确认 | `confirmDialog` | `ui.tsx` `ConfirmDialog` |
| 归档管理 | `archiveDialog` | `components/ArchiveDialog.tsx` |
| 软件更新 | — | `components/AppUpdateDialog.tsx` |
| 管理站登录 | `login-screen` `login-card` | `pages/LoginPage.tsx` |
| 设备条（仅远程壳） | `device-bar` | `components/DeviceBar.tsx` |
| 管理对话 | `chat-panel` | `components/ChatPanel.tsx` |

QuickDeploy 图标：原型 `quickDeploy()` → `components/QuickDeploy.tsx`。

## 验收

对照 gallery 效果图与可操作原型。用户能看出的差异都要改。1px 抗锯齿噪声不要加兼容层。

## 不要做

- 把 OpenDesign 桌面应用、Figma 或新设计系统设为前置。
- 只抄配色另做布局。
- 把原型模拟数据写成产品实现。
- 为对齐而恢复已删除的旧业务入口。
- 去找已不存在的 FeiSkillDock 仓库。视觉真值只在本仓库设计层与原型。
- 按页面向用户确认已经对齐的界面；要的是整次改完后的结果。
