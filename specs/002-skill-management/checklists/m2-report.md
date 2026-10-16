# M2 验收报告：部署链路

**日期**：2026-10-16 | **范围**：T021～T029（US3）、T056～T061（US6）、T067/T068/T071（US8 本机 Agent 部分） | **依据**：[quickstart.md](../quickstart.md) M2、[spec.md](../spec.md) US3/US6/US8、[tasks.md](../tasks.md) Phase 4/8/10

**证据说明**：CLI 与核心结论来自本机实际执行（临时 Git 库 + 临时 HOME + 独立 `--state-dir`）；GUI 结论来自真实 Tauri 桌面应用截图，位于 `/tmp/m2-gui/` 与 `/tmp/m2-hermes/`（1720 × 1040）。**未取得证据的项记为未验证。** 真实 `~/.codex`、`~/.zcode`、`~/.hermes`、`~/.agents` 均未被写入（只读探测已核对）。

## 一、场景核对表

| 场景号 | 预期 | 实际 | 结论 |
|---|---|---|---|
| 2.1 `agent detect` 与设置页检测 | 已装 Agent 分组正确；检测不启动、不部署 | 临时 HOME 下 `agent detect` 检出 3 个已装 Agent（Cursor/Codex/ZCode），检测只读路径存在性、不写状态、不部署；GUI 分组"已检测 3 · 已启用 3"与"其他支持项"折叠区（截图 `08-agent-manage.png`） | 通过 |
| 2.2 `deploy to <agent> <ref>`（带覆盖层与注释） | 产物 = 原件+覆盖层+中文注释；无 `.manager`；库内原件未变 | 产物含注入注释 `description: 检查依赖｜解析依赖`、覆盖层 `notes.txt`；无 `.manager`、无译文；库内原件逐字未变；合成产物测试 `crates/core/tests/deployment.rs` 11 项覆盖 | 通过 |
| 2.3 仅支持 Codex 的技能部署到其他 Agent / 仅 macOS 技能在 Windows 下 | 前者无该目标可部署；后者整项灰显禁部署 | Agent 侧：`deploy to zcode` 被支持声明过滤，报"未声明支持目标 agent:zcode"，退出码非 0。OS 侧：M1 已实现整项灰显（截图 `14-category-browser.png`，M1 报告）；**Windows 模拟未验证**（当前 macOS 无法模拟） | 部分通过：Agent 支持声明过滤通过；Windows 灰显未验证 |
| 2.4 未受管同名 → 部署 | 直接覆盖并接管 | 未受管同名副本被覆盖并登记（`action: deployed`）；`tests/cli_deploy.rs` 覆盖 | 通过 |
| 2.5 手改副本 → `deploy sync` | 该项 `paused_local_change` 并提示，其余正常同步 | 手改 alpha 副本后 `sync` 返回 `partial`：alpha `paused_local_change`、beta `unchanged`；`deploy status --paused` 列出暂停项；`sync --overwrite-local` 恢复并清零暂停（对齐裁决参数）；GUI 暂停提示含"复制定位信息给 Agent"(截图 `10-paused.png`) | 通过 |
| 2.6 GUI 保存影响部署的资料 / `deploy sync` | 已有部署更新，无新增目标 | 库内正文变化后已有部署 `updated`，未部署的 zcode 目录未出现，记录数不变。GUI 保存 alpha 注释后：`meta.yaml` 更新、已有 codex 副本 `SKILL.md` 自动变为 `changed-by-gui｜解析依赖`、记录仍 4 条（截图 `11-save-autorefresh.png`） | 通过 |
| 2.7 `deploy to shared <ref>` | 共享目录获得通用内容 | 仅通用覆盖层 `_/_` 生效（codex 专属层未进入），注释存在；"有 overlay 目录不机械拒绝"由测试断言；共享目录路径设置入口在部署对话框（截图 `05-shared-path.png`、`06-shared-saved.png`） | 通过 |
| 2.8 编码技能直接部署给龙虾目标 | 被管理域边界阻止 | `lobster`→`codex` 与编码技能→`hermes:default` 均被域边界阻止（硬约束，不可由支持声明绕过） | 通过 |
| 2.9 `skill delete` 已部署技能 + 外改副本 | 无外改副本清理成功；外改项库条目删除但副本与定位记录保留，报告清理暂停；裁决后显式继续清理 | 无外改项清理成功；外改项返回 `partial`、库条目已删、副本保留、记录 `paused_local_change` + `library_exists:false`，报"库已删除、该副本清理暂停"；`undo --discard-local-changes` 后副本移除、记录清除 | 通过 |
| 2.10 `deploy ... --dry-run` | 报告技能/目标/路径，文件未变 | 报告 `would_deploy` 与真实路径；目标副本未创建、`deploy status` 前后快照一致；`tests/cli_deploy.rs` 覆盖"文件与数据库均不变" | 通过 |
| 2.11 部署 `hermes:default` → `hermes enable <p>` → disable | default 可用；档案明确启用才可用；内容更新后启用选择保留 | `deploy to hermes:default lobster/alpha` → 副本 `<HOME>/.hermes/skills/alpha`；`hermes enable blank alpha` → 建立指向该共享副本的链接；随后部署 beta **未**在 blank 自动出现；库内容更新 + `deploy sync` 后 blank 仍 `alpha enabled=true` 且读到新内容；`hermes disable` 后链接移除 | 通过（假 Hermes 根，见下"设计决定"一节） |
| 2.12 档案独有技能 `skill adopt-hermes` | 进龙虾库 + 部署 default，原档案启用保持 | `skill adopt-hermes blank mydog`：`vault/lobster/mydog` 建立、default 部署 `mydog`、blank 仍启用；重复收编被同名阻止（退出码非 0 + 可定位原因） | 通过 |
| 2.13 支持声明收窄（去掉某 Agent） | 已有部署保留+警告，新部署禁止 | 收窄为 `[zcode]` 后：已有部署 `updated` 且带"支持范围已收窄…保留并警告"、新 codex 部署被禁止、`status --target zcode` 带 warning；GUI 复现（截图 `03-deploy-tab.png`：Codex 亮+叹号、其余暗、警告明细行） | 通过 |
| 2.14 `deploy to codex a b --also-target zcode` + 批量撤销/预演 | 一次涵盖多技能与多目标，逐组合过滤/暂停，其余继续；预演不落文件；撤销保留库技能 | 4 个组合全部处理；`undo` 预演 4 项 `would_remove` 且副本仍在；实撤销后两目标副本全删、库内技能保留 | 通过（CLI）；GUI 批量结果面板未取到截图（见下） |

