# 上游更新、采用版本与差异链路参考

2026-10-15 对 FeiSkillDock v1.44.0（HEAD `372a223414dc773f7aaefc887c75ab0e1b335be8`）源码的静态核对。
只做源码阅读与数据解析，没有运行旧测试、没有执行真实更新/同步，也没有重现网络失败。
文中"旧行为"是按该提交的代码与注释读出的事实；"新要求"出自本项目 `docs/requirements.md`。

本页覆盖需求「上游获取与更新管理」「持续维护：分清两种更新」「上游更新的保留边界」
「新增与安装的区别」「跳过当前上游版本」，以及旧能力 C01–C08 的对应实现。
**获取进度／取消、仓库扫描**不在此重复，见 [获取与发现参考](git-and-discovery.md)。

## 1. 旧实现如何登记"每个技能采用的上游版本"

源码事实：版本是**技能级（per-skill）**记录，存进 SQLite 的 `skills` 表，粒度是"一行技能 = 一套来源与版本字段"；
不存在"来源仓库"实体表。权威字段（`src-tauri/src/core/migrations.rs:72-92`）：

旧字段把来源地址、分支、子路径、当前采用提交、最近检查的远端提交、内容哈希与检查结果放在技能行上。
这些概念可参考；完整旧建表、来源类型枚举与 sidecar 格式不继承。

- `SkillRecord` 结构与之对应（`src-tauri/src/core/skill_store.rs:19-40`）。
- 安装时写入：安装流程造 `InstallSourceMetadata { source_revision: Some(revision), remote_revision: Some(revision), update_status: "up_to_date" }`
  （例如 `src-tauri/src/commands/skills.rs:2101-2103`、`1826-1828`），再经
  `store_installed_skill_unlocked` 落库（`skills.rs:3429-3500`）。
- 更新成功后把 `source_revision` 与 `remote_revision` 都推进到新远端提交（`skills.rs:2988-2998`，
  `update_skill_after_install`）。
- 另有 sidecar（`.skills-manager` 目录，`sync_metadata::write_all_from_db_unlocked`）是**派生索引**，
  不是权威；权威是 DB（`src-tauri/src/core/sync_metadata.rs:147-189`）。

与"按仓库登记 + 每技能记录采用版本（新要求）"的差距：

- 旧侧完全没有仓库级登记表；仓库信息只以 `source_ref_resolved`(clone_url) + `source_branch` 的形式散落在各行技能上。
- "同一仓库只检查一次"在旧实现里是**检查时的临时去重**，不是持久实体：`check_all_skill_updates`
  用 `RemoteKey { clone_url, branch }`（`skills.rs:2300-2321`）把同一 `ls-remote` 折叠成一次网络调用，
  检查完即丢弃，没有留下"这个仓库有哪些已入库技能/新技能"的清单。
- 新要求的"仓库级登记 + 上游路径 + 当前采用版本"是旧侧没有的载体；旧侧可复用的是"每技能一行版本字段 + 内容哈希"这一粒度。

## 2. 单项／全量检查怎么判定"可更新"

检查入口（`src-tauri/src/commands/skills.rs`）：
`check_skill_update`(2140) → `prefetch_skill_remote`(2342) → `check_skill_update_internal_with_remote`(3538)；
全量 `check_all_skill_updates`(2184) 分三段：Phase A 并发解析每个 distinct remote，
Phase B 逐技能写状态，Phase C 处理 vault 行。

判定逻辑（`skills.rs:3553-3653`）：

- **git / skillssh**：远端提交号由 `git_fetcher::resolve_remote_revision`（`git_fetcher.rs:1222`，
  优先 `git ls-remote`，失败回退 git2）取到后，比较 `source_revision == remote_revision`：
  - 相等 → `up_to_date`；
  - 不等 → `update_available`；
  - 本技能没有 `source_revision` → `unknown`。
- **local / import**：用 `installer::hash_local_source(source_path)` 现算内容哈希，与 `content_hash` 比；
  路径不存在 → `source_missing`；无路径 → `local_only`。
- **其他来源类型**（含 `skillvault`）→ `unknown`。

"失败"与"已是最新"的区分：失败写 `update_status = "error"` 并记 `last_check_error`，
与 `up_to_date` 是两个不同状态（`skills.rs:3608-3617`、`3639`），不会混同。
状态取值集合：`up_to_date` / `update_available` / `unknown` / `error` / `updating` / `local_only` / `source_missing`。

检查节流：`should_skip_update_check`（`skills.rs:3659-3684`）在非 force 且状态稳定、且
`last_checked_at` 在 TTL 内时跳过；TTL 来自 setting `update_check_ttl_minutes`，默认 60 分钟。
CLI 侧 `skills check --all --domain <域>` 逐个域检查（`skills-manager-cli.rs:2871-2917`）——
旧 CLI 的"全库检查"被域切开，且不产出"新增候选"。

