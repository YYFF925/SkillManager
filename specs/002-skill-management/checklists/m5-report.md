# M5 验收报告：预设、工具中心、界面环境与整体收尾

**日期**：2026-10-16 | **范围**：T062～T066（US7）、T074～T082（US9）、T083～T087（US10）、T088～T092（配套 Skill 与整体收尾） | **依据**：[quickstart.md](../quickstart.md) M5、[spec.md](../spec.md) US7/US9/US10 与 SC-001～008、[tasks.md](../tasks.md) Phase 9/11/12/13

**证据说明**：结论来自本机实际执行（临时 HOME + 临时 SkillManagerVault + 临时 `--state-dir` + 临时本地上游/裸远端 + 假 Agent 目录 + 假 Hermes 根 + 假工具检查脚本）；GUI 证据为真实 Tauri 应用截图，位于 `/tmp/m5-preset-env/`、`/tmp/m5-tools/`、`/tmp/m5-final/`（1720 × 1040）。真实用户目录未被写入。**未取得证据的项记为未验证。**

## 一、场景核对表（5.1～5.13）

| 场景号 | 预期 | 实际 | 结论 |
|---|---|---|---|
| 5.1 预设含 B,C 部署到已有 A,B 的目标 | 结果 A,B,C —— 叠加 | 叠加后目标为 alpha/beta/gamma；`deploy preset` 复用 US3 的域/支持过滤与暂停规则（证据 `/tmp/m5-preset-env/cli/5.1-5.2.txt`） | 通过 |
| 5.2 `deploy undo-preset <domain> <id> --from <target>` | 剩 A —— 按成员撤销，跨预设成员也撤销；两域同 id 不误选 | `undo-preset audit` 后仅剩 alpha（共享成员 beta 一并撤销）；`preset` 命令恒以 `(domain, id)` 为完整引用，不静默选域 | 通过 |
| 5.3 编辑预设成员 | 定义变化，部署不变；机 B 可见同一定义 | 机 A 编辑成员后本机部署记录不变；机 B `sync pull` 后读到同一定义（`5.3-5.12.txt`） | 通过 |
| 5.4 新电脑接入已有工具的库 | 未安装工具显示"未安装"（可暗显），不隐藏 | 工具清单中未安装的 ripgrep 常驻显示并暗显（截图 `02-tool-center.png`） | 通过 |
| 5.5 `tool register`/`set` 后 `tool check`；含未安装工具 | 保存检查结果并展示版本；未检查/失败/未登记命令/可更新分别表达；未安装工具获取最新版本仍显示未安装；安装事实不变 | 检查全部 → "可更新 2 个；检查不安装、不更新"（截图 `03-check-all.png`）；`tool check` 只写检查事实，不改 `tool_installs`；未登记命令 → `no_command` | 通过 |
| 5.6 `tool task --action install` → Agent 安装 → `tool set-install` | 任务含配套维护 Skill 库内路径与"按其规范执行"；生成≠已完成；回写后为真实版本，检查状态重置未检查 | 任务文本含"配套维护 Skill：未入库（…不编造库内路径）"与回写命令，剪贴板实测写入同一文本（截图 `05-tool-task.png`）；CLI 原文证据 `tool-task-cli.txt` | 通过 |
| 5.7 配套 Skill 候选入库 | 列出具体候选与资料，勾选入库，不自动部署 | 候选页显示原名/注释/用途/来源/勾选与已选数量；入库经 `previewRepo`→`importSkills`，**不部署**（截图 `07-tool-skills-tab.png`；jsdom 测试断言只调用预览与入库） | 通过 |
| 5.8 定时器（缩短周期验证） | 到期自动执行上游检查 + 库同步 + 工具检查，结果分别提示 | 诊断日志实测一次调度记录：`startup 调度：上游 ok，库拉取 error，工具检查 ok`（拉取 error 是临时库无远端，如实记录）。**定时到期路径未实测**——产品最短周期为 30 分钟，会话内无法到期，且明确不新增第四套周期；门控逻辑由 `scheduler::is_due` 单测覆盖 | **部分通过**：三类结果分别表达已验；定时到期未实测 |
| 5.9 主题/字号/关窗收托盘/托盘检查/诊断导出/容量报告 | US10 各场景 | 主题（浅/深/跟随系统，默认跟随系统，系统切换实时反映）、字号（小/默认/大/特大）持久化到本机 SQLite（截图 `win_settings_dark.png`、`win_theme_follow_system_dark.png`/`_light.png`）；关窗隐藏（窗口数 0、进程存活）、**隐藏期间**调度日志仍产生（`20:17:19 timer 调度`，隐藏时刻 20:15:38）、托盘"检查上游更新"产生 `manual 调度` + `tray.check`、托盘重开窗口、托盘退出终止进程（`hidden_state_fullscreen.png`、`win_reopened_from_tray.png`、`final_tray_menu.png`）；诊断面板显示真实日志路径并可导出（`win_diagnostics_export_notice.png`）；容量报告"库总占用 396 B（5 个文件）· 技能 4 个合计 234 B"及带路径排行（`win_settings_capacity.png`） | 通过（**崩溃记录未在真实 panic 下观察**，写入逻辑有 core 单测） |
| 5.10 配套维护 Skill 从本项目远端普通入库 → 推进版本 → `check`/`update` | 走普通上游流程，来源记录正确；任务书引用路径可解析 | 2026-10-17 T124：获授权后发布真实 Gitea，从 `c00e1a4` 入库并经普通检查/更新采用 `e2106f9`；`references/cli.md` 实际更新，来源登记及两类任务路径核查通过，见第十二节 | **通过** |
| 5.11 **SC-001 完整旅程** | 入库→维护→部署→吸收一次上游更新→上传→机 B 拉取继续使用 | 六步全绿（`/tmp/m5-t091.5F35so/journey.txt`）：入库（原名保留、来源登记）→ 维护（注释/标签/`skill move` 实际移动目录/译文写入 `translation_source_hash`）→ 部署（产物 `description: 演示技能（已维护）｜演示技能 v1`，无 `.manager`、无译文）→ 上游更新（check 报有更新且原文未变；update 采用 v2，上游同项覆盖、分类保留、译文保留、新增 `notes.md`；`deploy sync` 刷新已有部署）→ 上传（`sync push` 提交并推送整个工作区）→ 机 B `sync pull`（刷新已有部署 1 项，**未新增自动部署**） | 通过 |
| 5.12 预设间顺序与成员顺序（GUI + `reorder-members`），另一机拉取 | 两种显示顺序均共享；成员排序只改数组顺序，不增减成员、不动目录、不改已有部署 | 机 B 拉取后可见 audit→release 与 release 成员 `[gamma, beta]`；成员排序未增删成员、未动目录、未产生部署 | 通过（CLI）；**GUI 拖动排序未取得真实拖动证据**（见第四节） |
| 5.13 单技能与多技能 `skill task` | 上下文完整可定位；保留用户要求、库/技能路径、管理资料概况与已有部署；规范只引用维护 Skill；生成不改库/部署/上传 | 单项与多选任务文本均含上述上下文（`skill-task-cli.txt`）；生成过程不改库、不部署、不上传 | 通过 |

