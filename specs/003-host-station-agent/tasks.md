---
description: "003 Host、管理站与内置管理 Agent 开发任务，按用户故事组织"
---

# Tasks: 本机 Host、统一管理站与内置管理 Agent

**Input**: 本目录 `spec.md`、`plan.md`、`research.md`、`data-model.md`、`contracts/`、`quickstart.md`；约束 [宪章](../../.specify/memory/constitution.md) 2.1.0 与 [ADR 0002](../../docs/adr/0002-host-management-station-and-agent.md)。

**范围**: US1–US3（均可 P1，按 Clarifications 分里程碑 US1 → US2 → US3）。不实现冲突合并界面、主动维护、跨设备总览、唤醒休眠、管理站自行申请证书。对话必须自动带上**全局 + 当前界面**两层上下文（足够办事，举例非穷尽）：客户端只报引用，Host 从 core 补齐；快照为摘要+计数+引用；内置说明不入库；不要求用户为 Agent 拼现场。技能业务仍走 `skill-manager-core`，不在 Host/station 重写。

**测试**: 规格未要求 TDD。不对每个入口先写失败测试。锁、确认计划、station 不依赖 core 等后果不可逆的边界，在对应实现任务里带 cargo 断言。GUI 与双机场景按 `quickstart.md` 人工验收。

**界面**: 登录、设备条、对话等新界面先改 `design/` 与 `specs/001-skill-management-prototype/ui-prototype/`，同一轮按 `.agents/skills/frontend-design/SKILL.md` 对照表改 React。

## Format: `[ID] [P?] [Story] Description`

- `- [ ] Tnnn`；故事阶段附 `[USn]`；`[P]` 表示不同文件、无未完成前置时可并行。
- 同一文件串行。路径相对仓库根，代码在 `skill-manager/`。

## Phase 1: Setup（工程准备）

**Purpose**: 加上 host/station crate 与 CLI 子命令骨架，不实现业务。

- [X] T001 在 `skill-manager/Cargo.toml`、`skill-manager/crates/host/Cargo.toml`、`skill-manager/crates/station/Cargo.toml` 把 workspace members 扩为 core/cli/host/station/src-tauri；`station` 的 Cargo.toml **不得**依赖 `skill-manager-core`；host 依赖 core。
- [X] T002 在 `skill-manager/crates/host/src/lib.rs`、`skill-manager/crates/station/src/lib.rs` 建立最小库入口与模块划分（host: lock/runtime/http；station: auth/devices/proxy），保证 `cargo check -p skill-manager-host -p skill-manager-station` 通过。
- [X] T003 在 `skill-manager/crates/cli/src/main.rs` 增加 `host` / `station` / `vault clone` 子命令骨架（与 `specs/003-host-station-agent/contracts/cli.md` 名称一致），未实现时非零退出并说明，不得伪成功。
- [X] T004 [P] 在 `skill-manager/src/services/transport.ts` 定义 `TauriTransport | HttpTransport` 接口（invoke 同名、subscribe 事件），暂不改现有 `client.ts` 调用点。

**Checkpoint**: 两新 crate 可检查；CLI 新子命令存在但未做事。

---

## Phase 2: Foundational（故事共用底座）

**Purpose**: 抽出 Host 运行时、state-dir 单实例锁、桌面内嵌 Host。所有故事依赖本阶段。确认计划、管理站、对话不在本阶段。

**⚠️ CRITICAL**: 未完成本阶段不得开始 US1–US3。

- [X] T005 将 `skill-manager/src-tauri/src/commands/mod.rs` 的 `AppState`（上下文、`background_jobs`、`VaultWatcher`、`fetch_jobs`、`schedule_report`）迁到 `skill-manager/crates/host/src/runtime.rs`，Tauri 改为持有 Host runtime；core 仍无 HTTP、无 Agent 运行时。
- [X] T006 在 `skill-manager/crates/host/src/lock.rs` 实现 `state-dir/host.lock` 咨询锁：同一 `state-dir` 只允许一个 Host；第二实例失败并说明。Windows 独占 flock 挡住读 `host.lock` 时走 `host.lock.meta` peek，争用（含 `try_lock_exclusive` 的 `Ok(false)`）不得删锁。`skill-manager/crates/host/tests/lock.rs` 断言双开失败、持锁时可 peek、独占 flock 时 sidecar 仍可读。逐字落实 data-model「同一 `state-dir` 只能有一个进程持有 `host.lock`」。
- [X] T007 在 `skill-manager/crates/host/src/serve.rs`、`skill-manager/crates/cli/src/commands/host.rs` 实现 `host serve` / `host status`：前台运行、占锁、能报告是否接入 Vault。已有实例占锁则失败。
- [X] T008 在 `skill-manager/crates/cli/src/main.rs` 落实 Host 在场时写入类命令转发到本机 Host、不在场时直连 core 且占同一把锁（research D5、FR-006）。
- [X] T009 在 `skill-manager/src-tauri/src/lib.rs` 让桌面启动即内嵌 Host runtime（占锁、调度仍按 002）；关窗收托盘时 Host 继续跑，仅托盘退出才释放锁。
- [X] T010 在 `skill-manager/crates/core/src/state/` 只加后续表可用的 migration 钩子，本任务不创建 `agent_sessions` / `agent_messages` / `host_identity`（对话表归 T031，设备表归 T020）；保持「共享资产零入库」。

