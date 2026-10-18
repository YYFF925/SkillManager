# Implementation Plan: Skill Manager 正式产品

**Branch**: 当前 `main`，未创建功能分支；项目已初始化 Git 并配置 Gitea `origin` 与 GitHub `github`，按 feature 目录推进 | **Date**: 2026-10-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/002-skill-management/spec.md`

## Summary

构建 Skill Manager v1 桌面应用：以用户已有 SkillManagerVault（Git 仓库）为唯一技能内容真值源，Rust 核心承载库管理、部署合成、上游更新、跨机同步与工具中心业务，GUI（Tauri 2 + React/TypeScript）与 CLI（clap）双入口共用同一核心，本机状态（部署记录、档案启用、Agent 配置、设置、工具安装记录）存 SQLite，共享资产零入库、不做文件镜像。

实施资料分两类使用：技术实现先按 [技术参考索引](../../docs/technical-reference/README.md)定位对应主题，复用已提取的数据、素材与适用实现经验，不继承旧业务或旧架构；GUI 按 [原型页面与效果图索引](../001-skill-management-prototype/ui-prototype/README.md)、[视觉说明](../001-skill-management-prototype/visual-reference.md)和[交互细节](../001-skill-management-prototype/ui-interaction-audit.md)实现，必要时操作同目录 `index.html` 核对动态效果。原型是正式界面的视觉与交互验收基线，各期按 quickstart 对照，不以原型使用 HTML 或模拟状态为由另做一套界面；业务差异按 spec.md 处理并记录。

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

**Storage**: 共享资产 = SkillManagerVault 内文件（技能目录 + `.manager/`、预设定义文件、指令源文件、工具资料文件，随库 Git 同步）；本机状态 = SQLite（`skill-manager.db`，仅本机状态表，见 data-model.md）

**Testing**: cargo test（核心业务）、Vitest + React Testing Library（前端交互）、CLI 端到端验收（对照 quickstart.md 场景）

**Target Platform**: macOS（当前用户环境）；Agent 目录资产覆盖跨平台路径，界面不锁死平台

**Project Type**: desktop-app + CLI（双入口共用核心 crate）

**Performance Goals**: 几十至几百技能规模下记录启动扫描与浏览/筛选的实际耗时；文件为真值、浏览使用内存索引。不将未确认的“<1s”“微秒级”作为产品承诺或验收门槛，不预建数据库镜像或性能机制。

**Constraints**: 首次接入必须选择已有本地 SkillManagerVault；Git 认证复用本机配置；界面仅简体中文

**Scale/Scope**: 单用户个人库（几十至几百技能）、约 50 个内置 Agent 目标、单机运行

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| 原则 | 对照结论 |
|---|---|
| I. 用户对能力的掌控优先 | 规格全部状态表达诚实条款（SC-004）直译为实现约束：检查/任务/同步状态不得冒充完成。设计无违背 |
| II. SkillManagerVault 是统一维护中心 | 真值全部在库文件；SQLite 只存本机状态且不做镜像——比旧项目更贴合。通过 |
| III. 管理能力可持续维护 | `.manager` 字段即项、可由 GUI/CLI/Agent 直接读写、未知字段保留写回。通过 |
| IV. 维护成果与上游更新共存 | 同项上游值优先按管理资料字段粒度落地；来源记录绑定技能上游并随 SkillManagerVault 共享，库同步不重新绑定。通过 |
| V. GUI 与 Agent 同一规则 | 单一 Rust 核心、双壳调用；CLI 按 FR-050 清单全覆盖（contracts/cli.md 对照表）。通过 |
| VI. 最小充分控制复杂度 | 已裁决删除：改动归属跟踪、ID 体系、sidecar 镜像、DB 迁移体系（文件格式无迁移）、自动发现。通过 |

技术基线核对：与宪章"技术栈与架构基线"完全一致；"SQLite 经 rusqlite 访问，本机状态及必要的索引或缓存"的使用方式收缩为"仅本机状态"（索引用内存，必要时后续加缓存，不预建）。

**Phase 1 设计后复查（含 2026-10-16 规划补齐）**：原则 I——本机运行侧的双入口分类按"是否影响管理结果"划分，
并把 `git_auto_push`（上传长期授权）明确为**不得** CLI 可写，避免 Agent 绕过 FR-038 的授权边界，通过。
原则 V——本次补齐正是为消除"GUI 有入口、CLI 无入口"的不对称，`settings` 命令组与 GUI 设置页共用同一 core，通过。
原则 VI——契约取法为"一条读 + 两个写 + 两条只读报告"，并**拒绝**为每个配置键都加 CLI 入口、拒绝新增组合调度命令
（CLI 手动检查由 FR-029 的三条既有命令分别表达）、删除两处无写入点的死配置；新增复杂度均有具体后果支撑，通过。

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
└── tasks.md             # 已生成的开发任务；本轮需求质检后同步修订
```