## 一之二、场景核对表（5.14～5.19，2026-10-16 增量 T093～T099）

**证据**：临时库 + 临时 HOME + 独立 `--state-dir` 的 CLI 实跑（`settings` 命令组）；自动化断言见 `skill-manager/tests/cli_settings.rs`（8 项）与 `skill-manager/crates/core/tests/runtime_settings.rs`（3 项）。真实 `~/.codex`、`~/.zcode`、`~/.hermes`、`~/.agents` 与真实 Application Support 均未写入。

| 场景号 | 预期 | 实际 | 结论 |
|---|---|---|---|
| 5.14 `settings get`（不带/带参数） | 全部键的当前生效值（主题缺省 `system`、频率缺省 60、共享目录未设置为 `null`）；带参数只返回指定键；不省略键、不用空字符串冒充 | `settings get`（无参）返回 8 键：`theme=system`、`font_scale=default`、`auto_check_minutes=60`、`shared_dir=null`、`git_auto_push=false`、`vault_path=<临时库绝对路径>`、`last_sync_at=null`、`last_check_at=null`；`settings get theme shared_dir` 只返回两键。自动化断言同上文件 | 通过 |
| 5.15 `settings set-shared-dir --path` → `deploy to shared`；`--clear` 后 | 设置后共享部署成功；清除后如实报"尚未设置共享目录路径" | `set-shared-dir --path <临时目录>` → `deploy to shared code/alpha` 成功（`items[0].action=deployed`，副本 `SKILL.md` 实际存在）；`--clear` → `deploy to shared` 退出 1，`errors[0].message` = "目标无法解析：尚未设置共享目录路径（shared 是部署目标，不是第二个管理库）" | 通过 |
| 5.16 `set-auto-check` 取值 | 0/30/60/360/1440 接受并读回；45 被拒绝并给出合法取值 | 0 与 30 写入后 `settings get auto_check_minutes` 分别读回 0 与 30；`set-auto-check 45` 退出 1，报"自动检查频率只支持 0（关闭）/30/60/360/1440 分钟，收到 45" | 通过 |
| 5.17 `settings diagnostics`、`--export`、`settings capacity` | 诊断含版本/平台/真实日志路径；`--export` 写出真实文件并返回路径；容量输出总占用与排行，空库 0 与空排行 | 诊断输出"版本：0.1.0；平台：macos / aarch64"与真实 `运行日志：<state-dir>/logs/skill-manager.log`、`崩溃记录：.../crash.log`；`--export` 返回 `data.path` 指向实际存在文件（`<state-dir>/logs/diagnostics-*.txt`）；容量非空库报 1 项技能占用 8 字节 + 排行，空库（无技能）报 `total_size=0`、`file_count=0`、`skill_total_size=0`、`skills=[]` | 通过 |
| 5.18 不允许的写入边界 | `theme`/`font_scale`/`git_auto_push` 无 CLI 写入口（命令不存在或明确不支持），只能经 `settings get` 读取 | `settings set-theme`/`set-font-scale`/`set-git-auto-push` 均不存在，调用退出 2（clap `unrecognized subcommand`）；三者经 `settings get` 可读。工具安装事实回写仍只有 CLI（GUI 无入口，未改动） | 通过 |
| 5.19 清理项核对 | 不再存在自定义日志目录键；Hermes 根按 `HERMES_HOME` 与平台默认解析，`settings get` 不再返回可写 Hermes 根键 | `settings get` 不含 `log_dir`、`hermes_root`；`HERMES_HOME=<临时目录>` 时 `hermes profiles` 报 `root=<临时目录>`、`skills_dir=<临时目录>/skills`；core 断言（`runtime_settings.rs`）覆盖 `HERMES_HOME` 优先/平台默认回落、忽略残留 `hermes_root` 键、日志固定落 `<state-dir>/logs` | 通过 |

**验证方式说明**：本批不含 GUI 改动；`settings` 命令组与 GUI 设置页共用同一 core（诊断/容量/共享目录校验均走 core）。界面偏好（主题/字号）与上传长期授权仍仅 GUI 写。

## 二、SC-001～SC-008 结论