**Checkpoint**: 桌面与 `host serve` 互斥于同一 state-dir；CLI 写入有协调；尚无 clone、管理站、对话。

---

## Phase 3: User Story 1 — 在无窗口的机器上接入 Host（P1）🎯 MVP

**Goal**: 用 git clone 接入已有 Vault、Host 当开机服务、启动命令可拉起；安装 Skill 指导编码 Agent 完成同样步骤（不含管理 CLI 登录）。

**Independent Test**: 不打开图形窗口：`vault clone` + `host serve`，`host status` 显示该路径；同一 state-dir 再 serve 失败。不依赖管理站网页与对话。

### Implementation

- [X] T011 [US1] 在 `skill-manager/crates/core/src/sync/git.rs`、`skill-manager/crates/cli/src/commands/vault.rs` 实现 `vault clone <remote-url> --path <dir>`：调用本机 git；目标已存在且非空则失败不覆盖；成功后把该路径记为本机 Vault。不提供图形向导，不 init 新中央库。
- [X] T012 [US1] 第一版**不做** `vault clone --pair`。对接只用 US2 的 `host pair`。若日后加 `--pair`：语义等于 clone 成功后立刻 `host pair`；对接失败须分段报告，不得把「未对接」说成「库没 clone」（`contracts/cli.md`）。
- [X] T013 [US1] 在 `skill-manager/crates/host/src/service.rs`、`skill-manager/crates/cli/src/commands/host.rs` 实现 `host install-service` / `uninstall-service`：当前用户级开机启动（macOS launchd、Linux systemd --user、Windows 登录项或等价）；不唤醒休眠、不加看门狗。`host serve` 即启动命令。
- [X] T014 [P] [US1] 在 `skill-manager/assets/skills/skill-manager-install/` 编写安装 Skill 源（与 `skill-manager-maintenance` 分开）：clone、配置并启动 Host、对接管理站；登录任意 ACP CLI 的步骤由 T038 补。
- [X] T015 [US1] 按 `specs/003-host-station-agent/quickstart.md` §1 在无窗口环境下验收 clone、serve、锁、再启动，并执行 `host install-service` 确认已写入当前用户开机项（不必真重启机器）；写入本目录或实施记录，不把浏览器预览当 Host 真结果。

**Checkpoint**: 无窗口机器可接入 Vault 并跑 Host。对接管理站可等到 US2。

---

## Phase 4: User Story 2 — 登录管理站并切换设备（P1）

**Goal**: 一次登录、出站对接、切换当前设备看到该机完整 002 管理界面；管理站不持有 Vault。

**Independent Test**: 管理站 + 两份 `--state-dir` 各跑 Host。浏览器登录一次，切换后分别改注释，互不影响；离线设备不可操作。不依赖内置管理 Agent。

### 设计前置

- [X] T016 [US2] 在 `specs/001-skill-management-prototype/ui-prototype/index.html`、`prototype.css` 与 `design/DESIGN.md` 增加登录页、设备列表/切换条、离线不可用态；设备条只出现在管理站/远程壳，桌面原型不得出现「切换其它设备」；同一轮按对照表改 React 骨架（尚未接 API）。

### Implementation

