---
description: "003 Host、管理站与内置管理 Agent 开发任务，按用户故事组织"
---

# Tasks: 本机 Host、统一管理站与内置管理 Agent

**Input**: 本目录 `spec.md`、`plan.md`、`research.md`、`data-model.md`、`contracts/`、`quickstart.md`；约束 [宪章](../../.specify/memory/constitution.md) 2.1.0 与 [ADR 0002](../../docs/adr/0002-host-management-station-and-agent.md)。

**范围**: US1–US3（均可 P1，按 Clarifications 分里程碑 US1 → US2 → US3）。不实现冲突合并界面、主动维护、跨设备总览、唤醒休眠、管理站自行申请证书。对话必须自动带上**全局 + 当前界面**两层上下文（足够办事，举例非穷尽），不要求用户为 Agent 拼现场。技能业务仍走 `skill-manager-core`，不在 Host/station 重写。

**测试**: 规格未要求 TDD。不对每个入口先写失败测试。锁、确认计划、station 不依赖 core 等后果不可逆的边界，在对应实现任务里带 cargo 断言。GUI 与双机场景按 `quickstart.md` 人工验收。

**界面**: 登录、设备条、对话等新界面先改 `design/` 与 `specs/001-skill-management-prototype/ui-prototype/`，同一轮按 `.agents/skills/frontend-design/SKILL.md` 对照表改 React。

## Format: `[ID] [P?] [Story] Description`

- `- [ ] Tnnn`；故事阶段附 `[USn]`；`[P]` 表示不同文件、无未完成前置时可并行。
- 同一文件串行。路径相对仓库根，代码在 `skill-manager/`。

## Phase 1: Setup（工程准备）

**Purpose**: 加上 host/station crate 与 CLI 子命令骨架，不实现业务。

- [ ] T001 在 `skill-manager/Cargo.toml`、`skill-manager/crates/host/Cargo.toml`、`skill-manager/crates/station/Cargo.toml` 把 workspace members 扩为 core/cli/host/station/src-tauri；`station` 的 Cargo.toml **不得**依赖 `skill-manager-core`；host 依赖 core。
- [ ] T002 在 `skill-manager/crates/host/src/lib.rs`、`skill-manager/crates/station/src/lib.rs` 建立最小库入口与模块划分（host: lock/runtime/http；station: auth/devices/proxy），保证 `cargo check -p skill-manager-host -p skill-manager-station` 通过。
- [ ] T003 在 `skill-manager/crates/cli/src/main.rs` 增加 `host` / `station` / `vault clone` 子命令骨架（与 `specs/003-host-station-agent/contracts/cli.md` 名称一致），未实现时非零退出并说明，不得伪成功。
- [ ] T004 [P] 在 `skill-manager/src/services/transport.ts` 定义 `TauriTransport | HttpTransport` 接口（invoke 同名、subscribe 事件），暂不改现有 `client.ts` 调用点。

**Checkpoint**: 两新 crate 可检查；CLI 新子命令存在但未做事。

---

## Phase 2: Foundational（故事共用底座）

**Purpose**: 抽出 Host 运行时、state-dir 单实例锁、桌面内嵌 Host。所有故事依赖本阶段。确认计划、管理站、对话不在本阶段。

**⚠️ CRITICAL**: 未完成本阶段不得开始 US1–US3。

- [ ] T005 将 `skill-manager/src-tauri/src/commands/mod.rs` 的 `AppState`（上下文、`background_jobs`、`VaultWatcher`、`fetch_jobs`、`schedule_report`）迁到 `skill-manager/crates/host/src/runtime.rs`，Tauri 改为持有 Host runtime；core 仍无 HTTP、无 Agent 运行时。
- [ ] T006 在 `skill-manager/crates/host/src/lock.rs` 实现 `state-dir/host.lock` 咨询锁：同一 `state-dir` 只允许一个 Host；第二实例失败并说明。在 `skill-manager/crates/host/tests/lock.rs` 断言双开失败。逐字落实 data-model「同一 `state-dir` 只能有一个进程持有 `host.lock`」。
- [ ] T007 在 `skill-manager/crates/host/src/serve.rs`、`skill-manager/crates/cli/src/commands/host.rs` 实现 `host serve` / `host status`：前台运行、占锁、能报告是否接入 Vault。已有实例占锁则失败。
- [ ] T008 在 `skill-manager/crates/cli/src/main.rs` 落实 Host 在场时写入类命令转发到本机 Host、不在场时直连 core 且占同一把锁（research D5、FR-006）。
- [ ] T009 在 `skill-manager/src-tauri/src/lib.rs` 让桌面启动即内嵌 Host runtime（占锁、调度仍按 002）；关窗收托盘时 Host 继续跑，仅托盘退出才释放锁。
- [ ] T010 在 `skill-manager/crates/core/src/state/` 只加后续表可用的 migration 钩子，本任务不创建 `agent_sessions` / `agent_messages` / `host_identity`（对话表归 T031，设备表归 T020）；保持「共享资产零入库」。

