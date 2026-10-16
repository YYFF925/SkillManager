# 本机运行时与桌面外壳参考

依据旧项目 FeiSkillDock v1.44.0（HEAD `372a223414dc773f7aaefc887c75ab0e1b335be8`）静态源码阅读。
本文件覆盖：Agent 检测与目录配置、Agent 指令源与分发、外部修改自动刷新、偏好与托盘、未受管本地技能删除。
旧行为不等于新需求；文中已按本项目需求区分「可沿用模型」与「差异/剥离」。

## 0. 覆盖范围与剥离项

覆盖的需求与旧能力：

- 需求「Agent 接入与本地技能」（含「设置页沿用与统一」）：本机 Agent 检测、已检测/其他支持分组、启停、重新检测、技能目录与项目相对路径配置、指令管理、未受管本地技能删除。
- 需求「界面与窗口偏好」：主题、字号、关窗收托盘、托盘重开窗口与检查上游更新。
- 旧 A11（外部修改自动刷新，见 §3）；旧 G01/G03/G04/G06；旧 J06/J07；旧 F04（未受管本地技能删除，见 §5）。

明确剥离（需求已裁决，这里只说明旧实现长什么样，不构成可继承能力）：

- **G02 自定义 Agent 登记**：新需求不保留。旧实现 `add_custom_tool`/`remove_custom_tool`（`commands/tools.rs:439`/`484`）与设置键 `custom_tools` 仅作历史线索。内置 Agent 名单全量支持，路径修改能力独立保留（G03/G04）。
- **G05 共享目录读取声明**：不保留。旧 `set_tool_global_skills`（`commands/tools.rs:172`）+ 设置键 `global_skills_disabled_tools`（`core/tool_service.rs:41-87`）属于旧模型；新需求按「编码 Agent 都读取共享目录」的处理假设，不提供逐 Agent 声明。
- **旧多语言**：新需求只需简体中文。旧 zh/zh-TW/en 三语（`src/i18n/index.ts`、`Settings.tsx:1685-1710`）不改造成新能力。
- **旧托盘预设部署入口**：新需求托盘不提供按预设部署/撤销。旧 `apply_preset_from_tray`（`src-tauri/src/lib.rs:645-733`）及托盘预设子菜单（`lib.rs:490-550`）剥离。

**原型声明**：`agent-catalog.json` 与原型里的「已检测」「模拟分发」都是模拟数据，不是本机检测结果。旧 `is_installed` 只在真实运行时读本机路径。

## 1. Agent 检测与目录配置

### 1.1 检测条件（源码事实）

`is_installed()` 是**纯路径存在性判断**，不查进程、不启动 Agent（`core/tool_adapters.rs:222-231`）：

```rust
pub fn is_installed(&self) -> bool {
    // Product decision: when users explicitly provide a skills path (override/custom),
    // we treat the tool as available so sync can proceed without probing vendor install state.
    if self.is_custom || self.override_skills_dir.is_some() {
        return true;
    }
    Self::candidate_paths(&self.relative_detect_dir)
        .iter()
        .any(|path| path.exists())
}
```

- 检测目录来自 `relative_detect_dir`，与技能目录 `relative_skills_dir` **分开**（`ToolAdapter` 结构 `tool_adapters.rs:72-101`）。检测目录不等于技能目录，例如 Replit 检测 `.replit`、技能在 `.config/agents/skills`（另见 `agent-integration.md`）。
- `candidate_paths`（`tool_adapters.rs:129-156`）：非 Windows 只有 `home()/<relative>` 一条；Windows 额外把 `.config/` 映射到 `dirs::config_dir()`。macOS 不会把 `.config/*` 映射进 `Library/Application Support`（MAC-09 注释 + 测试 `tool_adapters.rs:1152-1164`）。
- 副作用：**自定义 Agent 或已设置技能目录覆盖时，`is_installed()` 无条件返回 true**——「已检测」在该分支下等于「用户显式指定了路径」，而不是磁盘存在。
- 检测不启动 Agent、不自动部署（需求「Agent 接入与本地技能」明确，旧实现一致）。