| 项 | 结论 | 证据 |
|---|---|---|
| SC-001 完整旅程 | **通过** | 5.11 journey（六步全绿） |
| SC-002 核心操作均有 CLI 路径 | **通过，除缺口①** | `contracts/cli.md` 的 T090 双入口核对表（15 行逐项）；维护 Skill 文档中 53 条调用 / 91 个 `--flag` 与真实 `--help` 逐条一致；106 条子命令实跑（103 通过 + 3 条有意失败）；前端 85 个调用名与 Tauri 87 个注册命令逐一对齐 |
| SC-003 部署内容边界 | **通过** | journey 第 3 步产物有注释注入、无 `.manager`、无译文、库内原件未变；M2 `deployment.rs` 11 项 + `cli_deploy.rs` 8 项 |
| SC-004 状态表达诚实 | **通过** | journey 中 `[译文待更新]`、任务"未入库"提示、`tool check` 的未检查/失败/未登记命令分别表达；`19c-settings-diag.png` 如实展示 error 日志 |
| SC-005 上传路径 | **通过** | journey 第 5 步：只有显式 `sync push` 才提交推送；M4 断言 `deploy sync`/`sync status` 等后台与只读路径不推送 |
| SC-006 跨机边界 | **通过** | journey 第 6 步：B 拉取后仅刷新已有部署 1 项，无自动首次部署/档案启用/工具安装；B 的部署记录与 Agent 配置不变 |
| SC-007 上游更新保留规则 | **通过** | journey 第 4 步 + M3 `upstream_merge.rs` 6 项 + `cli_upstream.rs` 9 项 |
| SC-008 较大仓库获取 | **通过** | CLI 原有 287 MB 证据保留；2026-10-17 T123 已用真实 `.app` 补齐 128 MiB 传输的在屏进度、取消后未入库、失败提示及成功入库，见 M3 报告末节 |

## 三、性能实测

环境：Apple M 系列 arm64 / macOS 27.0.1 / **debug 构建**（`/tmp/m5-perf.vd2XML/perf.txt`）。

| 规模 | `skill list` 全库 | 分类筛选 | 文本搜索 | 标签筛选 | `skill get` | `skill check --all` |
|---|---|---|---|---|---|---|
| 50 技能 | 0.02s | 0.01s | 0.01s | 0.01s | 0.01s | 0.01s |
| 200 技能 | 0.03s | 0.03s | 0.04s | 0.03s | 0.03s | 0.03s |
| 500 技能 | 0.07s | 0.08s | 0.06s | 0.05s | 0.05s | 0.05s |

这是本机 debug 构建的实测值，按 plan.md 的要求**不作为**"任何环境 <1s 或微秒级"的验收承诺；release 构建未测。

## 四、GUI 视觉与交互对照

对照基线：原型页面索引（`specs/001-skill-management-prototype/ui-prototype/README.md`），同尺寸 1720 × 1040。

**已实现并取得本次截图证据**：技能库（含深色主题 `01-library-code-dark.png`）、龙虾技能库、列表密度、技能详情（正文/管理资料/文件与部署/来源）、Agent 本地技能与空状态、工具中心清单、设置页外观/检查频率/上传确认/库路径/共享目录、设置页同步状态（含**"未连接"**：未连接徽标、远端未配置、分支无上游跟踪、库变化 3 项、提示"不提供断开远端/重新克隆修复入口"）、设置页容量与诊断、深色主题（`/tmp/m5-final/`，13 张）。

**已实现但本次未取得界面证据（保持未验证）**：卡片三点菜单、多选批量栏、入库弹窗（预览/确认）、部署弹窗（目标分组/结果面板）、上游更新与新增决定页、跳过/已跳过列表、收编资料弹窗、预设页与新建预设、工具详情/指南/配套 Skill/任务、指令编辑器、技能维护任务对话框。原因：WKWebView 的合成点击在本环境不稳定（同一坐标有时命中有时不命中，模态高度变化会移动控件）。这些界面的早期证据见 m1～m4 报告（部分已取得），**本次不重复声明通过**。

**原型不提供的能力（依据）**：首页/项目管理/在线市场（原型 README 已确认不恢复）；档案生命周期与 `SOUL.md`、按预设分配档案（spec Assumptions 首版范围排除）；原型的"电脑 A/B 切换、系统切换、重置、演示 Agent 回报"是演示机制而非产品能力；Windows 模拟未验证（**不声明平台兼容**，仅 macOS arm64 实测）。

## 五、m1～m4 待验项回填

| 来源 | 待验项 | 回填结论 |
|---|---|---|
| m1 | 1.1 首次选库原生对话框、1.3 切原文界面、1.4/1.6 GUI 保存落盘 | **仍未取得界面证据**（保持未验证） |
| m1 | `--preset`/`--deployed-to` "尚未接入" | 说法已失效：`--deployed-to` 与 `--preset` 均已接真实数据；`skill list` 的过时帮助文字已更正 |
| m1 | 1.7 同名检查 | 已在 m3 复核通过（保留） |
| m2 | Windows 模拟、部署弹窗结果面板、拖动排序与删除确认弹窗 | **仍未验证** |
| m2 | 共享目录缺 CLI 命令 | 已确认并记录为缺口（见第六节），未自行发明命令 |
| m2 | Hermes 符号链接 | 仍只有源码级依据，**未启动 Hermes 实测** |
| m3 | GUI 侧进度/取消、`update --to` 非最新提交、复制对话框、重新预览保留勾选 | **仍未验证**；本次 journey 实测了 `check → update` 与新增候选的完整链路 |
| m4 | GUI "未连接"界面 | **本次已补取证**（`19b-settings-lower.png`、`19e-settings-sync-warning.png`） |
| m4 | 冲突界面渲染、定时器长时行为 | **仍未验证** |
| m4 | 工具版本检查登记为 `not_implemented` | 已失效：`tool check <slug>`/`--all` 已实现并可写检查事实（不改安装事实） |

## 六、未闭合缺口与待用户裁决项

