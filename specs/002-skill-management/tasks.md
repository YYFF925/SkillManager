---
description: "Skill Manager 正式产品开发任务，按用户故事组织"
---

# Tasks: Skill Manager 正式产品

**2026-10-16 后续修订**：共享目录按当前用户主目录自动定位 `.agents/skills`，无需配置。下列已完成任务中关于手动设置路径、缺路径报错或前往设置的内容属于此前实施记录，已由本次修订取代；现行行为及验收以 FR-021/042/050/055/056、CLI 契约及 quickstart 为准，结果见 `checklists/m6-report.md` 第十节。

**同日交互修订**：此前“共享目录不收编”及空列表说明的任务描述也已取代。共享目录未受管技能支持纳入管理；卡片打开本机正文与文件详情，可点击控件显示手型。对应 FR-014/042/052/055，验收见 `checklists/m6-report.md` 第十一节。

**Input**：本目录的 `spec.md`、`plan.md`、`research.md`、`data-model.md`、`contracts/cli.md`、`quickstart.md`；约束依据 `.specify/memory/constitution.md` v2.0.2。

**范围**：全部 10 个 User Story（用户故事）与 FR-001～054；构建真实桌面应用与 CLI（命令行接口），不把 `001` 原型的模拟行为当作实现。所有源文件路径均相对仓库根目录，正式产品代码位于 `skill-manager/`。

**组织**：按故事优先级分阶段；同为 P1 的 US3 在 US2 前，先实现部署与接管能力，再接入获取链路。阶段是工作归属，不强制按文档顺序串行；实际执行按末尾 M1～M5 与依赖关系安排。阶段检查用于确认方向，**全部功能完成后才投入使用**，不把 MVP（最小可验证范围）作为提前交付承诺。

**测试**：依据 `quickstart.md` 明确要求，将 M1～M3 CLI 场景纳入集成测试；针对部署产物、同项合并和状态边界补核心断言。列出的自动化测试先于对应实现编写，确认失败后实现；GUI、双机、大仓库与 M4/M5 场景按既定人工验收执行，不把每项低影响改动都拆成测试任务。

**实施参考与界面基线**：技术任务先查 `docs/technical-reference/README.md` 的对应主题，按本项目需求评估其中实现经验；仅有证据缺口才回查旧源码。所有 GUI 任务开工前须查 `specs/001-skill-management-prototype/ui-prototype/README.md` 的对应页面与效果图，并结合 `visual-reference.md`、`ui-interaction-audit.md` 和可操作的 `ui-prototype/index.html`；最终视觉与交互须与原型一致，不能只借用图标和配色。各期收尾按 `quickstart.md` 的 GUI 对照要求记录证据，正式业务与演示有差异时以 002 spec 为准并说明。

## Format: `[ID] [P?] [Story] Description`

- 每项使用 `- [ ] Tnnn`，故事阶段附 `[USn]`；`[P]` 表示在所述前置完成后可与其他标记任务并行，涉及不同文件。
- 同一文件的修改串行；跨阶段修改 `Cargo.toml`、入口注册表及公共页面时也串行。
- 不引入技能 ID、共享资产数据库镜像、旧格式兼容、迁移链、改动归属账本或自动安装器。
- 2026-10-15 实地检查：本项目已经初始化 Git，`origin` 已配置为用户 Gitea，另有 `github` 远端；`plan.md`/`research.md` 中“未初始化、M1 git init”是过时前提，执行时复用现状。需求质检已同步纠正规格与设计中的旧前提；任务生成不执行提交或推送。

## Phase 1: Setup（工程准备）

**目的**：建立规划中的 Rust workspace（工作空间）、前端与两种入口，不搬运旧项目业务。

- [X] T001 在 `skill-manager/Cargo.toml`、`skill-manager/crates/core/Cargo.toml`、`skill-manager/crates/cli/Cargo.toml`、`skill-manager/src-tauri/Cargo.toml` 建立 Rust 2024 workspace 三成员与最小入口，按当前官方资料核对稳定版依赖兼容性；使用 clap、rusqlite bundled、notify 及成熟 YAML/序列化库，仓库操作调用本机 Git，不引入 git2。
- [X] T002 在 `skill-manager/package.json`、`skill-manager/vite.config.ts`、`skill-manager/tsconfig.json`、`skill-manager/src/main.tsx`、`skill-manager/src-tauri/tauri.conf.json` 建立 React/TypeScript、Vite、Tailwind 与 Tauri 2 开发构建入口，使用 npm 并保存锁文件。
- [X] T003 在 `skill-manager/tests/common/mod.rs`、`skill-manager/crates/cli/Cargo.toml`、`skill-manager/vitest.config.ts`、`skill-manager/src/test/setup.ts` 建立临时 SkillManagerVault、Git 上游、Agent 目录与独立状态目录测试设施；CLI crate 显式注册根 `tests/` 下的集成测试，确保 `cargo test --workspace` 真正执行它们。
- [X] T004 在 `skill-manager/src/App.tsx`、`skill-manager/src/styles.css`、`skill-manager/src/components/AppShell.tsx` 按原型的导航、布局、视觉密度与样式建立简体中文外壳，先阅读上述界面基线并查看对应效果图，复用 `docs/technical-reference/assets/` 的合法视觉素材及许可，覆盖两技能库、预设、Agent、Hermes、工具中心与设置入口，不嵌入演示数据。

**Checkpoint（检查点）**：Rust 两入口与前端可构建，测试入口可发现，业务规则尚未实现。

## Phase 2: Foundational（故事共用底座）

**目的**：只提供所有故事需要的连接、类型、本机存储及目标基础；部署合成、获取、工具检查等仍放在所属故事。

- [X] T005 在 `skill-manager/crates/core/src/domain.rs` 定义管理域、技能引用与目标引用；逐字落实 data-model 的“技能身份 = 管理域 + 技能名”“同域内唯一（大小写不敏感比较，macOS 文件系统不区分大小写）”“分类最多一级；未分类直接在域根下”，跨域同名允许，不以分类路径作身份。
- [X] T006 在 `skill-manager/crates/core/src/vault/connection.rs`、`skill-manager/crates/core/src/sync/git.rs` 实现选择已有本地 SkillManagerVault、确认 Git 仓库与远端、本机 Git 子进程与既有认证复用；提供后续获取/同步可复用的命令执行基础，不提供克隆 SkillManagerVault 或创建向导；路径无效时给出可定位原因。
- [X] T007 在 `skill-manager/crates/core/src/state/schema.sql`、`skill-manager/crates/core/src/state/mod.rs` 初始化仅本机状态 SQLite，采用 WAL 与 busy_timeout；实现 `settings`/`agents` 基础读写，共享资料原文不写 SQLite；引用使用 `(domain, name)`，`skills_dir` 的“可改，恢复默认=NULL”、`project_rel_dir` 的“仅配置保留”按 data-model 实现，不实现项目技能管理。
- [X] T008 在 `skill-manager/crates/core/src/output.rs` 落实 `specs/002-skill-management/contracts/cli.md` 已修订的统一 result/data/errors、非零部分失败退出码、全局 `--state-dir` 与可执行参数；检查可写本机检查记录或获取缓存，但不应用技能变化、不改本机安装事实，dry-run 不修改目标及业务状态；在 `specs/002-skill-management/quickstart.md` 校核实际参数，不能退回“检查不准写检查状态”的旧表述。
- [X] T009 在 `skill-manager/crates/core/src/deploy/targets.rs`、`skill-manager/crates/core/assets/agent-catalog.json` 从 `docs/technical-reference/agent-catalog.json` 接入全量 53 项 Agent 资产，按 `agent-integration.md`/`local-runtime-and-shell.md` 核对目标平台当前目录与检测条件；实现目标所属域、路径解析、只检测不启动不部署的基础能力，`shared` 是特殊部署目标，不是管理库。
- [X] T010 在 `skill-manager/crates/core/src/lib.rs`、`skill-manager/crates/cli/src/main.rs`、`skill-manager/src-tauri/src/lib.rs`、`skill-manager/src/services/client.ts` 接通同一 core 上下文、全局 `--vault`/`--state-dir`/`--json` 与类型化 Tauri 调用；首次 GUI 选库结果保存在本机状态，前端与入口不复制业务判断。

**Checkpoint**：GUI（图形界面）与 CLI 使用同一库与同一本机状态规则，具备显式隔离两台电脑的运行参数。后续故事均依赖本阶段；尚不存在的业务入口不得返回伪成功。

## Phase 3: User Story 1 — 日常维护自己的技能库（P1，M1）

**目标**：浏览与维护现有库，六项管理资料可持续读写，外部改动实时反映。

**独立验收**：使用预置技能完成组合筛选、正文切换、层级文件阅读、分类移动、标签治理和管理资料修改；无需上游或首次部署。M1 核对 `quickstart.md` 1.1～1.8，尚未实现的获取入口同名检查明确记为待 M3 复核。

### 设计前置

- [X] T011 [US1] 在 `specs/002-skill-management/data-model.md` 落定 spec.md 已明确的译文时效规则所需最小比较依据与存储：原文变化而译文未维护才提示；按当前原文保存或采用对应新译文后解除。`specs/002-skill-management/contracts/cli.md` 已有 translation/overlay 维护契约，按其实现设计；不建翻译历史或自动翻译。

### Tests（测试，先写后实现）

- [X] T012 [P] [US1] 在 `skill-manager/crates/core/tests/vault_management.rs` 编写实际文件断言：一级分类移动、同域大小写不敏感唯一性、跨域同名、分类关联标签候选、批量增删标签不丢其他标签、未知 YAML 字段写回保留、译文只读展示与正文变化提示。
- [X] T013 [P] [US1] 在 `skill-manager/tests/cli_library.rs` 编写 M1 查询与维护场景，验证 JSON 结果、正文选择、目录实际移动、全库标签更名/解除关联及错误退出码；使用预置已有库，不借助尚未实现的 `skill import` 创建前置。