### 1.2 重新检测与刷新链路

- 后端命令 `get_tool_status`（`commands/tools.rs:93-108`）→ `build_tool_status`（`tools.rs:45-66`）→ `tool_service::list_tool_info`（`core/tool_service.rs:308-346`），逐 adapter 调 `is_installed()`，再按顺序合并。
- 前端 Settings「重新检测」按钮 `handleRefresh` → `refreshTools()` → `get_tool_status`，重算 installed/enabled 并重排；列表拆成 detectedTools / undetectedTools（其他支持 Agent）/ customTools（旧）。

### 1.3 enabled_installed_adapters 如何组合（源码事实）

`core/tool_adapters.rs:1077-1090`：`is_installed() && !disabled.contains(key)`。

- disabled 列表来自设置键 `disabled_tools`（JSON 数组）。缺失/解析失败 → 空 → **新检测到的 Agent 默认启用**。
- 按管理域收窄：`enabled_installed_adapters_for_domain`（`tool_adapters.rs:1106-1116`），用唯一的 `ToolCategory → SkillDomain` 桥（`tool_adapters.rs:62-69`）。
- 指令分发（§2）、旧托盘预设、共享目录重复标记都用这个集合。

启停：`set_tool_enabled`（`tools.rs:117`）→ `set_tool_enabled_internal`（`tools.rs:135-157`）：启用则从 disabled 移除并 `sync_active_scenario_to_tool`；停用则 `unsync_all_for_tool`（删受管副本）并加入 disabled。

### 1.4 Agent 配置存哪里（源码事实）

全部在**本地 SQLite** 的 `settings` 表（key/value，`core/skill_store.rs:654-696`），不在磁盘配置文件、也不在 Git 跟踪的库内容树里：

| 设置键 | 含义 | 结构 |
|---|---|---|
| `disabled_tools` | 停用的 Agent | JSON `Vec<String>` |
| `tool_order` | 显示顺序 | JSON `Vec<String>` |
| `custom_tool_paths` | 内置 Agent 全局技能目录覆盖 | JSON `{key: abs_path}` |
| `custom_tool_project_paths` | 项目相对路径覆盖 | JSON `{key: rel_path}` |
| `custom_tools` | 自定义 Agent 定义（剥离项） | JSON `[{key, display_name, skills_dir, project_relative_skills_dir, category}]` |
| `global_skills_disabled_tools` | 共享目录读声明（剥离项） | JSON `Vec<String>` |

顺序合并逻辑 `merge_order`（`core/tool_service.rs:162-202`）：保存顺序优先、新优先级 Agent 插到其前驱之后、其余按 adapter 注册顺序追加；无保存时按 `DEFAULT_PRIORITY_ORDER`（`tool_service.rs:112-136`）。

对应需求「目录、Agent 启用状态与显示顺序是本机配置，不随 SkillManagerVault 同步」：旧实现正确地把它们放在本机 SQLite。旧 Git 备份仓库根是 `skills_dir()`（`core/git_backup.rs:282-290` 注释明确不把 `base_dir()` 提升为仓库根，以免提交 `skills-manager.db`），而 `skills_dir()=base_dir()/skills`、`lobster_skills_dir()=base_dir()/lobster-skills`（`central_repo.rs:299-330`）；DB 与 `settings` 不在其中。**这是可沿用的模型判断：本机配置落本机存储，不进库。**

### 1.5 G03 修改技能目录后的旧部署协调（重点参考）

写路径：`apply_tool_skills_dir`（`tools.rs:260-299`，内置落 `custom_tool_paths`，自定义落定义）与 `reset_custom_tool_path`（`tools.rs:312-334`）。两者在检测到 `old_skills_dir != new_adapter.skills_dir()` 时调用（`tools.rs:295-297`、`328-330`）：

