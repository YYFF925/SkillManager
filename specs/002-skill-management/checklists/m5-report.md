# M5 验收报告：预设、工具中心、界面环境与整体收尾

**日期**：2026-10-16 | **范围**：T062～T066（US7）、T074～T082（US9）、T083～T087（US10）、T088～T092（配套 Skill 与整体收尾） | **依据**：[quickstart.md](../quickstart.md) M5、[spec.md](../spec.md) US7/US9/US10 与 SC-001～008、[tasks.md](../tasks.md) Phase 9/11/12/13

**证据说明**：结论来自本机实际执行（临时 HOME + 临时 SkillVault + 临时 `--state-dir` + 临时本地上游/裸远端 + 假 Agent 目录 + 假 Hermes 根 + 假工具检查脚本）；GUI 证据为真实 Tauri 应用截图，位于 `/tmp/m5-preset-env/`、`/tmp/m5-tools/`、`/tmp/m5-final/`（1720 × 1040）。真实用户目录未被写入。**未取得证据的项记为未验证。**

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
| 5.10 配套维护 Skill 从本项目远端普通入库 → 推进版本 → `check`/`update` | 走普通上游流程，来源记录正确；任务书引用路径可解析 | **Gitea 路径待授权**（未提交未推送）。**机制已用临时本地上游验证**：preview → import（登记 repo/subpath/version）→ 上游 v2 → check（只展示不应用）→ update（显式采用，OID `89c938f2…`→`4816088e…`）→ `skill task` 引用真实库内路径 + "按其规范执行"（`/tmp/m5-t089.lHDtbO/t089-evidence.txt`） | **待授权**：Gitea 发布与入库未执行；机制通过 |
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
| SC-008 较大仓库获取 | **部分通过** | CLI：287 MB 仓库 208 帧真实进度、`SIGINT` 取消生效且不误报入库（M3 报告）；**GUI 侧在屏进度与界面取消仍缺证据** |

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
3. **Hermes 档案独有技能无只读查看入口**（GUI 与 CLI 都没有）：属交互缺口，非入口不对称。
4. **配套维护 Skill 的 Gitea 发布/入库待授权**：提交与推送未获授权，未执行；机制已用本地上游验证。
5. **`skill task --requirement`**：契约原写"必填"，实现为可选；已按"修订契约文字"收口（与 `tool task` 一致，FR-051 只要求保留用户要求，省略时如实写"未填写"）。
6. **平台兼容未声明**：仅 macOS arm64 实测；Hermes 启用所用符号链接在 Windows 的行为未知。
7. **未安装 rustfmt**，`cargo fmt --check` 未执行；代码按现有风格手写。
8. **托盘长时运行**仅有会话内证据（关窗隐藏 + 隐藏期间调度日志 + 托盘检查/重开/退出）。

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
- 更新：`README.md`（实现状态/验证基线/未闭合缺口/待授权）、新建 `skill-manager/README.md`（结构、开发构建测试运行、CLI 入口与退出码、SkillVault 与 SQLite 位置、配套 Skill 入库方式、已知缺口）、`plan.md`（Implementation Status M1～M5 + 7 项未闭合项）、`research.md`（D9 实施结果与 Gitea 待授权）、`contracts/cli.md`（T090 双入口核对一节）。
- 未执行 `git commit` / `git push`：项目仓库 HEAD 仍为 `c915649`。
