# 部署链路技术参考（Deployment pipeline）

本资料整理旧项目 FeiSkillDock v1.44.0（HEAD `372a2234`）中「库 → Agent 技能目录」的
部署、撤销、记录与检测实现，供本项目 Skill Manager 设计部署链路时参考。
只做静态源码阅读，**未运行旧应用、未执行真实部署**；凡"测试预期"均标注。

术语：Deployment（部署）、Support declaration（支持声明）、Overlay（覆盖层）、
Manager layer（管理层，即 `.manager`）、Management domain（管理域）。

## 需求关联

覆盖需求章节：**技能部署**（独立副本、批量部署/撤销、dry-run、共享目录部署、
管理域边界、库间复制、部署不带 `.manager`）、**维护技能**（按目标合成原始文件+覆盖层+中文注释）、
**技能获取与收编**（收编接管原部署、受管副本"本地有变更"暂停覆盖、删除库技能撤销部署）。
旧能力编号：D01–D10、F01–F04；旧行为"移动分类后修正符号链接"（D06 后半，明确不沿用）。

---

## 一、结论：部署链路时序

以下是从用户动作到文件落盘到记录更新的完整链路。**源码事实**，行号指向旧项目
`/Users/fei/Projects/workspace/FeiSkillDock/`。

```
用户动作（GUI 点单选/多选部署 或 CLI `skills deploy`）
  │
  ├─ 1. 目标解析（resolve target）
  │     adapter = find_adapter_with_store(tool)            scenario_service.rs:1267
  │        ├─ `_global` → global_agent_adapter()，目录 ~/.agents/skills，category=Coding
  │        │                                              tool_adapters.rs:245,250
  │        ├─ 内置 adapter（53 项）→ skills_dir() 展开 home/override 路径
  │        │                                              tool_adapters.rs:166
  │        └─ 自定义 tool → override_skills_dir 绝对路径    tool_adapters.rs:1001
  │     target = adapter.skills_dir().join(target_dir_name(source,name))  sync_engine.rs:283
  │
  ├─ 2. 准入闸门（gates，任一不过即拒绝）
  │     ├─ is_installed()（`_global` 豁免）                 scenario_service.rs:1274
  │     ├─ disabled_tools 不含该 key                       scenario_service.rs:1281
  │     └─ domain_block_for(source, adapter)（管理域边界）  scenario_service.rs:400,1310
  │        （vault 技能另加 vault_tool_allowed 支持声明闸门）scenario_service.rs:317,1331
  │
  ├─ 3. 产物合成（只对 source_type=="skillvault" 的技能）
  │     render_to_staging(source, tool) → 临时 TempDir      skill_render.rs:53
  │        = 中央目录副本(排除 .manager/.git/符号链接) + 该 tool 的 overlay 覆盖
  │     普通技能：effective_source = 中央目录本身（不合成）
  │                                              scenario_service.rs:1345-1352
  │
  ├─ 4. 落盘（sync_engine::sync_skill）
  │     ├─ is_target_current() 命中 → 跳过                  sync_engine.rs:400
  │     ├─ authorize_replacement() 按 ReplacePolicy 判定能否替换  sync_engine.rs:193
  │     └─ 删除旧目标 → Symlink 或 Copy（本项目只要 Copy）  sync_engine.rs:328-386
  │
  └─ 5. 记录更新（skill_targets 行）                        scenario_service.rs:1403-1415
        INSERT OR REPLACE INTO skill_targets（SQLite，含 source_hash）  skill_store.rs:468
```

批量部署走 `apply_skills_to_tools(Mode::Add)`（`scenario_service.rs:1443/1459`），
与单条部署共用 `sync_skill`，但多一次“整批预检”（见第九节）。

---

## 二、部署目标如何解析

**源码事实**：

1. Adapter 由 key 解析：`find_adapter_with_store`（`tool_adapters.rs:1046`）。
   内置 53 项在 `default_tool_adapters()`（`tool_adapters.rs:266`）；
   共享目录是保留伪 key `_global`（`GLOBAL_AGENT_TOOL_KEY`，`tool_adapters.rs:245`），
   由 `global_agent_adapter()` 构造：`relative_skills_dir = .agents/skills`，
   `relative_detect_dir = .agents`，`category = Coding`（`tool_adapters.rs:250-264`）。
   该伪 adapter 被刻意排除在所有"枚举全部工具"的路径之外，不进启用开关、工具顺序、
   各 Agent 工作区（源码注释 `tool_adapters.rs:239-249`；测试
   `builtin_adapters_never_claim_the_global_reserved_key` 预期）。