> **2026-10-16 更新**：第 1 条经用户要求按类重扫后**已升级为规划补齐项**（不只是共享目录一项，
> 而是"本机运行配置与只读报告"整类）。规格与命令契约已补齐，见 `plan.md` 的"规划补齐"一节、
> `research.md` D22/D23、`contracts/cli.md` 的 `settings` 命令组与本报告新增的 5.14～5.19 场景；
> 实现任务为 tasks.md 的 T093～T100。本节其余条目仍然有效。

1. **共享目录路径（`shared_dir`）缺 CLI 命令**：只有 GUI/Tauri（`shared_dir_get/set`）入口，纯 CLI 下 `deploy to shared` 报"尚未设置共享目录路径"。修复需新增产品入口（契约外命令），**未自行发明**，待裁决。（→ 已按上注补齐规划，待实现）
2. **`skill import` / `update-repo` 无 `tags`/`comment` 参数**：GUI 与 CLI 一致只能补充分类，标签/注释继承上游。**未自行发明**，待裁决。
3. **Hermes 档案独有技能只读查看已补齐**：2026-10-17 T125 闭合，见第十二节。
4. **配套维护 Skill 的 Gitea 发布/入库/更新已验证**：2026-10-17 获用户授权后完成 T124，见第十二节。
5. **`skill task --requirement`**：契约原写"必填"，实现为可选；已按"修订契约文字"收口（与 `tool task` 一致，FR-051 只要求保留用户要求，省略时如实写"未填写"）。
6. **平台兼容未声明**：仅 macOS arm64 实测；Hermes 启用所用符号链接在 Windows 的行为未知。
7. **未安装 rustfmt**，`cargo fmt --check` 未执行；代码按现有风格手写。
8. **托盘长时运行**仅有会话内证据（关窗隐藏 + 隐藏期间调度日志 + 托盘检查/重开/退出）。
9. **「共享目录技能清单」能力缺失——需求链在原型规格层断裂（2026-10-16 追查结论）**——**已按链条补齐**：
   需求 → 规格（FR-042 扩到"本机目标"、新增 FR-055）→ 规划（`plan.md` 规划补齐一节）→ 契约（`agent` 命令组 `<target>`）
   → 任务（T101～T108）→ 实现 → 验收（[m6-report.md](m6-report.md)），GUI 已从 `.app` 实测取证。
   以下保留当时的追查记录：原型的「共享 Agent」页
   （`ui-prototype/index.html:537` 的 `sideRow('共享 Agent','shared',…)`，效果图 `renders/03`）与旧版 `GlobalAgentView`
   都提供"查看共享目录里的技能、区分受管/仅本地、撤销与删除"，但这条**页面需求在正式链条里从未成立**：
   - 需求记录只把共享目录裁决为**部署目标**（`docs/requirements.md:198`：「共享目录是部署目标，不是第二个管理库」），
     全篇没有任何一条要求"页面／入口查看共享目录技能"；
   - 原型规格 `specs/001-skill-management-prototype/spec.md` 的体验范围表（`:209-220`）只列"本机 Agent 技能"，
     **漏列该页——断裂发生在此处**（原型的 README 登记了该页，但规格没立条目）；
   - 产品规格 `specs/002-skill-management/spec.md` 未接手：FR-041/FR-042（`:347-348`）只覆盖本机 Agent，
     `:386` 明写"共享目录为特殊部署目标"，没有任何 FR/SC 覆盖共享目录的浏览；
   - `plan.md` 的 M2/M3 范围表与 `tasks.md` 均无对应条目，也未登记为"暂缓"；
   - 实现因此既无页面与入口（`AppShell`/`AgentPage`/`App` 无 `共享`/`shared`）。**后端能解析共享目录路径**
     （`TargetResolver::resolve_with_override` 的 `Target::Shared` 分支，`deploy/targets.rs:219-232`），
     「扫描目录 + 按部署记录判定受管/仅本地」的逻辑也已存在（`LocalSkillService::list`，`deploy/local_skills.rs:76`），
     但该服务的入口只按 **agent key** 工作：`skills_dir(key)` → `agent_skills_dir(key)` 查内置 Agent 名单，
     查不到即报「{key} 不在内置 Agent 名单中」（`targets.rs:178-187`、`:201-205`），而 `shared` 是特殊部署目标、没有 key
     （`Target::Shared` 的 `agent_key` 为 `None`；前端 `lib/targets.ts:36` 标 `isAgent:false`），
     契约中 `agent local-skills <key>` 因此无法指向共享目录。
   **待用户裁决**：是否补该能力。补法不是从零造：现有 `LocalSkillService` 已具备扫描与受管判定，只需把入口从
   "agent key"扩到"目标"（复用 `Target::Shared` 的路径解析），再补契约命令与 GUI 入口/页面。
   交付前的 M2 验收曾把 `renders/03` 记为"通过"，经复核证据图为技能库页＋Agent 管理页＋Codex 页，**属误标**，
   已在 `m2-report.md` 第二节更正。

## 七、自动化验证（最终）

| 命令 | 结果 |
|---|---|
| `cargo test --workspace` | 通过：**252 项**，0 失败（含本批新增 `tests/cli_settings.rs` 8 项与 `crates/core/tests/runtime_settings.rs` 3 项） |
| `cargo build --workspace` | 通过，0 警告 |
| `npx tsc --noEmit` | 通过 |
| `npm test`（vitest） | 通过：6 文件 / 22 用例 |
| `npm run build` | 通过（dist JS 455.68 kB / gzip 132.04 kB，CSS 28.55 kB） |

## 八、配套 Skill 与文档

