# Quickstart: Host、管理站与内置管理 Agent 验收

**Date**: 2026-11-11 | 对照 [spec.md](spec.md) US1–US3 与 [contracts/](contracts/)。技能细节仍用 [002 quickstart](../../002-skill-management/quickstart.md)。本文件只验新入口。

实施完成前下列命令会不存在；实现后按此跑，不要用浏览器里打开静态页代替 Host 真结果。

## 前提

- 已有 SkillManagerVault 远端（与现在桌面使用的是同一个）。
- 一台可跑无窗口进程的机器（或本机另开 `--state-dir` 模拟第二台）。
- 管理站前面已有 HTTPS 反代，或本机回环只做开发验收。
- 本机 git 认证已能 clone 该远端。

## 1. 无窗口 Host 接入（US1）

```text
skill-manager vault clone <vault-remote-url> --path <empty-dir>
skill-manager --vault <empty-dir> --state-dir <state-a> host serve
```

期望：服务在跑；`host status` 给出该 Vault 路径；无图形窗口。再开一个同样 `state-dir` 的 `host serve` 必须失败（锁）。

关掉后再执行同一条 `host serve`（启动命令），应能再起来。不验收唤醒休眠。

```text
skill-manager --state-dir <state-a> host install-service
```

期望：当前用户开机项已写入（macOS launchd / Linux systemd --user / Windows 登录项或等价）。不必真重启机器。`host uninstall-service` 能清掉该项。

安装 Skill：把 `skill-manager-install` 交给该机已有编码 Agent，按其步骤 clone、serve、pair。期望与手工命令结果相同，且不改写配套维护 Skill。

## 2. 管理站切换设备（US2）

```text
skill-manager station serve --listen 127.0.0.1:8787
```

首次：本机创建唯一管理员并绑定 TOTP。浏览器登录后生成对接码。

```text
skill-manager --state-dir <state-a> host pair --station <url> --code <code> --name machine-a
```

第二份 `--state-dir <state-b>` 再对接一台。对一台**已经在线且 Vault 已接入**的设备，从打开登录页起计时（含登录与切换，不含该设备首次 clone 或 install-service），须在 3 分钟内切换并改一条中文注释（SC-001）。切换 B 看该注释不在 B。A 离线（停掉 serve）时列表显示离线，转发失败说清，不得显示成功。

远程打开设置：没有托盘/关窗/应用更新；路径填的是 Host 那台的路径。本机桌面打开同一前端时不得出现设备切换条，也不得切换其它设备。

两个浏览器同时改 A 的非破坏性字段：都成功。删除技能必须各自确认，用另一会话的 planId 确认必须拒绝。

## 3. 对话管理（US3）

本机桌面（不经过管理站）打开对话。技能详情说「完善中文注释」：该技能 `.manager` 注释变化，用户未敲 CLI。问「这个技能的上游是什么」：能答来源，用户未粘贴 `source.yaml`。

未打开那些技能详情时说「先处理译文待更新的」或「有哪些待更新」：Agent 对得上库里那些项，用户未点名或粘贴清单。

快照未含全文时说「把这个技能的说明读一下」：Agent 经工具读出片段，用户未粘贴正文。不验收 MCP。不要求为「译文待更新」准备新的 CLI 筛选开关。

进行中任务针对技能 A 时切到 B 再发「完善这个」：A 的任务仍跑完 A，新任务写 B。

进行中任务发出后立刻取消：该任务不得显示成功，库内不应出现该任务后续写入。

删除技能：界面出现计划预览，确认前库内还在。未开 `git_auto_push` 时对话触发的上传失败并指向设置；打开后上传不再逐步确认。

不发消息时空闲一段时间：库不被 Agent 自行改写（定时上游检查仍可按 002 发生，那是调度器不是对话）。

管理 CLI 未登录模型：对话说明不能执行，库文件不变。

制造一次库拉取冲突或 `paused_local_change` 后打开对话（只说「处理这个」）：Agent 侧已有涉及的技能/路径与原文错误；用户消息里没有粘贴报错或技能名。不验收专用合并界面。

## 4. 管理站不可用（SC-005）

停掉 `station serve`。本机桌面仍能改一条注释。CLI 在 Host 仍在时经 Host 执行；Host 也不在时直连 core 仍能读库。

## 5. 回归

002 的核心 CLI 场景（list/set/deploy/sync pull）在 Host 在场与不在场各跑一次，结果一致。不得出现第二份技能库目录。

## 6. 本轮实施取证（2026-11-11）

在 `cursor/adr-host-management-station-1ec4` 实现 US1–US3 代码后记录。云上补跑（赶工修复 + T063 快照对照）结果如下。**未做** D9 默认 CLI 锁定。

| 项 | 结果 |
|---|---|
| `cargo test -p skill-manager-host -p skill-manager-station` | **通过**（host 9 单测 + git_auto_push / lock / plans；station 6 单测）。含锁双开、确认计划绑 session 且确认后消耗、Agent 不得 `plans_confirm` / 不得写 `git_auto_push`、station 不依赖 core、恢复码登录 `pending_rebind`、station 409 映射、快照含 `newCandidates` / `pullConflicts` / `lastCommandFailures` / 无 ref 时 `view.pageObjects`、未检测到 CLI 不得标 running |
| `cargo test -p skill-manager-cli --bin skill-manager coord` | **通过**（preset/agent/tag/archive 写入映射到 Host invoke） |
| `cargo check -p skill-manager-host -p skill-manager-station -p skill-manager-cli -p skill-manager-core` | **通过** |
| 前端 `npm test`（vitest） | **23 files / 88 tests 通过**，无未处理异常。含 DeviceBar、LoginPage 恢复码重绑、planConfirm 须 `needsConfirmation`、HttpTransport 在无 `EventSource` 时不抛 |
| `cargo check -p skill-manager-desktop` | **通过**。本机 `gdk-3.0` 3.24.41 / `webkit2gtk-4.1` 可用；先 `npm run prepare:cli -- --offline` 写出 `src-tauri/binaries/skill-manager-x86_64-unknown-linux-gnu`（gitignore，不提交），再 `cargo check -p skill-manager-desktop --offline` 于 2026-11-11 通过。一条命令：`npm run check:desktop`。desktop `build.rs` 会从已构建 CLI 复制 sidecar；两者都缺时退出 101 并提示上述命令，不假装成功。不是缺 GTK |
| 快照对照表 | 已写入 [snapshot-inventory.md](snapshot-inventory.md)。只读事实有快照字段或工具命令；**不声称齐全**（部分写入对内置 Agent 未开放） |
| `vault clone` + 真机 `host serve` 互斥与开机项 | **未取证**：Cloud Agent 环境无用户级 launchd/systemd 登录项、无第二台真机 |
| 管理站双 state-dir 浏览器登录/切换/3 分钟改注释（SC-001） | **未取证**：无图形浏览器连到已配对 Host |
| 对话「完善中文注释」、切页不串、取消不得成功 | **未取证**：依赖本机已登录的管理 CLI；D9 默认 CLI **未锁定**（另开对话） |
| 管理站不可用时桌面改库（SC-005） | 代码路径：桌面内嵌 Host，station crate 不在桌面依赖链；**真机未取证** |
| 002 CLI Host 在场/不在场回归 | **未取证**本轮未对着真实 Vault 跑 list/set/deploy |

安装 Skill 源：`skill-manager/assets/skills/skill-manager-install/`。内置管理说明：`skill-manager/assets/skills/skill-manager-builtin-agent/`（不入库）。

术语与 ADR 0002 / GLOSSARY 对照：无新名，不改术语。