### Implementation（实现）

- [X] T014 [US1] 在 `skill-manager/crates/core/src/manager/mod.rs`、`skill-manager/crates/core/src/manager/files.rs` 实现 meta/translation/overlay/source 文件读写，逐字落实“标签，可多个；空=无标签”“中文短注释，一般 ≤20 字，不设强制上限”“缺省该键=管理域内全支持”“缺省=全支持”“保留软件不认识的字段”；`translation.md`“整份一项；仅管理器阅读，不作部署正文”，无上游的自建技能无 `source.yaml`。
- [X] T015 [US1] 在 `skill-manager/crates/core/src/vault/scan.rs`、`skill-manager/crates/core/src/vault/index.rs` 实现两域技能扫描与内存索引、原名/描述/注释/来源/分类/标签组合搜索、无标签筛选和分类联动标签；按 `docs/technical-reference/skill-detection.md` 核对技能入口识别；旧版从 frontmatter（文件头元数据）派生名称的观察不覆盖本项目目录名身份，不新增强制名称一致性检查；不扫描成第二套中央库，不把清单落 SQLite；部署与预设筛选待相应故事接入真实数据。
- [X] T016 [US1] 在 `skill-manager/crates/core/src/vault/manage.rs` 实现分类目录移动、单项/批量标签增删、跨两域标签重命名与删除关联、管理资料保存；共用同域同名校验供后续创建/入库/收编/复制复用，移动分类不改变名字或引用。
- [X] T017 [US1] 在 `skill-manager/crates/core/src/vault/watch.rs` 实现 notify 文件监听，外部内容/资料编辑与目录移动后刷新内存索引并通知 GUI；共享文件所有写入者地位平等，不因文件监听事件自动提交推送或首次部署。
- [X] T018 [US1] 在 `skill-manager/crates/cli/src/commands/skill.rs`、`skill-manager/src-tauri/src/commands/skill.rs` 接入 `skill list/get/read/set/tag/move` 与补齐后的译文/覆盖层维护命令；详情返回来源 repo/subpath/version、库路径、文件树和已存在部署的位置，缺资料如实为空，写入只走 core。
- [X] T019 [US1] 在 `skill-manager/src/pages/LibraryPage.tsx`、`skill-manager/src/components/SkillDetail.tsx`、`skill-manager/src/components/ManagerEditor.tsx` 实现两库列表/网格、组合搜索、分类联动、批量标签、层级文件阅读、中英文切换与六项资料编辑；有译文默认中文，无译文原文，显示译文待更新与来源分支/引用、版本、库和部署路径；当前 OS 不适用整项灰显，仍可维护资料和撤销已有使用，支持声明不丢本机未显示的 Agent。
- [X] T020 [US1] 运行本故事测试及 `quickstart.md` M1 场景，在 `specs/002-skill-management/checklists/m1-report.md` 逐项记录预期、实际、证据与通过/未过；同名约束在当前已有写入口验证，创建/收编等未建入口标待后续复核，不把部分覆盖报为 US1 全部通过。

**Checkpoint**：现有库可独立维护；GUI 保存的部署刷新与上传分别在 US3/US5 接通后复验。

## Phase 4: User Story 3 — 部署技能给本机 Agent（P1，M2）

**目标**：按域与支持声明合成独立副本，维护已有部署，外部改动逐项暂停。

**独立验收**：预置库技能单项/批量部署与撤销，检查原文未变、译文与 `.manager` 未进入产物；一个目标被外改时其余继续，dry-run 不落盘。预设入口在 US7 用同一引擎补齐，不将其计为 M2 已完成。

### 设计前置

- [X] T021 [US3] 在 `specs/002-skill-management/research.md`、`specs/002-skill-management/data-model.md` 完成 M2 设计：核对注释注入格式、落定覆盖层省略维度与优先顺序、共享通用产物判定、产物指纹；本机记录的 `target_path`/`deployed_hash`、暂停/删除保留语义及 `specs/002-skill-management/contracts/cli.md` 显式恢复参数已由本轮质检确定，按其细化实现，不重复增加目录标记、备份、合并界面或恢复队列。

### Tests

- [X] T022 [P] [US3] 在 `skill-manager/crates/core/tests/deployment.rs` 编写合成产物测试，覆盖按 Agent/OS 的覆盖层与整份 `SKILL.md` 替换、中文注释仅注入产物、译文/`.manager` 不部署、域边界与支持声明、支持收窄保留已有部署且警告、共享目录通用性。
- [X] T023 [P] [US3] 在 `skill-manager/tests/cli_deploy.rs` 编写 M2 CLI 场景，覆盖未受管同名直接接管、受管副本外改暂停且其余继续、已有部署同步、撤销/删库清理、多技能×多目标逐项过滤/部分完成与 dry-run 前后文件和数据库不变；先用预置库验证，不依赖 US2。

### Implementation

- [X] T024 [US3] 在 `skill-manager/crates/core/src/state/deployments.rs`、`skill-manager/crates/core/src/deploy/policy.rs` 落实本机记录及准入，`target` 为“`agent:<key>` / `shared` / `hermes:default`”，`status` 为“`ok` / `paused_local_change`”，保存 `target_path`、`source_hash` 与 `deployed_hash`；域边界不可由声明绕过，显式空支持集合不支持目标；收窄保留已有使用并警告，不禁止资料维护、撤销/禁用。
- [X] T025 [US3] 在 `skill-manager/crates/core/src/deploy/compose.rs` 实现原始文件 → 目标/系统 overlay → 中文注释合成；overlay“同路径文件一项”、允许整份 `SKILL.md` 替换且不自动合并原文；原始正文不回写、不部署译文和 `.manager`，共享目录仅接受通用结果。
- [X] T026 [US3] 在 `skill-manager/crates/core/src/deploy/apply.rs` 实现单项/多技能至多个明确目标的批量独立副本部署与已有副本接管接口，未受管同名直接覆盖并登记；提供供 US2/US6 收编调用的原副本接管能力，成功记录产物基线及真实路径，不新增软链接部署模式。
- [X] T027 [US3] 在 `skill-manager/crates/core/src/deploy/sync.rs`、`skill-manager/crates/core/src/deploy/remove.rs` 实现仅同步已有记录、目标外改逐项暂停、明确裁决后恢复、单项/多选撤销和删库清理；删库与上游删除共用该清理入口，暂停副本与处置所需记录保留，其余继续；dry-run 报告技能/目标/路径而不改文件与业务状态。
- [X] T028 [US3] 在 `skill-manager/crates/cli/src/commands/deploy.rs`、`skill-manager/src-tauri/src/commands/deploy.rs`、`skill-manager/crates/cli/src/commands/skill.rs` 接入 `deploy to/undo/sync/status`、暂停处置与 `skill delete`；CLI 修改库后同步部署不依赖 GUI，也不触发上传。
- [X] T029 [US3] 在 `skill-manager/src/components/DeployDialog.tsx`、`skill-manager/src/components/DeploymentStatus.tsx`、`skill-manager/src/pages/LibraryPage.tsx` 接入单项/多技能多目标部署、撤销、删除、本机共享目录路径设置、支持范围警告与暂停提示，GUI 保存影响合成的修改后自动刷新已有部署；技能库 `--deployed-to`/目标筛选接入真实记录，批量报告过滤项与暂停项，不提供差异合并界面。

**Checkpoint**：US3 除预设入口外可独立验证；M2 完整报告在 US6 收尾统一生成。

## Phase 5: User Story 2 — 获得技能进入自己的库（P1，M3）

**目标**：上游部分入库、本机收编、自建与跨域复制，保留原名且创建不自动部署。

**独立验收**：用测试来源仓库、一个未受管 Agent 副本和空分类，分别入库、收编和自建，再跨域复制；只收编接管其原有部署，其他新增无首次部署。

### 设计前置

- [X] T030 [US2] 在 `specs/002-skill-management/research.md` 落定上游获取缓存复用/清理和 Git 进度/取消实现，参照 `docs/technical-reference/git-and-discovery.md`，有证据缺口才查旧源码；`specs/002-skill-management/contracts/cli.md` 的 preview→按子路径选择 import、分支引用、分类/未分类与流式输出契约已经补齐，按其接入，不另造默认全量入库。

### Tests

- [X] T031 [P] [US2] 在 `skill-manager/tests/cli_acquire.rs` 编写 M3 获取场景：按仓库技能总数判断默认分类、只选部分技能、继承 `.manager`、同域大小写同名阻止、跨域同名允许、自建与复制不部署且副本独立、单项/批量收编接管原部署。

### Implementation