2. **技能目录 vs 检测目录**：`skills_dir()`（部署落点）与 `detect_dir()`（安装探测）
   是两条独立展开逻辑（`tool_adapters.rs:166`、`:184`）。检测目录命中的 `.config/x` 在
   Windows 追加 `%APPDATA%` 候选，macOS/Linux 不追加（`tool_adapters.rs:128-156`，
   注释 MAC-09）。`is_installed()` = 检测目录任一候选存在，或该 tool 是自定义/有路径覆盖
   （`tool_adapters.rs:222-231`）。

3. **启用状态参与**：`enabled_installed_adapters`（`tool_adapters.rs:1077`）=
   `is_installed() && !disabled_tools.contains(key)`；`disabled_tools` 存 setting
   `disabled_tools`（JSON 数组，缺失=空）。预设的候选 Agent 集合进一步按管理域收窄：
   `enabled_installed_adapters_for_domain`（`tool_adapters.rs:1106`）。

4. **`_global` 的例外**：共享目录按需创建，不探测 vendor 安装；在单条部署
   （`scenario_service.rs:1274`）、批量 Add（`scenario_service.rs:1481`）、
   预设开关安装闸门（`commands/sync.rs:281`、`:670` 测试）中 `_global` 均跳过
   `is_installed` 检查。

5. **落点目录名**：`target_dir_name(central_path, name)` 取中央目录的**文件名**
   （非技能 name），空时回退 name（`sync_engine.rs:283`）。库内同名技能由安装期
   冲突分配加 `-2` 后缀避免同落点（`commands/skills.rs:3900` 附近
   `resolve_install_destination`；测试 `sync_skill_to_tool_keeps_duplicate_skill_names_separate`
   `commands/sync.rs:620` 预期两技能分别落到 `skill123/` 与 `skill123-2/`）。

6. **未受管同名副本**：旧实现**拒绝覆盖**而非直接覆盖（见第九节与第十三节）。
   `classify_target`（`sync_engine.rs:232`）把目标分为 Absent / LinkToSource /
   ForeignLink / RealDir / RealFile；`ReplacePolicy::permits`（`sync_engine.rs:130`）
   决定能否删除。普通部署用 `NoClobber`（无已记录行时，`scenario_service.rs:273-278`），
   真实目录一律拒绝（`sync_engine.rs:818` 等测试预期 `Refusing to replace`）。

---

## 三、部署产物合成：哪些文件进入目标

**源码事实（普通技能，非合成）**：旧项目对非 vault 技能**不做产物合成**，直接把中央目录
交给 `sync_skill`。排除规则在 `copy_dir_recursive`（`sync_engine.rs:597`）：

- 排除 `.git`、`.manager`（目录，`sync_engine.rs:619`）、`.DS_Store`（`sync_engine.rs:629`）；
- 其余目录递归复制、文件 `std::fs::copy`。
- 测试预期：`copy_dir_recursive_skips_manager_directory`（`:1133`）、
  `sync_skill_copy_mode_leaves_no_manager_directory_in_the_target`（`:1179`）。

源码注释明确写出排除意图：`.manager` 是管理层不是技能内容，复制进 Agent 目录会在
库与 Agent 两处各存一份可编辑的同一信息（`sync_engine.rs:610-618`）。

**源码事实（vault 技能，合成）**：`skill_render::render_to_staging`
（`skill_render.rs:53`）先校验 overlay 树（`:58`），再把"中央目录（排除 `.manager`/
`.git`/符号链接）"复制进临时目录（`copy_render_tree(...exclude_manager=true)`，
`:62/196`），若存在 `.manager/overlays/<adapter_key>/` 则再把 overlay 树覆盖上去
（`exclude_manager=false`，`:64`）。overlay 里同名文件**覆盖**核心文件
（overlay-wins；测试 `render_codex_gets_core_plus_overlay_while_plain_agent_gets_core_only`
`:273` 预期 `config.yaml` 得 overlay 值）。**旧 overlay 校验器拒绝 overlay 中出现
`SKILL.md`/`skill.md`/`.manager`**（`validate_overlay_tree`/`validate_overlay_level`，
`skill_render.rs:109-185`）——这与本项目"允许整份正文替换"的需求冲突（见第十三节）。