### Source Code (repository root)

```text
skill-manager/
├── Cargo.toml               # workspace 根
├── crates/
│   ├── core/                # 业务核心（无 UI 依赖）：库扫描、.manager、部署合成、
│   │   └── src/             #   上游更新、同步、工具中心、状态存储
│   │       ├── vault/       #   SkillManagerVault 接入、目录模型、内存索引、文件监听
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

### 桌面安装包与 CLI 交付（2026-10-18）

对应 FR-050/051、US10 场景 5。现有安装包未携带 CLI，导致新电脑只有维护技能而无法执行管理动作；该后果阻断 Agent 维护，必须在打包层补齐。

- 使用 Tauri 原生 `bundle.externalBin`（外部二进制打包）携带 `skill-manager`，不新增运行时桥接、安装器或全局 PATH 修改。macOS 位于应用包 `Contents/MacOS/skill-manager`；Windows 位于桌面主程序同目录的 `skill-manager.exe`。
- `scripts/prepare-cli.mjs` 在 Tauri 开发／打包前编译同一 workspace 中的 CLI，按目标平台准备带 target triple（目标平台三元组）后缀的二进制；编译失败直接阻止后续流程，避免再交付缺 CLI 的安装包。生成文件留在忽略的 `src-tauri/binaries/`，不提交构建中间件。
- 维护技能说明以应用安装位置定位 CLI，先验证 `--version`／`--help`，再通过绝对路径执行管理命令；不依赖 GUI 进程或个人 shell 初始化。Windows 自定义安装位置以实际快捷方式目标为准。仅安装技能时明确缺少应用，不退回开发目录。
- 发布验收从真实安装包取出应用，在无源码、无命令搜索路径的外部进程验证版本、帮助、隔离库查询与资料修改；结果记入 `checklists/m6-report.md` 第二十节。未运行的平台保留未验证。

实现依据：[Tauri 外部二进制](https://v2.tauri.app/develop/sidecar/)与[构建钩子](https://v2.tauri.app/reference/config/#buildconfig)。

## Milestones

> 目标：全部完成后才投入使用。里程碑是开发管理与方向检查点——每期按依赖自底向上，收尾执行 quickstart.md 对应场景并输出"验收场景核对表"（逐条通过/未过与证据），供用户检查是否偏离规划方向。

| 期 | 内容 | 覆盖 FR | 收尾检查 |
|---|---|---|---|
| M1 底座 | workspace 骨架、SkillManagerVault 接入、扫描与内存索引、`.manager` 读写、文件监听、CLI `skill` 查询与资料命令、GUI 技能库（浏览/搜索/筛选/详情/资料编辑） | FR-001~010（库组织与管理资料） | US1 现有库维护场景；获取入口同名在 M3、真实部署位置在 M2、来源变化提示在 M3 补验，不提前报告 US1 全部通过 |
| M2 部署 | Agent 检测/目录/启停、部署合成、SQLite 部署记录、已有部署同步、撤销、共享目录、Hermes default 与档案启用/收编 | FR-016~022、FR-026~028、FR-041~042 | US3 + US6 场景；SC-003 抽查；预设入口在 M5 补验 |
| M3 获取与上游 | 上游入库（进度/取消/分类规则/继承、来源发现跳过 `.agents/skills/`）、收编（接管/批量）、自建创建、仓库级检查（跨域一次/周期）、变化展示、单项/批量更新、同项合并、新增候选、跳过版本、删除清理 | FR-011~015、FR-029~035、FR-058 | US2 + US4 场景；SC-008 / SC-009 实测 |
| M4 同步与指令 | 拉取（静默/部署与已有指令目标刷新/冲突暂停/断连态）、上传两路径（全量提交语义）、指令源编辑/拼接/分发 | FR-036~040、FR-043 | US5 + US8 场景；SC-006 双机模拟 |
| M5 预设/工具/收尾 | 预设全链路、工具中心全链路（登记/检查命令/任务/回写/配套 Skill/未安装展示）、任务书含配套 Skill 指引、配套维护 Skill 编写并从本仓库远端入库、设置页、主题/字号、托盘、诊断、容量 | FR-016~025（预设）、FR-044~054 | US7/9/10 + SC-001 完整旅程 |

每期收尾动作（固定流程）：跑该期 quickstart 场景 → 生成核对表（场景、预期、实际、结论）→ 与规格偏差项列出并说明（纠偏或修订规格）。

## Implementation Status（M1～M5 实际结果，2026-10-15）

**代码入口**：`skill-manager/`（Cargo workspace：`crates/core` 业务核心、`crates/cli` clap 命令、
`src-tauri` Tauri 2 壳、`src/` React/TypeScript 前端）。开发、构建、测试与 CLI 用法见
[skill-manager/README.md](../../skill-manager/README.md)。SkillManagerVault 是共享内容真值；
本机状态在 SQLite `skill-manager.db`（macOS：`~/Library/Application Support/SkillManager/`），
CLI 用 `--vault` / `--state-dir` 覆盖，可在一台机器上模拟两台电脑。

| 期 | 结果 | 报告 |
|---|---|---|
| M1 底座 | 实施完成；quickstart 1.1～1.8 通过（1.1 首次选库对话框、1.3/1.4/1.6 的 GUI 保存点按未取证） | `checklists/m1-report.md` |
| M2 部署 | 实施完成；2.1～2.14 通过；Windows 模拟、部署弹窗结果面板仍保持未验证 | `checklists/m2-report.md` |
| M3 获取/上游 | 实施完成；3.1～3.12 通过；SC-007 通过，SC-008 已补齐真实 GUI 进度/取消、成功与失败路径（2026-10-17 T123） | `checklists/m3-report.md` |
| M4 同步/指令 | 实施完成；4.1～4.8 通过；GUI"未连接"状态已补取证，冲突界面渲染与定时器长时行为未实测 | `checklists/m4-report.md` |
| M5 预设/工具/环境 | 实施完成；预设、工具中心、设置/主题/字号/容量/诊断均实现并有运行证据；托盘长时运行与部分界面点按流程未取证 | `checklists/m5-report.md`（由主代理写入） |

**验证基线**：`cargo test --workspace` 241 项通过、`npm test` 22 项通过、
`cargo build --workspace` / `npx tsc --noEmit` / `npm run build` 通过。

**未闭合项（保持未过/待授权，不用勾任务代替验收）**：

1. ~~**本机运行侧整类缺 CLI 入口**~~——**2026-10-16 已闭合**（T093～T099）：原记为"共享目录路径缺 CLI 命令"，
   逐项清点后确认是"本机运行配置与只读报告"整类（同时缺 CLI 的还有自动检查频率、运行配置读取、
   诊断资料获取与导出、容量报告）。**规格、命令契约与实现均已按类补齐**
   （spec.md FR-050/FR-052/FR-053、contracts/cli.md 的 `settings` 命令组、data-model.md §2.6、
   research.md D22/D23、`crates/cli/src/commands/settings.rs`），quickstart 5.14～5.19 全部通过
   （证据见 `checklists/m5-report.md` 5.14～5.19 与 `contracts/cli.md` 的 T090 表 `settings` 行）。
2. **新增候选入库无 `tags`/`comment` 参数**——GUI/CLI 一致地只能补充分类；属契约限制（缺口 ②）。
3. **Hermes 档案独有技能的只读查看**已于 2026-10-17 由 T125 补齐（core、`hermes read`、Tauri、GUI），查看不收编、不改变启用名单；阅读沿用 Markdown（结构化文本）视图和吸顶控件。
4. **配套维护 Skill 的 Gitea 交付已闭合**（2026-10-17 T124）：用户授权发布后，在隔离库从真实上游入库 `c00e1a4`，再显式采用 `e2106f9`；检查不改内容，实际采用后新增 Hermes 阅读命令参考，技能/工具任务引用的库内路径均可解析。证据见 `checklists/m5-report.md` 第十二节。
5. **平台兼容不声明**：仅 macOS（arm64）实际运行验证；Hermes 启用的符号链接在 Windows 行为未验证。
6. **性能数据**：Apple M6 / 24 GB / macOS 27.0.1 / debug 构建下，50/200/500 个技能的
   `skill list` 与筛选、`skill get`、`skill check --all` 实测均 ≤ 0.08s；这是本机实测值，
   不作为"任何环境 <1s"的承诺。
7. **GUI 人工取证不完整**：卡片三点菜单、多选批量栏、入库/部署弹窗部分点按流程、
   托盘隐藏后长时检查运行等，按各期报告保持"未验证"。

## 规划补齐：本机运行侧的双入口覆盖（2026-10-16）

**触发**：实施收尾核对（T090）只发现"共享目录路径缺 CLI 命令"一项；用户要求按**整类**重扫，
而不是只修这一项。逐项清点的完整覆盖面矩阵见 `contracts/cli.md` 的 T090 一节。

**判定准则**（已写入 FR-050）：按"**是否影响管理动作的结果**"分类，不按"是不是设置"分类。

| 类别 | 项 | 依据 |
|---|---|---|
| 必须 CLI 可写 | `auto_check_minutes`（FR-029 三项共用周期） | Agent 需能按用户指令调整自动检查；共享目录改为按当前用户主目录自动识别，不是配置前置 |
| 必须 CLI 可读 | 全部运行配置与调度事实（含仅 GUI 写入的 `theme`、`font_scale`、`git_auto_push`） | Agent 需能解释当前行为（例如为何未自动检查） |
| 不提供 CLI 写 | `theme`、`font_scale`（界面偏好）、`git_auto_push`（上传长期授权） | 界面偏好不改变管理结果；长期上传授权只能由用户在 GUI 给出（FR-038） |
| 有意单侧 | 工具本机安装事实回写（只有 CLI） | FR-047/FR-050 明令 GUI 只生成任务 |
| 清理 | 自定义日志目录（删除）、Hermes 根（改按 `HERMES_HOME` 与平台默认解析） | 两键无任何写入点；可写 Hermes 根会与 Hermes 自身定位规则分叉 |

**契约取法（最小充分）**：新增单一 `settings` 命令组 = 一条读命令（读全部配置与调度事实）
\+ 一个写命令（检查频率）+ 两条只读报告（诊断、容量）。**不新增组合调度命令**——
CLI 的手动检查由 `skill check --all`、`sync pull`、`tool check --all` 分别表达 FR-029 的三项。

**本次规划变更的文件**：`spec.md`（FR-029/050/052/053 与 Clarifications）、
`contracts/cli.md`（`settings` 命令组、FR-050 覆盖表、T090 缺口重述）、
`data-model.md` §2.6（逐键可达性表、清理两键）、`research.md` D22/D23、
`quickstart.md`（新增场景）、`tasks.md`（T093～T100）。

## 规划补齐：共享目录技能浏览（2026-10-16）

**触发**：交付后追查"共享 Agent 页"缺失，结论是需求在**原型规格层**就断了——原型有该页、规格未立条目，
002 规格／规划／任务三层连续未接手，实现与后端入口均缺失（证据：`checklists/m5-report.md` 第六节第 9 条、
`checklists/visual-comparison.md` 的差异清单 B 项）。用户已确认该能力必须做，故按整条链补齐。

**范围判定（最小充分）**：不新增第二套目录浏览能力，而是把既有的"本机目标技能浏览"从**只按 Agent key**
扩到**按目标**——路径解析复用 `TargetResolver::resolve_with_override` 的 `Target::Shared` 分支（`deploy/targets.rs`），
目录扫描与受管判定复用 `LocalSkillService`（`deploy/local_skills.rs`），`shared` 的受管判定沿用 `target=shared` 的部署记录。

**不做**：不把共享目录变成可登记 Agent 或第二个管理库（FR-021 已定性）；不在该页增加"从库部署"按钮；
共享目录未受管技能按用户后续修订支持收编，复用既有 `adopt_skills` 与原部署接管，进入编码库，不新增其他目标部署。

**契约取法**：扩展现有 `agent local-skills` / `agent delete-local` 的取值，并以 `agent read-local` 阅读实际本机副本（`<target>` = 内置 Agent key、`shared` 或 `hermes:default`），
不新开命令组——该能力与 Agent 目录同构，新开面只会制造两条平行入口。

**本次规划变更的文件**：`docs/requirements.md`（新增"共享目录技能浏览"一节）、
`spec.md`（FR-042 扩到本机目标、新增 FR-055）、`contracts/cli.md`（`agent` 命令组取值与说明）、
`tasks.md`（Phase 16，T101～T108）、`quickstart.md`（新增场景，见 T107）。

## 规划补齐：设置页与 Agent 页按原型归位（2026-10-16）

**触发**：用户核对原型设置页，指出实现"少了内容"。逐区块对照（原型 `ui-prototype/index.html:576` 的 `settings()`）后确认两件事：
① Agent 检测／启停／排序／目录行内编辑被放在「编码 Agent」页，而 001 规格 FR-031、001 体验范围表、`requirements.md:214/230/232`、
`legacy-skill-capabilities.md:65`、`quickstart.md:39` 都写"设置在设置页"——该移动只在 plan/tasks 层排布，从未作为决定登记；
② 设置页缺原型的部分配置行。

**范围（按原型）**：
- Agent 检测区块迁回设置页（首个区块），组件复用，不在两处重复同一套管理。
- 设置页按原型区块顺序组织；补只读展示行：部署方式（文件复制）、语言（简体中文）、关闭行为（托盘模式）。
- 「全部 Agents」按原型语义改为**跨目标部署清单**（FR-057）——Agent 管理迁走后它不再是管理页。

**刻意不做（附理由）**：
- **「打开文件夹」**：原型自身是模拟控件（`index.html:840` 明确"原型不调用系统文件管理器"，只弹路径列表）。实现真实打开目录需新增 Tauri opener 依赖与权限，收益不成立，故不做。
- **「托盘／菜单栏图标」开关**：原型的开关"仅改变模拟状态"，实现也没有对应设置键（托盘行为是固定的"关闭收起到托盘"）。按只读呈现，不新增设置键与能力。
- **「更改目录」（更换 SkillManagerVault）**：原型是模拟目录对话框，但这条在实现里是**真实能力缺口**（库接入后 GUI 无换库入口）。先不动，因为"换库后本机状态（部署记录／Hermes 启用／工具记录）怎么处理"没有文档依据，待用户裁决后再入规格。

**本次规划变更的文件**：`spec.md`（FR-041 补位置、新增 FR-056/FR-057）、`tasks.md`（Phase 17，T109～T114）。

## 需求质检后的实施边界

- 本次评审记录位于 `checklists/readiness.md`，勾选只表示需求文字质量通过，不表示代码、远端或第三方协议已验证。
- 已确认行为的技术细节可在对应设计任务先落定再实现：T011 译文时效的存储；T021 覆盖层顺序/注释格式/暂停处置；T030 获取缓存/取消；T050 GUI 自动上传确认记录；T056 当前 Hermes 协议；T074 非标准版本比较。不能以存在任务代替这些设计已经完成。
- SkillManagerVault 是各电脑同步的同一技能库，上游仓库是技能检查与获取更新的来源；来源记录随 SkillManagerVault 共享，不因另一机拉取而重新绑定。质检中把这两者混同所产生的“来源冲突”已撤销，不是业务待决项。用户已确认拉取到指令源变化后按本机已有启用配置自动刷新指令目标，CHK026 已收口；第三方协议查证是设计工作，不要求用户替 Agent 调查。

## Complexity Tracking

> 无宪章违背项需辩护。所有"新增复杂度"裁决均已在规格 Clarifications 与 research.md 记录理由；被拒绝的复杂方案（ID 体系、改动归属账本、sidecar 镜像、DB migration 链、路径级提交拆分）记录于 research.md 供回溯。


## 共享目录自动识别修订（2026-10-16）

依据用户最新要求及 FR-021/042/050/055/056，共享目标由 core 统一按系统当前用户主目录解析 `.agents/skills`。移除 SQLite 路径设置、CLI 写命令、Tauri 写命令及设置页/部署弹窗的路径编辑；不保留旧路径兼容或迁移。`settings get shared_dir` 与设置页只读展示同一结果，目标浏览直接复用目录服务，不存在时为空且不创建，首次部署沿用现有目录创建逻辑。

本机技能交互后续修订：空列表留白并移除底部解释；共享目录未受管技能复用收编与部署接管；`LocalSkillService::read` 通过 CLI 与 Tauri 共用，读取实际本机副本和目录内文件，GUI 用只读详情呈现，受管项可前往库内管理。文件阅读沿用现有路径边界，避免文件参数读到技能目录外的私人文件。全局基础样式统一可用控件的手型光标，卡片保留独立的勾选、收编、复制与删除动作，避免点击动作误开详情。

## 部署交互修订（2026-10-16）

部署交互后续修订（2026-10-16）：GUI 目标选择仅展示同域 `detected && enabled` 的 Agent。弹窗载入本机部署记录作为基线，逐目标保留多技能部分部署状态，用户点击改变期望状态，「保存」将差量分为部署和撤销并调用现有 core 操作；部分失败保留结果且更新成功项基线，重试不重复成功项。预设复用同一弹窗。各技能卡片复用快捷部署图标，点击即时执行并刷新真实记录。移除前后端、CLI 的预演参数与分支；共享目录及 Hermes default 仍可通过显式 CLI 目标管理。清理常驻规则说明，保留真实错误、必要状态和操作影响（FR-014/021/052）。

## 界面细节修订（2026-10-17）

本轮不改变部署与管理数据边界，只修订 FR-010/021/026 对应的界面表现：

- 技能库内容最大宽调整为约 1440px、分类列 132px，让宽窗口卡片获得更多横向空间；快捷部署图标保持紧凑右对齐。
- 编码库技能卡片快捷部署同时包含共享 Agent 目标；悬浮提示即时显示目标名，不附加“部署到/撤销部署”。
- 技能详情正文使用结构化 Markdown 阅读视图；页签和阅读状态控件吸顶；代码块跟随主题。
- 技能详情「文件与部署」目标区域只显示可点击图标；移除「本机已有部署」标题、页脚重复部署按钮、管理专用目录说明；来源页不重复展示库内位置。
- Hermes 档案选择显示 `profile.yaml` / `SOUL.md` 推导的 bot 显示名，回退目录 id；移除下拉底部说明。
- 系统 `.app` 图标改用老项目 `FeiSkillDock` 的多尺寸 Tauri 图标集。

逐项交付、测试和 GUI 证据见 `checklists/m6-report.md` 第十三～十八节；现行行为以 `spec.md`、`docs/requirements.md` 与实现为准。

## 来源发现跳过 `.agents/skills/`（2026-10-17）

依据 spec.md FR-058：从来源仓库识别可入库技能时，默认不去找 `.agents/skills/` 里的技能。这是发现范围规则，不是仓库内排除清单，也不扩展 FR-033 的「不入库并记住」。

- 改动集中在 `skill-manager/crates/core/src/upstream/discover.rs` 的候选扫描；`preview`、`import`、上游检查新增候选共用同一发现结果。
- 相对仓库根路径为 `.agents/skills` 或其子路径的目录不作为技能、也不再向下扫描。
- 默认分类按**跳过之后**的候选数量计算：只剩一个则不自动分类。
- 本机共享部署目录（当前用户主目录下的 `.agents/skills`）仍按 FR-021/042/055 作为部署与浏览目标，发现规则不改部署路径。
- 不新增 CLI 开关、排除文件或兼容打开该路径的入口。已入库技能不受影响。

**Phase 1 设计后复查**：原则 VI——用发现阶段跳过该路径，拒绝逐项排除清单与可配置扫描开关；原则 I——候选里根本不出现这些技能，不把未扫描写成用户拒绝。通过。

## 交互与部署修订（2026-10-17～18）

对应 spec.md FR-003/007/021/025/037/038/052。本批在实现中落定，未新增任务号；交付、测试与未取证项见 `checklists/m6-report.md` 第十九节，验收场景见 `quickstart.md` 的 M6 补充二。

- **后台库任务不占用界面上下文锁**：`AppContext::fork()` 复用当前库快照、另开一条本机状态连接；`AppState::with_background_context` 与 `commands::background()` 在 `spawn_blocking` 中执行阻塞的 Git 与文件操作，并由 `background_jobs` 互斥量串行化。界面读取不再等待耗时同步完成；上传授权与结果语义不变（FR-038）。实现：`crates/core/src/context.rs`、`src-tauri/src/commands/mod.rs`、`commands/{deploy,sync}.rs`。
- **共享 Agent 作为部署目标**：编码库的单项、多选与预设部署把共享目标与普通 Agent 同列，沿用既有目标解析、域边界与支持声明过滤，不新增准入规则；无已检测 Agent 时该目标仍可选。实现：`src/components/DeployDialog.tsx`、`src/App.tsx`。
- **全局库拉取入口**：页头按钮调用既有库拉取能力并刷新当前页面，不做上游检查、不触发上传，也不新增组合命令（FR-037）。
- **分类重命名**：core 增加 `vault::manage::rename_category`，整套移动一级分类目录；目标分类已存在时拒绝（避免隐式合并），同名返回 0 项；CLI 出口为 `skill category rename`，GUI 入口在分类栏，「全部分类」「未分类」不提供（FR-003）。实现：`crates/core/src/vault/manage.rs`、`crates/cli/src/commands/skill.rs`。
- **Hermes 档案技能池**：候选池取龙虾库中已安装进 Hermes 的部署记录，`hermes:default` 与 `agent:hermes` 视为同一次安装并按技能名去重；启用前置条件改为 default 目录下实际存在该技能目录，不再依赖特定记录名。设计登记见 `data-model.md` 2.2。实现：`crates/core/src/hermes/enables.rs`。
- **托盘左键**：关掉左键展开菜单，左键切换主窗口显示/隐藏并在显示时取消最小化，菜单交由右键（FR-052）。实现：`src-tauri/src/tray.rs`。
- **库列表不支持项位置**与**悬浮提示自绘定位**属显示层：前者只调整已过滤结果的排列顺序，不改查询条件（FR-007）；后者把提示层移入 portal 并按视口边缘翻转收边。