## 3. 更新前的差异计算与展示

- 差异按**技能目录内相对路径**枚举：`build_source_diff_entries(central_dir, source_dir)`
  （`skills.rs:807-901`）。以 `content_hash::list_content_files` 的文件集合建 `BTreeMap<相对路径, 条目>`，
  两端取并集后对每个路径判 added / removed / modified。
- 呈现类型：`classify_diff_bytes`（`skills.rs:792-802`）把内容分成 `text` / `binary` / `too_large`；
  超过 `MAX_DIFF_FILE_BYTES = 256 KiB` 或含 `\0` 的只给摘要行，不送正文（避免逐行 diff 卡 UI）。
  纯权限变化单独标 `permission_only`（`skills.rs:861-874`）。
- 对外命令 `get_skill_source_diff`（`skills.rs:903-981`）返回 `entries`；前端
  `SkillSourceDiffViewer.tsx`（`src/components/SkillSourceDiffViewer.tsx:11-15`）按
  added/removed/modified 着色，文本走 `DocumentDiffViewer` 逐行显示。
- **改名不单独识别**：因为按路径 key 对比，改名必然表现为"旧路径 removed + 新路径 added"两行。
  仓库内技能目录被上游改名/搬迁时，`resolve_skill_dir` 对普通 git 来源（无 locator）
  只在 `source_subpath` 仍有效时使用，否则直接报错（`skills.rs:3839-3880`）；skills.sh 才有 locator 回退。
  这只说明文件路径差异的表示可参考；技能目录改名的新增/删除候选仍需仓库级清单，旧路径失效报错不是新流程的完整实现。

删减性更新确认（C04）：

- `removals::removed_paths(current, replacement)`（`src-tauri/src/core/removals.rs:47-131`）
  列出"现在有、新版本没有"的路径；整目录缺失折叠为一条带 `/` 的条目，跳过 `__pycache__`/`.pyc`/`.DS_Store`。
- `pending_removals_for`（`skills.rs:95-131`）同时检查库内目录**和**每个 copy 型部署目标。
- `removal_approval_token`（`skills.rs:134-148`）= `SHA256(远端提交号 + 排序后的待删列表)`，
  即"这个版本 + 这份清单"的一次性批准令牌。清单非空且令牌不匹配时，更新**什么都不改**并返回待删列表；
  用户确认后带同一令牌再调（`update_git_skill_internal`，`skills.rs:2955-2971`）。
- 无人值守调用（批量、后台、CLI）传 `None` → 直接暂缓（`held_back`），不应用删除性更新。

## 4. 更新执行时的写入策略与 `.manager` 处理（核心冲突）

### 4.1 是整体替换目录，不是按文件合并

`update_git_skill_internal`（`skills.rs:2851-3045`）流程：clone/checkout 远端到临时目录 → 定位技能目录 →
算新哈希 → `install_skill_dir_to_destination(来源目录, name, staged_path, carried_comment)` 装到暂存目录 →
`swap_skill_directory(staged, central_path)` 原子换装（`skills.rs:3920-3944`：先把旧目录 rename 成 backup，
再 rename 暂存目录就位，失败回滚）。

`installer::install_skill_dir_to_destination`（`src-tauri/src/core/installer.rs:143-183`）是先
`remove_dir_all(destination)` 再整目录 copy，`copy_skill_dir`（`installer.rs:289-314`）只跳过
`.git`、`.DS_Store` 和所有 symlink，**其余文件（含 `.manager`）原样复制**。

### 4.2 `.manager`（管理资料）怎么处理

源码事实（关键，限内容变化且实际进入目录替换的分支）：

- 来源树没有某项本地管理资料时，复制来源树不会自然保留它；上游自带 `.manager` 是正常输入，不能假设来源总是不带。
- 旧代码**只搬 `comment` 一个字段**过桥：`carried_comment = read_metadata(central_path).comment`
  （`skills.rs:2918-2938`；本地重导入路径 `2589-2597`、`3355-3369` 同款），
  再由 `apply_install_comment` 写回（`installer.rs:192-214`）。
  其它管理资料没有在此分支显式搬运。旧目录被替换后，后续 DB/sidecar 写回可能重新生成部分字段；因此“整个更新结束后哪些值消失”仍须沿后续写回判断，不能把局部未搬运等同所有资料必然最终消失。
  这正是旧注释自陈的"the update would silently drop it"（`skills.rs:2920-2922`）——它只意识到 comment 会丢。
- 反过来，若**来源目录自带上游 `.manager`**，它会被整份复制，再在其上 merge 本地 comment。
  也就是说旧行为是"上游 `.manager` 整份采用 + 本地 comment 覆盖"。