- [X] T017 [US2] 在 `skill-manager/crates/station/src/db.rs` 建立 `station.db`：`admin`（username、password_hash、totp_secret、recovery_code_hashes、session_ttl_seconds 一行）、`sessions`、`devices`（device_id、display_name、device_key_hash、last_seen_at、revoked）、`pairing_codes`（code_hash、expires_at、consumed_at）。无技能表。
- [X] T018 [US2] 在 `skill-manager/crates/station/src/auth.rs`、`skill-manager/crates/cli/src/commands/station.rs` 实现首次 `station serve` 创建唯一管理员（密码 + TOTP 二维码/密钥/一次性恢复码只展示一次）、可配置 session_ttl、`station reset-admin` 仅本机；无公网注册。
- [X] T019 [US2] 在 `skill-manager/crates/station/src/http.rs` 实现 `contracts/station-http.md` 的 `/auth/*` 与静态前端托管；未登录不加载技能数据。
- [X] T020 [US2] 在 `skill-manager/crates/core/src/state/` 与 `skill-manager/crates/host/` 增加 `host_identity` 表：device_id「对接时生成，解除后可重新生成」；display_name 默认主机名可改；station_url 空=未对接；device_key 只存本机与管理站、不进 Vault；同一 state-dir 一行；解除清除 station_url 与 device_key。
- [X] T021 [US2] 在 `skill-manager/crates/host/src/pair.rs`、`skill-manager/crates/cli/src/commands/host.rs`、`skill-manager/crates/station/src/devices.rs` 实现出站对接：`host pair --station <url> --code` 与 `host unpair`（解除后清本机 `station_url` / `device_key`）；码过期/已用/填错失败且不影响已对接设备；`devices` 区分 online（长连接心跳）；`PATCH` 改名、`revoke` 后原密钥失效。
- [X] T022 [US2] 在 `skill-manager/crates/host/src/http.rs` 实现 `contracts/host-http.md`：`POST /invoke/<command>`（体与现有 Tauri invoke 参数一致）、`GET /events` SSE（至少 `vault-changed`、`schedule-finished`）、`POST /session` → `{ sessionId }`、`GET /host/status`（与 CLI `host status` 同一事实）。删除技能、覆盖类 command 未带有效 `planId` 时不得执行，只返回待确认计划（闸门与 T024 同一套）。默认只接受本机回环与已认证管理站长连接，不对公网提供登录页。
- [X] T023 [US2] 在 `skill-manager/crates/station/src/proxy.rs` 实现 `PUT /current`、`POST /current/invoke/<command>`、`GET /current/events` 转到当前 Host；设备离线或未选不得用缓存技能正文顶上，失败须说清。
- [X] T024 [US2] 在 `skill-manager/crates/host/src/plans.rs` 与 `skill-manager/crates/host/src/http.rs` 实现 `operation_plans` 及 `POST /plans`、`POST /plans/:id/confirm`：action 仅 `skill.delete` 或 `deploy.overwrite`；绑定签发时 session_id；过期或 session 不匹配拒绝；确认后才执行 core。普通部署、注释、push 不建计划。
- [X] T025 [US2] 在 `skill-manager/src/services/client.ts`、`skill-manager/src/services/transport.ts` 把业务函数改为走 Transport；`isDesktop()` 为真用 Tauri，为假用 HttpTransport。`git_auto_push` 仅图形界面会话可写（远程网页算 GUI）。
- [X] T026 [US2] 在 `skill-manager/src/pages/` 接入登录、设备条、当前设备界面。`isDesktop()` 为真：不渲染设备条、不调用 `PUT /current`，只管理本机 Host。为假：显示设备条；远程隐藏托盘/关窗收起/应用更新；路径类配置为文本输入，Host 校验存在性，不用浏览器文件选择器，第一版无 Host 目录树。
- [X] T027 [US2] 在 `skill-manager/src/pages/` 与 `skill-manager/crates/host/src/plans.rs` 把删除技能、覆盖部署接到确认计划（绑定当前 sessionId）；两浏览器非破坏性修改均允许；用另一会话 planId 确认必须拒绝。
- [X] T028 [US2] 按 `quickstart.md` §2 验收双 state-dir 切换改注释、离线、双浏览器与确认绑会话。对一台**已经在线且 Vault 已接入**的设备，从打开登录页起计时，含登录与切换、不含首次 clone，须在 3 分钟内完成一次非破坏性改注释（SC-001）。

**Checkpoint**: 管理站可登录并只管理当前在线 Host 的完整 002 能力。

---

## Phase 5: User Story 3 — 内置管理 Agent 对话（P1，可晚于 US1/US2 交付）

**Goal**: 在当前 Host 上对话完成与 GUI 同类管理；上下文钉死；不主动维护；确认与上传按 FR-019/020。

**Independent Test**: 本机桌面（不经管理站）打开对话，技能详情说「完善中文注释」写入该技能；切到 B 发新指令不串改进行中的 A。不打开详情时「有哪些译文待更新」能对上库内项；取消进行中任务不得显示成功。

### 设计前置

- [X] T029 [US3] 在 `specs/001-skill-management-prototype/ui-prototype/index.html`、`prototype.css` 与 `design/DESIGN.md` 增加对话面板（可见当前界面对象、流式结果、确认卡、取消进行中任务；不必把全局清单画成第二套界面）；同一轮改 `skill-manager/src/` 对应组件。