- 新增 `skill-manager/assets/skills/skill-manager-maintenance/SKILL.md`（十节：`.manager` 实际格式、目录与同名规则、原文/译文/注释/overlay 边界、来源更新与同项合并、修改后 `deploy sync` 与"询问后才上传"、冲突与暂停、CLI 使用、工具登记/检查命令/真实验证回写）与 `references/cli.md`（8 个命令组真实参数、JSON/退出码）。**文档中 53 条调用 / 91 个 flag 与真实 `--help` 逐条核对一致，并实跑 106 条子命令。**
- 2026-10-16 增量：`references/cli.md` 新增 `settings` 命令组（用法、两个写入口、三条只读边界与 Hermes 根解析说明），并把原"共享目录路径只有 GUI"缺口改为指向 `settings set-shared-dir`；`skill-manager/README.md` 同步 CLI 示例与边界，移除已解决的共享目录缺口。新增命令与 `settings --help`/子命令 `--help` 逐条核对一致。
- 更新：`README.md`（实现状态/验证基线/未闭合缺口/待授权）、新建 `skill-manager/README.md`（结构、开发构建测试运行、CLI 入口与退出码、SkillManagerVault 与 SQLite 位置、配套 Skill 入库方式、已知缺口）、`plan.md`（Implementation Status M1～M5 + 7 项未闭合项）、`research.md`（D9 实施结果与 Gitea 待授权）、`contracts/cli.md`（T090 双入口核对一节）。
- 未执行 `git commit` / `git push`：项目仓库 HEAD 仍为 `c915649`。

## 九、GUI 界面验收（2026-10-16，Computer Use 实操）

此前各期报告把"界面点按流程"列为无法取证，原因**不是应用或自动化的限制，而是启动方式**：
直接运行 `target/debug/skill-manager-desktop` 时进程没有 bundle 标识（`bundle_id=null`），
macOS 既不向辅助功能暴露 webview 内容，也拒绝为该窗口截图（`capture_app: no usable
pixel-to-pointer rectangle`）。改用打包后的 `.app` 启动后三条路径全部可用：
`bundle_id=com.fei.skillmanager`、无障碍树能读到 webview 内的按钮与输入框、截图正常。
**因此后续 GUI 验收必须从 `.app` 启动**（`npx tauri build --debug` 产出
`target/debug/bundle/macos/Skill Manager.app`，用 `open --env` 或直接运行其
`Contents/MacOS/skill-manager-desktop` 传入临时 HOME）。

该次构建同时验证了**可分发性**（此前未验）：`npx tauri build --debug` 成功产出
`Skill Manager.app`（42.67 MiB），32×32 的 `icon.png` 被接受。

### 已取证场景（截图存于 [evidence/](evidence/)，1280×774 光栅）

| 场景 | 实际 | 结论 |
|---|---|---|
| 1.1 首次选库 | 原生目录选择器（标题"选择已有的 SkillManagerVault 目录"、`Cancel`/`Open`、无克隆或创建向导）→ 选定后**直接进入技能库**；两域、分类列、标签栏、中文短注释、译文待更新徽标、预设归属 chip、侧栏检测到的 Agent 全部就位 | 通过 |
| 1.2 分类联动标签 | 点"未分类"后标签候选由 `[全部标签, 无标签, matt, 工具, 路由]` 收窄为 `[全部标签, 无标签, matt, 路由]`（`工具` 消失，因其唯一技能在工具分类）、网格只剩 3 个未分类技能；点"全部分类"恢复 | 通过 |
| 1.3 译文与原文切换 | 详情显示"译文待更新"徽标与说明"原文已变化，译文尚未按当前原文维护；译文保留…"，译文正文未被清除；点"原文"后显示 SKILL.md 的 frontmatter 与正文 | 通过 |
| 1.6 GUI 保存落盘并刷新 | 注释改为 `GUI 保存验证` → 保存：`meta.yaml` 落盘为 `comment: GUI 保存验证`，**未知字段 `foo: bar` 与 `tags: [工具]` 均保留**；`skill list` 读回一致；卡片标题与详情标题自动变为"fmt-helper （GUI 保存验证）"，无需重启 | 通过 |
| FR-038 首次上传确认 | 首次保存后弹出"保存后自动上传本机 SkillManagerVault？"，含"仅上传你自己的 SkillManagerVault，不向来源仓库提交"与"软件会提交当前工作区全部改动，不区分改动来源；外部文件改动与同步已有部署不会触发上传"；取消后提示如实写"已刷新本机已有部署；未上传（尚未确认自动上传）" | 通过 |
| 卡片三点菜单（原型 23/38） | 六项齐全：复制到另一库…、移动到分类…、添加标签…、生成维护任务…、部署 / 撤销…、删除技能… | 通过 |
| M2 部署对话框（原型 26，此前列为未验证） | 部署/撤销分段、预演开关、**已检测 4**（共享目录、Cursor、Codex、ZCode）、**其他支持项 44**（与 47 项编码域减 3 项已检测一致）、逐项过滤与暂停说明、底部"将处理 1 个技能 × 0 个目标" | 通过 |
| M2 Agent 页面（T071） | Codex 页显示本机技能目录、`0 个受管理`、筛选（全部/受管理/仅本地/有变更）、`local-only` 标为"仅本地"并提供收编与删除入口，附"仅本地：直接删除实际文件，不改 SkillManagerVault；受管理：按撤销部署规则处理，删除本机副本不会删库" | 通过 |

### 仍待取证（本轮未做，保持未验证）

- 多选批量栏；添加标签 / 移动到分类 / 复制到另一库 / 删除 四个弹窗的具体操作
- **部署的结果面板与实际产物**（本轮只验到对话框与目标分组；执行部署时索引漂移点错，未产出部署，`deploy status` 为空）
- 入库弹窗其余点按组合仍按原记录保留；SC-008 已在 2026-10-17 T123 补齐真实 GUI 证据（见 M3 报告末节）。
- 上游更新与新增决定页、跳过与已跳过列表、收编弹窗
- 预设页与新建预设；工具中心详情 / 指南 / 配套 Skill / 任务；指令编辑器；技能维护任务对话框
- 设置页同步状态与"未连接"渲染（M4 已有截图）、冲突界面渲染；定时器到期与托盘长时运行
- Windows 模拟下的整项灰显（本机为 macOS，无法验证）