- [X] T032 [US2] 在 `skill-manager/crates/core/src/upstream/fetch.rs` 实现来源仓库获取与缓存，通过本机 Git 流式读取真实进度，GUI 取消和 CLI Ctrl-C 均能终止本次获取并如实返回取消；获取失败/取消不将技能登记为已入库。
- [X] T033 [US2] 在 `skill-manager/crates/core/src/upstream/discover.rs` 实现仓库技能候选扫描与预览，技能名按目录身份保留，旧参考 frontmatter 名称不替代目录名；保留来源子路径，不解析或自动补齐技能依赖；单个候选也列出，默认分类依据来源仓库技能总数而不是本次选择数量。
- [X] T034 [US2] 在 `skill-manager/crates/core/src/vault/acquire.rs` 实现选定候选复制入域、同名阻止、分类选择与上游管理资料继承，登记技能上游 repo/ref/subpath、实际采用完整提交 OID 与 registered_at；`skipped_version` 是“可选：已跳过的上游版本（含删除类），随库共享”；来源及采用版本属于同一 SkillManagerVault 共享资产，另一电脑拉取不重新入库，不把 SkillManagerVault 远端当成技能上游。
- [X] T035 [US2] 在 `skill-manager/crates/core/src/vault/adopt.rs` 实现 Agent 未受管技能单项/批量收编至对应域，支持逐技能资料输入补充注释/标签/分类/声明/译文并调用 US3 接管原部署，不把同一段注释擅自套给全部技能；不强制来源，同名库技能阻止而不覆盖，不新增其他目标部署。
- [X] T036 [US2] 在 `skill-manager/crates/core/src/vault/create.rs`、`skill-manager/crates/core/src/vault/copy.rs` 实现自建技能目录及管理资料创建和跨域复制；复制保留原件与来源记录，两份独立维护，创建与复制均不部署；复用 US1 同名与分类校验。
- [X] T037 [US2] 在 `skill-manager/crates/cli/src/commands/acquire.rs`、`skill-manager/src-tauri/src/commands/acquire.rs` 接入候选发现、`skill import/adopt/create/copy` 与进度/取消事件；所有选择由显式参数传入，不要求 Agent 回 GUI 补裁决，结构化结果与进度输出可区分。
- [X] T038 [US2] 在 `skill-manager/src/components/ImportDialog.tsx`、`skill-manager/src/components/AdoptDialog.tsx`、`skill-manager/src/pages/AgentPage.tsx` 接入候选勾选、已选数量、分类和资料补充、真实进度与取消、仅本地技能单项/批量收编及跨域复制；没有选择不等于拒绝新增候选。

**Checkpoint**：三种获取途径与跨域复制可执行。大仓库实测在 M3 报告中记录，不能用测试夹具代替 SC-008。

## Phase 6: User Story 4 — 吸收上游变化并保留维护成果（P1，M3）

**目标**：按仓库检查，按技能采用版本，显式更新与新增选择，拒绝/跳过决定随库共享。

**独立验收**：让测试来源产生正文、管理资料、新增、删除与改名变化，跨两域检查一次，分别执行单项更新、仓库更新、拒绝新增、跳过/恢复/明确采用；分类与未提供的管理资料保留。

### 设计前置

- [X] T039 [US4] 在 `specs/002-skill-management/data-model.md`、`specs/002-skill-management/research.md` 对照既定 §1.3/1.6 与 `specs/002-skill-management/contracts/cli.md` 细化仓库/管理域/子路径拒绝记录、仓库地址去重及完整 OID 比对设计；技能上游绑定来源仓库、另一机同步 SkillManagerVault 不改变来源已明确，不再将两条链路混同为“来源继承冲突”；决定与采用仍保持显式；落实 spec.md 首次未选 C≠后来新增 B 的发现基线，更新已有技能不能让未决定、仍存在的 B 丢失提示。具体基线形式在本任务设计，不先增加历史/候选状态共享系统或 URL 身份体系。

### Tests

- [X] T040 [P] [US4] 在 `skill-manager/crates/core/tests/upstream_merge.rs` 编写字段粒度更新断言：meta 每键、整份译文、overlay 同路径文件，上游提供则采用、未提供则保留，含本地已改同项与仅管理资料变化；分类不变，采用版本推进且不重复应用。
- [X] T041 [P] [US4] 在 `skill-manager/tests/cli_upstream.rs` 编写 M3 检查/更新场景：跨域同仓库只获取一次、文件增改删汇总、检查不应用、单项范围隔离、仓库批量选择与跳过、拒绝新增共享、区分技能内文件删除与整个技能删除/改名清理、首次未选与后来新增及暂停副本不被覆盖。

### Implementation

- [X] T042 [US4] 在 `skill-manager/crates/core/src/upstream/check.rs`、`skill-manager/crates/core/src/state/upstream_checks.rs` 实现单技能/仓库/全库检查，跨两域按来源仓库只检查一次，每技能分别对照采用版本；本机记录“`remote_version, checked_at, result`”，检查只更新检查事实，不更新技能内容、采用版本或部署。
- [X] T043 [US4] 在 `skill-manager/crates/core/src/upstream/changes.rs` 实现已有技能文件新增/修改/删除与后来新增候选资料汇总、跳过后提示计数过滤；按 T039 基线区分首次未选与后来出现的候选，不因更新其他技能而丢失未决定、仍存在的新增提示；上游改名按删除加新增，不生成逐行差异或自动重命名结论。
- [X] T044 [US4] 在 `skill-manager/crates/core/src/manager/merge.rs` 实现 D3 同项上游值优先合并，保留未知字段与上游未提供资料，分类目录由本地保留；原文变化保留译文并更新待译提示，版本随成功采用推进，不以正文未变跳过管理资料更新。
- [X] T045 [US4] 在 `skill-manager/crates/core/src/upstream/apply.rs` 实现显式单技能与仓库批量更新：单项仅所选技能，批量处理未采用且未跳过条目和选定新增；更新后同步已有部署，新增不首次部署，确认整个技能删除才走 US3 清理并保留暂停副本处置记录；仅删除技能内文件时移除相应文件、其他变化照常应用，保留技能和已有部署并刷新。
- [X] T046 [US4] 在 `skill-manager/crates/core/src/upstream/decisions.rs` 实现拒绝新增的共享记录、技能当前版本跳过/列表/恢复/明确采用；跳过只影响该域该技能该版本，下一版本恢复提示，仓库批量更新尊重决定，返回列表或不选择不写拒绝。
- [X] T047 [US4] 在 `skill-manager/crates/cli/src/commands/upstream.rs`、`skill-manager/src-tauri/src/commands/upstream.rs` 接入 `skill check/update/update-repo/skip/unskip/skipped` 及新增决定命令；检查结果先展示，应用通过独立显式调用表达用户裁决，自维护 Gitea 来源也无自动采用例外。
- [X] T048 [US4] 在 `skill-manager/src/components/UpstreamDialog.tsx`、`skill-manager/src/components/UpstreamCandidates.tsx` 接入仓库级文件变化、单项/批量确认、具体候选选择、拒绝记住、跳过查看与恢复；仅检查不显示“已更新”，失败/未检查与已查无变化分开。
- [X] T049 [US4] 执行 `quickstart.md` M3 全部场景与 `spec.md` US2/US4 全部验收，在 `specs/002-skill-management/checklists/m3-report.md` 记录证据，实测 ≥100MB 或慢速真实来源获取的 GUI/CLI 进度与取消；回填 `specs/002-skill-management/checklists/m1-report.md` 中各获取入口同名检查，核对 SC-007/008，未实测不得标通过。

**Checkpoint**：获取、更新与维护资料保留规则完整成立，上传自己的库仍是 US5 的独立动作。

## Phase 7: User Story 5 — 跨电脑共享维护成果（P1，M4）

**目标**：共享文件拉取与明确同意后的全量上传，本机状态留本机，冲突/断连如实显示。

**独立验收**：同一远端的两个克隆、两个 `--state-dir` 模拟两机，上传后另一侧拉取；已有部署刷新而新增不部署，两机启用名单、路径与工具安装记录不串机；制造冲突与断连验证本地工作继续。

### Implementation

- [X] T050 [US5] 在 `specs/002-skill-management/data-model.md`、`specs/002-skill-management/contracts/cli.md` 明确 GUI 自动上传既有确认的本机保存方式与首次获得确认的交互、上传失败后用户显式重试和后台只拉取的边界、冲突由 Agent 修复后的继续命令；不增加改动来源跟踪、不把任何 CLI 修改或部署同步当作上传同意。
- [X] T051 [US5] 在 `skill-manager/crates/core/src/sync/pull.rs`、`skill-manager/crates/core/src/sync/status.rs` 实现复用本机 Git 的状态/拉取/冲突详情与继续路径；无冲突应用库文件后刷新索引、同步已有部署并在指令源变化时刷新本机已有启用指令目标（指令服务于 T069 接通后复验），冲突暂停交用户与 Agent，远端不可用显示未连接而本地操作继续；无克隆修复、断开远端或历史恢复入口。
- [X] T052 [US5] 在 `skill-manager/crates/core/src/sync/push.rs` 实现上传工作区全部改动并推送自己的 SkillManagerVault，包含非软件产生的修改；仅 GUI 保存已确认策略或 Agent 获明确同意后显式调用可达，失败保留可读原因并支持用户主动重试，不向任何来源仓库提交、不在后台重试推送。
- [X] T053 [US5] 在 `skill-manager/crates/core/src/scheduler.rs`、`skill-manager/crates/core/src/state/settings.rs` 实现启动/手动/定时上游检查和库拉取的统一调度；逐字落实“关闭/30 分钟/1 小时/6 小时/24 小时，默认 1 小时”，存储值“0=关闭/30/60/360/1440，默认 60”；为 US9 登记工具检查入口，三类结果分别表达，关闭定时不禁用手动操作。
- [X] T054 [US5] 在 `skill-manager/crates/cli/src/commands/sync.rs`、`skill-manager/src-tauri/src/commands/sync.rs` 接入 `sync pull/push/status/conflicts` 与继续路径；GUI 保存事件在部署刷新后按已确认策略调用上传，外部文件监听与 `deploy sync` 不触发上传。
- [X] T055 [US5] 在 `skill-manager/src/components/SyncStatus.tsx`、`skill-manager/src/pages/SettingsPage.tsx` 接入连接状态、共同检查频率、自动上传确认与上传失败显式重试；分别显示库变化和上游变化，冲突只提示并可复制给 Agent 的定位信息，不加专用合并界面。

**Checkpoint**：用 quickstart M4 的双机人工场景验证 US5 全部条款，统一报告待 US8 指令链路接通后生成；SC-005 特别核对任何后台路径都不推送。