### Implementation

- [X] T030 [US3] 按 `research.md` D9 探测清单测 OpenCode → Codex CLI → Gemini CLI（会话、流式、取消、工具调用、stdio、**内置说明注入路径或「无目录则初始化文本」**）；第一家通过者仅作引导宿主，**不得**把三家写成唯一选项。用户已登录的任意 ACP 兼容 CLI 都可拉起；清单外用 `host agent-cli` / `PUT /agent/runtime` / `SKILL_MANAGER_AGENT_CLI`。全部失败且用户未登记则对话入口如实不可用，不得假回复。实施记录：2026-11-11 探测 OpenCode 1.19.5 通过（注入 `$OPENCODE_CONFIG_DIR/skills/`）；原生 Codex 非 ACP；Gemini initialize 过、未登录则 `session/new` 失败。
- [X] T031 [US3] 在 `skill-manager/crates/core/src/state/` 增加 `agent_sessions`（界面会话、钉死快照 JSON：`global` 为摘要+计数+引用——技能/分类/标签、待更新、译文待更新、部署摘要、失败与暂停、预设/Hermes/工具/本机 Agent 目录摘要；`view` 为 Host 按引用补齐的当前页详情）、`agent_messages`（角色、正文、task_id）、`agent_runtime`（cli_id、binary_path、detected_at / installed_by_app）。不进 Vault、不存整库正文。模型密钥不得出现在这些表。任务状态含已取消。
- [X] T032 [US3] 在 `skill-manager/crates/host/src/agent/` 用 ACP stdio 拉起**当前** CLI（用户指定 / PATH 示例 / 引导宿主 OpenCode），模型由该 CLI 决定。没有兼容 CLI 才软件内安装引导宿主到本机目录，不打进 Skill Manager 安装包。启动时把 `skill-manager/assets/skills/skill-manager-builtin-agent/` 注入 OpenCode 的 `$OPENCODE_CONFIG_DIR/skills/`（无目录则 ACP 初始化文本 + prompt 前缀）。该资产不入库、不进用户 `skill list`。不注入配套维护 Skill。未登录模型时只说明无法执行，不写库。第一版不实现 MCP 服务。
- [X] T033 [US3] 在 `skill-manager/crates/host/src/agent/tools.rs` 经 ACP 声明工具，映射现有 `skill-manager` JSON CLI **经 Host** 执行，供快照不够时主动查询（读正文、按现有 JSON 字段列待更新、查部署等）与写入。不得直打 core。不得为对话新增 002 筛选开关。删除/覆盖必须走 T024 计划；Agent 文本「已同意」无效。工具失败须交回错误，不得编造正文。在 `skill-manager/assets/skills/skill-manager-builtin-agent/` 编写短说明（两层上下文已注入、细节用工具、003 确认与上传），不得抄 002「每次询问上传」。
- [X] T034 [US3] 在 `skill-manager/crates/host/src/http.rs` 实现 `PUT /agent/context`（只收页面与选中引用，不含正文/注释/报错）、`POST /agent/messages`、`POST /agent/tasks/:id/cancel`、会话与消息读取（`contracts/host-http.md`）。收消息时 Host 从 core 组装 `global`，并按引用补齐 `view` 详情，一并钉死到该条任务；之后的 `view` 只影响新消息。引用失效则如实标空。不得要求用户粘贴报错、清单或技能名。不把整库正文写入快照。
- [X] T035 [US3] 在 `skill-manager/crates/host/src/agent/tools.rs` 落实 FR-020：有 `git_auto_push` 则内置管理 Agent 可 push 且不逐步确认；无授权则失败并说明去设置开启，Agent 不得写该设置。外部编码 Agent 路径不改 002。
- [X] T036 [US3] 在 `skill-manager/crates/host/src/agent/` 只实现对话响应；不实现巡检/自动采用上游/自动入库/无人值守 push。002 调度器保持与对话无关。
- [X] T037 [US3] 在 `skill-manager/src/` 接通对话 UI：只上报当前界面引用（页面、选中 `ref` 等），不得把详情 JSON 当快照源；全局与详情由 Host 组装。提供取消进行中任务。用户能说「待更新的那些」「这个技能的上游」而无须点名或粘贴。进行中任务切页不改对象；局部失败分项说明；历史从 Host 本机状态读取。第一版不要求多会话工作区。
- [X] T038 [P] [US3] 在 `skill-manager/assets/skills/skill-manager-install/` 补登录任意 ACP 兼容 CLI、以及清单外 CLI 登记步骤，不与配套维护 Skill 合并。
- [X] T039 [US3] 按 `quickstart.md` §3 验收：注释写入、切页不串、删除确认、未授权上传、空闲不改库、未登录不冒充成功；未打开详情时「译文待更新的那些」能对上全局清单；详情页问上游不必粘贴；冲突/暂停不必粘贴报错；快照无全文时 Agent 能用工具读出片段且用户未粘贴；取消进行中任务不得显示成功；内置说明不出现在用户技能列表。