```rust
fn reconcile_tool_sync_after_path_change(store: &SkillStore, tool_key: &str) {
    // Remove existing synced artifacts/records (old path), then re-sync to current adapter path.
    unsync_all_for_tool(store, tool_key);              // 删除旧路径的受管副本 + target 记录
    let disabled = get_disabled_tools(store);
    if !disabled.contains(&tool_key.to_string()) {
        sync_active_scenario_to_tool(store, tool_key); // 在新路径重新同步
    }
}
```

- `unsync_all_for_tool`（`tools.rs:74-80`）：遍历 `get_all_targets()` 中该 tool 的记录，`sync_engine::remove_target(target_path)` 删实际文件，再 `delete_target`。
- **旧策略 = 先按记录删除旧路径产物，再在新目录重建；不保留旧产物、不做用户确认。** 这是新需求「修改目录后已有部署记录协调方式待设计」的唯一旧参照。
- 局限：依赖旧 `skill_targets`（按 `(skill_id, tool)`）模型；且 reconcile 内假设「启用即按当前 active scenario 重建」，新统一模型没有互斥「当前预设」，这一假设不成立。若新需求不接受静默删旧副本，本函数不能直接搬。

### 1.6 G04 项目相对路径

- 写入：`set_custom_tool_project_path`（`tools.rs:389`）→ `apply_tool_project_skills_dir`（`tools.rs:338-386`）；重置 `reset_custom_tool_project_path`（`tools.rs:402-422`）。
- 归一化 `normalize_project_relative_skills_dir_input`（`core/tool_service.rs:286-306`）：必须相对路径、禁止 `..`、空值等于清除。内置存 `custom_tool_project_paths`，自定义存定义字段。
- 解析：`project_relative_skills_dir()`（`tool_adapters.rs:191-195`）无覆盖时回退 `relative_skills_dir`。
- 边界：新需求「项目专属技能管理当前暂不纳入」，只保留该**配置项**；旧的项目工作区扫描/部署/启停不在本文件范围。

## 2. Agent 指令源与分发

拼接规则已在 `content-composition.md`「第二轮补充：Agent 指令合成」写明，此处只补**落点、分发范围与 GUI 状态**。

### 2.1 源内容落点（源码事实）

`core/agent_instructions.rs` 顶部注释：源是纯文件，`<central_repo::base_dir()>/instructions/` 下 `global.md` + `<agent_key>.md`，**不进 SQLite**。`instructions_dir() = base_dir().join("instructions")`（`agent_instructions.rs:29-31`）。`save_sources` 写这两类文件，空内容也写空文件（`agent_instructions.rs:74-86`）。`read_source`（`agent_instructions.rs:42-52`）：文件不存在 → 空串；其它读取失败**也折叠为空串**并 warn（这是 `content-composition.md` 已标出的不能复用点）。

**新需求差异（事实冲突）**：旧 `base_dir()` 不是 Git 跟踪根（Git 备份根是 `skills_dir()`，`git_backup.rs:282-290`），`instructions/` 与 `skills/` 是同级、在当前备份仓库之外，因此旧指令源**不随库同步**。新需求明确「指令文件的源内容也放进 SkillManagerVault，跨电脑同步」。旧存储位置不满足，需重新设计；旧只提供「源即文件、非 DB」这一模型。

### 2.2 拼接

`compose_instructions`（`agent_instructions.rs:58-67`）：两侧 trim 后「全局 + 一个空行 + 专属」；单侧为空返回另一侧；双空返回空串。测试固化四态（`agent_instructions.rs:175-193`）。

### 2.3 分发目标范围（源码事实）

`write_all_agents_unlocked`（`agent_instructions.rs:109-116`）遍历 `enabled_installed_adapters(store)`——与设置页「已启用」**同一集合**（已检测 且 未停用）。对每个 Agent 直接覆盖写 **`<detect_dir>/AGENTS.md`**（注意是 detect_dir，不是 skills_dir），父目录不存在则创建；单个失败记入该 Agent 的 `outcome.error`，不中断其余（`agent_instructions.rs:119-138`）。