## Phase 8: User Story 6 — 通过 Hermes 档案使用龙虾技能（P2，M2）

**目标**：单 Hermes 根目录、default 部署、多档案明确启用与档案独有技能收编。

**独立验收**：预置龙虾库技能部署 default，为另一现有档案启用/禁用及批量启停，更新不重置选择；收编档案独有技能后 default 可用且原档案选择不变。仅依赖 US1/US3，无需等待 US2/US4/US5。

### 设计前置

- [X] T056 [US6] 在 `specs/002-skill-management/research.md`、`specs/002-skill-management/data-model.md` 核对当前 Hermes 官方协议与本机实例，结合 `docs/technical-reference/hermes-integration.md` 明确单根档案发现、default 候选池、外部启用配置与本机 SQLite 名单的读写职责；旧参考不足以视为当前已验证，无法核实则报告该接入限制，不能伪造档案成功。

### Tests

- [X] T057 [P] [US6] 在 `skill-manager/tests/cli_hermes.rs` 编写 M2 档案 CLI 场景：明确启用、不自动扩散、内容更新保持选择、支持收窄保留已有启用且警告/阻止新增、档案独有收编至龙虾库和 default、批量启停；按核实后的真实外部目录/配置协议构造夹具。

### Implementation

- [X] T058 [US6] 在 `skill-manager/crates/core/src/hermes/profiles.rs`、`skill-manager/crates/core/src/hermes/enables.rs`、`skill-manager/crates/core/src/state/hermes.rs` 实现本机单 Hermes 根目录配置、现有档案发现与外部启用协议，名单使用“`(profile, domain, name)` → 启用关系”；default 由“`hermes:default`”部署记录表达，其他档案明确启用/禁用与批量操作，更新不重置选择；域与支持声明沿用 US3。
- [X] T059 [US6] 在 `skill-manager/crates/core/src/hermes/adopt.rs` 实现档案独有技能收编到龙虾库并部署 default，复用 US1 资料读写、同名阻止及 US3 副本接管/部署能力，保持原档案启用状态；不依赖 US2 的来源获取，不创建第二套库内技能内容。
- [X] T060 [US6] 在 `skill-manager/crates/cli/src/commands/hermes.rs`、`skill-manager/src-tauri/src/commands/hermes.rs`、`skill-manager/src/pages/HermesPage.tsx` 接入 `hermes profiles/list/enable/disable`、`skill adopt-hermes`、只读技能详情与前往库编辑入口；筛选复用分类联动标签规则，不提供档案生命周期、活动切换、多实例或 SOUL.md 管理。
- [X] T061 [US6] 运行 US3/US6 测试与 `quickstart.md` M2 场景，在 `specs/002-skill-management/checklists/m2-report.md` 记录真实目标目录与 Hermes 效果、SC-003 部署内容边界和暂停/恢复证据；补核 US3/US6 在 quickstart 未逐项列出的验收场景，预设相关项标明由 M5 补齐。

**Checkpoint**：M2 部署与 Hermes 链路可检查，不为等待后续 P1 故事而推迟这期验收。

## Phase 9: User Story 7 — 用预设组合部署与撤销（P2，M5）

**目标**：共享组合定义与本机部署分离，按组叠加、按成员撤销。

**独立验收**：预设 B/C 部署到已有 A/B 的目标得到 A/B/C，撤销后只剩 A；编辑成员不改变部署；另一库克隆得到相同图标、顺序与成员。

### Implementation

- [X] T062 [US7] 在 `skill-manager/crates/core/src/preset/files.rs` 实现 `presets/code.yaml` 与 `presets/lobster.yaml` 读写，逐字落实 id“预设内唯一标识（人写，域内不重复即可）”、成员“技能名引用（域内唯一可解析）”、定义“含顺序与图标”；缺失成员“界面标注‘缺失成员’，不阻塞其他成员”，编辑定义不触发部署。
- [X] T063 [US7] 在 `skill-manager/crates/core/src/preset/manage.rs`、`skill-manager/crates/cli/src/commands/preset.rs` 实现新建/删除/重命名/图标、预设排序、成员增删/读取及 `reorder-members` 成员显示排序（复用 members 数组顺序，不另建字段），限定同域成员；遵守 `specs/002-skill-management/contracts/cli.md` 的显式 `(domain, id)` 预设部署/撤销参数，不在两域同 id 时静默选域。
- [X] T064 [US7] 在 `skill-manager/crates/core/src/preset/deploy.rs`、`skill-manager/crates/cli/src/commands/deploy.rs` 接入预设叠加部署、按成员撤销及适用的 dry-run；复用 US3 域/支持过滤与暂停规则，不追踪来源预设或引用计数，成员编辑不自动部署或撤销。
- [X] T065 [US7] 在 `skill-manager/src-tauri/src/commands/preset.rs`、`skill-manager/src/pages/PresetPage.tsx`、`skill-manager/src/pages/LibraryPage.tsx` 实现预设 CRUD（增删改查）、预设及成员两种显示排序/图标选择、成员全库浏览、归属查看及按预设筛选；分类与标签联动复用 US1，跨预设成员撤销如实说明影响范围。
- [X] T066 [US7] 执行 `quickstart.md` 5.1～5.3、5.12 与 US7 全部场景，将叠加/撤销实际目录、成员修改不动部署、跨机定义相同的证据写入 `specs/002-skill-management/checklists/m5-report.md`；复核此前 US3 的预设入口待验项，不以组合定义存在冒充已经部署。

**Checkpoint**：预设完整使用同一部署引擎，无新增共享部署状态。

## Phase 10: User Story 8 — 管理本机 Agent 与指令（P2，M2/M4）

**目标**：Agent 目标管理完整可见，指令源共享、分发目标本机决定。

**独立验收**：检测、启停、排序、修改目录与删除未受管本地技能；编辑全局/专属指令后实际 `AGENTS.md` 为全局在前、专属在后；两机源一致而目标配置独立。

### Implementation

- [X] T067 [US8] 在 `skill-manager/crates/core/src/deploy/agent_config.rs`、`skill-manager/crates/cli/src/commands/agent.rs`、`skill-manager/src-tauri/src/commands/agent.rs` 完成 `agent list/detect/enable/disable/reorder/set-dir` 及对应 GUI 调用，检测只读本机存在事实，保留全量名单并区分已检测/其他支持项；启停、排序、技能目录、仅配置的项目相对路径与独立指令目标只写本机状态，后续部署使用新目录。
- [X] T068 [US8] 在 `skill-manager/crates/core/src/deploy/local_skills.rs`、`skill-manager/crates/cli/src/commands/agent.rs`、`skill-manager/src-tauri/src/commands/agent.rs` 实现 Agent 本地技能列表与单项/多选删除：未受管直接删除且不改库，受管走 US3 撤销及暂停规则；复用收编需要的受管判定，不把删除 Agent 本地副本变为删库。
- [X] T069 [US8] 在 `skill-manager/crates/core/src/vault/instructions.rs` 实现 `instructions/global.md` 与 `<agent-key>.md` 读取/保存与拼接分发，落实“无专属文件 → 仅全局内容”；保存按全局在前专属在后覆盖各机启用目标，空内容正常保存，指令路径独立，不扩展 Hermes 档案指令；按用户已确认的 CHK026，在库拉取指令源变化后自动刷新本机已有启用目标；不改变启用名单/指令路径，失败如实表达。
- [X] T070 [US8] 在 `skill-manager/crates/cli/src/commands/instruction.rs`、`skill-manager/src-tauri/src/commands/agent.rs` 接入 `instruction get/set`（文件/stdin）及已有 Agent 本机目录/启停配置；沿用单保存动作，不增独立分发选择或保存/分发双入口，指令路径与技能路径独立。
- [X] T071 [US8] 在 `skill-manager/src/pages/AgentPage.tsx` 接入 Agent 分组、重新检测、启停排序、目录行内修改、本地技能列表删除和受管/仅本地状态；本项在 M2 完成，不加入远程 Agent 管理，后续 US2 收编入口在同页串行追加。
- [X] T072 [US8] 在 `skill-manager/src/components/InstructionEditor.tsx`、`skill-manager/src/pages/AgentPage.tsx` 沿用全局/逐 Agent 的单编辑区和单“保存”：保存后保持打开，切换工具保留未保存草稿；使用各机 Agent 启用配置与指令路径，不加分发勾选或第二个分发按钮，本项 M4 完成。
- [X] T073 [US8] 执行 `quickstart.md` M4 与 US5/US8 全部场景，在 `specs/002-skill-management/checklists/m4-report.md` 记录两克隆/两状态目录下的文件、SQLite 与 Git 证据；核对 GUI 保存与 Agent 同意两上传路径、全量提交、显式重试/后台不推送、冲突继续/断连、本机状态隔离、指令拼接与拉取后的已有指令目标刷新，验收 SC-005/006。

**Checkpoint**：Agent 检测不等于部署；指令内容同步不等于另一机已选择分发目标。

## Phase 11: User Story 9 — 用工具中心掌握本机工具（P2，M5）

**目标**：共享工具资料、本机安装事实、软件执行版本检查、外部 Agent 安装维护与回写。

**独立验收**：在空本机安装状态下打开已有共享工具清单，看到未安装工具；完成登记/修改、单项/全部检查、任务复制与真实版本回写；配套候选由用户勾选入库，版本互不混淆。

### Implementation