**Checkpoint**: 对话在当前 Host 上可用；管理站切到哪台即用哪台的 Agent。

---

## Phase 6: Polish & Cross-Cutting

**Purpose**: 文档、002 指向、回归。

- [X] T040 在 `docs/requirements.md`、`specs/002-skill-management/spec.md` US8 场景 6 与 US5 场景 5（拉取冲突）各增加指向 003 的一句：有内置管理 Agent 时现场由软件交给对话，不重写 002 全文、不改 FR-039 的「无三选一界面」。
- [X] T041 [P] 核对 `GLOSSARY.md` 与实现名、ADR 0002；确认安装 Skill 路径已写入仓库。无新名则勾掉并注明无差异，不改术语。
- [X] T042 按 `quickstart.md` §4–5 跑管理站不可用时桌面仍能改库、Host 在场/不在场 CLI 与 002 核心命令回归。
- [X] T043 [P] 在 `specs/003-host-station-agent/quickstart.md` 记录实际验收证据与未取证项。

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: 无依赖
- **Foundational (Phase 2)**: 依赖 Setup；**阻塞**全部故事
- **US1 (Phase 3)**: 依赖 Foundational — MVP
- **US2 (Phase 4)**: 依赖 Foundational（需要 host HTTP/锁）；不依赖 US1 的 clone，但需要至少一台能 `host serve` 或桌面内嵌 Host
- **US3 (Phase 5)**: 依赖 Foundational 与确认计划（T024，在 US2）。若先做桌面对话，须把 T024 提前或在 US3 内完成后再接远程
- **Polish**: 依赖已选定要交付的故事

### User Story Dependencies

- **US1**: Foundational 之后即可；不依赖管理站与对话
- **US2**: Foundational 之后即可用桌面 Host 对接；clone 服务来自 US1 更完整
- **US3**: 需要 Host 确认计划；可先做桌面、再接到管理站网页

### Parallel Opportunities

- T004 与 T001–T003 在文件不冲突时可并行
- T014 安装 Skill 文案与 T011–T013 代码可并行
- T016 原型与 T017–T021 后端可并行（接 UI 前 T016 要完成）
- T029 原型与 T030 探测可并行

### Parallel Example: User Story 1

```text
T014 写安装 Skill（assets/）
T011 实现 vault clone（core/cli）
# T012 依赖 T011；T013 与 T011 不同文件可并行
```

---

## Implementation Strategy

### MVP First (US1)

1. Phase 1 Setup
2. Phase 2 Foundational
3. Phase 3 US1
4. 按 quickstart §1 验收后可停：无窗口 Host 已能 clone 并常驻

### Incremental Delivery

1. US1 → 云电脑可跑 Host
2. US2 → 外出用管理站管已对接设备
3. US3 → 对话管理（D9 探测通过后）
4. 每段不破坏前一段

---

## Notes

- `[P]` = 不同文件且无未完成前置
- 新界面必须走设计层同一轮落地，不要只改 React
- `crates/station` 任何时候都不得依赖 core
- 不要把 002 的「外部 Agent 每次问上传」套到内置管理 Agent
- 规格「一键命令」= `vault clone` 再 `host serve`（契约允许日后加 `--pair`），不另发明第三条入口
- 内置管理说明不入库；客户端只报引用；不为对话新增 002 CLI 筛选开关
- 提交按任务或逻辑组进行；本文件生成不代替实现

---

## Phase 7: Convergence

对照 `spec.md` / `plan.md` / 契约与当前代码（勾选不算证据）于 2026-11-11 追加。完成后再跑 `/speckit-implement`。