**中文注释注入**：旧实现在**安装期**把注释写入 `SKILL.md` frontmatter 的
`description` 前缀（`installer.rs::apply_install_comment` `:192`，调用
`skill_metadata::set_description_prefix_in_content` `:105`），即**库内原始文件被改写**；
DB 的 `content_hash` 保持"未注入注释"的原始指纹，故 reindex 需 strip 注释再哈希
（`sync_metadata.rs:471-487`，`content_hash::hash_directory_normalized` `:159`）。
注入细节与旧前缀→新前缀文本变换已在 [content-composition.md](content-composition.md)
覆盖，本资料不重复。**这里只点出关键冲突**：旧项目注释是"安装时写库"，本项目要求
"部署时注入、不回写库内正文"。

**Copy/Symlink 分支**：`SyncMode`（`sync_engine.rs:261`）二值；`sync_mode_for_tool`
（`:275`）默认 `Symlink`，仅当 setting `sync_mode=="copy"` 时 Copy；vault 技能强制 Copy
（`scenario_service.rs:582`、`:1353`）。Symlink 分支在 Windows 还会尝试目录 junction，
再回退到 copy（`:337-375`）。本项目只要独立副本（Copy），不需要这套分支。

---

## 四、部署记录与"已部署/受管"判定

**存在哪里**：部署记录**不在技能库 sidecar 里**，而在本机 SQLite 表 `skill_targets`。

- 结构 `SkillTargetRecord`（`skill_store.rs:43-57`）：`id`、`skill_id`、`tool`、
  `target_path`、`mode`（`"copy"`/`"symlink"`）、`status`（`"ok"` 等）、`synced_at`、
  `last_error`、`source_hash`（**上次成功同步时中央技能的哈希**）。
- 唯一约束 `UNIQUE(skill_id, tool)`（`skill_store.rs:727` 注释），
  **不唯一于 `target_path`** —— 多个 tool 可共用同一物理目录（如 `_global`/cline/warp
  都解析到 `~/.agents/skills`），同一路径可被多行认领。
- 写入 `insert_target`（`INSERT OR REPLACE`，`skill_store.rs:468`），
  读取 `get_targets_for_skill`（`:488`）/`get_all_targets`（`:512`），
  删除 `delete_target(skill_id, tool)`（`:530`）。
- 库 sidecar（`.skills-manager/`）只镜像技能元数据与预设，**不写 targets**
  （`sync_metadata::write_all_from_db_unlocked` `:165` 只调技能/预设写入）。
  与 ADR `docs/adr/0001-library-and-local-state.md`（部署记录留本机）一致。

**"已部署/受管"判定**：基本判据是"该 (skill_id, tool) 存在 `skill_targets` 行"。
前端受管/未受管匹配用 `find_verified_center_match`
（`commands/agent_workspace.rs:118`），优先级 = (source_ref 指向该路径|该行 target_path
指向该路径) 得 3 > content_hash 相等得 2 > 无匹配（`max_by_key`）。状态串由
`classify_sync_status`（`commands/projects.rs:490`）给出：
`project_only` / `in_sync` / `project_newer` / `center_newer` / `diverged`。

**"被外部修改"检测（"本地有变更"的核心）**——**源码事实**，`classify_sync_status`
`commands/projects.rs:490-540`：

1. 快速路径：Agent 副本 `content_hash == 库行 content_hash` → `in_sync`。
2. 若库 DB 哈希可能过期：读库目录实时文件列表
   （`content_hash::list_content_files`）算 `hash_entries`，与副本哈希比；相等 → `in_sync`。
3. 否则用 **mtime** 判方向：副本最新 mtime 与库最新 mtime
   （`content_hash::latest_modified_ms`）相差超过阈值 `1000 ms`
   （`projects.rs:532`）：
   - `project_modified > center_modified + 1000` → `project_newer`（前端"本地有变更"）
   - 反之 → `center_newer`
   - 否 → `diverged`。

前端文案映射：`project_newer`→`localNewer`="本地有变更"
（`src/i18n/zh.json:1193-1199`；`GlobalAgentView.tsx:35-62`、
`WorkspaceView.tsx:43-57` 用同一 `sync_status` 词表）。