- [X] T074 [US9] 在 `specs/002-skill-management/data-model.md`、`specs/002-skill-management/contracts/cli.md` 落定非标准工具版本的比较约定，保留“单个登记命令只取上游最新版本、软件解析对比”的已修订边界；安装事实与工具检查表、候选来源、资料修改和技能任务契约已补齐，按其细化实现，不恢复本机版本查询冒充上游检查、`upstream_command` 第二条查询路径或按来源类型的解析器。
- [X] T075 [US9] 在 `skill-manager/crates/core/src/toolcenter/files.rs`、`skill-manager/crates/core/src/toolcenter/catalog.rs` 实现 `tools/<tool-slug>/tool.yaml` 与 `guide.md` 读写、共享登记/修改、搜索、一级分类与状态筛选；落实“工具 slug（目录名）即工具身份”，`summary` 是卡片短摘要，`intro` 为“概览长简介（可读、多段）”，配套引用 `(domain, name)` 缺失时显示“未入库”，候选来源资料沿用现有入库链路。
- [X] T076 [US9] 在 `skill-manager/crates/core/src/state/tool_installs.rs`、`skill-manager/crates/core/src/state/tool_checks.rs` 实现已核实本机安装事实与独立检查记录；安装字段“`version, method, path, config_path`”只由 Agent 回写，无安装行显示“未安装”；检查状态“`unchecked` / `checking` / `checked` / `failed` / `no_command`”独立保存，不因检查插入安装行，安装/更新回写后重置检查为 `unchecked`。
- [X] T077 [US9] 在 `skill-manager/crates/core/src/toolcenter/check.rs` 执行单个登记的上游版本命令、解析最新版本并与安装记录对比，支持单项/全部及共同周期；成功检查记录为 `checked`，已安装且可比较时派生“最新/可更新”，未安装只显示上游版本，未登记命令/失败/未检查分别表达；检查不安装、不更新、不改本机版本，不能比较不冒充确定结论。
- [X] T078 [US9] 在 `skill-manager/crates/core/src/toolcenter/tasks.rs`、`skill-manager/crates/core/src/vault/tasks.rs` 实现工具与技能维护任务文本生成：工具任务含用户要求、简介、本机资料、检查结果与指南，技能任务支持单项/多选，含用户要求、本机 SkillManagerVault 路径、各技能库内路径、注释/分类/标签/支持声明、译文/覆盖层概况和本机已有部署；两类任务均引用配套维护 Skill 的真实库内路径及“按其规范执行”，引用规范不取消必要事实上下文；缺技能时如实提示未入库，不编造路径或内联另一份规范，不启动 Agent/任务队列、不把任务生成标为完成；在 `skill-manager/src-tauri/src/commands/skill.rs`、`skill-manager/src/pages/LibraryPage.tsx` 接通技能卡片及多选栏的维护任务生成/复制入口，与 CLI 使用同一核心。
- [X] T079 [US9] 在 `skill-manager/crates/cli/src/commands/tool.rs`、`skill-manager/src-tauri/src/commands/tool.rs`、`skill-manager/crates/cli/src/commands/skill.rs` 接入 `tool list/get/check/register/set/set-install/task/skill-link`、资料修改及技能任务命令；登记与回写均 CLI 可达，所有共享资料变更沿用询问后才上传的规范，生成任务不自动执行安装。
- [X] T080 [US9] 在 `skill-manager/src/pages/ToolCenterPage.tsx`、`skill-manager/src/components/ToolDetail.tsx` 接入搜索/筛选、未安装条目常驻、可读简介、本机事实、检查状态/结果、指南、配套 Skill 和任务复制；登记/资料维护/安装回写入口生成外部 Agent 任务，Agent 经同一 CLI/core 核实写入，不做自动发现或内置 Agent。
- [X] T081 [US9] 在 `skill-manager/src/components/ToolSkillCandidates.tsx`、`skill-manager/crates/core/src/toolcenter/skills.rs` 复用 US2 候选获取/选择入库流程，单候选也显示原名/注释/用途/来源/勾选与已选数量，允许补充分类/标签/注释，必须由用户选择，不自动部署；工具版本与 Skill 来源版本各自展示，不复制技能正文进工具资料。
- [X] T082 [US9] 执行 `quickstart.md` 5.4～5.8、5.13 与 US9 全部场景，向 `specs/002-skill-management/checklists/m5-report.md` 写入共享登记/修改、本机未安装、各检查状态、周期、真实 Agent 安装或更新后核实回写及候选入库证据；指南须覆盖用途/入口、实际安装方式、安装/配置位置、检查命令、更新步骤、验证与注意事项，失败如实记录。

**Checkpoint**：工具程序不随库同步，维护任务不冒充执行成果，工具与配套 Skill 分别管理版本。

## Phase 12: User Story 10 — 界面环境与可维护性（P3，M5）

**目标**：提供日常桌面环境、后台运行、诊断与库容量信息。

**独立验收**：无技能数据时仍能调主题/字号、关窗收托盘、重新打开与退出；有测试库时导出诊断并看到容量，故障有可获取记录。

### Implementation

- [X] T083 [P] [US10] 在 `skill-manager/src/pages/SettingsPage.tsx`、`skill-manager/src/styles.css` 实现浅色/深色/跟随系统主题及字号调整、本机持久化；落实“`theme`（light/dark/system，默认 system）”，系统主题变化实时反映，共同检查频率复用 US5，不添加第四套周期。
- [X] T084 [P] [US10] 在 `skill-manager/src-tauri/src/tray.rs`、`skill-manager/src-tauri/src/lib.rs` 实现 Tauri 2 关窗隐藏、托盘重开/检查上游/退出，隐藏时共同调度继续运行，只有退出才终止；核对当前 Tauri 2 官方 API（接口）后实现，业务检查仍调用 core。
- [X] T085 [P] [US10] 在 `skill-manager/crates/core/src/diagnostics.rs`、`skill-manager/src-tauri/src/diagnostics.rs` 实现可获取的运行日志、崩溃记录与诊断导出，覆盖下载失败/部署异常定位；参照 `docs/technical-reference/diagnostics-and-documents.md` 避免记录认证秘密，提供真实文件路径，不增加上传诊断或历史恢复功能。
- [X] T086 [P] [US10] 在 `skill-manager/crates/core/src/vault/capacity.rs`、`skill-manager/src/components/CapacityReport.tsx` 实现按当前库文件计算总占用与较大技能排行，结果不建立另一套共享资产存储；空库如实显示，界面可定位具体技能。
- [X] T087 [US10] 在 `skill-manager/src-tauri/src/commands/environment.rs`、`skill-manager/src/components/DiagnosticsPanel.tsx` 接通设置/诊断/容量 UI，执行 `quickstart.md` 5.9 与 US10 全部场景并写入 `specs/002-skill-management/checklists/m5-report.md`；以真实桌面操作验证托盘隐藏后检查仍运行，浏览器页面预览不能代替该结论。

**Checkpoint**：桌面行为与维护信息经过实际 macOS 运行验证，未验证的平台不声称已经兼容。

## Phase 13: Polish & Cross-Cutting Concerns（配套维护与整体收尾）

**目的**：交付普通上游维护 Skill，完成各期回验和全部核心操作的双入口验收，不新增产品能力。

- [X] T088 在 `skill-manager/assets/skills/skill-manager-maintenance/SKILL.md` 及必要的同目录 `references/cli.md` 编写单一配套维护 Skill：说明 `.manager` 实际格式、目录/同名/支持规则、原文/译文/注释/overlay 边界、来源更新、修改后同步部署与询问上传、冲突/暂停处理、CLI 使用、工具安装/登记/检查命令编写/真实验证回写；以已实现契约为依据，不写第二套业务规则或虚构命令。
- [X] T089 在 `specs/002-skill-management/quickstart.md` 固定本项目已配置 Gitea `origin` 的配套 Skill 来源子路径 `skill-manager/assets/skills/skill-manager-maintenance` 与实际命令；将可审阅的代码与技能按用户授权推送 Gitea 后，通过普通 `skill import` 显式选择入库并登记来源，再推进源版本验证普通检查/确认更新；若发布授权或远端访问缺失，保留待完成并说明，不改成专用分发或本地路径假验收，证据写入 `specs/002-skill-management/checklists/m5-report.md` 5.10。
- [X] T090 对照 FR-050/051 在 `specs/002-skill-management/contracts/cli.md` 逐项核对全部核心 GUI/CLI 操作与真实参数、JSON/退出码、无 GUI 裁决依赖、维护任务 Skill 路径；同步更新 `skill-manager/assets/skills/skill-manager-maintenance/references/cli.md`，运行已要求的 `cargo test --workspace` 与前端构建，GUI 按 quickstart 对原型索引中的页面与交互逐项人工验收、记录实现截图与差异，修复实际失败，不新增兼容路径或仅复述实现的测试。
- [X] T091 执行 `specs/002-skill-management/quickstart.md` 5.11 的真实完整旅程，对照 `spec.md` US1～US10 和 SC-001～008，补齐 `specs/002-skill-management/checklists/m1-report.md`、`specs/002-skill-management/checklists/m2-report.md`、`specs/002-skill-management/checklists/m3-report.md`、`specs/002-skill-management/checklists/m4-report.md`、`specs/002-skill-management/checklists/m5-report.md` 早期待验项；记录几十至几百技能下扫描/筛选实际耗时与环境，不把未确认的 <1s/微秒级目标作为验收承诺，不凭空声明平台兼容。
- [X] T092 在 `README.md`、`skill-manager/README.md`、`specs/002-skill-management/plan.md`、`specs/002-skill-management/research.md` 更新实际开发/构建/CLI 入口、SkillManagerVault 与本机状态位置、实现范围及各期结果；保留本轮质检已纠正的 Git 现状、库同步/上游更新边界与单保存指令交互，不恢复失效指引，未过保留原因，不以勾任务代替验收。

## Phase 14: User Story 10 增量 — 本机运行侧双入口（T093～T099）