- [X] T044 在 `skill-manager/crates/host/src/agent/bridge.rs` 用 ACP stdio 真正拉起当前 CLI（流式、工具、取消）；「完善中文注释」须写库；未登录模型不得把任务标成 `running` 或成功 per FR-014, FR-015, US3/AC2, plan D6, agent-bridge (missing)
- [X] T045 本机没有用户指定或已装的 ACP CLI 时，安装 D9 引导宿主 OpenCode 到本机目录并写入 `agent_runtime`，不打进 Skill Manager 安装包；不得把三家写成唯一安装回退 per FR-017, agent-bridge
- [X] T046 启动该 CLI 时把 `skill-manager/assets/skills/skill-manager-builtin-agent/` 注入其 skills 搜索目录；无该目录则写入 ACP 初始化文本，且不入库、不进 `skill list` per FR-015, T032, agent-bridge (missing)
- [X] T047 补全 `assemble_snapshot` 的全局层：上游待更新、可入库新增、失败与暂停、预设/Hermes/工具/本机 Agent 目录的摘要+计数+引用，仍不含正文 per FR-015, agent-bridge, T031 (partial)
- [X] T048 Host 在场时 CLI 把 preset / hermes / agent / instruction / tool 等写入转发到 Host invoke，不得只提示「请通过管理界面执行」 per FR-006, plan D5 (partial)
- [X] T049 管理站界面提供生成一次性对接码、改设备名、解除对接，列表立即反映；不要只留 HTTP API。远程首次接入（无设备、无 Vault）也须在正常界面显示 DeviceBar／生成对接码，先对接再接入库 per US2/AC4, US2/AC5, US2/AC11, FR-009
- [X] T050 用恢复码登录后必须能重新绑定 TOTP（二维码/密钥/新恢复码），不得只发会话并消耗恢复码 per US2/AC3, FR-007 (partial)
- [X] T051 管理站把 Host invoke 的 409 / `needsConfirmation` 原样交给远程前端；不得把待确认计划当成 200 成功体 per FR-019, US2/AC9, host-http (contradicts)
- [X] T052 桌面删除技能与覆盖部署走 Host 确认计划并绑定桌面会话，不得经 `src-tauri` 直打 core 绕过 `planId` per FR-013, FR-019 (partial)
- [X] T053 提供配置 `session_ttl` 的界面或 CLI，使登录保持时长可被操作者改，而不仅有 `PUT /auth/session-ttl` per FR-007 (partial)
- [X] T054 远程破坏性确认用现有 `ConfirmDialog` 展示对象预览，不得只用无预览的 `window.confirm` per FR-019 (partial)
- [X] T055 按 plan 把 `src-tauri` command 收成薄壳、转发 Host invoke，去掉与 `crates/host/src/commands` 平行的第二套入口 per plan: 薄壳 (partial)

---

## Phase 8: Convergence

对照代码复查 T044–T055 勾选后的赶工缺口（2026-11-11）。真机验收不在本阶段。