**Checkpoint**: 桌面与 `host serve` 互斥于同一 state-dir；CLI 写入有协调；尚无 clone、管理站、对话。

---

## Phase 3: User Story 1 — 在无窗口的机器上接入 Host（P1）🎯 MVP

**Goal**: 用 git clone 接入已有 Vault、Host 当开机服务、启动命令可拉起；安装 Skill 指导编码 Agent 完成同样步骤（不含管理 CLI 登录）。

**Independent Test**: 不打开图形窗口：`vault clone` + `host serve`，`host status` 显示该路径；同一 state-dir 再 serve 失败。不依赖管理站网页与对话。

### Implementation

- [ ] T011 [US1] 在 `skill-manager/crates/core/src/sync/git.rs`、`skill-manager/crates/cli/src/commands/vault.rs` 实现 `vault clone <remote-url> --path <dir>`：调用本机 git；目标已存在且非空则失败不覆盖；成功后把该路径记为本机 Vault。不提供图形向导，不 init 新中央库。
- [ ] T012 [US1] 第一版**不做** `vault clone --pair`。对接只用 US2 的 `host pair`。若日后加 `--pair`：语义等于 clone 成功后立刻 `host pair`；对接失败须分段报告，不得把「未对接」说成「库没 clone」（`contracts/cli.md`）。
- [ ] T013 [US1] 在 `skill-manager/crates/host/src/service.rs`、`skill-manager/crates/cli/src/commands/host.rs` 实现 `host install-service` / `uninstall-service`：当前用户级开机启动（macOS launchd、Linux systemd --user、Windows 登录项或等价）；不唤醒休眠、不加看门狗。`host serve` 即启动命令。
- [ ] T014 [P] [US1] 在 `skill-manager/assets/skills/skill-manager-install/` 编写安装 Skill 源（与 `skill-manager-maintenance` 分开）：clone、配置并启动 Host、对接管理站；**不含**管理 CLI 登录（D9 锁定后 US3 再补）。
- [ ] T015 [US1] 按 `specs/003-host-station-agent/quickstart.md` §1 在无窗口环境下验收 clone、serve、锁、再启动，并执行 `host install-service` 确认已写入当前用户开机项（不必真重启机器）；写入本目录或实施记录，不把浏览器预览当 Host 真结果。

**Checkpoint**: 无窗口机器可接入 Vault 并跑 Host。对接管理站可等到 US2。

---

## Phase 4: User Story 2 — 登录管理站并切换设备（P1）

**Goal**: 一次登录、出站对接、切换当前设备看到该机完整 002 管理界面；管理站不持有 Vault。

**Independent Test**: 管理站 + 两份 `--state-dir` 各跑 Host。浏览器登录一次，切换后分别改注释，互不影响；离线设备不可操作。不依赖内置管理 Agent。

### 设计前置

- [ ] T016 [US2] 在 `specs/001-skill-management-prototype/ui-prototype/index.html`、`prototype.css` 与 `design/DESIGN.md` 增加登录页、设备列表/切换条、离线不可用态；设备条只出现在管理站/远程壳，桌面原型不得出现「切换其它设备」；同一轮按对照表改 React 骨架（尚未接 API）。

### Implementation

