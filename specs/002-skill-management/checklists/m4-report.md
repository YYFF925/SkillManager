# M4 验收报告：跨电脑同步与指令

**日期**：2026-10-16 | **范围**：T050～T055、T069、T070、T072 | **依据**：[quickstart.md](../quickstart.md) M4、[spec.md](../spec.md) US5/US8、[tasks.md](../tasks.md) Phase 7/10

**证据说明**：结论来自本机实际执行（两个临时克隆 + 两个 `--state-dir` + 临时裸远端 + 临时 HOME）；GUI 证据为真实 Tauri 应用截图，位于 `/tmp/m4-gui/shots/`（1720 × 1040）。真实用户目录未被写入（已核对 `~/.codex/AGENTS.md` 等 mtime 早于本会话）。**未取得证据的项记为未验证。**

## 一、场景核对表

| 场景号 | 预期 | 实际 | 结论 |
|---|---|---|---|
| 4.1 机 A：GUI 保存修改 | 自动提交推送自己的库；来源仓库无变化 | GUI 保存指令源后：**先刷新已有部署**（结果行"已刷新 Codex → …/AGENTS.md"）→ 首次弹出"保存后自动上传本机 SkillManagerVault？"→ 确认后 `git log` 出现"更新 SkillManagerVault（Skill Manager 上传）"并已推送远端；无改动时如实显示"没有需要上传的改动，已是最新"（截图 `win_instruction_saved.png`、`win_a_after_upload.png`） | 通过 |
| 4.2 机 A：Agent 修改 → `sync push`；已同意上传失败后主动重试 | 仅获同意才上传；主动重试可继续；后台不提交不推送 | CLI 实测：上传失败保留可读原因、用户主动重试成功；`push_commits_all_workspace_changes_to_own_vault_only` 断言工作区**全部**改动进入提交且来源仓库 HEAD 不变；`background_and_readonly_paths_do_not_push` 断言 `deploy sync`/`sync status` 不推送 | 通过 |
| 4.3 机 B：`sync pull`（无冲突） | 静默应用 + 已有部署刷新；库内新增技能**不**自动部署 | 拉取后 B 的已有部署被刷新，**新增技能只入库、未部署**；内存索引重建 | 通过 |
| 4.4 制造双端冲突 → pull | 同步暂停+提示，`sync conflicts` 输出详情；修复后继续 | `sync pull` 返回 `partial` 并给出未合并路径；Git 现场保留；处理后再次 `sync pull` **完成继续**（不新增合并界面） | 通过 |
| 4.5 断开远端 | 显示未连接；本地管理、部署、`deploy sync` 正常 | `sync status` 返回 `disconnected`；本地命令与 `deploy sync` 继续可用 | 通过（CLI）；**GUI"未连接"截图未取** |
| 4.6 机 B 已有本机部署记录/档案启用 | 拉 A 的库变化后 B 的部署记录、启用名单、Agent 配置不变 | 两个独立 `--state-dir` 下 B 的本机状态未被库内容影响（与 M1 `context::tests::separate_state_dirs_model_two_computers` 及 M2/M4 测试一致） | 通过 |
| 4.7 `instruction set <key> --file`；GUI 单编辑区单保存 | 目标 `AGENTS.md` = 全局前 + 专属后覆盖；无专属用全局；清空可保存；无独立分发勾选/双按钮，保存后保持打开、切换工具保留草稿 | CLI：拼接"全局 + 空行 + 专属"、清空可正常保存。GUI（截图 `win_instruction_global.png`、`win_instruction_codex.png`、`win_draft_codex.png`、`win_draft_back_global.png`）：单编辑区、全局默认选中、无专属显示"仅使用全局指令"、切回全局草稿 `UNSAVED DRAFT GLOBAL` 保留、单保存按钮、保存后保持打开 | 通过 |
| 4.8 机 B 拉取到指令源变化 | 按 B 已有启用配置/指令路径自动刷新；不新增启用目标、不改本机路径；失败如实显示 | 机 B 拉取后 `~/.codex/AGENTS.md` 自动刷新为新全局内容；**停用的 zcode 未被写入**、启用名单未改变、未新增目标（截图 `win_b_after_sync.png`、`win_b_instruction.png`）；Tauri 薄封装命令 `settings_get`/`settings_set_auto_check`/`settings_set_git_auto_push` 只做读写 | 通过 |