**重要区别**：`skill_targets.source_hash` 只用于**跳过冗余写**（is_target_current，
`sync_engine.rs:400`），它比的是"库技能指纹"，**不是**"Agent 副本是否被改"。
旧链路**不校验目标目录内容是否被外部改过**（`legacy-skill-capabilities.md` D04 行
亦如此记录："旧链路不校验目标文件内容是否被外部改过"）。所以"本地有变更"检测依赖
`classify_sync_status` 的哈希+mtime 比较，工作区/共享目录页面在**扫描时**实时算，
而不是部署记录里的某个"脏标记"。

> **推断（非源码明示）**：本项目需求里"受管副本被外部修改时暂停该技能覆盖"若沿用旧机制，
> 最接近的现成依据就是 `classify_sync_status`。但旧项目**自动同步链路并不统一调用它**：
> 预设/启动同步（`sync_desired_targets_counted`）只看 `source_hash`+`is_target_current`，
> 不比对副本内容，因此"暂停覆盖"在旧自动同步里并未真正实现——它是工作区页面的显示状态，
> 不是同步引擎的写前闸门。这是需要本项目明确设计的缺口。

---

## 五、撤销部署（undeploy）做什么、不做什么

**单个撤销**（`commands/sync.rs::unsync_single_skill_internal` `:115`；GUI
`unsync_skill_from_tool` `:167`，与 CLI `skills undeploy` 共用）：

1. 找该 (skill, tool) 行；
2. **幸存者检查**：该 `target_path` 是否被其他 (skill, tool) 行仍引用
   （`get_all_targets` 全表扫，`:131`）；是则**只删行、保留文件**（`:139`）；
3. 无幸存者时用 `remove_recorded_target(path, mode)`（`sync_engine.rs:487`）：
   只有目标当前类型**仍匹配记录模式**才删（copy 行删真实目录、symlink 行删链接）；
   不匹配则**保留文件、只删记录**并 warn（`:147`）；
4. 删 `skill_targets` 行。

**不做**：不删库内技能、不删 `.manager`（测试 `unsync_keeps_central_skill_and_manager_available_after_uninstall`
`commands/sync.rs:826` 预期）；不删共享目录里仍被其他 tool 引用的对象
（`uninstall_via_one_tool_keeps_shared_files_for_remaining_tool` `:783` 预期）。

**批量撤销**：`apply_skills_to_tools(Mode::Remove)`（`scenario_service.rs:1795`），
先删行、再按"去重后的记录模式"决定删文件；同一路径多行模式**不一致时保留对象只删记录**
（`:1887-1899`，注释"contradictory records: keep the object"）。

---

## 六、删除库技能时的清理范围与失败呈现

`delete_managed_skills_by_ids`（`commands/skills.rs:1057`）：

1. 对每个技能取其全部 target 行；
2. `other_claimed`：被**其他技能**的行认领的路径**跳过**（`:1085-1090`、`:1123`）；
3. **未跳过者用 `sync_engine::remove_target` 无条件删除目标**（`:1133`）——
   注意此处**不是**撤销部署用的 `remove_recorded_target`，即删除库技能时对部署副本
   不做"当前是否仍匹配记录"的校验，直接删路径（`remove_target` `sync_engine.rs:556`
   注释标注为 Unchecked）；
4. 删除中央目录（`remove_dir_all`，`:1139`），但先做"越界守卫"：
   `central.exists() && domain_of(central).is_none()` → 拒绝（`:1114-1120`）；
5. 全部成功才 `delete_skill`（级联删 target 行）；有失败则**保留中央目录与全部记录**，
   把失败路径写入 `failed` 列表返回（`:1145-1159`）。

**失败呈现**：单技能命令在失败时返回 `AppError::io(failed.join("; "))`，不伪装成
not_found（`commands/skills.rs:1034-1040`，注释 LIFECYCLE-04）。批量返回
`BatchDeleteSkillsResult { deleted, failed }`；CLI `run_remove` 要求 `--yes`，
`--dry-run` 只解析引用并回报数量、不删（`skills-manager-cli.rs:2953-2999`）。

---

## 七、管理域边界与共享目录资格

