# 内置管理 Agent 快照对照（core / GUI / 工具）

**Date**: 2026-11-11 | 对照 [spec.md](spec.md) FR-015、[contracts/agent-bridge.md](contracts/agent-bridge.md)。  
原则：GUI 当时能回答的管理事实，对话里不必再由用户复述。举例不是穷尽清单；本表按 **当前** core JSON CLI、Host invoke 与 GUI 页面核对。

本表**不宣称齐全**。只读事实均指向快照字段或 Agent 工具；写入类命令以 Host invoke / GUI / CLI 为准，仅部分对内置 Agent 开放（见命令表「对话工具」列）。

快照由 `assemble_snapshot` 在发消息时从 **当前 Host 的 core** 组装：`global` 为摘要 + 计数 + 引用，`view` 按当前页补齐该页 GUI 已知对象。不含 `SKILL.md` / 指南 / 译文全文（正文走 `skill_read` / `hermes_read` / `agent_read_local`）。

---

## 一、命令表（JSON CLI = Host invoke 同名）

| 命令 | GUI 谁在用 | 快照字段 | 对话工具 |
|---|---|---|---|
| `skill list` | 技能库列表、分类、标签候选 | `global.skills`；`view.pageObjects`（library-*） | `skill_list` |
| `skill get` | 技能详情（资料、文件树、部署位置） | `view.opened`（有 skill ref；正文已省略） | `skill_get`；正文 `skill_read` / `skill_file_read` |
| `skill read` | 详情阅读原文/译文 | 无正文 | `skill_read` |
| `skill create/copy/import/adopt/preview` | 入库/自建对话框 | 无（动作） | 未开放；走 GUI/CLI |
| `skill set` / `tag *` / `move` / `category rename` | 详情与批量编辑 | 结果反映在下次 `global.skills` | `skill_set`；tag/move 未开放 |
| `skill delete` | 删除确认 | 确认计划 | `skill_delete`（须界面确认） |
| `skill archive *` | 设置页归档 | `skill_archive_list` 工具 | `skill_archive_list`；写入未开放 |
| `skill translation *` / `overlay *` | 详情译文/覆盖层 | 无正文 | `skill_translation_get`、`skill_overlay_list`、`skill_overlay_get`；写入未开放 |
| `skill check` | 设置「立即检查」、技能库上游对话框 | 检查后：`global.pendingUpdates`、`global.newCandidates`、`global.failedChecks` | `environment_schedule_current` 读最近一次；检查动作走 GUI |
| `skill update*` / `reject-new` / `skip*` | 上游对话框 | `global.newCandidates`、`skill_rejected` / `skill_skipped` | `skill_rejected`、`skill_skipped`；写入未开放 |
| `skill task` | 生成维护任务 | 无 | `skill_task` |
| `deploy to/undo/sync/status` | 部署对话框、Agent 页暂停条 | `global.deployments`、`global.pausedDeployments`（含 `paused_local_change` + 路径 + 原因） | `deploy_status`、`deploy_to`、`deploy_sync`；undo 未开放 |
| `preset *` | 预设页 / 侧栏 | `global.presets`；`view.pageObjects`（presets）；打开的预设 `view.opened` | `preset_list`、`preset_get`；写入未开放 |
| `hermes profiles/list/read/enable/disable` | Hermes 页 | `global.hermes`；`view.pageObjects`（hermes） | `hermes_profiles`、`hermes_list`、`hermes_read`；启停未开放 |
| `tool list/get/check` | 工具中心 | `global.tools`；`view.pageObjects`（tools） | `tool_list`、`tool_get`、`tool_skill_candidates`；check 走 GUI |
| `sync pull/push/status/conflicts` | 顶栏同步、设置页 | `global.pullConflicts`（路径 + `git status` 原文）；设置页 `view.pageObjects.sync` | `sync_status`、`sync_conflicts`、`sync_push`（须已开自动上传） |
| `agent list/detect/local-skills/read-local` | Agent 页 | `global.agents`；`view.pageObjects`（agents-*）；打开目标 `view.opened` | `agent_list`、`agent_local_skills`、`agent_read_local` |
| `instruction get/set` | 设置里的指令编辑 | 无全文 | `instruction_get`；写入未开放 |
| `settings get` / `diagnostics` / `capacity` | 设置页 | `view.pageObjects`（settings）；`global.lastCommandFailures`（错误原文） | `settings_get`、`environment_diagnostics`、`environment_capacity` |
| `vault current` / `host status` | 壳层库路径、Host 状态 | settings 的 `vault` | `vault_current`、`host_status` |
| `environment_schedule_current` | 设置检查结果、技能库「新增」 | `global.newCandidates`、`global.failedChecks[].error` | `environment_schedule_current` |

---

## 二、页面表

客户端只报 `page` / `domain` / 选中引用（技能 `ref`、预设 `presetId`、Agent `agent`）。详情由 Host 补齐。

| 页面 `page` | GUI 当时能看见 | `view` 去处 |
|---|---|---|
| `library-code` / `library-lobster` | 该域技能列表、分类、标签候选、译文待更新标记、上游新增入口 | `pageObjects.kind=library`（categories / tagCandidates / skills 摘要）；选中技能 → `opened`（无正文） |
| `presets` | 当前域预设名、成员、缺失成员 | `pageObjects.kind=presets`；选中 `presetId` → `opened` |
| `hermes` | 档案列表、启用数、路径 | `pageObjects.kind=hermes` |
| `tools` | 工具清单、检查状态、是否安装 | `pageObjects.kind=tools`；选中 `tool` → `opened`（指南正文省略） |
| `agents-code` / `agents-lobster` | 该域 Agent 检测/启用/目录 | `pageObjects.kind=agents` |
| `agent:<key>` / `agent:shared` | 该目标本机技能、`paused_local_change`、路径 | `opened` 为 localSkills 摘要（无副本正文） |
| `settings` | 主题/频率/自动上传、同步状态、Vault 路径、最近检查 | `pageObjects.kind=settings`（settings / sync / vault） |
| 登录页 / 设备条 | 管理站账号、对接码、设备名单 | **不进 Host 快照**（另一台设备的库 Agent 看不见；本机设备条不是当前 Host 的库事实） |
| 顶栏同步冲突 / 诊断 | 冲突路径、git 状态行、最近失败日志 | `global.pullConflicts`、`global.lastCommandFailures` |

无技能 `ref` 时不再只带 page 名；`pageObjects` 必须能看出该页对象种类（或该页因未接入 Vault 的错误）。

---

## 三、FR-015 点名的事故字段

| 事实 | 快照 | 工具补读 |
|---|---|---|
| 可入库新增 | `global.newCandidates`（最近一次调度 `upstream_report.candidates`；尚未检查则为空，与 GUI 一致） | `environment_schedule_current` |
| 拉取冲突路径 + git 原文 | `global.pullConflicts.conflicts` + `status`（`git status --porcelain` 各行）+ `message` | `sync_conflicts` |
| `paused_local_change` | `global.pausedDeployments[]`：`ref` / `target` / `targetPath` / `status` / `reason`；Agent 页另见 `view.opened.skills[].status` | `deploy_status`、`agent_local_skills` |
| 最近命令失败原文 | `global.lastCommandFailures[]`：`action` + `message`（诊断日志 error/panic，以及调度 upstream/pull/tool_check 的 `detail`） | `environment_diagnostics` |

正文、指南、冲突文件内容不进快照；定位后用对应 read 工具。