- [ ] T017 [US2] 在 `skill-manager/crates/station/src/db.rs` 建立 `station.db`：`admin`（username、password_hash、totp_secret、recovery_code_hashes、session_ttl_seconds 一行）、`sessions`、`devices`（device_id、display_name、device_key_hash、last_seen_at、revoked）、`pairing_codes`（code_hash、expires_at、consumed_at）。无技能表。
- [ ] T018 [US2] 在 `skill-manager/crates/station/src/auth.rs`、`skill-manager/crates/cli/src/commands/station.rs` 实现首次 `station serve` 创建唯一管理员（密码 + TOTP 二维码/密钥/一次性恢复码只展示一次）、可配置 session_ttl、`station reset-admin` 仅本机；无公网注册。
- [ ] T019 [US2] 在 `skill-manager/crates/station/src/http.rs` 实现 `contracts/station-http.md` 的 `/auth/*` 与静态前端托管；未登录不加载技能数据。
- [ ] T020 [US2] 在 `skill-manager/crates/core/src/state/` 与 `skill-manager/crates/host/` 增加 `host_identity` 表：device_id「对接时生成，解除后可重新生成」；display_name 默认主机名可改；station_url 空=未对接；device_key 只存本机与管理站、不进 Vault；同一 state-dir 一行；解除清除 station_url 与 device_key。
- [ ] T021 [US2] 在 `skill-manager/crates/host/src/pair.rs`、`skill-manager/crates/cli/src/commands/host.rs`、`skill-manager/crates/station/src/devices.rs` 实现出站对接：`host pair --station <url> --code` 与 `host unpair`（解除后清本机 `station_url` / `device_key`）；码过期/已用/填错失败且不影响已对接设备；`devices` 区分 online（长连接心跳）；`PATCH` 改名、`revoke` 后原密钥失效。
- [ ] T022 [US2] 在 `skill-manager/crates/host/src/http.rs` 实现 `contracts/host-http.md`：`POST /invoke/<command>`（体与现有 Tauri invoke 参数一致）、`GET /events` SSE（至少 `vault-changed`、`schedule-finished`）、`POST /session` → `{ sessionId }`、`GET /host/status`（与 CLI `host status` 同一事实）。删除技能、覆盖类 command 未带有效 `planId` 时不得执行，只返回待确认计划（闸门与 T024 同一套）。默认只接受本机回环与已认证管理站长连接，不对公网提供登录页。
- [ ] T023 [US2] 在 `skill-manager/crates/station/src/proxy.rs` 实现 `PUT /current`、`POST /current/invoke/<command>`、`GET /current/events` 转到当前 Host；设备离线或未选不得用缓存技能正文顶上，失败须说清。
- [ ] T024 [US2] 在 `skill-manager/crates/host/src/plans.rs` 与 `skill-manager/crates/host/src/http.rs` 实现 `operation_plans` 及 `POST /plans`、`POST /plans/:id/confirm`：action 仅 `skill.delete` 或 `deploy.overwrite`；绑定签发时 session_id；过期或 session 不匹配拒绝；确认后才执行 core。普通部署、注释、push 不建计划。
- [ ] T025 [US2] 在 `skill-manager/src/services/client.ts`、`skill-manager/src/services/transport.ts` 把业务函数改为走 Transport；`isDesktop()` 为真用 Tauri，为假用 HttpTransport。`git_auto_push` 仅图形界面会话可写（远程网页算 GUI）。
- [ ] T026 [US2] 在 `skill-manager/src/pages/` 接入登录、设备条、当前设备界面。`isDesktop()` 为真：不渲染设备条、不调用 `PUT /current`，只管理本机 Host。为假：显示设备条；远程隐藏托盘/关窗收起/应用更新；路径类配置为文本输入，Host 校验存在性，不用浏览器文件选择器，第一版无 Host 目录树。
- [ ] T027 [US2] 在 `skill-manager/src/pages/` 与 `skill-manager/crates/host/src/plans.rs` 把删除技能、覆盖部署接到确认计划（绑定当前 sessionId）；两浏览器非破坏性修改均允许；用另一会话 planId 确认必须拒绝。
- [ ] T028 [US2] 按 `quickstart.md` §2 验收双 state-dir 切换改注释、离线、双浏览器与确认绑会话。对一台**已经在线且 Vault 已接入**的设备，从打开登录页起计时，含登录与切换、不含首次 clone，须在 3 分钟内完成一次非破坏性改注释（SC-001）。

**Checkpoint**: 管理站可登录并只管理当前在线 Host 的完整 002 能力。

---

## Phase 5: User Story 3 — 内置管理 Agent 对话（P1，可晚于 US1/US2 交付）

**Goal**: 在当前 Host 上对话完成与 GUI 同类管理；上下文钉死；不主动维护；确认与上传按 FR-019/020。

**Independent Test**: 本机桌面（不经管理站）打开对话，技能详情说「完善中文注释」写入该技能；切到 B 发新指令不串改进行中的 A。

### 设计前置

- [ ] T029 [US3] 在 `specs/001-skill-management-prototype/ui-prototype/index.html`、`prototype.css` 与 `design/DESIGN.md` 增加对话面板（可见当前界面对象、流式结果、确认卡；不必把全局清单画成第二套界面）；同一轮改 `skill-manager/src/` 对应组件。

### Implementation