### 附带发现（非缺陷）

- 保存时若用户在首次确认弹窗选择"取消"，下次保存会**再次询问**（同意记录只在确认后写入），与"未确认时不记住"的语义一致；界面提示如实说明未上传。
- 本轮误点回到 Agent 页面，属操作失误而非应用行为异常；已在表中注明未产出部署，避免把未发生的事记为通过。

## 十、全程 GUI 实操验收（2026-10-16，第一批：US1 / US3 / US7 / US8 部分）

方式：从 `.app` 启动 + Computer Use 驱动真实窗口，夹具库刻意放进边界与异常数据（空注释空标签、
中文目录名、跨域的 `dup`、仅 Linux、仅 Codex、含缺失成员的预设、上游与库内同名、档案独有技能、
无检查命令/无指南的工具）。证据照片见 [evidence/](evidence/)（前缀 `vfy-`）。

### 已验证（本人实操所见）

| 需求 | 实测 |
|---|---|
| FR-002 首次接入 | 原生目录选择器（"选择已有的 SkillManagerVault 目录"、无克隆/创建向导）；选定后直接进入技能库；**重启后按本机记住的库自动接入** |
| FR-003/007 分类与灰显 | `linux-only`（`oses: [linux]`）整项灰显 + "不支持当前系统"徽标；**资料仍可维护**（注释/标签/系统声明均可编辑），符合"不阻止库内维护" |
| FR-005/009 资料与筛选 | 空注释/空标签边界正确（卡片无注释括号、不显示标签）；"无标签"筛出 5 个无标签技能（排除有标签的两个）；分类联动标签成立 |
| FR-006 译文边界 | 摘要匹配的技能**不**提示待更新，摘要不匹配的才提示；"原文"切换显示 frontmatter 原文 |
| FR-010 详情 | 覆盖层页列出两个维度的文件（`_/_` 与 `codex/_`）并可阅读/移除；来源页对自建技能如实显示"无外部来源"，空值显示为"—" |
| FR-016 部署合成 | 产物 = 原件 + `codex/_/notes.txt` + `_/_/common.txt` + 注入注释 `description: 格式化辅助｜工具分类下的技能`，正文未变 |
| FR-016/SC-003 边界 | 产物**无 `.manager`、无 `translation.md`** |
| FR-018/019 记录 | 部署记录含 `target=agent:codex`、真实 `target_path`、`status=ok`、`source_hash`/`deployed_hash` |
| FR-020 暂停 | 外部改副本后 `deploy sync` → `partial` + 可读原因 + `status=paused_local_change`，**外改副本未被覆盖** |
| FR-024/US7 缺失成员 | 预设页显示"缺失成员：demo-zh、不存在的技能（库内已无同名技能；不阻塞其他成员）"；预设页复用库页的分类栏、标签栏与分类联动 |
| FR-038 上传确认 | 首次保存弹出确认（"仅上传你自己的 SkillManagerVault，不向来源仓库提交"）；取消后如实提示"已刷新本机已有部署；未上传（尚未确认自动上传）" |
| FR-041/T071 Agent 页 | Codex 页显示本机技能目录、"0 个受管理"、仅本地技能提供收编与删除入口，并注明"删除本机副本不会删库" |
| 部署结果面板 | 成功："部署完成：处理 1 项"；被阻止时："处理结果：成功 0 项，未完成 1 项 / 过滤·不适用" + `code/linux-only → Codex：禁止新增部署：当前系统 macos 不在该技能声明支持的系统内（linux）` |
| 可分发性 | `npx tauri build --debug` 产出 `Skill Manager.app`（42.67 MiB） |

### 挑刺发现

1. **[UX 一致性，低—中] 系统不适用的技能仍可发起部署。** 卡片灰显正确，但详情「部署」按钮可点击、
   对话框允许选中目标并执行，最后由核心返回"禁止新增部署"。核心阻止 + 原因可读满足 FR-007 的字面要求，
   但入口没有阻止，等于邀请用户做一次必然被拒的操作。建议入口处禁用并在对话框内说明原因。
   **→ 2026-10-16 续作已修复**（含同类入口一并补齐，见第十一节；修复仅由代码复核、组件测试与构建验证，
   尚未经 GUI 实操复验）。
2. **[文案，低] 预设成员计数不区分缺失。** 预设标题显示"成员 3 个"，而其中 2 个是缺失成员；
   缺失成员另有一行说明，但计数本身容易被读成"3 个可用"。**→ 2026-10-16 续作已修复**（见第十一节）。
3. **[已排除，非缺陷] 译文摘要按逐字节计算（含末尾换行）。** 我手写摘要时用 `$(cat)` 去掉了末尾换行，
   于是应用提示"译文待更新"——应用判定正确；配套维护 Skill 文档已写明"不要手工伪造 `translation_source_hash`"，
   用 `skill translation set` 写入后提示消失（正向用例已验证）。
4. **[行为观察，非缺陷] 取消首次上传确认后，下次保存会再次询问**（同意记录只在确认后写入），
   与"未确认就不记住"一致，界面提示如实说明未上传。

### 本批未做完（保持未验证）

US2 获取（入库预览/部分入库/继承/收编/自建/复制/进度取消）、US4 上游（检查/变化/单项与批量更新/
新增候选/拒绝/跳过）、US5 同步（状态/未连接/拉取上传/冲突）、US6 Hermes（档案/启停/收编/只读详情）、
US9 工具中心（清单/未安装/各检查状态/详情/指南/候选/任务）、US10 环境（主题/字号/容量/诊断/托盘）、
以及 US1 的文件与部署页、多选批量栏、四个操作弹窗与「生成维护任务」、US3 的未受管同名接管与撤销/
删除/共享目录/预演在 GUI 的呈现、US7 的新建预设与拖动排序、US8 的指令编辑器与目录行内修改。