**管理域边界**：唯一谓词 `domain_block_for`
（`scenario_service.rs:400`）：`domain_of(central_path)` 与
`SkillDomain::from(adapter.category)` 必须一致；`domain_of` 返回 `None`（路径不属于
任何已知根）也拒绝，**不默认 Coding**。唯一 `ToolCategory → SkillDomain` 桥在
`tool_adapters.rs:62`（`impl From<ToolCategory>`，注释禁止第二处）。强制点分布在：
单条部署（`scenario_service.rs:1310`）、批量 Add 每对（`:1532`）、planner 防御断言
（`:144`）、预设候选收窄（`enabled_installed_adapters_for_domain` `tool_adapters.rs:1106`）。
**Remove 路径刻意不做域检查**（`apply_remove` `:1780` 注释 DEPLOY-05：转域后旧部署仍须可清理）。

**共享目录 `_global` 资格（旧版口径）**：`vault_tool_allowed`
（`scenario_service.rs:317`）：

- 非 `_global`：按 `vault_declared_support`（读 `.manager` 的 `agent_support`，
  缺失=全支持）判断该 tool 是否在声明内；
- `_global`：要求**声明为 mode-all（无声明）** 且 **`!manager_meta::has_overlays(central_path)`**
  ——即"全支持且无 overlay"才允许共享部署（`scenario_service.rs:318-321`）。
  另有 `sweep_vault_global_for_skill`（`:430`）清理"曾合法、后来加了 overlay 而失效"的
  `_global` 行。

**与本项目差异**：本项目要求"不自动按是否存在 overlay 目录机械拒绝共享部署"
（需求「技能部署」）。旧版对 vault 技能恰恰**机械拒绝有 overlay 的共享部署**——
这是明确冲突点，不能照搬该资格判断。`_global` 的 category=Coding 也意味着旧版
靠 `domain_block_for` 天然挡住龙虾技能进共享目录（源码注释 `scenario_service.rs:1307-1309`）。

---

## 八、dry-run / 部署预演

**源码事实**：

- CLI `skills deploy/undeploy`（`run_skill_deployment` `skills-manager-cli.rs:2002`）：
  `dry_run=true` 时**跳过** `apply_skills_to_tools` 与 `verify_deployment_state`，
  也不写审计日志，只算 `pair_count`、`changed_pairs`（与现有 `status=="ok"` 行求差）
  与 `blocked`（`domain_blocked_pairs`）；返回 `SkillDeploymentReport{dry_run:true,...}`。
  其余步骤（`resolve_skill_references_for_deployment`、`select_preset_agents`、
  `get_all_targets`）均为只读。
- CLI `skills sync`（`run_sync` `:3051`）：dry-run 走 `preview_scenario_sync`
  （`scenario_service.rs:176` → `collect_scenario_sync_targets`）并直接返回预览，
  不调用 `sync_desired_targets`。`preview_scenario_sync` **刻意不调用**清理
  `sweep_lapsed_vault_global_targets`（源码注释 `scenario_service.rs:455-458`：
  "a read-only preview must never delete rows or directories"）。
- 删除：`run_remove --dry-run` 只解析引用（`skills-manager-cli.rs:2972`）。

结论：上述预演分支不执行部署写入；旧 CLI 启动初始化与索引重建仍可能写本机状态，不能据此声称整个命令绝对无写操作。**局限**：预演基于
`existing_targets` 快照计算，不检测真实受管副本"本地有变更"，也不计算 `_global`
资格在预览中的完整过滤（`preview_scenario_sync` 用 collect，仍会按域/vault 过滤）。

---

## 九、批量部署/撤销的编排与部分失败

**批量 Add**（`apply_add` `scenario_service.rs:1459`）：

1. 过滤掉 disabled、未知、未安装（`_global` 豁免）的 tool（`:1468-1488`）；
2. **先规划全部 pair**，vault pair 立刻渲染 staging（TempDir 必须存活到执行）；
3. **整批预检**（`:1660-1705`）：
   - 两个不同技能规划到同一目标路径 → `duplicate_targets` → `invalid_input`，**什么都不改**；
   - 逐目标 `preflight_replace`，收集 `conflicts`；有冲突 → 整批 `target_conflict` 拒绝，
     **什么都不改**；探测 IO 失败 → 整批 `io` 拒绝；
4. 通过后逐 pair `sync_skill` 执行；**执行期失败只计数不中止**
   （`failed += 1`，`:1761`），返回成功写入数 `synced`。
   同一批内共享路径按"本批已写模式"优先于旧行（`:1731-1737`）。