- [ ] T030 [US3] 按 `research.md` D9 探测清单测 OpenCode → Codex CLI → Gemini CLI（会话、流式、取消、工具调用、stdio）；将通过者写入 `agent_runtime.cli_id` 约定与实施记录。全部失败则对话入口如实不可用，不得假回复。
- [ ] T031 [US3] 在 `skill-manager/crates/core/src/state/` 增加 `agent_sessions`（界面会话、钉死快照 JSON：`global` 库级摘要——技能/分类/标签、待更新、译文待更新、部署摘要、失败与暂停；`view` 当前页与打开对象详情/上游/部署）、`agent_messages`（角色、正文、task_id）、`agent_runtime`（cli_id、binary_path、detected_at / installed_by_app）。不进 Vault、不存整库正文。模型密钥不得出现在这些表。
- [ ] T032 [US3] 在 `skill-manager/crates/host/src/agent/` 用 ACP stdio 拉起已锁定 CLI；检测已装，没有则软件内安装到本机目录，不打进 Skill Manager 安装包。启动时把 `skill-manager/assets/skills/skill-manager-builtin-agent/` 注入该 CLI 的 skills 目录（不注入配套维护 Skill）。未登录模型时只说明无法执行，不写库。第一版不实现 MCP 服务。
- [ ] T033 [US3] 在 `skill-manager/crates/host/src/agent/tools.rs` 经 ACP 声明工具，映射现有 `skill-manager` JSON CLI **经 Host** 执行，供快照不够时主动查询（读正文、列待更新、查部署等）与写入。不得直打 core。删除/覆盖必须走 T024 计划；Agent 文本「已同意」无效。在 `skill-manager/assets/skills/skill-manager-builtin-agent/` 编写短说明（两层上下文已注入、细节用工具、003 确认与上传），不得抄 002「每次询问上传」。
- [ ] T034 [US3] 在 `skill-manager/crates/host/src/http.rs` 实现 `PUT /agent/context`（只收 `view`）、`POST /agent/messages`、会话与消息读取（`contracts/host-http.md`）。收消息时 Host 从 core 组装 `global`，与 `view` 一并钉死到该条任务；之后的 `view` 只影响新消息。不得要求用户粘贴报错、清单或技能名。不把整库正文写入快照。
- [ ] T035 [US3] 在 `skill-manager/crates/host/src/agent/tools.rs` 落实 FR-020：有 `git_auto_push` 则内置管理 Agent 可 push 且不逐步确认；无授权则失败并说明去设置开启，Agent 不得写该设置。外部编码 Agent 路径不改 002。
- [ ] T036 [US3] 在 `skill-manager/crates/host/src/agent/` 只实现对话响应；不实现巡检/自动采用上游/自动入库/无人值守 push。002 调度器保持与对话无关。
- [ ] T037 [US3] 在 `skill-manager/src/` 接通对话 UI：只上报当前界面 `view`；全局由 Host 组装。用户能说「待更新的那些」「这个技能的上游」而无须点名或粘贴。进行中任务切页不改对象；局部失败分项说明；历史从 Host 本机状态读取。
- [ ] T038 [P] [US3] 在 `skill-manager/assets/skills/skill-manager-install/` 补「配置已锁定管理 CLI 登录」章节，不与配套维护 Skill 合并。
- [ ] T039 [US3] 按 `quickstart.md` §3 验收：注释写入、切页不串、删除确认、未授权上传、空闲不改库、未登录不冒充成功；未打开详情时「译文待更新的那些」能对上全局清单；详情页问上游不必粘贴；冲突/暂停不必粘贴报错；快照无全文时 Agent 能用工具读出片段且用户未粘贴。

**Checkpoint**: 对话在当前 Host 上可用；管理站切到哪台即用哪台的 Agent。

---

## Phase 6: Polish & Cross-Cutting

**Purpose**: 文档、002 指向、回归。

- [ ] T040 在 `docs/requirements.md`、`specs/002-skill-management/spec.md` US8 场景 6 与 US5 场景 5（拉取冲突）各增加指向 003 的一句：有内置管理 Agent 时现场由软件交给对话，不重写 002 全文、不改 FR-039 的「无三选一界面」。
- [ ] T041 [P] 核对 `GLOSSARY.md` 与实现名、ADR 0002；确认安装 Skill 路径已写入仓库。无新名则勾掉并注明无差异，不改术语。
- [ ] T042 按 `quickstart.md` §4–5 跑管理站不可用时桌面仍能改库、Host 在场/不在场 CLI 与 002 核心命令回归。
- [ ] T043 [P] 在 `specs/003-host-station-agent/quickstart.md` 记录实际验收证据与未取证项。

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
- 提交按任务或逻辑组进行；本文件生成不代替实现
