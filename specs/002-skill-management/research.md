# Research: 设计决策记录与待调研项

**Date**: 2026-10-15 | **Status**: 讨论决策已确认；标注"实施时调研"的项已在对应里程碑查证（结果见 §四～§六 各 D 条目）。
实现已完成 M1～M5，实际结果与未闭合项见 [plan.md 的 Implementation Status](plan.md#implementation-statusm1m5-实际结果2026-10-15) 与各期报告。

依据：spec.md（含 Clarifications）、与用户的规划讨论（2026-10-15）、FeiSkillDock 旧项目调查（code-explore 子代理报告）。

## 一、已确认的设计决策

### D1 技能身份：管理域 + 技能名，不引入 ID

- **Decision**: 技能的唯一标识是"管理域（code/lobster）+ 技能名（目录名）"。不设数字 ID 或 UUID。
- **Rationale**: 同域禁同名（FR-004）保证唯一性；无改名功能、上游改名按删除+新增处理、改分类不改名，保证稳定性。所有引用场景（部署记录、档案启用、预设成员、工具配套关联）用名字字符串即可解析。旧项目的数字 ID 是数据库中心架构的产物（外键 + sidecar 文件名），并导致丢库后"stranded 技能"不可修复。
- **Alternatives**: UUID 存 `.manager`（被拒：复制/上游带 ID/两域同 ID 需要额外规则，纯增实体）；库内路径作身份（被拒：分类移动导致身份漂移）。

### D2 SQLite 仅存本机状态，共享资产零入库

- **Decision**: SQLite（rusqlite，WAL）只存本机状态：部署记录、Hermes 档案启用、Agent 本机配置、指令分发目标、应用设置、工具本机安装记录与独立检查结果、上游检查状态。共享资产（技能内容、`.manager`、预设、指令源、工具资料）全部是 SkillManagerVault 文件。不做任何 sidecar 镜像、不做 DB↔文件双向同步。
- **Rationale**: 旧项目 13 张表中 8 张是可重建镜像（含 sidecar 双写与 reindex 体系，是其复杂度大头）；DB 独占的只有本机状态。新项目真值全在文件，DB 丢了只影响本机（部署记录可重部署恢复）。保留 SQLite 而非纯文件的理由：GUI 与 CLI 并发写是真实场景（WAL + busy_timeout 旧项目已验证）；检查状态与诊断日志有追加/裁剪/按条查需求。
- **Alternatives**: 纯 JSON/TOML 状态文件（被拒：并发写需自建锁与合并，风险大于收益）；沿用旧项目 DB 中心 + sidecar 镜像（被拒：双重真值是旧项目病根）。

### D3 `.manager` 文件格式：字段即项

- **Decision**:

  ```text
  .manager/
  ├── meta.yaml        # tags: [..]; comment: 中文短注释; agents: [..]; oses: [..]
  │                    # agents/oses 缺省即所属管理域内全支持
  ├── translation.md   # 中文译文（整份为一项）
  ├── overlay/         # 覆盖层文件树：<agent>/<os>/<相对路径>；同路径文件为一项
  └── source.yaml      # 来源记录：仓库地址、上游子路径、采用版本、登记时间
  ```

  "同项"粒度 = `meta.yaml` 的**每个字段**、`translation.md` 整份、`overlay/` 内同 Agent×系统同路径的文件。上游更新时按此粒度执行"上游提供同项则用上游值，否则保留本地"（FR-032）。
- **Rationale**: 六项管理能力与更新粒度一一对应，语义可解释。`source.yaml` 记录该技能的上游及采用版本，随 SkillManagerVault 共享；从 B 引入的技能始终通过来源记录检查/更新 B，各电脑拉取同一个 SkillManagerVault 不发生重新入库或来源重绑定。标签、注释、译文、支持声明和覆盖层按既定同项上游优先规则处理；本机技能采用版本由实际更新结果登记。
- **格式演进**: 新字段 = `meta.yaml` 新键，读取缺省、无迁移。**写回必须保留未知字段**（按原始键值合并，防跨版本静默丢字段）。
- **Alternatives**: 文件即项（每项一个文件，被拒：粒度过粗，上游改声明会连带覆盖本地标签）；全部塞一个 YAML 含译文（被拒：长文档与结构化数据混放，diff 噪声大）。

### D4 性能模型：文件为真值 + 内存索引

- **Decision**: 启动时全量扫描一次，构建内存索引（名字/标签/分类/注释/支持声明/部署引用）；浏览、搜索、筛选全部查内存；详情页按需读单个技能文件；外部修改经 notify 文件监听增量重读。技能清单不落库。几十至几百技能为当前规模，实际耗时在验收记录；此前 <1s 与微秒级表述缺少确认依据，移除其承诺含义。
- **Rationale**: 旧项目标签倒排索引同样是内存重建（未用 DB）；此规模下 DB 镜像换不来可感知性能。宪章基线保留"必要时 SQLite 做缓存"的退路，不预建。
- **Concurrency**: GUI 与 CLI 是两个进程，CLI 写 `.manager` 后 GUI 靠文件监听刷新（FR-005/A11）。

### D5 上传语义：全量提交，不区分改动来源

- **Decision**: 软件执行上传（GUI 保存自动上传、Agent 获同意后的上传）时提交当前工作区**全部**改动并推送，不做路径级拆分、不跟踪改动归属。Agent 修改后主动询问的维护规范保留（Agent 行为约定），但混同上传的后果用户接受（少数情况、可改回，2026-10-15 裁决）。
- **Rationale**: 完全避免该场景需要"改动归属账本"，需求记录明确不预设；概率与后果均低。
- **Alternatives**: 按文件路径暂存提交（被拒：用户裁决不区分；同文件双改仍无法拆分）。

### D5a 拉取遇脏工作区：未重叠则拉，重叠则暂停问用户

- **Decision**（2026-11-09 修订，取代同日早先的「一律 stash 再 merge」）：未提交改动与远端未重叠时直接 merge，本机改动保留。与远端同一技能/正文路径重叠时**不开始合并**（`blocked_uncommitted`），工作区不动，维护 Skill 要求 Agent 询问用户先上传或丢弃，禁止自行 stash / checkout 覆盖。仅 `upstream-decisions.yaml` 按 `(repo, domain)` 并集。已提交之间的冲突仍按 FR-039。拉取路径不提交、不推送。
- **Rationale**: 一律 stash 会把未提交改动变成冲突标记，Agent 难以区分「还没问过用户的本地修改」和「已经进入 Git 合并的冲突」。重叠时应停下来问用户，而不是用 Git 技巧先拉下来。未重叠的脏文件 Git 本就可以 merge，不必暂存。账本文件是追加型，并集不会丢掉拒绝/基线。
- **Alternatives**: 一律 autostash（被拒：重叠时制造冲突现场，削弱掌控）；重叠时仍合并其它技能（被拒：同一次拉取部分成功、部分本地被远端盖住，Agent 更难解释）；拉取前自动提交（违反 FR-038）。

### D6 CLI 形状：领域子命令 + --json + 无交互

- **Decision**: 顶层子命令按领域：`skill` / `deploy` / `preset` / `hermes` / `tool` / `sync` / `agent` / `instruction`。全局 `--json` 输出结构化结果（Agent 解析）；默认人类可读。CLI 不提问不等待输入——需用户决策的动作由 Agent 对话确认后作为参数传入；检查只保存检查事实、不应用变更，应用类命令显式执行。
- **Rationale**: Agent 是 CLI 主要用户（需求记录裁决）；领域名与 GUI 导航一致。完整命令表与 FR-050 覆盖对照见 [contracts/cli.md](contracts/cli.md)。

### D7 里程碑标准：开发管理与方向检查点

- **Decision**: 全部完成后才投入使用；里程碑按依赖自底向上（M1 底座 → M2 部署 → M3 获取与上游 → M4 同步与指令 → M5 预设/工具/收尾），每期收尾跑 quickstart 对应场景并输出验收核对表供用户检查方向偏差。
- **Rationale**: 用户 2026-10-15 明确：里程碑服务于 Agent 开发节奏与用户阶段性检查，不服务于增量交付。

### D8 仓库操作：调用本机 git 命令

- **Decision**: 不使用 libgit2/git2.rs，仓库操作（clone/fetch/commit/push/status）通过调用本机 `git` 命令执行，解析 `--progress` 进度输出。
- **Rationale**: 需求记录既有裁决（不内置 Git 引擎、复用本机 Git 与认证）；旧项目同方案。进度与取消依赖命令子进程的流式输出与终止（SC-008 验收项）。

### D9 配套维护 Skill：单一技能、上游入库、任务书引用

- **Decision**: 配套维护 Skill 为**单一技能**（库维护规范 + CLI 使用 + 工具安装登记流程合一，用户裁决 2026-10-15：正文长不是问题，Agent 准确完成任务优先，不行再拆）。技能源文件维护在本项目仓库 `skill-manager/assets/skills/skill-manager-maintenance/` 中；本项目推送用户 Gitea 远端后，该技能从本仓库作为**普通上游技能**入库（`skill import` 登记来源），更新完全走既有上游检查/更新流程——零新增机制，同时化解"同域同名阻止"与技能更新的冲突（上游更新规则而非覆盖）。软件生成的维护任务说明（技能维护任务、工具任务）包含该技能在库内的路径与"按其规范执行"的指引，不内联规范全文。
- **Rationale**: 规范单一真值——细节只维护在技能里，任务书自动跟随技能更新；入库/更新复用全部既有规则（FR-051"不新增专用分发机制"）。
- **前置**: 2026-10-15 质检时本项目已初始化 Git，当前 `main`，Gitea `origin` 已配置；不重复初始化。M5 需使技能源版本在 Gitea 实际可获取，再做普通入库/更新验收；配置远端不等于技能已经发布或已经验收。
- **实施结果（2026-10-15）**：技能源文件已按本决定写在 `skill-manager/assets/skills/skill-manager-maintenance/`（`SKILL.md` + `references/cli.md`，其中出现的每条命令都在临时环境实际执行过）。
  `quickstart.md` 已固定来源子路径 `skill-manager/assets/skills/skill-manager-maintenance` 与实际命令。
  **Gitea 路径的入库/更新保持"待授权"**：本项目 `origin` 指向用户 Gitea，但提交与推送未获用户授权，未执行。
  为验证**机制本身**，已用临时本地 Git 仓库（内容复制自本项目该目录）作为普通上游完成
  `preview → import（登记来源）→ 推进上游新版本 → check（只展示不应用）→ update（显式采用）`，
  来源记录与采用版本推进正确，任务书可解析到技能的真实库内路径；这**不是** Gitea 发布验收。
- **真实交付补验（2026-10-17，T124）**：用户明确授权提交并推送后，本项目已发布到真实 Gitea `origin/main`。
  在隔离库从 `c00e1a4` 显式选择该子路径入库，再发布本轮 `e2106f9`，普通 `check` 只提示
  `references/cli.md` 变化、不改变采用版本；`update` 显式采用后来源版本推进且实际文件更新。
  技能/工具维护任务引用实际存在的库内路径，部署记录为空。此前“待授权”状态已闭合，证据见 `checklists/m5-report.md` 第十二节。
- **Alternatives**: 拆多个技能（被拒：触发场景合并可行时优先单一，用户裁决）；任务书内联维护约定（被拒：双份维护，skill 更新后任务书过期）；应用内专用更新机制（被拒：FR-051 裁决不新增）。

## 二、实施时调研项（对应里程碑开工时查证）

| 项 | 查证来源 | 关联 |
|---|---|---|
| 中文注释在 SKILL.md 的注入位置与格式 | 技术参考索引（旧版部署实现）+ 旧源码 `FeiSkillDock` | M2 部署合成 |
| Hermes 档案启用的外部协议（黑名单/挂载线索） | Hermes 官方文档 + 技术参考 + 旧源码 | M2 档案启用 |
| 53 个内置 Agent 目录资产与检测条件 | 技术参考（Agent 目录 53 项） | M2 检测 |
| 共享目录路径与通用产物判定实现 | `docs/requirements.md` 已确认：仅编码域，默认编码 Agent 读取该目录，全部消费者需受支持且内容相同；不因有 overlay 目录机械拒绝 | M2 T021 → **§四 D12** |
| git clone/fetch 进度解析与取消的稳定实现 | 旧源码 `git_fetcher.rs` | M3 |
| 上游仓库获取的缓存策略（浅克隆目录、复用与清理） | 旧源码 `<base>/cache` 机制 | M3 |
| 受管副本变更检测与用户裁决后的恢复参数 | 本机部署记录为受管依据；增加实际目标路径与产物基线，定义见 data-model.md §2.1，不增加目录内标记 | M2 T021 → **§四 D13** |
| 指令拼接与目标路径的实际接入 | 源目录已定为 `instructions/global.md + <agent>.md`；沿用单编辑区/单保存按钮，不增加独立分发勾选；用户已确认拉取源变化后自动刷新本机已有启用目标，不改启用配置 | M4 |
| 主题/托盘/诊断的 Tauri 2 实现细节 | Tauri 2 文档 + 旧源码 | M5 |

**查证结论（2026-10-15）**：上表各项均已在对应里程碑开工时查证并落定，结果即 §四 D10～D14（注释注入与覆盖层顺序、
共享目录判定、产物指纹与本地变更检测、部署边界）、§五 D15～D18 与 Hermes 协议核实、§六 D19～D21（获取缓存/进度/取消、
候选发现、检查与拒绝跳过基线）。Tauri 2 的主题/托盘/诊断按官方 API 实现并在 macOS 实际运行验证；
其余平台未验证，不声明兼容。

## 三、被拒绝的复杂度（回溯清单）

- 技能 ID/UUID 体系（D1）；改动归属账本与路径级提交（D5）；DB sidecar 镜像与 reindex（D2）；数据库 schema migration 链（文件格式无迁移，本机状态表首版即定，后续如需变更属常规开发）；自动工具发现/扫描（规格 FR-047 裁决）；按来源类型的版本解析器（FR-046 裁决：检查命令由 Agent 编写）。

## 四、M2 部署链路设计（T021 落定）

依据：[部署链路技术参考](../../docs/technical-reference/deployment-pipeline.md)、
[内容合成参考](../../docs/technical-reference/content-composition.md)、
[管理层资料参考](../../docs/technical-reference/manager-layer-data-model.md)；
需求来源：spec.md FR-006/FR-007/FR-008/FR-016～FR-022 与 US3 的 12 个验收场景。
参考中标注“旧实现”的部分只用于判断可复用性，冲突点按下文明确不采用。

### D10 中文注释注入（FR-006）

- **Decision**：注释在**部署时**注入合成产物，不回写库内原始正文。注入对象是技能入口文件
  （`SKILL.md` 优先，其次 `skill.md`，与扫描谓词一致），位置是 frontmatter 的 `description`：
  文本形态 `<中文注释>｜<原 description>`，分隔符为全角竖线 `｜`（U+FF5C，与
  `content-composition.md` 的 `图表生成｜Create diagrams` 案例一致）。
  - 原 `description` 缺失或为 null：写为 `<中文注释>`。
  - 入口文件没有 frontmatter：在文件头新增一个最小 frontmatter 块承载 `description`，
    正文内容保持在块之后（保证注释一定进入产物）。
  - `description` 是非字符串形状（列表/映射）：不注入并如实记一条提示，不损坏元数据。
  - 注释为空：不注入，`description` 保持原文。
  - 产物中的 frontmatter 由 YAML 映射重排后写回：保留键顺序与未知键，但不保留其中的 YAML 注释；
    这只发生在**部署产物**，库内原件逐字未动。
- **重复注入与清除**：合成输入永远是**库内原始文件 + 覆盖层**，从不以已注入的产物为输入，
  因此重复部署不会堆叠前缀。附加保护：若当前 `description` 已以 `<注释>｜` 开头则不再次前缀，
  用于覆盖层直接提供了“已带管理器前缀”的 `SKILL.md` 的情形。清空注释即改库内 `comment`，
  下次合成不再产生前缀，无需额外的“剥离”逻辑。
- **不做**：不把中文译文（`translation.md`）当作部署正文（FR-006）；不写“管理层标记”到产物。
- **依据**：`content-composition.md` 第 7～29 行（旧前缀→新前缀变换可参考）、
  `deployment-pipeline.md` 第 128～135 行（旧版安装期写库与本项目冲突，不采用）。

### D11 覆盖层维度省略、优先级与整份替换（FR-008、Edge Case 覆盖层整份替换）

- **布局**（M1 已定）：`overlay/[<agent>|_]/[<os>|_]/<相对路径>`，`_` 表示该维度通用。
- **Decision（本阶段落定）**：某次部署对某个相对路径只产出一个文件（**同路径文件一项**）。
  候选按“具体优先于通用”生效，应用顺序（后者覆盖前者）为：
  1. `_/_`（两维通用）
  2. `_/<os>`（仅系统）
  3. `<agent>/_`（仅 Agent）
  4. `<agent>/<os>`（Agent + 系统）
  即“具体维度数量多者优先”；同数量时 **Agent 维度优先于 OS 维度**（唯一确定的裁决，
  避免两个单项在无 `<agent>/<os>` 候选时的非确定性）。冲突不合并，胜者整份文件生效。
- **整份替换**：覆盖层允许出现入口文件 `SKILL.md`/`skill.md`，整份替换原文，**不自动合并**；
  上游新正文不进入替换结果，由操作者承担（Edge Case 明确）。旧校验器拒绝 overlay 含
  `SKILL.md` 的行为**不采用**（`content-composition.md` 第 37 行、`deployment-pipeline.md` 第 124 行）。
- **同名文件/目录**：若覆盖层新增文件 `x` 而基础树已有目录 `x/`（或反之），基础树中与覆盖层
  路径冲突的条目被移除，保证产物是一棵可写出的合法目录树；这是覆盖层“整份覆盖”的直接推论。
- **依据**：需求“同路径文件为一项”；`content-composition.md` 第 31～42 行案例清单。

### D12 共享目录通用产物判定（FR-021）

- **Decision**：`shared` 是**部署目标**，不是第二个管理库，只接受编码域技能。
  - 消费者受支持：按“编码 Agent 均读取共享目录”的既有产品假设处理，不增加逐 Agent
    读取声明。因此要求技能的 `agents` **未声明**（= 所属域内全支持）；`agents` 为显式
    名单或显式空集合时，表示“并非所有消费者都受支持”，禁止**新增**共享部署
    （已有部署按 D13 保留并警告）。
  - 内容通用：共享部署只应用**通用 Agent 维度**（`_`），当前系统的 OS 维度照常生效
    （同一台机器上所有消费者看到同一份内容）。**不因技能存在 overlay 目录机械拒绝**，
    也不自动把 Agent 专属覆盖层应用到共享产物；被应用的覆盖层在结果中逐项列出，便于核对。
  - 路径来自设置中的共享目录（未设置时如实报错，不猜测默认路径）。
- **依据**：`requirements.md` 第 198～200 行（仅编码域、所有消费者受支持且内容相同、
  不按 overlay 目录机械拒绝、不设读取声明）；`deployment-pipeline.md` 第 256～269 行
  的旧 `vault_tool_allowed`（其“有 overlay 即拒绝”部分明确不采用）。

### D13 产物指纹与本地变更检测（FR-019/FR-020/FR-022）

两个指纹都用 SHA-256 十六进制，输入是“排序后的相对路径 + 文件字节”的确定性序列
（排除 `.git` 与 `.DS_Store`；不跟随符号链接）：

- **`source_hash`（库内容指纹，部署时记录）**：合成**输入**的指纹 = 基础树（排除
  `.manager`/`.git`/`.DS_Store`）+ 本目标生效的覆盖层条目 + 库内 `comment`。库内正文、
  覆盖层或注释任一变化都会改变它。
- **`deployed_hash`（产物基线）**：**最近成功部署的合成产物**指纹（已含注释注入与覆盖层）。
  用于识别目标副本的外部变更：实际副本指纹与它不等即视为被外部修改。

- **判定**：`deploy sync` 先读取记录 `target_path` 的当前内容指纹——与 `deployed_hash`
  相等则“受管未外改”，按需重写；不等则该项 `paused_local_change`，**不覆盖**，其余项继续。
  副本缺失不算外部变更，按记录重建。用户经 Agent 裁决后以
  `deploy sync --ref <ref> --target <target> --overwrite-local` 显式按库覆盖并恢复 `ok`。
- **不用 mtime 判向**：`deployment-pipeline.md` 第 409～411 行明确指出不能只比内容再靠 mtime
  断言“用户改过”；本设计按“实际副本 vs 上次产物基线”判断，不引入 ±1000ms 阈值。
- **受管身份**：以本机 `deployments` 行为唯一依据，**不向技能产物写受管标记文件**；
  数据库丢失时按未受管处理，可由用户显式部署重新接管（data-model §2.1）。

### D14 部署内容边界与写入方式（FR-016、FR-018、T024～T027）

- **进入产物的内容**：基础树去掉 `.manager`（含 `translation.md`、`overlay`、`source.yaml`、
  `meta.yaml`）与 `.git`，叠加生效覆盖层，再注入注释。**不部署**中文译文、`.manager`、
  `.git`、`.DS_Store`、符号链接。
- **落盘**：先在同父目录的临时目录内写全量产物，再整体替换目标副本（先移除旧副本再
  rename 临时目录到位），避免“写到一半留下半个技能目录”；不保留备份、不加目录标记文件。
- **未受管同名**：直接覆盖并登记为受管（FR-018，不做同名冲突检查）；旧版 `NoClobber` 拒绝
  覆盖的行为明确不采用（`deployment-pipeline.md` 第 386 行）。
- **只做独立副本**：不提供 symlink 部署模式，也不做 Windows junction 回退
  （`deployment-pipeline.md` 第 385 行）。
- **暂停与删除保留**：暂停副本保留记录与 `target_path`；库内条目删除而副本暂停时保留该行，
  报告“库已删除、副本清理暂停”；**不新增待删任务队列、备份、合并界面或恢复队列**。
- **依据**：`deployment-pipeline.md` 第四、五、六、十二、十三节；`content-composition.md`
  第 65～73 行（同目录临时文件 + rename 的原子写）。

## 五、M2 Agent 配置与本地技能管理（T067/T068/T029 落定）

依据：[本机运行时与桌面外壳参考](../../docs/technical-reference/local-runtime-and-shell.md)、
spec.md FR-041/FR-042、US8 场景 1～3；data-model §2.1/§2.3。

### D15 修改技能目录后已有部署的协调（T067）

- **Decision**：不引入额外的协调机制。部署解析（`TargetResolver`）按本机配置的覆盖目录优先，
  因此：
  - `deploy to` 使用**当前**目录写入，并用当前解析路径更新记录的 `target_path`（与
    data-model §2.1 一致）：技能随后落在新目录，旧目录的副本保留在原处，由用户自行处置；
  - `deploy sync` 按记录中的 `target_path` 定位并写入，**不会**因目录设置改变而迁移旧副本；
  - `deploy status` 逐条给出 `target_path`、`status` 与支持收窄警告，如实表达旧副本位置。
- **Rationale**：`local-runtime-and-shell.md` §1.5 的旧做法是“先删旧副本再按当前预设重建”，
  它依赖互斥的“当前预设”模型，新模型没有这一前提；静默删除用户目录下的副本会造成不可逆的
  内容丢失，属于不可接受的后果，因此不采用。选择“新部署写新路径、旧副本交用户处置”的后果是
  旧路径可能残留副本，但可见（`deploy status`/撤销均可定位）、可恢复、影响有限。
- **不新增**：不建目录迁移、备份、待删队列或路径重写逻辑；`project_rel_dir` 仅配置保留。

### D16 检测、启停与默认启用（T067）

- **Decision**：检测只调用 `AgentEntry::detected(home)`（纯路径存在性），不启动 Agent、不写状态。
  全量 53 项名单保留并区分“已检测/其他支持项”；启停、顺序、技能目录、项目相对路径与指令目标
  只写本机 SQLite `agents` 表（不进 SkillManagerVault）。
  **无本机记录时默认启用**（`enabled` 缺省为 true），与旧行为“新检测到的 Agent 默认启用”一致；
  用户显式停用后写入记录，重新检测不会恢复启用。
- **Rationale**：Agent 目标是部署前提，默认启用使首次接入即可看到可部署目标；显式停用状态持久化
  保证用户选择不被检测覆盖。若默认停用，界面初始为空、用户需逐项开启，反而增加操作负担。
- **目录形状校验**：`skills`/`instructions` 要求绝对路径（相对路径会随进程工作目录漂移，
  导致部署写到意料之外的位置）；`project-rel` 要求相对路径、禁止 `..`，空值等于清除
  （沿用 `local-runtime-and-shell.md` §1.6 的归一化规则，不搬旧代码结构）。

### D17 本地技能删除的受管判定与路径包含性（T068）

- **Decision**：受管判定以本机部署记录与实际路径为依据——存在 `target == agent:<key>` 且
  `target_path`（规范化后）等于候选目录的部署记录即为受管；该判定同时供 US2 收编复用。
  未受管：删除实际文件，不改 SkillManagerVault；受管：调用 US3 的撤销入口（保留库内技能），
  副本被外部修改时按暂停规则拒绝并如实报告。
- **路径包含性校验**：名字来自用户输入，删除前校验候选项规范化后必须严格落在该 Agent 的
  `skills_dir` 内（且不等于目录本身）。防止的具体后果：符号链接或构造路径导致删除技能目录之外
  的无关文件，该后果不可逆且不可接受。
- **不新增**：不建待删队列、不加目录标记文件、不把删除本机副本变为删库。

### D18 GUI 部署结果的分类展示与共享目录设置入口（T029）

- **Decision**：批量部署/撤销结果逐项展示；未完成项按“暂停（message 含“暂停”）”与
  “过滤/不适用”两类分开展示，其余继续，不做整批回滚。暂停项只提示并可复制定位信息交给 Agent，
  **不提供差异合并界面**。部署目标列表来自 `AgentCatalog` 真实数据与 `AgentEntry::detected`。
- **共享目录路径（2026-10-16 修订）**：按系统当前用户主目录自动定位 `.agents/skills`，无需配置。
  `shared_dir_get` 与 `settings get shared_dir` 同源只读；不保存路径设置，也不提供 GUI、CLI 或 Tauri 写入口。
  浏览不存在的目录返回空列表且不创建，首次部署沿用现有目录创建逻辑。
- **GUI 保存后自动刷新部署**：保存注释/支持声明/overlay（影响合成结果）后调用 core 的
  `deploy sync`（等价 `deploySync()`），不触发上传、不新增目标；暂停项在结果中如实表达。

## 六、M2/US6 Hermes 接入核实（T056，2026-10-15）

本节是对**本机真实 Hermes 安装**的只读核实结论，证据来源逐条标注（文件 + 行号，或命令输出）。
与 `docs/technical-reference/hermes-integration.md`（明确标注“2026-10 参考观察、未运行验证”）冲突处以本节为准。
`hermes-agent` 源码路径为 `~/.hermes/hermes-agent/`；本次未启动 Hermes、未写入任何真实 Hermes 路径。

### 5.1 根目录解析与 default

- **证据**：`hermes_constants.py:112-124` `get_hermes_home()`：上下文覆盖 → 环境变量 `HERMES_HOME` → 平台默认；
  `hermes_constants.py:50-57` `_get_platform_default_hermes_home()`：非 Windows 为 `Path.home()/".hermes"`（可带 `HERMES_DATA_DIR_SUFFIX`），Windows 为 `%LOCALAPPDATA%/hermes`。
- **证据**：`hermes_constants.py:280-300` `profile_name_for_home()` 文档串明确：“The default home is the Hermes root itself … not the profile id `default`”。
- **结论**：**default 档案就是根目录自身**，不存在 `profiles/default`。本项目单根定位：按 `HERMES_HOME` 环境变量与平台默认解析（2026-10-16 清理 T097：原先规划的 `hermes_root` 可写设置键已移除，见 D23 与 `data-model.md` §2.6；`hermes:default` 部署路径与 `HermesService` 共用该解析）。
- **证据**：`hermes_constants.py:216-232` `get_default_hermes_root()`：`HERMES_HOME` 指向 `<root>/profiles/<name>` 时回退到 `<root>`（`env_path.parent.parent`）。因此“进程 home”与“含全部档案的根”需要分开，本项目只使用**根**。

### 5.2 具名档案发现（不能凭任意子目录认作档案）

- **证据**：`hermes_cli/profiles.py:220-226` `_get_profiles_root()`：档案根固定为 `<hermes-root>/profiles`（锚定根，不是当前 `HERMES_HOME`）。
- **证据**：`hermes_constants.py:314-327` `_PROFILE_IDENTITY_MARKERS = ("config.yaml", ".env", "SOUL.md", "profile.yaml", "auth.json", "state.db")`；`named_profile_has_identity()` = 目录含任一身份标记（文件或符号链接）。
- **证据**：`hermes_constants.py:320` `PROFILE_ID_RE = ^[a-z0-9][a-z0-9_-]{0,63}$`（小写字母/数字/`-`/`_`；中文档案名以 URL 编码形式落盘）。
- **证据**：`hermes_cli/profiles.py:396-412` `_iter_named_profile_dirs()`：条件为 `is_dir()` ∧ 名 ≠ `default` ∧ 匹配 `PROFILE_ID_RE` ∧ `named_profile_has_identity()` ∧ 非墓碑。
- **证据**：`hermes_constants.py:237` `_DELETED_PROFILES_DIR = ".deleted"`；`307-311` `profile_tombstone_path` = `profiles/.deleted/<name>`；`356-360` `named_profile_is_live` = 目录存在 ∧ 有身份标记 ∧ **无墓碑**。
- **结论**：本项目发现规则 = `<root>/profiles/<名>`，名匹配上述正则、目录含身份标记、且 `profiles/.deleted/<名>` 不存在；否则不列为档案（cron/日志等副作用产生的无标记目录不会被误认）。
- **本机实例核对**（只读 `ls`）：`~/.hermes/profiles/` 下为 `blank`、`u7ed8-u753b-u52a9-u624b`（“绘画助手”的 URL 编码）与 `.deleted/`（内含 `blankdemo`、`narrowtest`、`narrowtest2`、`research`）。与上述规则一致。

### 5.3 技能目录

- **证据**：`hermes_constants.py:1195-1197` `get_skills_dir()` = `get_hermes_home() / "skills"`。故 default 的技能目录是 `<root>/skills`；具名档案为 `<root>/profiles/<名>/skills`。
- **证据**：`agent/skill_utils.py:418-426` `get_all_skills_dirs()` = 本 profile 的 `skills/` + `skills.create_dir` + `skills.external_dirs`；**不含根 `skills/`**。具名档案是独立岛，不天然继承 default 的技能目录。
- **证据**：`agent/skill_utils.py:781-800` `iter_skill_index_files()` 用 `os.walk(skills_dir, followlinks=True)` 找 `SKILL.md`，即**跟随符号链接**。
- **结论**：
  - `hermes:default` 部署应写到 `<root>/skills/<name>`（现有 `TargetResolver` 的 `hermes` 目录 `.hermes/skills` 已一致）。
  - 具名档案“可用某技能”= 该技能目录在其 `skills/` 下可被 `SKILL.md` 遍历到；根 `skills/` 对它不可见，因此**新增到 default 的技能不会自动出现在具名档案**。

### 5.4 外部启用协议

- **证据**：`agent/skill_utils.py:298-310` `get_disabled_skill_names()`：从该 profile `config.yaml` 的 `skills.disabled`（∪ `skills.platform_disabled[<平台>]`）读**禁用名单**，再减去 `ESSENTIAL_SKILLS`（不可禁用，如 `hermes-agent`）。**不是白名单**。
- **证据**：`hermes_cli/skills_config.py:25-46 / 49-58` `get_disabled_skills` / `save_disabled_skills`：启用/禁用即“从 `disabled` 集合移除/加入技能名”；`web_routers/skills.py:343-387` GET `/api/skills` 的 `enabled = name not in disabled`、PUT `/api/skills/toggle` 同一语义。
- **证据**：`agent/skill_utils.py:357-384` `get_external_skills_dirs()`：`skills.external_dirs` 挂载**整个目录**（只读，本地优先）；`hermes-integration.md` 亦记录该点。
- **证据（多种配置形态）**：`hermes_cli/skills_config.py:313-333` `parse_config_string_list` / `_normalize_string_set`：`disabled` 可能是 YAML 列表、JSON/Python 风格列表字符串或单个标量字符串，`null` 视为空。
- **结论（本项目协议）**：
  - 具名档案的“明确启用”= 在该档案 `skills/` 下建立指向 **default 共享副本** 的**符号链接** `<profile>/skills/<名>` → `<root>/skills/<名>`；同时确认该名字**不在**该档案的 `disabled` 名单内。禁用 = 移除该链接（内容从该档案消失，Hermes 不再加载）。
  - **不采用** `external_dirs` 整目录挂载：它会让以后新增到 default 的技能对该档案**自动可见**，与“新增不自动启用其他档案”（FR-026/US6-1）冲突。
  - 符号链接使“所有档案共用库内一份技能内容”（US6、FR-026）成立；`followlinks=True` 保证 Hermes 能发现链接技能；内容更新发生在共享副本上，档案启用选择不重置（US6-2）。
  - **不写用户 `config.yaml`**：启用/禁用不依赖写 `disabled`（可见性由链接表达）；而本机 `config.yaml` 为 12 万字节级、含大量注释，`hermes_cli/config.py:2509-2550` `save_config()` 用 ruamel 保注释 + 原子替换 + **读失败即拒绝写**（`require_readable_config_before_write`/`_refuse_failed_read`）。用不支持注释保留的 YAML 库整份重写会破坏用户其他设置与注释，后果不可接受（宪章 VI）。故本项目**只读** `disabled` 名单用于状态判定与诚实报告；若该名字在档案 `disabled` 中被 Hermes 显式禁用，启用会**如实报告并阻止**（不伪造成功），不擅自改写用户配置。
  - **读失败与不存在区分**：`config.yaml` 不存在 = 无名单（合法空）；存在但读不了 = 报错并在结果中标注“档案配置读取失败，启用状态不作有效结论”，不折叠成“全未禁用”。

### 5.5 本机实例的只读观察（作为夹具对照）

- 根 `~/.hermes/config.yaml` 的 `skills:` 段（grep）：`disabled:` 为 null（空名单），`external_dirs` 被注释掉（未启用整目录挂载）。与 5.4 结论一致。
- 具名档案 `blank`、`u7ed8-u753b-u52a9-u624b` 各有自己的 `config.yaml`（`blank` 无 `skills:` 段）与 `skills/`（`blank` 下仅 `.bundled_manifest`/`.curator_state`/`.hub`，无用户技能）。
- CLI 入口 `/Users/fei/.local/bin/hermes` 存在；本任务未运行其任何会修改用户状态的命令。

### 5.6 未核实 / 未运行验证（如实标注）

- **未运行验证**：本次未启动 Hermes，未构造真实档案并让 Hermes 实际加载一个符号链接技能；5.4 的符号链接可见性结论来自源码 `iter_skill_index_files`（`followlinks=True`）与 `get_all_skills_dirs` 的阅读，属**源码级核实**，非端到端运行验证。
- **未核实**：Hermes 其它入口（WebUI/`hermes skills` TUI）对“符号链接技能 + 未被 `disabled` 列出”的展示是否与 CLI 加载一致；不影响本项目的启用/禁用语义（我们以链接与自身名单为准）。
- **未核实**：Windows 上符号链接权限/联结（junction）行为；`hermes_cli/profiles.py:1200-1215` 显示 clone 会保留 junction。本项目在 Windows 上尝试 `symlink_dir`，失败时如实报错，不静默降级。
- **影响**：若真实 Hermes 版本不跟随符号链接（与本次源码阅读不符），则具名档案启用不会生效；本项目在结果表达上不会声称“已生效”，仅报告已建立链接与自身名单变更，界面注明依据。

## 七、M3 获取与上游更新设计（T030/T039 落定）

依据：[获取与发现参考](../../docs/technical-reference/git-and-discovery.md)、[上游更新参考](../../docs/technical-reference/upstream-updates.md)；
需求来源：spec.md FR-011～FR-015、FR-029～FR-035，US2/US4 全部场景，data-model §1.3/§1.6。
参考中标注“旧实现”的部分只用于判断可复用性，冲突点按下文明确不采用。

### D19 上游获取缓存、进度与取消（T030）

- **缓存位置与复用**：缓存目录 `<state-dir>/upstream-cache/<repo_key 的 SHA-256 前 16 位>/`，位于本机状态目录内，
  随 `--state-dir` 隔离（两台电脑模拟互不干扰）。采用**浅克隆**（`--depth 1`）。已存在且 `rev-parse HEAD`
  可用时复用：`git fetch --progress --depth 1 origin <ref|HEAD>` 后 `git reset --hard FETCH_HEAD`；
  指定 `--ref` 时首次克隆用 `--branch`。取消或失败留下的半成品（不可复用）在下次先清理再重克隆。
- **边界**：只承诺“可复用的获取缓存”，**不承诺清除所有临时字节**（取消后可能残留半成品，下次获取会先清理）。
  获取失败或取消**不把技能登记为已入库**（登记只发生在 `vault/acquire.rs`）。
- **进度**：调用本机 Git 的 `--progress`；stderr 按 `\r` 与 `\n` **双分隔**读取（终端进度常在同一行刷新）。
  解析 `<阶段>: NN% (a/b)`；未知总量时 `percent`/`counts` 为 `None`、保留原始 `raw`，
  **不虚构百分比、不用循环动画**。CLI 进度写标准错误，最终 JSON 结果写标准输出，二者可区分。
- **取消**：共享 `AtomicBool`；每次读取后检查，置位则终止子进程并返回 `CoreError::Cancelled`，
  后续不继续该获取、不误报入库。CLI 的 Ctrl-C 由 `ctrlc` 捕获后置位——**新增该依赖的理由**：
  否则 Ctrl-C 会先终止 CLI 自身，用户无法看到“本次获取已取消、未入库”的结论，直接违背 FR-013/US2-4
  的诚实表达要求；这是防止该具体后果的最小手段。GUI 通过 Tauri 的 job 注册表 + `skill_fetch_cancel`
  置位同一标记。
- **检查用轻量解析**：`skill check` 只用 `git ls-remote` 解析远端 OID（不克隆），peeled 标签优先。

### D20 仓库内容识别与候选发现（T030，2026-10-17 补 FR-058）

- **识别谓词**（与库内扫描一致）：`is_dir() && 存在精确文件名 SKILL.md 或 skill.md`；
  `SKILL.md` 在前、命中即**不向下递归**；`README.md`/`CLAUDE.md` 不作标记。
- **扫描范围**：跳过 `.git`、`node_modules`、`__pycache__`；不跟随符号链接；深度上限 8（防链接环/异常深树）。
- **主动排除路径（FR-058）**：相对仓库根为 `.agents/skills` 或其子路径的目录**不是可入库候选**，也不再向下扫描。
  首次引入与后来新增共用同一发现结果。这不是用户声明的排除清单，也不写入 `rejected_subpaths`。
- **候选**：`{ subpath（`/` 分隔，仓库根自身即技能时为 `.`）, name = 目录名（目录身份，**不采用 frontmatter 的
  `name` 替代**）, description（仅展示） }`；**单个候选也列出**；用户指定子路径不存在时错误，**不回退仓库根**。
  指定子路径若落在 `.agents/skills/` 下，按“不是技能”报错，不偷偷改扫其他位置。
- **默认分类**依据**跳过排除路径之后**的候选数量：>1 按仓库名（URL 末段去 `.git` 后缀），=1 不分类；
  可显式 `--category`/`--uncategorized` 覆盖。不解析或自动补齐技能依赖。
- **与本机共享目录分开**：本机部署/浏览目标仍是当前用户主目录下的 `.agents/skills`；来源仓库内同名路径只影响发现，不改变部署。

### D24 来源发现跳过 `.agents/skills/` 的取舍（2026-10-17）

- **Decision**：在 `discover::scan_repo` 默认跳过相对路径 `.agents/skills/`；`preview`、`import`、上游新增候选都走这一结果。
- **Rationale**：用户引入自己的库时，仓库里该路径常放流程技能（例如 speckit），会被误当成可入库候选。产品主动不找这个位置，比让用户逐项「不入库并记住」更小、也更准确。
- **Alternatives considered**：
  1. 仓库内排除清单 / 扩展 `rejected_subpaths` 到首次引入——被拒：用户明确不是声明排除，且会把未勾选与排除混在一起。
  2. 扫描后展示再默认不勾选——被拒：候选里仍然出现，问题没消失。
  3. 增加「包含 `.agents/skills`」开关——被拒：当前无实际需要，增加入口没有对应后果。

### D21 上游检查、采用版本、拒绝与跳过基线（T039）

- **仓库地址去重**按 `SourceRecord::repo_key`（去首尾空白与末尾 `/`）：同地址跨两域**只获取/检查一次**，
  各所选 `ref` 分别解析；不推断 SSH/HTTPS 等不同写法是否同源（可接受的重复获取，不增建 URL 身份体系）。
- **版本比对**：采用版本（`source.version`）、跳过版本（`source.skipped_version`）与检查所得
  `remote_version` 一律用**完整提交 OID 字符串相等**判断；标签仅用于显示，不假设 tag 有大小顺序。
- **新增决定与基线文件** `upstream-decisions.yaml`，按 `(repo, domain)` 记录两类：
  - `rejected_subpaths`：用户明确“不入库并记住”的来源子路径；**随库共享**、可跨域独立选择；
    返回、未勾选、取消都不写记录。
  - `seen_subpaths`：**发现基线**——该 `(repo, domain)` **首次观察**（`import` 时）已存在的候选子路径集合。
- **基线规则**：只在首次观察时建立一份集合，**从不自动扩充**。候选的“后来新增”判定为
  `基线存在且 subpath ∉ 基线`。由此：首次未选的 C 不冒充后来新增、也不被记作拒绝；
  基线之后新出现且当前仍存在的 B 不会被“更新其他技能”清除提示；C 仍可手动 `preview` 选择。
  `check` 只**读**基线、**不写**该文件——遵守“检查只保存本机检查事实，不改共享资产”的边界。
- **不是历史系统**：只保存一份一次性集合，不记录时间序列、不做候选队列或改动归属账本。
- **来源绑定**：技能上游绑定来源仓库；另一机 `sync pull` 是 SkillManagerVault 库同步，不重新入库、不改变来源，
  不把 SkillManagerVault 远端写成技能上游。该关系已明确，不构成待裁决项。

## 八、本机运行配置与只读报告的双入口覆盖（2026-10-16 规划补齐）

### D22 判定准则：按“是否影响管理动作的结果”分类，不按“是不是设置”分类

- **Decision**：`spec.md` FR-050 的“核心管理操作”判定准则明确为——**该配置是否影响技能、部署、同步或检查的实际结果**。据此把本机运行侧的每一项分成四类：
  1. **必须 CLI 可写**：`auto_check_minutes`（FR-029 三项自动检查共用周期，需可由 Agent 按用户指令调整）。共享目录按用户后续修订自动识别且只读，不再是可写设置或部署前置配置。
  2. **必须 CLI 可读**：全部运行配置与调度事实，含只由 GUI 写入的 `theme`、`font_scale`、`git_auto_push`，使 Agent 能解释当前行为（例如“为何刚才没有自动检查”）。
  3. **不提供 CLI 写**：`theme`、`font_scale`（界面偏好，不改变任何管理结果）、`git_auto_push`（GUI 保存自动上传的**长期授权**记录；Agent 不得自行开启，其上传仍需每次明确同意）。
  4. **有意单侧**：工具本机安装事实回写只有 CLI（FR-047/FR-050 明令 GUI 只生成任务），不是缺口。
- **Rationale**：原先把“设置”整体视为 GUI 事务，导致 `shared_dir`、`auto_check_minutes`、运行配置读取、诊断获取与导出、容量报告五项整类缺 CLI 入口（逐项清点见 `contracts/cli.md` 的 T090 节）。按上表分类后，规格自身能表达"哪一侧应当有入口"，后续核对不必再逐次重新发现同类问题。
- **Alternatives**：①为每个配置键都加 CLI 写（被拒：为主题/字号这类界面偏好增加入口无实际收益，且给 Agent 打开长期上传授权会削弱 FR-038 的授权边界）；②只补已发现的 `shared_dir`（被拒：同类缺口会在后续核对中被反复重新发现，本次要求按类补齐）。
- **实现取法（最小充分）**：单一 `settings` 命令组承载“读全部 + 检查频率写入 + 两项只读报告”，不新增组合调度命令——CLI 的手动检查由 `skill check --all`、`sync pull`、`tool check --all` 分别表达（FR-029 的三项）。

### D23 清理两处无写入点的配置项

- **Decision**：移除“自定义日志目录”配置（日志固定 `<state-dir>/logs`）；Hermes 根改为按 `HERMES_HOME` 环境变量与平台默认解析，不设可写设置键。
- **Rationale**：两键在实现中**没有任何写入点**（只能直接改 SQLite），属死配置；`hermes_root` 若另设可写值，会与 Hermes 自身 `get_hermes_home()`（上下文覆盖 → `HERMES_HOME` → 平台默认，证据见本节第六节）的定位规则分叉，产生"配置说 A、Hermes 实际用 B"的静默不一致。
- **Alternatives**：保留键并补入口（被拒：当前无实际需要，且新增入口反而制造分叉风险；确有非默认根需求时再按真实场景加）。