- [X] T056 禁止 Agent 调用 `plans_confirm` 或把口头同意当成确认；破坏性工具须把计划绑到图形界面会话并弹出 ConfirmDialog per FR-019, US3/AC6 (contradicts)
- [X] T057 确认计划执行后消耗，同一 `planId` 不得重复执行 per FR-013, FR-019 (partial)
- [X] T058 恢复码登录会话在完成 TOTP 重绑前不得操作设备/转发；重绑成功后才是完整会话 per US2/AC3, FR-007 (partial)
- [X] T059 ACP `initialize` 声明工具清单，并路由 `tools/call` per FR-015, T044 (missing)
- [X] T060 Host 在场时 CLI 转发 skill tag/move/archive/import/create 等写入；409 不得当成功 per FR-006, T048 (partial)
- [X] T061 对话任务完成后清除进行中状态，取消不得显示成功 per FR-022, FR-023 (partial)
- [X] T062 确认计划识别须带 `needsConfirmation`；无 ConfirmDialog 时拒绝而不是 `window.confirm` per FR-019, T054 (partial)
- [X] T063 对照 core JSON CLI / GUI 页面补 `assemble_snapshot`：可入库新增、冲突路径与 git 原文、`paused_local_change`、最近命令失败原文；无 skill ref 时 view 按当前页补齐对象。对照表见 `snapshot-inventory.md`。不声称齐全。D9 任意 ACP CLI 见 T030。 per FR-015, T047 (partial)
- [X] T064 Linux 云上 `cargo check -p skill-manager-desktop`：`prepare-cli` 为本机 triple 准备 sidecar（本机不传 `--target`）；desktop `build.rs` 缺失时从已构建 CLI 复制，找不到则失败并提示 `npm run prepare:cli` / `npm run check:desktop`，不嵌套 cargo、不提交 `binaries/`、不假装成功。D9 不在本任务。
- [X] T065 按 ACP 声明 Host 工具：`session/new.mcpServers` stdio（`host mcp`）带 `inputSchema`；initialize 不再塞 `tool_descriptors`；`fs/write*` 不得空成功。D9 任意 CLI 登记不在本任务。 per FR-015, C1
- [X] T066 同一 Host 对话复用 ACP session；新 session 把 `agent_messages` 回灌 prompt；不要每条消息 initialize+session/new 后 kill。 per US3, C2
- [X] T067 `session/request_permission` 不得一律放行；CLI 自带 bash/edit 拒绝，Host 工具（skill_set 等）才允许。确认等待不堵死 ACP 读循环（工具经 Host HTTP / 后台线程）。 per FR-001, FR-019, C3, H3
- [X] T068 恢复码登录把新 TOTP/恢复码哈希挂在该会话（`pending_rebind` / `pending_totp_secret` / `pending_recovery_hashes`），不得立刻改写 `admin.totp_secret`；`POST /auth/totp/rebind` 用本会话待定密钥校验后才写入 admin，该会话才成为完整管理员会话。已用恢复码作废，其余恢复码与旧验证器在确认前仍有效。`skill_delete` 工具 schema 声明 `references[]`。D9/#7 不在本任务。 per US2/AC3, FR-007, H4
- [X] T069 远程网页登录后、尚无设备且 Vault 未接入时，首次接入页显示 DeviceBar 与「生成对接码」；未选当前设备时禁用「接入该路径」，不调用 Host invoke，从而消除「尚未选择当前设备」死锁 per US2/AC11, T049
- [X] T070 Host 与管理站长连接每 15s WebSocket Ping；在线以仍打开的 LiveLink 为准，不得因无业务流量超过 45s 判离线。套接字关闭或心跳发送失败才移除。 per US2/AC6, FR-010, T021
- [X] T071 默认管理 CLI 钉死 OpenCode：不扫描厂商 PATH 清单；`host agent-cli` / 对话在未安装时返回 `needsInstall`（`cliId` 仍为 `opencode`，不是 `null`）；软件内 `--install` / `--login` 与对话面板「安装 OpenCode」「打开登录」。装好写入 `agent_runtime`，之后不再重探。不把「没有已登录 CLI」当成测试跳过。 per US3/AC9, FR-017
- [X] T072 Windows 软件内安装 OpenCode：经 `cmd.exe /C` 启动 `npm.cmd`（CreateProcess 不能直接跑 `.cmd`）；失败不留空的 `agent-runtime/opencode`；对话等 `agent_runtime_get` 返回后再显示「安装 OpenCode」，不得先闪「打开登录」 per US3/AC9, FR-017, T071
- [X] T073 Windows `cmd /C` 安装参数分开传入，不得预包 `\"路径\"`（否则 argv 转义成 `\"C:\Program Files\nodejs\npm.cmd\"`，cmd 当命令名失败） per T072
- [X] T074 Windows「打开登录」/`host agent-cli --login` 用 `cmd /c start` 弹出可见交互控制台（不要 `CREATE_NEW_CONSOLE` + 继承 Host 重定向 stdio，也不要 `start /B`），以便完成 Select provider 与浏览器授权 per US3/AC9, T071
- [X] T075 Windows ACP 握手拉起 `opencode.cmd`（经 `cmd /C`），不得 CreateProcess 无扩展 Unix `#!/bin/sh` 垫片（os error 193）；`ready: true` 表示该路径可在 Windows 拉起；登录仍走 `start` 可见控制台 per US3, T071, T074
- [X] T076 Windows ACP 优先直接拉起 `opencode.exe`（stdio 接到 CLI，不经 cmd 包装）；超时 `taskkill /T /F` 杀掉整棵进程树，对话必须收到成功或失败，不得一直「发送中」；登录仍用 `start` per T074, T075
- [X] T077 Windows ACP 拉起原生 `opencode.exe` 不得加 `CREATE_NO_WINDOW`（否则 Bun 把管道 stdin 当已关闭，`acp`/`--acp`/无参都立刻退出）；握手失败须带 exit code 与 stderr，不得把进程退出说成未登录 per T076
- [X] T078 Windows ACP 探测/握手用 `STARTF_USESHOWWINDOW|SW_HIDE` 隐藏控制台（仍不用 `CREATE_NO_WINDOW`）；`taskkill` 后等待真实 exit code 与 stderr，超时/立即退出/仍在运行分开报告，不得写 `exit unknown`；登录仍用 `cmd /c start` 可见控制台 per T074, T077
- [X] T079 Windows ACP 用隐藏 ConPTY 把 stdin/stdout 接到 `opencode.exe`（`CREATE_NEW_CONSOLE` 会让 Bun 读空控制台，握手一直超时）；`--print-logs acp` 优先，超时不再试 `--acp`/无参；失败须带 stderr/非 JSON 输出；initialize 列出 `authMethods` 不得当成未登录 per T078
- [X] T080 Windows ACP 超时必须先 `ClosePseudoConsole` 再 `taskkill /T /F`，stdout 用超时 recv 不得堵死「发送中」；杀掉 ConPTY 留下的 headless conhost；已钉 `["acp"]` 按 `--print-logs acp` 拉起并反映到 runtime.acpArgs per T076, T079
- [X] T081 Windows ACP ConPTY：`UpdateProcThreadAttribute` 传 HPCON 本身（不是 `&HPCON`），不设 `STARTF_USESTDHANDLES`（空 stdin/stdout 会 0xC0000142 STATUS_DLL_INIT_FAILED）；`bInheritHandles` FALSE；仍 `SW_HIDE`、不用 `CREATE_NO_WINDOW`/`CREATE_NEW_CONSOLE`；握手失败文案只列实际试过的参数（超时不得写 `--acp`/无参）per T079, T080
- [X] T082 Windows ACP：隐藏 ConPTY + `SW_HIDE` 只给 Bun 控制台；ACP NDJSON 走独立管道（`STARTF_USESTDHANDLES` 三个句柄都有效）；不试 `--acp`/无参（无参会进 TUI/`npm list -g`）；用户可见错误与 Host 日志去掉 CSI/OSC per T079, T081
- [X] T083 Windows ACP：Bun 在有 ConPTY 时把 stdin/stdout 绑到伪控制台，另接的 `STARTF_USESTDHANDLES` 管道收不到 initialize。ACP JSON 走 CreatePseudoConsole 那对管道（`pty_in_w`/`pty_out_r`），`USESTDHANDLES` 句柄为空 + `bInheritHandles` FALSE；等到 stderr/PTY 出现 `setup connection` 再发 initialize（cooked TTY 用 `\r\n`）；只试一次 `--print-logs acp`；UI 只报超时/exit code，INFO 进 Host 日志。仍 `SW_HIDE`，不用 `CREATE_NO_WINDOW`/`CREATE_NEW_CONSOLE`。per T082
- [X] T084 Windows ACP：从 ConPTY 上与 `--print-logs` INFO 交错、cooked 折行的字节流里抽出 JSON-RPC；看到 initialize `protocolVersion`/`agentInfo` 就算成功，继续 `session/new` 与用户消息，不得等 12s 看门狗。UI 失败只出现一次、无 U+FFFD。`authMethods`/`opencode-login` 仍不当成未登录。per T083
- [X] T085 管理站网页：Host WS 不得在 select 循环上阻塞后续 invoke；`/current/session` 本站签发，不转发 Host `__session__`。`agent_runtime_get` 失败须结束「正在检查管理 CLI…」并给出真实错误，ready 时启用发送。`skill_list` 失败或非 `{skills,total}` 不得把技能库计数写成 0。per US3
- [X] T086 管理站网页 invoke 不得 15s 全超时：Host 出站长连接与 `host serve` 共用运行时；invoke 在独立 OS 线程执行并经 mpsc 回写，禁止在额外 Runtime 上 `tokio::spawn` 后再 `.await spawn_blocking`（任务可能永不轮询，station 收不到 `invoke_result`）。设备列表与技能计数仍走既有路由。per T085
- [X] T087 Windows ACP：ConPTY 回显的 initialize/`session/new` 请求不得当 Agent 方法回 `未知方法`（会变成 unknown request 1，并让 UI 把回显错误当成 session/new 失败）。initialize 成功后发 `notifications/initialized`，再用 ACP 必选的 `session/new`（cwd 绝对路径 + mcpServers）；`sessionCapabilities` 的 list/resume/fork/close 是附加能力不是建会话。忽略回显请求。`authMethods`/`opencode-login` 仍不当成未登录。per T084, T086
- [X] T088 Windows ACP：OpenCode 1.19.5 没有 `notifications/initialized`（MCP 方法，-32601）。initialize 成功后直接 `session/new`，从混合 INFO 流解析 `sessionId` 并发送 `session/prompt`。已登录（`ready && !needsLogin`）不得再挡「打开登录」/「我已完成登录」。`agent_post_message` 等本轮结束（前端 10min），列表/设备仍 15s。per T087
- [X] T089 Windows ACP：OpenCode 1.19.5 `--print-logs` 建会话 INFO 是 `message=created id=ses_…`，不是 `created session ses_`。Host 从该行取出 sessionId（仍接受 JSON-RPC session/new result），立刻 `session/prompt`，不得空等 12s 看门狗。per T088