## 二、GUI 视觉与交互对照

| 原型页面/状态 | 实际截图 | 结论与差异 |
|---|---|---|
| 共享 Agent / Codex 页（`renders/03`、`04`） | `/tmp/m2-gui/01-library.png`、`08-agent-manage.png`、`09-agent-local.png` | 通过：Agent 分组（已检测/其他支持项）、启停开关、拖动排序柄、技能目录行内铅笔与恢复默认、本地技能列表"已同步·受管 / 本地有变更·受管 / 仅本地"三态与筛选。侧栏随启用状态与顺序变化 |
| 部署目标亮暗（`renders/26`） | `02-detail.png`、`10-paused.png` | 通过：已部署目标亮显（含部署目标 chips）、未部署灰暗、暂停项提示与"复制定位信息给 Agent" |
| 已有部署不适用警告（`renders/28`） | `03-deploy-tab.png` | 通过：不适用目标带叹号 + 警告明细行 + 图例 |
| 部署字符（无直接原型页） | `04-deploy-dialog.png`、`05-shared-path.png`、`07-deploy-targets-selected.png` | 部分通过：目标分组（已检测 4 / 其他支持 44）、多技能×多目标选择、共享目录路径设置与未设置时如实提示；**结果面板（过滤/暂停分组）未取到截图**（弹窗 footer 按钮未暴露给 macOS Accessibility，自动点击无法触发） |
| Hermes 档案页（`renders/05`、`06`） | `/tmp/m2-hermes/05-hermes.png`、`05b-named.png`、`06-picker.png` | 通过：档案选择器层级/路径/启用数/对号/底部说明；档案技能列表复用技能库分类栏、标签栏、卡片网格与分类联动标签；启停开关与批量启停 |
| Hermes 技能统一详情（`renders/21`）、只读管理资料（`renders/30`） | `21-detail.png`、`30-readonly-manager.png` | 部分通过：只读四分区 + 语言切换 + "前往技能库编辑"入口一致；**差异**：`30` 原型用禁用态输入框展示字段，本实现用只读文本值 + 提示条（信息与只读语义一致，控件形态不同） |
| 未收编技能查看 | 无 | **未实现**：点击未收编技能弹出收编确认框，未实现"用同一只读分区查看档案本机内容"（需新增读取档案技能正文的入口，CLI 契约中无此命令） |