- GUI `save_instructions`（`commands/agent_instructions.rs:88-121`）：在同一个 `with_repo_lock` 临界区里先 `save_sources` 再 `write_all_agents_unlocked`。
- CLI `instructions push` 走 `deploy_agent_instructions`（`agent_instructions.rs:142-146`）取锁后同一函数；单 Agent 路径 `deploy_agent_instructions_for`（`agent_instructions.rs:151-164`）只写显式指定 key，**不按禁用收窄**。
- 默认范围 = 已检测且启用的 Agent；测试 `deploy_skips_disabled_agents`（`agent_instructions.rs:302-352`）固化「disabled 的不写、未检测的不写」。

### 2.4 GUI 编辑器状态保持（源码事实）

`components/InstructionsDialog.tsx`：

- 左列「全局指令」虚拟项固定第一项、默认选中（`selectedKey` 初值 `null`，第 28 行）；搜索只过滤 Agent 段（54-62）。
- **同一时刻只有一个编辑区**：选中全局显示全局框，选中某 Agent 切到该 Agent 专属框（181-189）。
- 各 Agent 的 content 存在 `agents` state 中，`updateSelectedContent` 只改当前项（66-71），切换工具时未保存内容留在 state 里 → **切换工具保留未保存内容**。
- 只有底部一个「保存」按钮（194-202）；保存后弹窗保持打开（90 行注释，handleSave 无 onClose 调用）。
- 读取源 `get_instructions_sources`（`commands/agent_instructions.rs:36-57`）只列已启用口径的 Agent。

这与需求「沿用旧版指令界面与编辑方式」一致：左搜索+选择、右单编辑区、底部单保存、保存自动拼接、切工具保留。

## 3. 外部修改自动刷新（A11）

`core/file_watcher.rs`。链路：notify 事件 → 后端 emit `app-files-changed` → 前端 `AppContext` 监听刷新。

### 3.1 监听什么

`collect_watch_paths`（`file_watcher.rs:154-212`）：

- `central_repo::skills_dir()`、`lobster_skills_dir()`、`scenarios_dir()`（两个物理根都监听，BACKUP-03）。
- 每个 adapter 的 `skills_dir()` + `all_scan_dirs()`。
- 项目工作区目录（linked / project）。
- watcher 以 `RecursiveMode::Recursive` 监听（`file_watcher.rs:251`）。

覆盖判定：`.manager` 与技能内容都在 skill 目录（`skills_dir` 的子目录/文件）下，被根递归监听覆盖；**目录移动**在父根产生事件，另有 3 秒轮询重扫兜底（`WATCH_RESCAN_INTERVAL=3s`，`file_watcher.rs:12`）。

### 3.2 去抖与自写抑制

- `WATCH_EMIT_DEBOUNCE = 500ms`：合并事件，最多每 500ms 发一次（`file_watcher.rs:13`）。
- `SELF_WRITE_MUTE = 1200ms`：应用自身写入路径的记录窗口内，其回声判为 `SelfWrite → Skip`；被归类为 `Foreign` 的事件会延后补发；这不是按真实写入者识别。见注释 15-35、`decide_emit`（108-125）、测试 483-542。动机 #248：应用自己 sync 的写会回声成 `app-files-changed`，造成刷新风暴。
- 分类按路径前缀（`classify_mute` 66-84）：窗口内落在写入根下的事件视为自写，其他路径事件延后上报；无路径信息（重扫）按 `Foreign` 处理。它不识别真正写入者，因此外部 Agent 恰好改同根时也可能被当成自写，不能声称所有并发外部改动都不丢。
- `should_emit`（265-274）丢弃纯 `.git/` 子树事件，避免 fetch 回环。

### 3.3 前端刷新链路

`src/context/AppContext.tsx:373-412`：`listen("app-files-changed")` → 500ms 去抖 → **先** `api.refreshManagerIndex()`（`commands/skills.rs:478` → `refresh_manager_index_internal` 重建 `ManagerIndex`，`core/manager_index.rs:67`）→ `finally` `refreshAppData()`。