### 第二轮复查补充：仅管理资料变化可能不落地

`update_git_skill_internal` 用 `content_hash::hash_directory` 算来源内容哈希
（`skills.rs:2908-2910`）；`content_hash.rs:17` 的忽略集合包含 `.manager`。
只有 `content_changed` 时才生成暂存目录并替换；否则更新版本记录、刷新旧副本并写镜像
（`skills.rs:2918-2942,2973-3015`）。

因此，在内容哈希相同、仅上游 `.manager` 变化的条件下，旧普通 Git 更新路径可能推进采用版本，
却未复制新管理资料。这是静态调用链结论，未运行样例。新项目不能直接用“部署内容没变”
判断“管理资料无需更新”，否则用户自己的上游管理资料无法随更新进入库。

### 4.3 旧 SkillVault 来源模型：vault 覆盖 `.manager`、拒绝软件直接编辑

这是与"上游同项值优先"最相关的一段历史成因：

- 模块头（`src-tauri/src/core/skill_vault.rs:1-8`）：vault 仓库里的 `.manager/` 是"管理真值来源"，
  应用以 raw 形式导入并标 `source_type = "skillvault"`，以便之后**拒绝**本地写 `.manager`。
- `import_vault_skill`（`skill_vault.rs:669-723`）调用同一个 `install_skill_dir_to_destination`，
  **整目录替换（含 `.manager`）**，并把 vault 侧 `.manager` 的 comment 带过来；注释明写
  "whole-directory replace, `.manager` included — no separate update branch"（`skill_vault.rs:666-668`）。
  vault 刷新时 `source_revision = head_revision`（行 717）。
- `ensure_manager_writable`（`src-tauri/src/commands/skills.rs:4064-4075`）：`source_type == "skillvault"`
  时任何中文名/简介/正文写入都被后端拒绝（"管理信息由 SkillVault 管理，请在 skill-vault 仓库修改"）。

### 4.4 旧行为 vs 新要求（冲突点，必须剥离）

| 维度 | 旧行为（v1.44.0 事实） | 新要求（requirements.md） | 如何剥离 |
|---|---|---|---|
| 上游没带某项管理资料 | 替换分支只显式搬运 comment，部分字段可能由后续镜像写回 | 「上游未带管理资料时保留本地资料」 | 不能再"整目录替换 + 只搬 comment"；需改为按项合并，保留本地未覆盖项 |
| 上游带了同一项管理资料 | 整份采用上游 `.manager`（本地仅 comment 覆盖） | 「上游提供同项管理资料时采用上游值」 | 方向一致，但要**逐项**判定"上游是否提供该项"，而非整份覆盖 |
| 冲突裁决 | 不存在（谁赢由"整目录替换"隐式决定） | 不做管理资料冲突裁决 | 删除旧的"来源类型决定是否合成/覆盖"分支，改为无条件逐项 merge |
| 本地可编辑性 | skillvault 来源禁止软件改 `.manager` | 本地资料是用户维护成果、随库同步 | 去掉 `ensure_manager_writable` 这类来源相关只读门；本地编辑不再分来源 |
| 分类 | vault 布局决定 category | 「用户维护的目录分类保留」 | 不采用"上游布局决定分类"的旧模型 |

要点：旧侧的"上游优先"是通过**整目录替换**顺带实现的副作用，不是"逐项比较"的规则；
它同时导致"上游没有的本地资料被删"。新要求要的是"逐项：上游有则用上游，无则留本地"，
既不能照搬整目录替换，也不能照搬只搬 comment 的补丁。

## 5. "上游新增候选"与"不入库决定"（对应问题 5）

**旧项目无此实现。** 依据：

- `skills` 表只有"已入库技能"行，没有"来源仓库"实体（`migrations.rs:72-92`；全表清单见
  `migrations.rs` 的 `CREATE TABLE`，无 repo/source 表）。
- 没有任何"从某仓库发现但未入库的技能"的记录结构；`check_all_skill_updates` 只对已入库技能判
  `update_available`，不枚举仓库里多出来的技能（`skills.rs:2184-2294`）。
- 最接近"记住看到过什么"的表是 `discovered_skills`（`migrations.rs:108-116`，字段 `fingerprint`、
  `imported_skill_id`），但它是 **Agent 本地目录扫描**（旧 B08/F01）用的，记录的是 Agent 已安装的
  未受管技能及其导入状态，与"上游仓库新增候选"不是一回事（用法见 `src-tauri/src/commands/scan.rs:34-87`）。
- 旧动词"新增/fresh"只出现在 vault 对账里（`skill_vault.rs` 的 `fresh` 计数 / `SkillVaultCheckCounts`），
  指"vault 里有、中央库没有的技能"，且是**每次检查现算**，不持久记住用户"不入库"的决定。