**批量 Remove**（`apply_remove` `scenario_service.rs:1795`）：先删全部目标行，
再对"无幸存引用"的路径按一致模式删除；读幸存引用失败时**保留全部路径**并返回 0
（fail-safe，不 fail-open，`:1842-1854`）。

**部分失败呈现**：CLI `run_skill_deployment` 执行后立即
`verify_deployment_state`（`:2106`）核对每个 pair 的落盘/记录状态；有
`failures` 则 `bail!("deployment incomplete: ...")`，但已成功落盘的不回滚——
报告里保留 `preserved`（目标被他人接管而有意保留）与 `succeeded` 计数。
GUI 侧批量部署的 Tauri 命令为逐对调用 `sync_skill_to_tool`（注释
`commands/sync.rs:29-31`，预设 Bar 每个 (skill, agent) 一次调用）。

---

## 十、库间复制（旧版实际语义）

`copy_skill_to_domain_internal` / `_unlocked`（`commands/skills.rs:4910`/`:4932`）：

- 目标域 = `other_domain(source_domain)`，纯函数推导（`:4952`）；
- **create-only**：目标 leaf 已存在 → `CopySkillOutcome::Refused{removed_paths}`
  （`:4982-5010`），列出覆盖会删掉的路径；
- 整树复制用 `central_repo::copy_dir_recursive`（**保留 `.manager`**，`:5019`，
  注释强调不能用部署 walker，否则变空壳）；
- 新行 `source_type = "local"`，**清空所有来源指针**
  （`source_ref/source_ref_resolved/source_subpath/source_branch/source_revision/remote_revision`
  全 `None`，`:5041-5047`），`update_status="local_only"`；
- `insert_skill`（新 id），**不写任何部署记录、不写预设成员**（`:5059-5077`）。

**与本项目冲突**：需求「技能部署」要求"副本保留上游来源"。旧实现**刻意清除来源**
（注释 COPY-02：两行共享一个上游会导致一次远端更新写进两个库），与本项目
"副本保留上游来源、单项更新只影响所选副本"的既定规则相反。**不能照搬"清来源"语义**。

---

## 十一、更新后刷新部署 / 收编接管

- **更新后同步已有部署**：`resync_copy_targets`（`commands/skills.rs:3946`）遍历该技能
  全部 target 行，只处理 `mode=="copy"`，用 `Recorded{mode}` 策略重写并把
  `source_hash` 刷新为新的库哈希（`:4009-4018`）。即"库内容更新 → 已有部署跟着更新"
  的机制在旧版就是**重跑 copy 落盘 + 刷新 source_hash**。GUI 保存/Agent CLI 的触发
  差异属本项目规则，旧版由安装/更新流程下游调用（`:2640/2999/3014/3264/3411`）。
- **收编接管原部署**：`import_agent_local_skill_to_center`
  （`commands/agent_workspace.rs:358`）。若副本已能匹配受管
  （`find_verified_center_match`），走 `install_from_local_to_destination` 覆盖库目录并
  携带旧 `.manager` 注释（`:376-399`），随后
  `sync_single_skill_to_tool(..., DeployIntent::AdoptExisting)` 登记受管 target
  （`:419`）。新技能则 `install_from_local` + `insert_skill` + 再登记 target；登记失败
  回滚库行（`:461-469`）。`AdoptExisting` → `ReplacePolicy::UserConfirmed`
  （`scenario_service.rs:1374`），即"用户明确要求接管，允许覆盖目标"。
- **启动回填**：`backfill_stranded_agent_targets`（`agent_workspace.rs:573`）修复
  "有 source_ref 但无 target 行"的历史技能；仅当副本与库哈希相等才动作，否则跳过
  （`:641`、`:659-676`）。

---

## 十二、撤销部署的对象保留边界（"本地有变更"的旧近似）

旧实现对"目标被外部改动"的处理集中在**写/删前的所有权判定**（`ReplacePolicy`，
`sync_engine.rs:111-159`），逻辑是不看内容只看**类型 + 记录模式**：

- `NoClobber`（无记录）：只允许 Absent / LinkToSource；真实目录、外来链接、真实文件
  一律拒绝并保留字节（测试 `sync_skill_refuses_unmanaged_existing_directory` `:852`）。
- `Recorded{mode:"copy"}`：允许替换真实目录；`Recorded{mode:"symlink"}`：允许替换链接。
- `UserConfirmed`：接管/`--force`/拉取覆盖。