顺序重要（注释 380-385）：`get_managed_skills` 的 tags/group/supported_agents 从索引取，先列后建会显示改动前的旧值，即「外部编辑要重启才更新」的旧问题。旧托盘预设 apply 成功后也会 emit `app-files-changed`（`lib.rs:718`）。

### 3.4 覆盖能力小结

`.manager` 编辑、技能内容编辑、目录移动都被覆盖，索引不持久化（`manager_index.rs` 顶部及测试 523-544）每次重建，因此外部改动无需重启即被读取。这是 A11 的界面刷新链路，可独立借鉴；它不等于“外部改动已同步到所有部署”。

## 4. 偏好与托盘

### 4.1 偏好存储与默认值（源码事实）

都存在本地 SQLite settings（前端另有 localStorage 镜像），取值见 `Settings.tsx:330-353`、`AppContext.tsx:347`：

| 偏好 | 设置键 | 旧默认 | 位置 |
|---|---|---|---|
| 主题 | `theme` | **`dark`**（无 localStorage/setting 时） | `src/hooks/useTheme.ts:25-30` |
| 字号 | `text_size` | `default`（未设置时不改 zoom） | `src/lib/textScale.ts`；`AppContext.tsx:347-350` |
| 界面语言 | `language` | 按 OS locale 探测 zh/zh-TW/en | `src/i18n/index.ts` |
| 关窗行为 | `close_action` | 空 = 弹选择框；macOS 直接隐藏 | `src/components/CloseActionGuard.tsx:11-37` |
| 托盘图标 | `show_tray_icon` | 缺省视为开 | `Settings.tsx:334-337` |

- 主题：浅/深/跟随系统三值；`system` 用 `matchMedia`（`useTheme.ts:9-13,41-47`），`resolvedTheme` 给 `<html>` 加 `.dark`。**默认值是 `dark`，与需求「默认跟随系统」不一致**——新实现需改默认。分段按钮 UI 在 `Settings.tsx:1630-1652`。
- 字号：`TEXT_SIZE_SCALE_MAP = {small:0.9, default:1, large:1.1, xlarge:1.2}`，作用于 `document.documentElement.style.zoom`（`textScale.ts`）；分段按钮 `Settings.tsx:1654-1683`。
- 语言：三语分段 `Settings.tsx:1685-1710`（剥离为简体中文）。
- 关窗：`CloseActionGuard` 监听后端 `window-close-requested`；非 macOS 先读 `show_tray_icon`（关托盘则直接退出），再读 `close_action`：`close`→退出、`hide`→`hideToTray`、空→弹框询问并可选记住。**新需求是「正常关闭一律收起到托盘」，旧的三态选择+询问框是差异点。** macOS 分支为「关窗即隐藏窗口」。
- 这些偏好与新配置共用同一设置分区、白色面板、分段按钮样式（需求「设置页沿用与统一」）。

### 4.2 托盘（源码事实）

`ensure_tray_icon`（`src-tauri/src/lib.rs:869-952`）：macOS 左键默认开菜单，其它平台左键 `restore_main_window`（932-947）。

菜单项（`build_tray_menu_from_data`，`lib.rs:420-586`）：应用名、状态行、有更新时的「有 N 个 Skill 更新」（`TRAY_OPEN_UPDATES_ID`）、**预设子菜单（Code/Lobster 两组，剥离项）**、打开 Skills Manager（`show`）、检查 Skill 更新（`TRAY_CHECK_UPDATES_ID`）、打开 Skill 文件夹、退出（`quit`）。

事件分发（`lib.rs:885-915`）：

- `show` → `restore_main_window`；`quit` → `request_quit`。
- `TRAY_OPEN_UPDATES_ID` → 恢复窗口 + emit `tray-open-updates`（`AppContext.tsx:356` 监听并跳转 `/my-skills`）。
- `TRAY_CHECK_UPDATES_ID` → `check_updates_from_tray`。
- 其它 → 旧预设 id（剥离）。