**续跑配方**：夹具 `/tmp/vfy`（库=`vault`、假 home=`home`、Hermes 根=`hermes`、上游=`up`/`up-single`、
裸远端=`remote.git`）；启动命令见本报告第九节；证据目录 `checklists/evidence/`。

## 十一、挑刺发现修复（2026-10-16，第二批 GUI 验收之前的代码修复）

第十节两处发现已修复，并对同类入口做了扫描。**本节结论来自代码复核、组件测试与构建，
未经 GUI 实操复验**：修复在真实窗口里是否如期呈现，仍应在第二批 GUI 验收中确认。

### 修了什么

1. **部署入口按系统支持前置判断**（发现 1）。先确认一条边界事实：core 只在**新增**部署时按支持声明拒绝
   （`crates/core/src/deploy/apply.rs:234` 取准入，`:241` 的 `existing.is_none()` 分支才返回
   "禁止新增部署"），**撤销不受支持声明约束**。因此不能整块禁用「部署 / 撤销…」入口，只能阻止新增部署：
   - 新增纯函数 `unsupportedDeployReason(entries, current)`（`src/lib/support.ts:68-89`）：选中项
     **全部**不受当前系统支持时返回可读原因，其余情况（空列表、部分支持、系统未知、未声明）返回 `null`。
     部分支持时刻意不阻止：既有"过滤 / 不适用"逐项结果面板本就如实报告，前置全禁会连能成的项一起挡掉。
   - `DeployDialog`（技能库）与 `PresetDeployDialog`（预设组合部署）各增可选属性 `unsupportedReason`：
     部署模式下显示 warn Notice 并禁用主按钮，切到撤销模式自动恢复可点。
   - 详情页「部署」按钮在 `unsupportedOs` 时禁用并给出 `title`（`src/components/SkillDetail.tsx`，
     该组件早已算出 `unsupported`（`:150`）并显示"不能部署或启用"的 Notice，此前只是没作用到按钮）。
2. **预设成员计数区分缺失**（发现 2）：标题改为 `成员 3 个（2 个缺失）`，缺失片段用警示色
   （`src/pages/PresetPage.tsx:520`）；缺失数量与同句的成员总数同源取 `members.filter(missing)`，
   与其下"缺失成员：…"整行不冲突。

### 同类入口扫描（扫的是"部署入口未按支持声明前置判断"这一类）

| 入口 | 位置 | 处置 |
|---|---|---|
| 技能库页部署对话框 | `src/pages/LibraryPage.tsx:1116` | 已按系统支持阻止新增部署 |
| 预设页「组合部署 / 撤销…」 | `src/pages/PresetPage.tsx:888` | 已同样处理（该路径在 core 复用同一个 `DeploymentService`，准入规则一致） |
| 预设页内的单技能部署对话框 | `src/pages/PresetPage.tsx:944` | 无需改：唯一入口是「技能详情」的部署按钮，已由 `SkillDetail` 自身禁用 |
| 卡片三点菜单 / 多选批量栏的「部署 / 撤销…」 | `src/pages/LibraryPage.tsx:705、1013` | 保持可点：撤销必须仍可达，阻止发生在对话框内 |
| Agent 页 | `src/pages/AgentPage.tsx` | 已核：无部署入口（启停只写本机状态） |

预设页把成员名映射到技能用**大小写不敏感**比较，与 core 的 `SkillIndex::get`
（`crates/core/src/vault/index.rs`，用 `domain.rs:194` 的 `names_equal`）一致；映射不到（含缺失成员、
清单未加载完）一律按"视为支持、不阻止"处理——这类成员本就由 core 逐项报告，前置阻止反而会挡掉可用项。
映射的前提是成员按**预设所在域**解析：core 的 `preset/manage.rs` 用 `view(domain, preset, index)` 查同名技能，
同域内技能名大小写不敏感唯一，因此成员名到技能是一对一的。

### 验证（事实）

| 命令 | 结果 |
|---|---|
| `cargo test --workspace` | 252 项通过、0 失败（未改 Rust；主代理复跑） |
| `npm test` | 33 项通过（原 22 + `unsupportedDeployReason` 5 + 部署阻止组件测试 6，文件 `src/components/DeployDialog.test.tsx`） |
| `npx tsc --noEmit`、`npm run build` | 通过 |
| 新测试证伪 | 去掉两个对话框的 `deployBlocked` 条件后 4 个用例确实失败（`toBeDisabled` 报元素未禁用），恢复后全绿。首次断言因"未选目标时按钮本就禁用"无区分力，已改为先选中目标 |

未覆盖：本次为纯前端展示层阻止，未改 core 准入；`busy`、预演分支与撤销模式下的真实调用未纳入测试。

**报告内已过期陈述**：第八节"未执行 `git commit` / `git push`：项目仓库 HEAD 仍为 `c915649`"是该节写作时的事实，
此后已有 `dbfacdb`、`3a31e50`、`a873d89` 与本批提交，该句只作当时记录。

## 十二、Phase 18 查漏补缺实施与验收（2026-10-17）

本节为当前增量结论；原章节中的测试数量与未取证陈述保留其当时语境。