**即**：旧版对"副本被外部改过"的保护 = "记录模式与实际类型是否一致"，**不比对内容**。
若用户在受管 copy 目录里改了文件、目录仍是真实目录，旧版**照样覆盖**（它会删除并重写）。
真正意义上的"内容级本地变更检测"只存在于 `classify_sync_status`（第四节），且未接入
自动同步写路径。本项目的"暂停覆盖"需要实现已确认的写前内容变更判断，旧代码**没有直接可抄的实现**。

---

## 十三、旧机制不采用清单（及原因）

| 旧机制 | 来源 | 不采用原因（对照本项目需求） |
|---|---|---|
| Symlink 部署模式与 Windows junction 回退 | `sync_engine.rs:328-375`、setting `sync_mode` | 需求「技能部署」确定只要独立副本，不提供 Copy/Symlink 切换；分支与 junction 权限处理都无必要 |
| 未受管同名目标拒绝覆盖（NoClobber） | `sync_engine.rs:130-150`、`scenario_service.rs:273` | 需求「技能部署」D04：遇到 Agent 中未受管同名技能**直接覆盖并纳入受管**，不做冲突检查（旧版恰好相反，会返回 `target_conflict`） |
| 安装期把中文注释写入库内 `SKILL.md` | `installer.rs:192`、`skill_metadata.rs:105` | 需求「中文注释与原始内容的边界」：注释在**部署时**加入，不回写库内原始正文；旧做法改写了库原件 |
| 共享部署要求"全支持且无 overlay"（vault） | `scenario_service.rs:318-321` | 需求「技能部署」：**不自动按是否存在 overlay 目录机械拒绝共享部署** |
| overlay 禁止 `SKILL.md` | `skill_render.rs:155`/`:109` | 需求明确允许整份正文替换（[content-composition.md](content-composition.md) 亦已记录旧校验器冲突） |
| 库间复制清除上游来源 | `commands/skills.rs:5041-5047` | 需求：副本**保留上游来源**；旧版为防"一次更新写两库"而清空来源，语义相反 |
| 移动分类后修正已部署符号链接 | `commands/skills.rs:1355-1377` | 旧行为 D06 后半，明确不沿用（本项目独立副本，无链接需修） |
| `.skills-manager` 中央库与 sidecar 双写体系 | `sync_metadata.rs:85-91`、`reindex_from_metadata_unlocked` | 需求 B02：只管理一个 SkillVault 根目录，不另存中央库内容；旧边车/库格式不继承 |
| `skills enable/disable` | `skills-manager-cli.rs:3003` | 旧兼容命令，enable 仅修 legacy，disable 是 no-op；本项目以部署/撤销表达启停 |
| 部署记录写入 sidecar / 随库同步 | （旧版未做，targets 仅在本机 SQLite） | 与本项目一致，**不是**要移除；此处列出以免误以为 targets 随库走 |

---

## 十四、验证状态与未确认事项

- **验证状态**：本文全部为**静态源码阅读**；标注"测试预期"处为旧项目单元测试断言，
  **未运行**这些测试，**未执行**任何真实部署/撤销。
- **未确认（旧项目无对应实现）**：
  - "部署时按目标合成原始文件 + 覆盖层 + 中文注释"中的**中文注释注入**在旧 vault 合成
    路径（`render_to_staging`）里**没有**体现——旧注释走安装期。本项目需要的
    "部署期注入 + overlay + 注释合并顺序"在旧代码中没有完整链路，需自行设计。
  - "复制副本保留上游来源"旧版无实现（相反）。
  - 旧版"路径被另一行认领则跳过删除"的幸存者判定用 `get_all_targets` 全表扫
    （`commands/sync.rs:131`），规模化代价未评估。
- **实现设计事项**：不能仅比较当前库与目标内容、再用 mtime 判向，就断言副本被用户修改。
  两者不同也可能只是库更新。应依据上次部署产物与目标的关系实现已确认的暂停规则；
  具体哈希算法不作为本轮用户业务裁决，也不照搬 ±1000ms 判向。
- **相邻主题未涉及（供查漏）**：预设（Preset）部署成员切换的完整语义；
  Hermes 档案启用链路的挂载细节；Git 备份/同步引擎；CLI 命令语法与输出格式；
  工具中心。这些不在本资料范围。