**目的**：把 FR-050「本机运行侧」整类补齐（不只是已发现的共享目录一项），并落实 FR-053 的"诊断与容量两入口都可获取"。判定准则、契约取法与逐项清点结论见 `plan.md` 的"规划补齐"一节、`research.md` D22/D23 与 `contracts/cli.md` 的 `settings` 命令组。

**独立验收**：在无 GUI 参与的环境下用 CLI 完成"设置共享目录 → 部署到 shared"与"调整自动检查频率"；取到诊断报告与容量报告；`settings get` 能读到全部运行配置与调度事实，且主题、字号、上传授权没有 CLI 写入口。**不新增产品能力，只把已有 core 能力接出 CLI。**

### Tests（测试，先写后实现）

- [X] T093 [P] [US10] 在 `skill-manager/tests/cli_settings.rs`（并在 `skill-manager/crates/cli/Cargo.toml` 注册 `[[test]] name = "cli_settings" path = "../../tests/cli_settings.rs"`）按 quickstart 5.14～5.19 编写 CLI 断言：`settings get` 返回全部键的**当前生效值**（主题缺省 `system`、检查频率缺省 60、未设置的共享目录为 `null` 且不省略键、不用空字符串冒充）；纯 CLI 下 `settings set-shared-dir --path <临时目录>` 后 `deploy to shared <ref>` 成功，`--clear` 后如实报"尚未设置共享目录路径"；`settings set-auto-check` 接受 0/30/60/360/1440 并拒绝 45；`theme`/`font_scale`/`git_auto_push` **没有** CLI 写入口；`settings diagnostics --export` 返回真实文件路径；`settings capacity` 空库返回 0 与空排行。
- [X] T094 [P] [US10] 在 `skill-manager/crates/core/tests/` 补核心断言：Hermes 根按 `HERMES_HOME` 环境变量与平台默认解析（不读可写设置键）；自定义日志目录键移除后诊断报告仍给出正确的 `<state-dir>/logs` 路径；`settings get` 不再返回已删除的键。

### Implementation（实现）

- [X] T095 [US10] 在 `skill-manager/crates/cli/src/commands/settings.rs` 实现 `settings get [<key>...]`（汇总运行配置与调度事实；未设置的键返回 `null`、不省略键、不用空字符串冒充）、`settings set-shared-dir (--path <绝对路径> | --clear)`、`settings set-auto-check <0|30|60|360|1440>`；取值校验**复用 core 既有校验**，不新增一套判断；只提供这两个写入口——`theme`/`font_scale` 不提供写（界面偏好，FR-052），`git_auto_push` 不提供写（GUI 保存自动上传的长期授权，Agent 不得自行开启，FR-038）。
- [X] T096 [US10] 在 `skill-manager/crates/cli/src/commands/settings.rs` 实现 `settings diagnostics [--export]` 与 `settings capacity`，复用 `skill-manager/crates/core/src/diagnostics.rs` 与 `skill-manager/crates/core/src/vault/capacity.rs` 的既有能力；GUI 与 CLI 共用同一 core；导出返回**真实文件路径**；不提供上传诊断、不建第二套资产存储。
- [X] T097 [US10] 在 `skill-manager/crates/core/src/state/settings.rs`、`skill-manager/crates/core/src/hermes/` 清理两处无写入点的配置项：删除自定义日志目录键（固定 `<state-dir>/logs`）；Hermes 根改为按 `HERMES_HOME` 环境变量与平台默认解析，移除可写设置键，避免与 Hermes 自身的定位规则分叉。
- [X] T098 [US10] 在 `skill-manager/crates/cli/src/main.rs`、`skill-manager/crates/cli/src/commands/mod.rs` 注册 `Settings` 顶层子命令与 `commands::settings` 模块（**公共入口注册串行合入**，不与 T095/T096 并行），并核对 `--help` 输出的子命令与 `contracts/cli.md` 的 `settings` 命令组一致。
- [X] T099 [US10] 执行 quickstart 5.14～5.19，把 `specs/002-skill-management/contracts/cli.md` 的 T090 表中 `settings` 行结论更新为"通过"（附验证方式），把证据与差异写入 `specs/002-skill-management/checklists/m5-report.md` 的 5.14～5.19；**未取得证据的项保持未过并写明原因**。

## Phase 15: 增量收尾（T100）

- [X] T100 同步 `skill-manager/README.md` 与 `skill-manager/assets/skills/skill-manager-maintenance/references/cli.md` 的 CLI 参考（新增 `settings` 命令组、两个写入口与三条只读边界的说明）；运行 `cargo test --workspace` 与前端构建，不新增兼容路径或仅复述实现的测试。

## Phase 16: 增量 — 共享目录技能浏览（T101～T108）

来源：原型「共享 Agent」页（`renders/03`）在需求链上被遗漏，用户 2026-10-16 确认该能力必须做。
规格依据 FR-042（扩到本机目标）与 FR-055；契约变更见 `contracts/cli.md` 的 `agent` 命令组；规划见 `plan.md` 的"规划补齐：共享目录技能浏览"。

- [X] T101 [P] [US3] 在 `skill-manager/tests/`（按需在 `crates/cli/Cargo.toml` 注册 `[[test]]`）先写 CLI 断言：共享目录未设置时 `agent local-skills shared` 与 `agent delete-local shared <name>` 均如实报"尚未设置共享目录路径"且退出码非 0；设置共享目录后 `agent local-skills shared` 列出目录内技能并区分受管（有 `target=shared` 部署记录）与未受管；`agent delete-local shared <未受管名>` 删除实际目录、`shared <受管名>` 按撤销部署规则处理；非编码域的库不受影响。
- [X] T102 [US3] 在 `skill-manager/crates/core/src/deploy/local_skills.rs` 让本机目标技能浏览按**目标**工作：路径解析复用 `TargetResolver` 的 `Target::Shared` 分支（`deploy/targets.rs`），受管判定沿用该目标的部署记录；不新增第二套扫描或判定逻辑，不引入新抽象层。
- [X] T103 [US3] 在 `skill-manager/crates/core/src/deploy/` 让"删除本机技能"支持共享目录目标：未受管项删除实际文件、受管项走撤销部署；沿用既有确认语义，不新增额外保护机制。
- [X] T104 [US3] 在 `skill-manager/crates/cli/src/commands/agent.rs` 让 `agent local-skills` 与 `agent delete-local` 接受 `shared` 取值（复用既有目标解析），并核对 `--help` 输出与 `contracts/cli.md` 的 `agent` 命令组一致。
- [X] T105 [US3] 在 `skill-manager/src/services/client.ts`、`skill-manager/src/App.tsx`、`skill-manager/src/components/AppShell.tsx`、`skill-manager/src/pages/AgentPage.tsx` 增加编码域「共享目录」入口与页面：复用本机 Agent 技能视图（受管／未受管／本地有变更筛选、删除），未设置共享目录路径时**保留入口**并如实提示 + 前往设置；不增加"从库部署"按钮，不实现收编。
- [X] T106 [P] [US3] 前端测试：覆盖"未设置共享目录时入口保留与提示文案"、"受管／仅本地渲染与删除确认文案"；不为此引入整页 mock 基础设施。
- [X] T107 [US3] 在 `specs/002-skill-management/quickstart.md` 增加共享目录浏览场景，并执行验证（含 GUI 实操取证：从 `.app` 启动、设置共享目录、列出、删除未受管项），结果写入 `specs/002-skill-management/checklists/m6-report.md`；未取得证据的项保持未过并写明原因。
- [X] T108 收尾：同步 `skill-manager/README.md` 与 `skill-manager/assets/skills/skill-manager-maintenance/references/cli.md` 的 CLI 参考；运行 `cargo test --workspace`、`npm test`、前端构建；把 `m5-report.md` 第六节第 9 条状态更新为"已按链条补齐"。

## Phase 17: 增量 — 设置页与 Agent 页按原型归位（T109～T114）

来源：用户核对原型设置页指出实现缺内容；规格依据 FR-041（补位置）、FR-056（设置页分组与只读行）、FR-057（「全部 Agents」＝跨目标部署清单）；
规划见 `plan.md` 的"规划补齐：设置页与 Agent 页按原型归位"。

- [X] T109 [P] [US8] 在 `skill-manager/src/pages/SettingsPage.tsx` 与相关组件间完成 Agent 检测区块的迁移：把「支持的 Agent（已检测/内置总数）」摘要、全部启用／全部禁用／重新检测、已检测 Agent 卡片网格（拖动排序、启停、技能目录与项目相对路径行内编辑、恢复默认）与「其他支持的 Agent」折叠区**以可复用组件**接入设置页作为首个区块；不复制实现、不在两处同时渲染。
- [X] T110 [US8] 按原型区块顺序组织设置页：「支持的 Agent」→「全局配置」→「自己的 SkillManagerVault 同步」→「后台检查与上游更新」→「诊断与日志」；全局配置补只读展示行：部署方式（文件复制）、语言（简体中文）、关闭行为（托盘模式）；后台检查区补「立即检查」（复用既有检查与同步能力，**不新增组合命令**）；保留实现已有的自动上传确认、技能库容量、诊断与运行日志分区。
- [X] T111 [US3] 在 `skill-manager/src/App.tsx`、`src/pages/AgentPage.tsx` 把「全部 Agents」改为**跨目标部署清单**（FR-057）：标题「全部编码 Agents」、副标题「本机部署清单 · 同一个技能可部署到多个目标」，搜索 + 受管／未受管／本地有变更筛选 + 刷新，卡片按「技能 × 目标」列出并可发起从库部署；Agent 管理不再出现在该页。
- [X] T112 [P] [US8] 前端测试：设置页首区块为「支持的 Agent」且含摘要与三个动作；只读展示行存在且无可编辑控件；「全部 Agents」渲染的是跨目标部署清单而非 Agent 管理卡片；既有行为（共享目录未设置提示等）不回归。
- [X] T113 [US8] 执行 GUI 实操取证（从 `.app` 启动）：设置页顶部为 Agent 检测区块、只读行可见、启停与目录编辑可用；「全部 Agents」为部署清单；结果与未过项写入 `specs/002-skill-management/checklists/m6-report.md`（或新增 m7 报告）。
- [X] T114 收尾：同步 `skill-manager/README.md` 的页面说明；运行 `cargo test --workspace`、`npm test`、构建；把 `visual-comparison.md` 的 B6/B7 与第二节 19 行结论更新为已对齐，并记录刻意未做的三项（打开文件夹、托盘开关、更改目录）。