| 工作 | 当前结果与证据 |
|---|---|
| T115 / T116 部分完成 | 上游删除保留未清理的外改副本及定位；库拉取保留库已更新事实，同时汇总部署暂停与指令写入失败。CLI（命令行接口）返回 partial（部分完成）与退出码 2；GUI 摘要保留具体未完成原因。`cli_upstream`、`cli_sync` 的真实文件场景通过 |
| T117 统一检查 | 启动、手动、定时、托盘复用同一调度；三类结果分别表达，后台报告刷新 GUI。最近实际报告仅在内存保留以避免启动早于订阅丢失；关闭定时不影响手动。`runtime_settings` 覆盖启动、手动、关闭与到期定时路径；SettingsPage 组件验证各类失败可见。属于自动化触发验证，未等待实际 30 分钟 |
| T118 / T119 上游采用 | 仓库批量更新直接刷新该技能已有部署；再次采用同一版本返回 unchanged（未变化），不覆盖之后的本地维护。`cli_upstream` 12 项通过 |
| T120 批处理 | 单项实际文件错误转入逐项报告，部署、同步、撤销与清理继续处理可执行项。首目标为普通文件、第二目标正常的 CLI 场景确认部分完成且后目标实际落盘 |
| T121 保存后流程 | 预设定义与排序、Agent 跨域复制接通既有保存流程；沿用原上传确认与失败处理。PresetPage、librarySave、AgentPage 组件测试通过；不把外部文件变化作为上传授权 |
| T122 新增部署准入 | core（业务核心）输出目标限制，GUI 复用；不支持的新目标禁用，已有部署可撤销，多技能逐项过滤。DeployDialog、QuickDeploy、既有 core deployment 测试通过；不以纯前端判断代替核心准入 |
| T123 大仓库 GUI | **通过**。真实 128 MiB Git 传输、进度、取消、失败与成功；来源登记、文件大小、无自动部署和未授权上传均核实。夹具与本会话证据详见 [M3 报告末节](m3-report.md#t123-补验真实-gui-大仓库获取2026-10-17) |
| T125 Hermes 阅读 | `hermes read`、Tauri 与 GUI 可只读阅读档案独有技能；查看不收编。库内阅读采用 Markdown 视图及吸顶控件。CLI 实际文件、附属文件及查看前后启用名单不变，HermesPage 组件测试通过 |
| T126 工具配套技能 | 先获取真实来源资料并显示原名、用途、注释、标签，再由用户勾选；来源失败和来源缺资料分别展示。预览不入库，确认后才入库、不部署。CLI 来源元数据断言与 ToolSkillCandidates / ToolCenterPage 组件测试通过 |
| T127 跨页定位 | 两个管理域均接收已有 focusReference，编码 Agent 与共享目录的库内管理导航不再丢失目标；域切换重建库页，避免沿用另一域的筛选 |
| T128 诊断 | 手动 preview / import 失败写入既有脱敏日志；部署直接文件错误进入逐项报告并记录，索引提前失败也记录。CLI 实测诊断包含操作、时间和实际失败路径 |
| T129 组合筛选 | GUI 新增指定实际部署目标与所属预设，和搜索、分类、标签组合；client 传入已有 Tauri deployed_to / preset 查询。LibraryPage 组件测试与已有 CLI 真实部署/预设筛选测试通过 |

**本轮验证**：`cargo test --workspace` 共 270 项通过；`npm test` 共 54 项通过；
`npm run build`（含 TypeScript 类型检查）通过；`npm run tauri -- build --debug --bundles app`
成功生成并签名 `skill-manager/target/debug/bundle/macos/Skill Manager.app`（43.33 MiB）。
组件测试用于验证接线与行为，不代表全部 GUI 点按或其他平台已完成人工验收。

### T124 真实 Gitea 交付验收（已通过）

用户于本会话明确回复“提交并推送”，授权此前列明的 Gitea 发布与后续隔离验收。
真实来源：`http://192.168.200.7:3000/yfpoyuas/SkillManager.git`；
选择的子路径：`skill-manager/assets/skills/skill-manager-maintenance`。
复用本机 Git 的已有鉴权；验收库和 SQLite 状态均在隔离临时目录，未部署到真实 Agent 目录。

| 步骤 | 实际结果 |
|---|---|
| 发布初始项目版本 | `origin/main` 从 `c915649` 快进到 `c00e1a49ed03f567010e8c4aa5d6182d1a697895` |
| 普通预览与显式选择入库 | `skill preview` 列出 11 个来源候选且不写库；`skill import <真实来源> --domain code --skills <上述子路径> --uncategorized` 只入库 `code/skill-manager-maintenance`；source.yaml 登记真实来源、子路径与 `c00e1a4` 版本 |
| 推进真实上游版本 | 本轮实现提交 `e2106f981fc9694fe233f333aa35f60b25c3c845` 推送到 `origin/main`，包含维护技能 `references/cli.md` 的 Hermes 阅读新命令 |
| 只检查不采用 | `skill check code/skill-manager-maintenance` 返回 `update_available`、`modified: [references/cli.md]`；随后实际读取仍为旧内容，采用版本仍是 `c00e1a4` |
| 显式采用 | `skill update code/skill-manager-maintenance` 成功，`from_version=c00e1a4…`、`to_version=e2106f9…`；来源登记同步推进，实际参考文件包含 `hermes read <profile> <name> [--file <path>]` |
| 维护任务路径 | `skill task code/skill-manager-maintenance`、`tool task --action register` 均包含实际库内绝对路径与“按其规范执行”，该目录内 SKILL.md 实际存在 |
| 入库与任务边界 | `deploy status` 返回空部署列表；未自动部署、未安装工具，验收技能未写入用户初始统一库 |

临时证据目录：`/private/tmp/sm-t124-gitea-wrr0zu4z/`，保留 `01-preview.json` 至
`10-deployment-status.json`、相应 stderr（标准错误）与 `meta.json`。技能实际路径：
`/private/tmp/sm-t124-gitea-wrr0zu4z/vault/code/skill-manager-maintenance`。
本次为真实 Gitea 上游获取与版本推进，已替代 5.10 此前仅用临时本地来源的验证限制。
Phase 18 的 T115～T129 共 15 项至此全部完成；其他平台及历史未取证交互不因此自动视为已验收。