## 二、指令与同步的边界核对

- **拼接四态**：双空 / 仅全局 / 仅专属 / 两者（"全局 + 一个空行 + 专属"），core 单测断言。
- **读取失败与不存在分开**：源文件不存在=空内容；真实读取失败返回错误，**不用空内容覆盖原有设置**。
- **分发范围**：默认 `<detect_dir>/AGENTS.md`，可用本机 `instructions_target` 覆盖；指令路径与技能路径独立；单个目标失败不中断其余并逐项报告。
- **上传授权**：只有"GUI 保存按已确认策略"与"Agent 获明确同意后显式调用"两条路径可达（`src/lib/librarySave.tsx` 固定顺序：先 `deploySync` → 读 `git_auto_push` → 未确认则弹一次确认 → `syncPush`）。外部文件监听只刷新界面、不经过该流程；`deploy sync` 与纯部署动作不触发上传。
- **调度**：`auto_check_minutes` 逐字落实 0/30/60/360/1440、默认 60；启动/手动总执行，定时门控；关闭定时不禁用手动。工具检查按设计登记为 `not_implemented`（待 T077），**未冒充已执行**。

## 三、GUI 视觉与交互对照

| 原型页面/状态 | 实际截图（`/tmp/m4-gui/shots/`） | 结论与差异 |
|---|---|---|
| 全局指令 14 / Cursor 专属 15 | `win_instruction_global.png`、`win_instruction_codex.png` | 通过：左列"全局指令"固定第一项并默认选中、右侧单编辑区、底部单保存按钮、合成预览 |
| 切换工具保留草稿（原型 README 第 8 条） | `win_draft_codex.png`、`win_draft_back_global.png` | 通过：无专属时显示"仅使用全局指令"，切回全局后未保存草稿仍在，保存后弹窗保持打开 |
| 设置页 19 | 设置页截图 | 部分通过：共同检查频率分段按钮（关闭/30 分钟/1 小时/6 小时/24 小时，默认 1 小时，注明三项共用）、自动上传确认状态、SkillManagerVault 路径、共享技能目录行内修改、同步状态分区，分区行与分段按钮样式一致。**差异**：本期未放主题/字号（属 M5 T083）；Agent 检测分区在 Agent 页（M2 已有），不重复 |
| Agent 询问上传 29 | `win_instruction_saved.png` | 通过：首次保存后询问"保存后自动上传本机 SkillManagerVault？"，确认后沿用 |

**未取得界面证据、记为未验证**：**"未连接"状态**与**冲突提示**的界面渲染（本次远端始终可达、未制造界面级冲突；两种状态的渲染已实现，并有 core/CLI 层面的覆盖）；并发多保存未串行化（可能并发触发 deploySync/push 并出现 `index.lock`），按"影响可见、可恢复"接受，未额外加锁。

## 四、自动化验证

| 命令 | 结果 |
|---|---|
| `cargo test --workspace` | 通过：175 项（M3 为 162；新增 `cli_sync` 6 项 + core 单测） |
| `cargo build --workspace` | 通过，0 警告 |
| `npx tsc --noEmit` / `npm run build` | 通过 |
| `npm test`（vitest） | 通过：4 文件 / 13 用例（新增 `InstructionEditor.test.tsx`、`librarySave.test.tsx`，覆盖单编辑区/草稿保留/保存保持打开、部署刷新先于上传、首次确认后沿用、拒绝则不上传、失败保留原因） |

## 五、遗留与待后续复核

1. **M2 遗留项已收口**：M2 报告第五节第 5 条（`agent set-dir --instructions` 的 GUI 入口）已由本期指令编辑器承接。
2. **GUI"未连接"与"冲突"状态的界面证据**未取得，需人工复核。
3. **定时器运行时行为**未在真实桌面长时间实测（`spawn_scheduler` 已接入 Tauri：启动一次 + 每 60 秒检查周期）；quickstart 5.8 属 M5，届时按缩短周期验证。
4. **工具版本检查**登记为 `not_implemented`，待 M5 T077。
5. **SC-005 / SC-006** 已在本期获得主要证据（后台路径不推送、上传两条路径、拉取不自动首次部署/不自动启用）；M5 收尾时按 T090/T091 再整体复核。
6. 未执行 `git commit` / `git push`（仅 /tmp 内临时仓库在验证中产生了提交与推送）。