## Dependencies & Execution Order（依赖与执行顺序）

### 故事依赖图

```text
Setup → Foundational → US1（库维护）
                         ├─→ US3（部署） ─┬─→ US2（获取） → US4（上游）
                         │               ├─→ US6（Hermes）
                         │               ├─→ US7（预设）
                         │               └─→ US5（库同步）
                         └─→ US8（Agent/指令；Agent 部分 M2，指令部分 M4）
Foundational → US9（工具资料/检查；候选入库依赖 US2，周期依赖 US5）
Foundational → US10（环境；托盘检查依赖 US4/US5，容量依赖 US1）
各故事实现 → 配套 Skill 实际入库/更新 → US9 任务路径验收 + 双入口核对与完整验收
```

- US3 使用 Foundational 的目标基础，不等待 US8 指令功能；M2 验收时补齐 US8 的 Agent 管理界面与 CLI。
- US2 收编依赖 US3 接管接口；US6 收编也使用该接口和 US1 文件能力，不依赖 US2 上游获取。
- US4 依赖 US2 获取/发现和 US3 部署清理；只有主动应用更新后才同步已有部署。
- US5 的库同步可用预置资产独立验证；三类定时任务依次在 US4、US5、US9 完成时接入，不把未建服务报告为检查成功。
- US7 功能依赖 US1/US3，跨机验收依赖 US5；US8 Agent 管理依赖基础，指令源读写依赖 US1，跨机验收依赖 US5。
- US9 工具清单/检查/回写本身不依赖技能部署；任务路径成功解析的最终验收依赖普通维护 Skill 入库，缺失前只能显示未入库。
- 各故事自动测试可先并行编写；未明确落定的协议以所属设计任务为准，再调整测试中的协议夹具。测试必须先确认失败，再运行相应实现后的通过验证。
- 新增任何保护、检查或恢复分支仍需遵守宪章 VI，指出所防止的不可接受后果；不能借“收尾”加入新机制。

### 按规划执行里程碑

| 里程碑 | 执行范围 | 收尾证据 |
|---|---|---|
| M1 底座 | Phase 1/2 + US1；复用现有 Git，不重复初始化 | `checklists/m1-report.md`；获取入口同名、部署位置等依赖项明确待后续回验 |
| M2 部署 | US3 + US6 + US8 的 Agent 配置/本地技能管理任务与页面部分 | `checklists/m2-report.md`，SC-003；预设入口 M5 回验 |
| M3 获取/上游 | US2 + US4；补验 M1 各入口同名与译文原文变化 | `checklists/m3-report.md`，SC-007/008 |
| M4 同步/指令 | US5 + US8 指令部分/跨机验收 | `checklists/m4-report.md`，SC-005/006 |
| M5 预设/工具/环境 | US7 + US9 + US10 + 配套 Skill 与跨故事收尾 | `checklists/m5-report.md`，全部场景与 SC-001～008 |
| M5 补充（2026-10-16 规划补齐） | 本机运行侧双入口：`settings` 命令组、诊断/容量的 CLI 入口、两处无写入点配置的清理 | 更新 `checklists/m5-report.md` 的 5.14～5.19 与 `contracts/cli.md` T090 表中 `settings` 行 |
实际任务顺序（同批仍按各任务前置串行/并行）：

1. M1：T001～T020。
2. M2：T021～T029 → T067、T068、T071（Agent 管理）及 T056～T061（Hermes）；T061 在这些任务全部验证后收尾。
3. M3：T030～T049；T049 回验 M1 获取入口约束。
4. M4：T050～T055 → T069、T070、T072、T073（指令与双机验收）。
5. M5：T062～T066（预设）、T074～T081（工具核心）、T083～T087（环境）；T082 工具真实任务路径验收等待 T088～T089 配套 Skill 普通入库后完成，最终执行 T090～T092 整体核对。
6. M5 补充：T093、T094（测试先写并确认失败，两项可并行）→ T095、T096（`settings` 命令实现，两项都改同一文件，串行）→ T097（清理无写入点配置，与 T095/T096 不同文件可并行）→ T098（公共入口注册，串行）→ T099（场景验证与 T090 表更新）；T100 收尾同步文档。T095/T096 与 T100 都会改配套 Skill 的 CLI 参考，**串行合入**。
7. 共享目录浏览：T101（测试先行，确认失败）→ T102、T103（core 两处不同文件，可并行）→ T104（CLI 取值）→ T105（GUI 入口与页面）→ T106（前端测试，可与 T105 并行）→ T107（场景与 GUI 取证）→ T108（收尾与被遗漏项的链条回填）。T104 与 T108 都改配套 Skill 的 CLI 参考，**串行合入**。

阶段号和故事编号用于归属，以上里程碑用于真实开工顺序；US8 已将 M2 Agent 页面与 M4 指令页面拆为独立任务。每期报告使用 `场景号 | 预期 | 实际 | 结论（通过/未过+偏差说明）`，证据缺失保持未过，不把未实现功能当作成功。

### Parallel Opportunities（可并行项）

- 自动化测试中不同文件的 `[P]` 项可并行编写；故事核心和入口按依赖推进，公共入口注册串行。
- M2 的 US8 Agent 页面与 US3 核心可分工，但 `AgentPage.tsx` 在 US2 追加收编、US8 追加指令入口时串行。
- US9 资料/检查与 US7 预设可在共同底座完成后分工，先落定各自文件契约；共享 `commands/deploy.rs`、`commands/skill.rs` 或注册表的改动串行合入。
- US10 的主题、托盘、诊断与容量任务分别改不同主体文件；它们声明的业务依赖完成后可并行，再统一接通入口。
- 下表是每故事的并行例子，只有实际无依赖且不碰相同文件才并行；没有合适 `[P]` 任务的故事沿串行最短路径，不为并行而拆模块。

| 故事 | 并行执行例子与前置 |
|---|---|
| US1 | 底座完成后，`core/tests/vault_management.rs` 与 `tests/cli_library.rs` 两项测试并行编写；实现后统一运行 |
| US3 | 设计落定后，`core/tests/deployment.rs` 与 `tests/cli_deploy.rs` 并行编写；不能将 compose 与依赖它的 apply 并行宣称完成 |
| US2 | 候选/分类契约落定后，`tests/cli_acquire.rs` 可与 `upstream/fetch.rs` 实现分工，测试需先取得失败证据；获取与依赖它的入库串行 |
| US4 | 合并规则与来源版本契约明确后，`core/tests/upstream_merge.rs` 与 `tests/cli_upstream.rs` 并行；apply 等待 merge 和部署清理 |
| US5 | pull/status 与 push 在公共 Git/上传契约完成后可分工于不同文件；调度接入等待对应服务完成，GUI 接入不与入口注册抢写 |
| US6 | 协议核实后，`tests/cli_hermes.rs` 与不依赖实现细节的 `HermesPage.tsx` 展示可分工；启用/收编完成后才能验收 |
| US7 | 文件接口完成后，`preset/deploy.rs` 与 `PresetPage.tsx` 可分工；CLI deploy 注册串行整合，UI 等真实接口完成再验收 |
| US8 | M2 的 `deploy/agent_config.rs` 与 M4 的 `vault/instructions.rs` 属不同文件，可在对应前置就绪时分工；共享 CLI/Tauri 入口及 Agent 页面串行接入 |
| US9 | 资料/状态/检查契约完成后，`toolcenter/tasks.rs` 与 `ToolSkillCandidates.tsx` 可分工；候选调用等待 US2，真实任务路径等待维护 Skill 入库 |
| US10 | US5/US4/US1 相应前置完成后，主题、托盘、诊断、容量四项 `[P]` 可并行；最后统一接通 environment 命令与设置页 |

## Implementation Strategy（实现策略）

### 最小可验证范围

先完成 Setup + Foundational + US1，以已有 SkillManagerVault 验证文件真值、管理层读写、分类/标签/正文与外部刷新。它是开发检查点；获取、部署、同步、工具与桌面环境都完成并验收后才投入使用。

### 增量完成

按 M1 → M2 → M3 → M4 → M5 执行，每期运行对应 quickstart 并回验所属故事全部场景；后续故事补齐前期依赖时更新原报告，不复制一套现行验收规则。CLI 先贯通真实核心，GUI 接同一能力，不在前端模拟完成。

### 完成判定

所有任务实施完成且实际结果通过规格验收；完整旅程与两个独立本机状态的同步有证据，配套维护 Skill 从 Gitea 走普通入库/更新，全部核心 GUI 操作有 CLI 路径。无法执行的真实远端/桌面/Agent 验证要保留未过及具体原因，不静默降级。提交或发布按当时用户授权执行，任务文件不预设远端写入已经获准。

2026-10-16 部署交互后续修订：历史任务中 dry-run 验收已撤销，预演从全部运行路径移除。GUI 改为读取现有部署、切换选择后保存差量；卡片提供已检测且启用 Agent 的即时部署图标；界面常驻规则说明清理。当前定义见 spec.md FR-014/021/052，实施与验证见 checklists/m6-report.md 第十二节。


## Phase 18: Convergence