`check_updates_from_tray`（`lib.rs:735-826`）：**不打开主窗口即可检查**。`TRAY_CHECK_UPDATES_RUNNING` 防重入；逐 skill `prefetch_skill_remote` + `check_skill_update_internal`，每次 sleep 200ms 让出锁、锁不跨越网络往返（#315）；完成后 `record_round_completion` 发 `skills-auto-updated`，前端 `AppContext.tsx:527` 监听刷新。

关闭收托盘：`lib.rs:1281-1298` 拦截 `CloseRequested`，`QUITTING` 时放行，否则 emit `window-close-requested` + `prevent_close()`，前端按 §4.1 决定隐藏/退出。

托盘刷新去抖：`schedule_tray_refresh` 300ms 合并（`lib.rs:626-643`）。

对应新需求「托盘提供重开窗口及直接检查上游更新，无需先打开主窗口」：可直接沿用「检查更新不依赖窗口 + 完成后回刷」这条链路。**剥离项**：托盘预设子菜单（`lib.rs:490-550`）与 `apply_preset_from_tray`（`lib.rs:645-733`）。

## 5. 未受管本地技能的发现与删除（旧 F04）

- 发现：`get_global_local_skills`（`commands/agent_workspace.rs:147-167`）用 `project_scanner::read_linked_workspace_skills(adapter.skills_dir(), ...)` 扫目录，再 `enrich_center_status`（`agent_workspace.rs:57-75`）标记 `in_center` / `center_skill_id` / `sync_status` / `tags`。
- 受管匹配：`find_verified_center_match`（`agent_workspace.rs:118-145`）：先按 `source_ref` 或 target_path 匹配（得分 3），再按 `content_hash` 匹配（得分 2），取最高分。
- 显示：`WorkspaceView` 用 `ProjectSkillInfo` 渲染；「仅本地」等文案在 `src/i18n/zh.json:1194`。
- 删除：前端 `deleteGlobalLocalSkill`（`src/lib/tauri.ts:1171`）→ 命令 `delete_global_local_skill`（`agent_workspace.rs:781`）→ `delete_agent_local_skill`（`agent_workspace.rs:793-819`）：
  - 若命中受管（`find_verified_center_match` 且 target 仍指向该路径）→ **拒绝**，提示先从 Agent 移除受管部署。
  - 否则 `ensure_agent_skill_path` 限定路径必须落在 `skills_root` 内，再 `sync_engine::remove_target(target_path)` 删实际文件。
- 多选删除：`WorkspaceView` 有 `batchDeleteConfirm` / `selectedDeletable`（`WorkspaceView.tsx:941-951`、`1124-1133`），逐项调用同一命令（`WorkspaceView.tsx:403`）。
- Agent 维度：命令按 agent key 取 `adapter.skills_dir()`；`_global` 共享目录也走同一命令（`adapter_for_agent` 经 `find_adapter_with_store`，注释 `agent_workspace.rs:30-36`）。
- 局限：这是旧「Agent 工作区」视图的删除，依赖 `skill_targets` 表与 `content_hash`。新需求「直接删除 Agent 中未受管本地技能」可参考「路径必须落在 skills_dir 内 + 受管则拒删」，不必沿用整套 content_hash 匹配。

## 6. 已确认边界与待设计细节

指令源进 SkillManagerVault、默认跟随系统、关窗收托盘、托盘重开/退出/检查更新均已确认，
不因旧实现不同而重新询问。

技能目录修改后的已有部署协调、有路径覆盖时如何表达“已检测”、托盘具体视觉设置仍需设计。
这些只保留旧实现线索，不默认继承“改目录先删旧副本再按当前预设重建”或“禁用 Agent 就撤销全部部署”。
Agent 检测目录需按实施时实际安装核对；原型计数不是检测结果。

## 未涉及（供查漏）

- SkillManagerVault 同步 / Git 引擎、上游检查与更新应用（其他子代理负责）。
- Hermes 档案、部署合成与覆盖层、预设、工具中心、CLI 命令语法细节。
- 内容合成、中文注释注入与覆盖层合成（见 `content-composition.md`）。