**未取得界面证据、记为未验证**：部署弹窗的结果面板分组、拖动排序的实际拖放、启停联动侧栏的即时变化、删除技能二次确认弹窗。拖动与会话内确认属人工操作；其余由 CLI/核心测试覆盖命令路径，但**不据此声称 GUI 已人工验收通过**。

## 三、自动化验证

| 命令 | 结果 |
|---|---|
| `cargo test --workspace` | 通过：131 项（core 单元 58 + vault_management 16 + deployment 11 + agent_management 9 + CLI 集成 9/8/6/14），0 失败 |
| `cargo build --workspace` | 通过，0 警告 |
| `npx tsc --noEmit` / `npm run build` | 通过 |
| `npm test`（vitest） | 通过：2 文件 / 8 用例（支持声明合并 7 + HermesPage 渲染 1） |

## 四、需要用户知悉的设计决定

1. **Hermes 其他档案的"启用"用符号链接实现**（指向 default 共享副本）。
   - 依据（源码级核实，见 `research.md` §五）：Hermes 用 `os.walk(followlinks=True)` 扫描档案技能目录，链接能被发现；`skills.disabled` 是**黑名单**而非白名单，`external_dirs` 是整目录挂载（会让以后新增自动可见，与 FR-026 冲突），因此本项目不写用户 `config.yaml`。
   - 收益：满足"档案共用库内一份技能内容"且"新增不自动启用其他档案"；不动用户配置，避免破坏其注释与其他设置。
   - **与 tasks.md T026"不新增软链接部署模式"的关系**：T026 约束的是**部署**（Agent/共享目录），未约束档案启用；此处的链接位于档案技能目录，是启用的表达方式。**Windows 下的 symlink/junction 行为未验证**（失败会如实报错，不静默复制）。
   - 待用户确认：若更希望"每档案一份独立副本"，代价是放弃"共用一份内容"；当前实现取链接方案。
2. **`deploy to hermes:default` 的路径**仍按内置 Agent 目录资产解析 `<root>/skills`（与 `HermesService` 的根解析在默认情况下一致）。若将来把 Hermes 根设为非默认位置，两者会分叉——本轮未处理，未新增设置入口。
3. **共享目录路径设置缺 CLI 命令**：已记录在 `research.md`（§五 D18），由 T090 按 FR-050 双入口核对统一处理；GUI 经 Tauri `shared_dir_get/set` 可达。**不自行发明契约外命令。**
4. **Agent 目录修改后的已有部署协调**：语义写在 `research.md`（D15）——`deploy to` 使用新目录并更新 `target_path`，旧路径副本交用户处置；不采用旧版"先删旧副本再重建"。

## 五、遗留与待后续里程碑复核

1. **2.3 的 Windows 模拟**、**部署弹窗结果面板截图**、**拖动排序/删除确认弹窗的界面证据**：需人工复核。
2. **未收编档案技能的只读查看**：交互缺口（见第二节）。
3. **Hermes 端到端运行未验证**："符号链接技能被 Hermes 实际加载"仅有源码级依据，未启动 Hermes 实测；WebUI/TUI 对链接技能的展示未核对。
4. **预设入口**（`deploy preset`/`undo-preset`）属 M5，本期未实现，help 中亦不出现；M2 报告不含预设验收。
5. **指令目标配置**（`agent set-dir --instructions`）仅 CLI/Tauri 可达，GUI 属 M4/T072。
6. 未执行 `git commit` / `git push`。