本轮按现行 spec.md、plan.md、全部 114 项既有任务及宪章检查；下列 F1～F15 对应本次检查发现。仅追加剩余工作，不将既有勾选或自动化测试通过等同于全部验收完成。

- [X] T115 [US4] [F1] CRITICAL 修复上游整技能删除的清理结果传播：`skill-manager/crates/core/src/upstream/apply.rs` 当前只检查 `SkillDeleteOutcome.failures`、丢弃 `paused`，导致副本清理暂停仍返回 `action=removed`、完整成功。保留“库已删除、该副本清理暂停”的技能、目标、实际路径与原因，经 CLI/Tauri/GUI 如实报告部分完成，CLI 使用非零退出码；在 `skill-manager/tests/cli_upstream.rs` 验证库条目已删除、外改副本及记录保留且结果不冒充全部清理 per Constitution I、FR-020/022/035、SC-004 (contradicts)
- [X] T116 [US5] [F2] CRITICAL 修复库拉取后附属操作失败的结果表达：`skill-manager/crates/cli/src/commands/sync.rs` 不能仅凭 `pulled/up_to_date` 返回完整成功；汇总 `deployment_errors` 与失败的 `instruction_targets`，保留库文件已拉取的事实并输出部分完成、非零退出码及逐项原因。同步修正 `skill-manager/crates/core/src/sync/pull.rs` 的摘要，以及 `skill-manager/src/pages/LibraryPage.tsx`、`skill-manager/src/pages/SettingsPage.tsx` 等 GUI 消费入口；在 `skill-manager/tests/cli_sync.rs` 验证外改副本暂停及指令目标写入失败，不丢失成功项或定位信息 per Constitution I、FR-020/037/043、SC-004、T051/054/055 (contradicts)
- [X] T117 [US5] [US9] [F3] HIGH 补齐统一检查的执行与可见结果：`skill-manager/src/pages/SettingsPage.tsx` 的“立即检查”须复用 `skill-manager/crates/core/src/scheduler.rs` 同时执行上游检查、库拉取、工具检查；接通 `skill-manager/src-tauri/src/lib.rs` 启动/定时及托盘调度结果到 GUI，让库内更新提示、库同步状态和工具检查状态能刷新并分别表达。调度汇总不得把仓库获取失败或附属操作未完成归为完整成功；保留各类失败原因，不新增 CLI 组合命令、不自动采用上游更新或后台推送。验证启动、手动、定时三条路径及关闭定时后仍可手动检查 per FR-029/037/046、US4/AC9、US5/AC8、T053/055 (partial)
- [X] T118 [US4] [F4] HIGH 在 `skill-manager/crates/core/src/upstream/apply.rs` 的仓库批量更新中，对实际更新成功的既有技能同步本机已有部署，并把暂停/失败结果传递到 CLI/Tauri；不能依赖 GUI 后续调用保存流程补做，新增入库仍不首次部署。向 `skill-manager/tests/cli_upstream.rs` 补充“仅调用 update-repo 后库与已有部署均为新内容、其他条目与新增目标不变”的行为验证 per FR-019/031、US3/AC7、T045 (missing)
- [X] T119 [US4] [F5] HIGH 在 `skill-manager/crates/core/src/upstream/apply.rs` 的单技能更新中，依据实际采用版本跳过同一提交的重复应用，返回真实的 unchanged 结果，避免再次覆盖用户随后维护的正文、注释等资料；保留明确采用尚未采用的跳过版本的能力。向 `skill-manager/tests/cli_upstream.rs` 补充“采用 v2 → 本地维护注释 → 再次 update v2，维护成果与部署均不被重复改写”的验证 per FR-030、US4/AC3、T034/044/045 (contradicts)
- [X] T120 [US3] [F6] HIGH 修复 `skill-manager/crates/core/src/deploy/apply.rs`、`skill-manager/crates/core/src/deploy/sync.rs`、`skill-manager/crates/core/src/deploy/remove.rs` 批处理在单项文件错误时整体提前返回的问题：把单项失败纳入既有逐项报告、保留已完成结果，并继续后续可处理的技能/目标；覆盖部署、同步、撤销和清理的相同处理边界，不引入整批事务或回滚。向 `skill-manager/tests/cli_deploy.rs` 补充首目标实际写入失败、后目标可正常完成的验证，防止一个失效目录阻断整批有效目标 per spec: Edge Cases“批量部分完成”、FR-021、T026/027 (partial)
- [X] T121 [US5] [US7] [F7] HIGH 在 `skill-manager/src/pages/PresetPage.tsx` 的预设新建/删除/重命名/图标/成员及两种排序保存点，以及 `skill-manager/src/pages/AgentPage.tsx` 的跨域复制成功点，接通既有 `onLibrarySaved` 保存后流程；核对其余 GUI 共享资产写入点有无同类遗漏。沿用现有上传确认和失败重试，不让文件监听或纯本机状态变更触发上传；组合编辑仍不新增部署或撤销。验证已授权保存会上传、未授权须先确认、外部改动不上传 per FR-023/025/038、US5/AC1、T054/065 (partial)
- [X] T122 [US3] [F8] HIGH 在 `skill-manager/src/components/DeployDialog.tsx`、`skill-manager/src/components/QuickDeploy.tsx`、`skill-manager/src/components/DeploymentStatus.tsx` 及其库/预设调用处，补齐技能 Agent/OS 支持声明对新增部署选项的限制；当前只按目标检测与启用筛选、提交后才由 core 拒绝。沿用 core 准入结果与既有支持资料，不另建一套业务规则；保留不再受支持的已有部署及警告，并允许撤销，多技能/预设按条目过滤。验证“仅支持 Codex”不会提供 ZCode 的新增部署操作 per FR-007/017、US3/AC2～4、T029 (partial)
- [X] T123 [US2] [F9] HIGH 完成 `specs/002-skill-management/quickstart.md` 的大仓库真实 GUI 获取/取消验收：从实际 `.app` 观察真实阶段和进度，点击取消后确认本次获取停止、不继续入库、不误报成功；同时记录成功及失败路径。将证据和实际限制写回既有 `specs/002-skill-management/checklists/m3-report.md`、`specs/002-skill-management/checklists/m5-report.md`；目前只有 CLI 大仓库证据，未取得 GUI 证据前不得将 SC-008 标为通过 per FR-013、SC-008、US2/AC4、T049/091 (partial)
- [X] T124 [US9] [F10] HIGH 完成配套维护 Skill 的真实普通上游交付链：依据 `skill-manager/assets/skills/skill-manager-maintenance/` 准备可审阅的项目源与技能交付，在获得所需远端发布授权后推送本项目 Gitea，再从该真实上游显式选择入库并登记来源，推进一次源版本验证普通 check/update，核实技能/工具维护任务引用的实际库内路径；补齐 `specs/002-skill-management/checklists/m5-report.md` 5.10 与相关现状记录。现有临时本地上游验证不能替代该项；无授权或远端不可达时保留未完成及具体原因，不另造分发机制 per FR-051、T088/089/092 (partial)
- [X] T125 [US6] [F11] MEDIUM 补齐 Hermes 档案技能的只读阅读链：`skill-manager/src/pages/HermesPage.tsx` 当前对未入库独有技能只打开收编确认，须能先阅读实际正文与文件，且查看本身不收编；将对应读取能力经 core、CLI 与 Tauri 接通，复用已有文件阅读能力。`skill-manager/src/components/HermesSkillDetail.tsx` 对库内技能的只读展示须跟随现行库内阅读视图（Markdown 结构、主题与吸顶阅读控件），保持只读及前往库内编辑，验证查看不会修改库、部署或启用名单 per FR-010/027、US6/AC3、T060 (partial)
- [X] T126 [US9] [F12] MEDIUM 在 `skill-manager/src/components/ToolSkillCandidates.tsx` 中，把来源预览前移到确认选择之前，并实际使用预览资料展示候选原名、用途、可用中文注释及来源；`skill-manager/crates/core/src/toolcenter/skills.rs` 当前仅从已入库技能取资料，待入库候选始终缺资料，确认时的 preview 结果又被丢弃。复用现有获取/候选服务，对来源失败和上游本身缺资料分别如实表达；确认选择后才入库且不自动部署 per FR-049、US9/AC6、T081 (partial)
- [X] T127 [US3] [F13] MEDIUM 修复 `skill-manager/src/App.tsx` 只向龙虾库传递 `libraryFocus` 的导航遗漏，使编码 Agent 与共享目录详情的“前往技能库管理”能在正确管理域打开对应技能详情；沿用现有 `focusReference/onFocusConsumed`，不增加导航状态体系 per FR-042/055、T105 (partial)
- [X] T128 [US10] [F14] MEDIUM 在既有诊断链补记手动获取失败和直接返回的部署文件错误：核对 `skill-manager/crates/cli/src/commands/acquire.rs`、`skill-manager/src-tauri/src/commands/acquire.rs`、部署入口与 `skill-manager/crates/core/src/diagnostics.rs` 的记录调用，确保诊断获取/导出包含实际操作、发生时间及可定位原因。实测 preview 来源不存在后日志为空，部署入口的提前返回错误也绕过现有逐项日志；复用现有脱敏与日志写入，不新增日志系统或自动上传 per FR-053、US10/AC3、T085 (partial)
- [X] T129 [US1] [F15] MEDIUM 在 `skill-manager/src/pages/LibraryPage.tsx` 补齐指定已部署 Agent 与所属预设的组合筛选入口，并通过 `skill-manager/src/services/client.ts` 传入既有 core 查询条件；当前只有“本机已部署/未部署”二分与预设归属文本，不能与分类/标签/搜索组合按指定目标或预设筛选。复用现有真实部署记录与预设定义，不另建索引或复制筛选业务 per FR-009/025、T019/029/065 (partial)