结论：新要求的"上游新增候选 + 不入库决定随库同步、其他电脑不重复提示"在旧侧没有载体可参照；
只有"每技能版本字段"和"discovered_skills 的 fingerprint/imported 关联"两个相邻但不等价的结构。

## 6. 自动更新检查器与"自动应用"（C07/C08）

启动/定时检查器：`src-tauri/src/core/skill_auto_updater.rs`。

- 启动挂载：`core::skill_auto_updater::start(...)`（`src-tauri/src/lib.rs:1245`），内部先
  `sleep(INITIAL_DELAY = 60s)` 再循环，`POLL_INTERVAL = 15min` 唤醒一次判断是否到期
  （`skill_auto_updater.rs:22-59`）。逐技能检查之间 `FOREGROUND_YIELD = 200ms` 让位前台操作（行 35、166）。
- settings key：`auto_update_check_interval`、`auto_update_last_run_at`、`auto_update_apply`
  （`skill_auto_updater.rs:14-16`）。
- 周期解析 `parse_interval`（行 88-96）：`off/manual/disabled/空` → 不跑；`1h`、`6h`、`24h`。
  **旧选项是 关闭 / 1h / 6h / 24h（无 30 分钟），且默认"无设置 = 关闭"**（前端
  `Settings.tsx:952-957`；默认 state `"off"`，`Settings.tsx:189`）。
  新要求：关闭 / 30 分钟 / 1 小时 / 6 小时 / 24 小时，**默认启用每 1 小时**。
- 启动时前端还会延迟 3s 触发一次 `checkAllSkillUpdates(false)`（`src/context/AppContext.tsx:450-522`）；
  托盘"检查更新"复用 `record_round_completion` 写 `last_run_at` 并发 `skills-auto-updated` 事件
  （`lib.rs:798-812`；`skill_auto_updater.rs:61-81`）。

旧"自动应用"（C08）：`apply_enabled` 读 `auto_update_apply == "on"`（行 139-144，默认关闭）；
开启后检查到 `update_available` 就调 `update_git_skill_internal`（行 200-219）。
写路径**只有 git updater**（第 200-218 行只此一个调用），且删除性版本会暂缓（`pending_removals` 非空 → `held_back`）。
启动腿的自动应用同理（`AppContext.tsx:470-505`）。

新要求移除上游自动应用：`requirements.md`「持续维护」段与 C08 裁决——外部上游一律显式确认后再应用，
自维护 Gitea 也显式；不按托管平台/作者身份另设策略。旧实现的 `update_git_skill_internal` 本身
已经"检查与更新分离"（检查只写状态，更新独立命令），可复用的是这个分离，而不是 `auto_update_apply` 开关。

## 7. "跳过当前上游版本"（对应问题 7）

**旧项目无此实现。** 依据：

- `skills` 表、settings、`skill_auto_updater` 中都没有"跳过某版本/某提交"字段或命令
  （字段全集见 `migrations.rs:72-92`；`skill_auto_updater.rs` 只有 interval/last_run/apply 三个 key）。
- 唯一的"暂缓"是删除性更新的 `held_back`（`skills.rs:2485-2537`、`skill_auto_updater.rs:201-209`），
  语义是"这次不应用，下次仍会提示"，与"用户主动跳过这个版本、不再提示"不同。
- 检查状态只有 `update_available` 等，没有 `skipped` 之类的值（`skills.rs:3592-3653`）。
- 没有"已忽略/removed 记录"可供借用：vault 的 `removed` 是每次对账现算的列表，
  不是持久决定（见第 5 节）。

可借鉴的落点（仅作参考，由本项目设计决定）：旧侧"每个技能一行版本字段"里 `remote_revision`
与 `update_status` 相邻，是记录"已检查到的远端版本 + 状态"的天然位置；若要实现跳过，可能
需要一个"记住被跳过的版本/清单"的持久结构，但旧侧没有同类先例，不能照搬。

## 未验证 / 存疑

- 全部为静态阅读结论；未运行旧测试、未实际触发一次更新，因此"更新后 `.manager` 其他字段被丢弃"
  是从 `install_skill_dir_to_destination`（整目录替换）+ 只搬 comment 的代码路径推断的事实链，
  旧测试（如 `skills.rs:6225-6236`）只断言 comment 存活，未断言其他字段。如后续确需复现，用隔离的合成仓库覆盖资料保留与仅管理资料变更即可，不要求用户提供或操作真实技能库。
- `resolve_remote_revision` 的 git2 回退路径与本项目"只用本机 Git"的取向不同，未核对其在
  大仓库/网络失败下的具体表现；进度与超时问题见获取参考。
- 旧 vault 只读门（`ensure_manager_writable`）在旧版 GUI/CLI 的完整触发面未逐命令枚举。
