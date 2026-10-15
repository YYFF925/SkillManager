# Implementation Plan: Skill Manager 正式产品

**Branch**: `002-skill-management`（项目当前未初始化 Git，按 feature 目录推进） | **Date**: 2026-10-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/002-skill-management/spec.md`

## Summary

构建 Skill Manager v1 桌面应用：以用户已有 SkillVault（Git 仓库）为唯一技能内容真值源，Rust 核心承载库管理、部署合成、上游更新、跨机同步与工具中心业务，GUI（Tauri 2 + React/TypeScript）与 CLI（clap）双入口共用同一核心，本机状态（部署记录、档案启用、Agent 配置、设置、工具安装记录）存 SQLite，共享资产零入库、不做文件镜像。

核心设计决策（详见 [research.md](research.md)）：

- 技能身份 = 管理域 + 技能名，不引入数字 ID 或 UUID；
- `.manager/` 字段即项：`meta.yaml`（tags/comment/agents/oses）+ `translation.md` + `overlay/` + `source.yaml`，写回保留未知字段；
- 上游更新"同项上游值优先"按上述字段粒度执行；
- 软件执行上传时提交工作区全部改动，不区分改动来源（用户裁决 2026-10-15）；
- SQLite 仅存本机状态，绝不镜像共享资产（吸取旧项目 sidecar 双写教训）；
- 配套维护 Skill 为单一技能，从本项目 Gitea 远端作为普通上游技能入库与更新，维护任务书引用该技能而非内联规范（D9）。

## Technical Context

**Language/Version**: Rust（stable，2024 edition）、TypeScript（React 18+）

**Primary Dependencies**: Tauri 2、Vite、Tailwind CSS、clap、rusqlite（bundled）、notify（文件监听）、git2 不使用——仓库操作调用本机 `git` 命令（用户裁决：不内置 Git 引擎）

**Storage**: 共享资产 = SkillVault 内文件（技能目录 + `.manager/`、预设定义文件、指令源文件、工具资料文件，随库 Git 同步）；本机状态 = SQLite（`skill-manager.db`，仅本机状态表，见 data-model.md）

**Testing**: cargo test（核心业务）、Vitest + React Testing Library（前端交互）、CLI 端到端验收（对照 quickstart.md 场景）

**Target Platform**: macOS（当前用户环境）；Agent 目录资产覆盖跨平台路径，界面不锁死平台

**Project Type**: desktop-app + CLI（双入口共用核心 crate）

**Performance Goals**: 启动全库扫描 < 1s（几百技能规模）；浏览/筛选走内存索引（微秒级）；无其他量化指标承诺（遵循项目"不凭空承诺性能"纪律）

**Constraints**: 首次接入必须选择已有本地 SkillVault；Git 认证复用本机配置；界面仅简体中文

**Scale/Scope**: 单用户个人库（几十至几百技能）、约 50 个内置 Agent 目标、单机运行

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| 原则 | 对照结论 |
|---|---|
| I. 用户对能力的掌控优先 | 规格全部状态表达诚实条款（SC-004）直译为实现约束：检查/任务/同步状态不得冒充完成。设计无违背 |
| II. SkillVault 是统一维护中心 | 真值全部在库文件；SQLite 只存本机状态且不做镜像——比旧项目更贴合。通过 |
| III. 管理能力可持续维护 | `.manager` 字段即项、可由 GUI/CLI/Agent 直接读写、未知字段保留写回。通过 |
| IV. 维护成果与上游更新共存 | 同项上游值优先按字段粒度落地（source.yaml 天然继承"上游未提供则保留"）。通过 |
| V. GUI 与 Agent 同一规则 | 单一 Rust 核心、双壳调用；CLI 按 FR-050 清单全覆盖（contracts/cli.md 对照表）。通过 |
| VI. 最小充分控制复杂度 | 已裁决删除：改动归属跟踪、ID 体系、sidecar 镜像、DB 迁移体系（文件格式无迁移）、自动发现。通过 |

技术基线核对：与宪章"技术栈与架构基线"完全一致；"SQLite 经 rusqlite 访问，本机状态及必要的索引或缓存"的使用方式收缩为"仅本机状态"（索引用内存，必要时后续加缓存，不预建）。

## Project Structure

### Documentation (this feature)

```text
specs/002-skill-management/
├── spec.md              # 规格（$speckit-specify 输出，含 Clarifications）
├── plan.md              # 本文件
├── research.md          # Phase 0：设计决策记录与待调研项
├── data-model.md        # Phase 1：库文件格式与本机状态模型
├── quickstart.md        # Phase 1：按里程碑的验收验证指南
├── contracts/
│   └── cli.md           # Phase 1：CLI 命令契约与 FR-050 覆盖对照
├── checklists/
│   └── requirements.md  # 需求质检清单
└── tasks.md             # Phase 2 输出（$speckit-tasks，本次不创建）
```

### Source Code (repository root)

```text
skill-manager/
├── Cargo.toml               # workspace 根
├── crates/
│   ├── core/                # 业务核心（无 UI 依赖）：库扫描、.manager、部署合成、
│   │   └── src/             #   上游更新、同步、工具中心、状态存储
│   │       ├── vault/       #   SkillVault 接入、目录模型、内存索引、文件监听
│   │       ├── manager/     #   .manager 读写与字段合并
│   │       ├── deploy/      #   部署合成（overlay+注释）、目标管理、同步
│   │       ├── upstream/    #   来源获取、检查、更新、新增候选、跳过
│   │       ├── sync/        #   git 封装（本机 git 命令）、拉取、上传、冲突
│   │       ├── hermes/      #   档案启用与收编
│   │       ├── preset/      #   预设定义文件
│   │       ├── toolcenter/  #   工具资料、检查命令执行、任务生成
│   │       └── state/       #   SQLite 本机状态（部署记录、档案启用、配置、设置）
│   └── cli/                 # clap 入口（Agent 主要用户）
│       └── src/main.rs
├── src/                     # 前端（React + TS + Vite + Tailwind）
│   ├── pages/               #   技能库（code/lobster）、预设、Agent、Hermes、工具中心、设置
│   ├── components/
│   └── services/            #   Tauri command 调用封装（类型化）
├── src-tauri/               # Tauri 2 壳：command 层薄封装 core，无业务逻辑
├── assets/
│   └── skills/              # 配套维护 Skill 源文件（随本仓库开发维护，
│                            #   经本项目 Gitea 远端作为普通上游技能入库，见 D9）
└── tests/                   # 集成测试（CLI 验收、双入口一致性）
```

**Structure Decision**: Cargo workspace 三成员（core / cli / src-tauri 复用 core）。业务规则只存在于 `core`，CLI 与 Tauri command 都是薄入口层，保证宪章 V"同一套规则、不形成两套体系"在代码结构上成立。

## Milestones

> 目标：全部完成后才投入使用。里程碑是开发管理与方向检查点——每期按依赖自底向上，收尾执行 quickstart.md 对应场景并输出"验收场景核对表"（逐条通过/未过与证据），供用户检查是否偏离规划方向。

| 期 | 内容 | 覆盖 FR | 收尾检查 |
|---|---|---|---|
| M1 底座 | workspace 骨架、SkillVault 接入、扫描与内存索引、`.manager` 读写、文件监听、CLI `skill` 查询与资料命令、GUI 技能库（浏览/搜索/筛选/详情/资料编辑） | FR-001~012（管理资料与库组织部分） | US1 全部场景 |
| M2 部署 | Agent 检测/目录/启停、部署合成、SQLite 状态库与部署记录、已有部署同步、撤销、dry-run、共享目录、Hermes default 与档案启用/收编 | FR-013~022、FR-026~028、FR-041~042 | US3 + US6 场景；SC-003 抽查 |
| M3 获取与上游 | 上游入库（进度/取消/分类规则/继承）、收编（接管/批量）、自建创建、仓库级检查（跨域一次/周期）、变化展示、单项/批量更新、同项合并、新增候选、跳过版本、删除清理 | FR-011~015（获取部分）、FR-029~035 | US2 + US4 场景；SC-008 实测 |
| M4 同步与指令 | 拉取（静默/部署刷新/冲突暂停/断连态）、上传两路径（全量提交语义）、指令源编辑/拼接/分发 | FR-036~040、FR-043 | US5 + US8 场景；SC-006 双机模拟 |
| M5 预设/工具/收尾 | 预设全链路、工具中心全链路（登记/检查命令/任务/回写/配套 Skill/未安装展示）、任务书含配套 Skill 指引、配套维护 Skill 编写并从本仓库远端入库、设置页、主题/字号、托盘、诊断、容量 | FR-016~025（预设）、FR-044~054 | US7/9/10 + SC-001 完整旅程 |

每期收尾动作（固定流程）：跑该期 quickstart 场景 → 生成核对表（场景、预期、实际、结论）→ 与规格偏差项列出并说明（纠偏或修订规格）。

## Complexity Tracking

> 无宪章违背项需辩护。所有"新增复杂度"裁决均已在规格 Clarifications 与 research.md 记录理由；被拒绝的复杂方案（ID 体系、改动归属账本、sidecar 镜像、DB migration 链、路径级提交拆分）记录于 research.md 供回溯。
