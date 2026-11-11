# Implementation Plan: 本机 Host、统一管理站与内置管理 Agent

**Branch**: `003-host-station-agent`（git 工作分支 `cursor/adr-host-management-station-1ec4`） | **Date**: 2026-11-11 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/003-host-station-agent/spec.md`

## Summary

在不改技能管理业务规则的前提下，把今日只活在 Tauri 里的进程内协调抽成 **Host**：本机唯一操作入口。再加 **管理站**（一次登录、出站对接、切换当前设备、转发到该机 Host）和 **内置管理 Agent**（ACP 拉起成熟 CLI，经 Host 用现有 JSON CLI 做事）。Vault 仍只在各机克隆；管理站 crate 不依赖 core。

核心决策见 [research.md](research.md)：invoke 同名 POST + SSE；Host 出站长连接；CLI 在 Host 在场时走 Host；确认计划只覆盖删除/覆盖；`git_auto_push` 仍仅 GUI 可写；ACP 默认 CLI 按 OpenCode → Codex → Gemini 探测锁定；内置 Agent 自动附带全局 + 当前界面两层上下文（D13），不做冲突合并 UI；新 UI 先走设计层。

## Technical Context

**Language/Version**: Rust（stable，与现仓库 edition 一致）、TypeScript（React 18+）

**Primary Dependencies**: 宪章基线（Tauri 2、Vite、Tailwind、clap、rusqlite）。新增限于 Host HTTP、SSE、出站长连接、argon2、TOTP、ACP stdio 客户端；不换桌面框架，不内置 Git 引擎

**Storage**: 共享资产 = 既有 SkillManagerVault 文件。Host 本机 SQLite 增设备/对话/计划表。管理站独立 SQLite（管理员、会话、设备）。对话不进 Vault

**Testing**: cargo test（锁、确认计划、station 不依赖 core、转发离线失败）；Vitest（Transport 切换、远程隐藏桌面专属控件）；quickstart 场景作人工/端到端验收

**Target Platform**: 与 002 相同的 macOS / Windows 桌面；无窗口 Host 与管理站另支持在 Linux 云电脑/反代后以服务运行

**Project Type**: desktop-app + CLI + 本机服务 + 管理站入口（同一软件三角色）

**Performance Goals**: 个人单操作者、数台设备。切换设备后一次非破坏性保存应在规格 SC-001 的三分钟预算内完成（含登录，不含首次 clone）

**Constraints**: 管理站不持有 Vault；同一 state-dir 单 Host；不对每台开端口；不唤醒休眠电脑；界面简体中文；不把第三方 Agent CLI 打进安装包

**Scale/Scope**: 一个管理员、数台 Host、复用现有全部技能管理页面；新增登录、设备条、对话三块界面

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| 原则 | 对照结论 |
|---|---|
| I. 掌控优先 | FR-022、确认计划、未登录模型不得冒充已写库。通过 |
| II. Vault 是统一维护中心 | 管理站 crate 不依赖 core；clone 已有远端。通过 |
| III. 管理能力可持续维护 | 远程与对话走同一套 `.manager` 写入。通过 |
| IV. 上游与个人成果共存 | 不改 002 更新规则。通过 |
| V. GUI 与 Agent 同一规则 | 工具经 Host 调既有 CLI；确认不可口头伪造。通过 |
| VI. 最小充分 | 无任务队列、无自研 FRP 产品、无第二套 REST 资源、无看门狗。通过 |
| VII. 个人自用 | 单管理员、无角色租户；多设备不是多用户。通过 |

技术基线：HTTP/SSE/TOTP 是基线内入口扩展。ACP 客户端与隧道库为辅助依赖，实施时选成熟实现，不修订宪章。

**Phase 1 设计后复查**：data-model 把对话/设备放本机或管理站 SQLite、不镜像技能；contracts 让 station 只做登录与转发；`git_auto_push` 仍禁止 Agent/CLI 写。通过。

## Project Structure

### Documentation (this feature)

```text
specs/003-host-station-agent/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md
│   ├── host-http.md
│   ├── station-http.md
│   └── agent-bridge.md
├── checklists/
│   └── requirements.md
└── tasks.md              # 由 speckit-tasks 生成，本命令不创建
```

### Source Code (repository root)

```text
skill-manager/
├── crates/
│   ├── core/                 # 不变：技能业务
│   ├── host/                 # 新增：AppState、锁、HTTP invoke/SSE、确认、Agent 桥、调度
│   ├── station/              # 新增：登录/TOTP/设备/长连接转发；不得依赖 core
│   └── cli/                  # 增加 host/station/vault clone；Host 在场时写入转发
├── src/                      # React：Transport、登录/设备条/对话；远程隐藏托盘与更新
├── src-tauri/                # 薄壳：内嵌 Host，保留托盘与更新
└── assets/skills/
    ├── skill-manager-maintenance/
    └── skill-manager-install/  # 新增安装 Skill 源
```

**Structure Decision**: 在现有 Cargo workspace 增加 `host` 与 `station`。桌面、无窗口、管理站共用 `skill-manager` CLI 二进制（桌面包仍用 Tauri 主程序 + 已有 externalBin）。前端仍是一份 Vite 应用。新界面必须先改 `design/` 与 `ui-prototype`，同一轮再改 React。

## Complexity Tracking

> 无原则违规。两新 crate 是为「管理站拿不到 Vault」的硬边界，不是多余分层。
