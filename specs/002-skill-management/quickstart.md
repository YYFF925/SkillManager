# Quickstart: 按里程碑的验收验证指南

**Date**: 2026-10-15 | 用途：每个里程碑（M1~M5）收尾时执行对应章节的场景，逐条记录 通过/未过 与证据，形成验收核对表供用户检查方向偏差。全部通过的判定以 [spec.md](spec.md) 的验收场景为准，本文是它们的可执行化。

## 前置条件（一次性准备）

- 一个**真实本地 SkillVault**：Git 仓库、已配置远端（可用临时私有远端）；首次启动在 GUI 中选择该目录；
- 一个**测试上游仓库**：含多个技能（其中至少一个带 `.manager/`）+ 后续可制造新提交（改名/删除/管理资料变化各备一个场景）；
- 本机安装 2 个以上真实 Agent（或以真实目录结构模拟），启用状态已知；
- **双机模拟**（M4 起）：同一 SkillVault 远端的两个克隆 + 两个独立本机状态目录（分别传 `--vault <clone>` 和 `--state-dir <state-dir>`）。两份克隆代表同一个库，来源记录均指向技能的上游，不把另一份克隆当作技能来源重新入库。

验收总原则：状态表达诚实（未检查/失败/待处理不得显示为最新或完成）。上游/工具检查可以保存本机检查事实与获取缓存，不应用技能变化、不改采用版本、部署或工具安装事实；dry-run 才要求目标文件和业务状态不变化。命令以 `contracts/cli.md` 为准，表中省略的参数不表示允许缺少必要对象。

## GUI 视觉与交互对照（各期适用）

以[原型页面与效果图索引](../001-skill-management-prototype/ui-prototype/README.md)为页面清单，结合[视觉说明](../001-skill-management-prototype/visual-reference.md)、[交互细节](../001-skill-management-prototype/ui-interaction-audit.md)及可操作原型核对；原始旧版截图用于追溯，不覆盖当前原型和正式业务规则。

- 每期对本期实现的页面、详情、弹窗和关键状态，在相同窗口尺寸、主题与默认字号下比较布局、层级、配色、图标、间距、密度及操作位置，保存实际界面截图；用真实数据核对展示，不要求复刻演示文字或数量。
- 实际操作悬停菜单、多选、拖动、行内编辑、筛选、弹窗及返回/保存流程，验证对应交互与原型一致。原型尚未覆盖的正式状态沿用同一视觉体系。
- 在已有 `checklists/m<N>-report.md` 中按“原型页面/状态 | 实际截图/操作证据 | 结论与差异”记录；正式规格要求造成的差异说明依据，其他未对齐项保持未过。M5 收尾核对整个原型索引，不能仅因功能可用或构建通过就认定界面完成。

## M1 — 底座：库与 `.manager`

| # | 操作 | 预期（对应场景） |
|---|---|---|
| 1.1 | 启动 GUI，首次选择 SkillVault | 直接进入技能库；两域技能、分类、标签、注释可见（US1） |
| 1.2 | 搜索/按分类+标签组合筛选；选中分类 A 看标签栏 | 标签候选只含 A 内实际标签；"全部分类"恢复全部（US1-3） |
| 1.3 | 打开带译文的技能详情 | 默认中文、可切原文；无译文技能显示原文（US1-1） |
| 1.4 | 改分类 A→B（GUI 与 `skill move` 各一次） | 目录实际移动、名字不变、筛选可找到（US1-2） |
| 1.5 | `skill set` 改标签/注释/支持声明 → GUI 查看 | 界面自动刷新，无需重启（US1-5、外部修改监听） |
| 1.6 | 手工编辑 `meta.yaml`（含未知字段 `foo: bar`）→ GUI 改注释保存 → 查看文件 | `foo: bar` 仍存在（D3 未知字段保留） |
| 1.7 | 现有库写入口核对同域同名限制；M3 再覆盖创建/入库/收编/复制 | 同域阻止、跨域允许（US1-6）；尚不存在的 M3 入口记待验，不将 M1 报为全部通过 |
| 1.8 | `skill list --json` / `skill get --json` | 稳定 JSON schema，Agent 可解析 |

## M2 — 部署链路

| # | 操作 | 预期 |
|---|---|---|
| 2.1 | `agent detect` 与设置页检测 | 已装 Agent 分组正确；检测不启动、不部署（US8-1） |
| 2.2 | `deploy to <agent> <ref>`（带覆盖层与注释的技能） | 目标目录 = 原始文件+覆盖层+中文注释；**无 `.manager`**；库内原件未变（US3-1、SC-003） |
| 2.3 | 部署仅支持 Codex 的技能到其他 Agent / 在 Windows 模拟下查看仅 macOS 技能 | 前者无该目标可部署；后者整项灰显禁部署（US3-2/3） |
| 2.4 | 目标已有未受管同名技能 → 部署 | 直接覆盖并接管（US3-5） |
| 2.5 | 手改部署副本文件 → `deploy sync` | 该项 `paused_local_change` 并提示，其余正常同步（US3-6、`deploy status --paused`） |
| 2.6 | GUI 保存影响部署的资料 / `deploy sync`（Agent 路径） | 已有部署更新，无新增目标（US3-7） |
| 2.7 | `deploy to shared <ref>` | 共享目录获得通用内容（US3-8） |
| 2.8 | 编码技能直接部署给龙虾目标 | 被管理域边界阻止（US3-9） |
| 2.9 | `skill delete <ref>` 已部署技能，并另备一个外改副本 | 无外改副本清理成功；外改项库条目删除但副本和定位记录保留，报告清理暂停；用户经 Agent 裁决后显式继续清理（US3-10） |
| 2.10 | `deploy to ... --dry-run` | 报告技能/目标/路径，文件未变（US3-11） |
| 2.11 | 部署技能到 `hermes:default` → `hermes enable <p> <name>` → 再 disable | default 可用；档案明确启用才可用；内容更新后启用选择保留（US6-1/2） |
| 2.12 | 档案独有技能 `skill adopt-hermes` | 进龙虾库+部署 default，原档案启用保持（US6-4） |
| 2.13 | 支持声明收窄（去掉某 Agent） | 已有部署保留+警告，新部署禁止（US3-4） |
| 2.14 | GUI 及 `deploy to codex code/a code/b --also-target zcode`，随后同范围批量撤销或预演 | 一次涵盖多技能与多目标，逐组合过滤不适用项/暂停外改副本，其余继续；预演不落文件，撤销保留库技能（FR-021） |

## M3 — 获取与上游更新

| # | 操作 | 预期 |
|---|---|---|
| 3.1 | `skill preview <repo>` 后 `skill import <repo> --domain code --skills <subpath,...>`；按仓库总技能数确认分类 | 只有选中项入库、原名保留（US2-1）；多技能仓库默认仓库名分类可改，单技能仓库不自动分类（US2-2） |
| 3.2 | 上游带 `.manager` 的技能入库 | 标签/译文/注释/声明被继承；缺声明=域内全支持（US2-3） |
| 3.3 | 较大仓库（≥100MB 或慢速远端）获取；分别 GUI 取消、CLI Ctrl-C | 有真实阶段及进展量；取消后获取停止、不继续获取、不误报入库；可保留获取缓存，不要求所有临时字节清零（US2-4、SC-008） |
| 3.4 | `skill adopt` Agent 未受管技能（单项+批量），补资料 | 入库+接管原部署，无需再部署（US2-5） |
| 3.5 | `skill create` 只建不部署 | 无任何部署产生（US2-6） |
| 3.6 | 上游推进新版本（含文件增/删/改、管理资料变化、新增技能、改名）→ `skill check --all` | 按仓库展示变化；跨两域只查一次；检查不应用（US4-1/2/9） |
| 3.7 | `skill update <ref>` 单项 | 仅该技能更新+部署同步；同仓库其他不受影响（US4-3） |
| 3.8 | 更新含上游 `.manager` 同项资料 | 上游值生效（本地改过同项也是）；未提供项本地保留；分类保留（US4-5） |
| 3.9 | 新增候选：选 1 个入库、1 个明确不入库 → 再次检查 | 只有选中项入库；被拒项不再提示（US4-6） |
| 3.10 | `skill skip` 当前版本 → 仓库批量更新 | 提示移除且批量更新尊重跳过；`skill skipped` 可查看/恢复（US4-7） |
| 3.11 | 分别制造技能内文件删除、整个技能删除、技能改名 | 单文件删除只移除对应文件，其他变化照常应用、技能及部署继续保留更新；整个技能删除经确认才清库+部署（外改副本暂停）；改名呈现为删除旧技能+新增新技能（US4-8） |
| 3.12 | 首次仓库已有 A/C，只入库 A；之后新增 B，再更新 A，B 暂不决定 | C 不冒充后来新增，也不被记作明确拒绝；当前仍存在的 B 保持新增候选提示，更新 A 不等于处理 B；手动预览仍可选择 C（FR-011/033） |

## M4 — 跨机同步与指令

| # | 操作 | 预期 |
|---|---|---|
| 4.1 | 机 A：GUI 保存修改 | 自动提交推送自己的库；来源仓库无变化（US5-1） |
| 4.2 | 机 A：模拟 Agent 修改 → `sync push`；制造一次已获同意上传失败后用户主动重试 | 仅获同意才上传；主动重试可继续该次上传，后台路径不提交或推送；实际提交包含整个工作区，不按改动归属拆分（US5-2/3） |
| 4.3 | 机 B：`sync pull`（无冲突） | 静默应用+已有部署刷新；库内新增技能**不**自动部署（US5-4、SC-006） |
| 4.4 | 制造双端冲突 → pull | 同步暂停+提示，`sync conflicts` 输出详情；修复后继续（US5-5） |
| 4.5 | 断开远端（改坏 URL） | 显示未连接；本地管理、部署、deploy sync 正常（US5-6） |
| 4.6 | 机 B 存在本机部署记录/档案启用 | 拉 A 的库变化后 B 的部署记录、启用名单、Agent 配置不变（US5-7、SC-006） |
| 4.7 | `instruction set <key> --file <path>`；GUI 沿用单编辑区与单保存动作 | 目标 `AGENTS.md` = 全局前+专属后覆盖；无专属用全局，清空可保存；无独立分发勾选/双按钮，保存后保持打开、切换工具保留草稿（US8-4） |
| 4.8 | 机 B 拉取到全局或专属指令源变化后 | 源一致，按 B 已有启用配置/指令路径自动刷新相应 AGENTS.md，全局前专属后；不新增启用目标或改变本机路径，失败如实显示（US8-5） |

## M5 — 预设、工具中心与收尾

| # | 操作 | 预期 |
|---|---|---|
| 5.1 | 预设含 B,C 部署到已有 A,B 的目标 | 结果 A,B,C——叠加（US7-1） |
| 5.2 | `deploy undo-preset <domain> <id> --from <target>` | 剩 A——按成员撤销，跨预设成员也撤销（US7-2）；两域同 id 不误选 |
| 5.3 | 编辑预设成员 | 定义变化，部署不变（US7-3）；机 B 可见同一定义（US7-4） |
| 5.4 | 新电脑（新本机状态）接入已有工具的库 | 未安装工具显示"未安装"可暗显，不隐藏（US9-1） |
| 5.5 | Agent 通过 `tool register --dir <path>` 登记及 `tool set <slug> --dir <path>` 修改，再 `tool check <slug>`；含未安装工具 | 软件保存检查结果并展示版本；未检查/失败/未登记命令/可更新分别表达，未安装工具获取最新版本仍显示未安装；安装事实不变（US9-2） |
| 5.6 | `tool task <slug> --action install` 交给 Agent → Agent 安装 → `tool set-install <slug> --version <actual>`；GUI 生成/复制相同任务 | 任务包含配套维护 Skill 库内路径与“按其规范执行”指引（FR-051）；生成≠已完成，实际安装与回写由外部 Agent 负责；回写后为真实版本，检查状态重置未检查；失败如实（US9-4） |
| 5.7 | 配套 Skill 候选入库 | 列出具体候选与资料，勾选入库，不自动部署（US9-6） |
| 5.8 | 定时器（缩短周期验证） | 到期自动执行上游检查+库同步+工具检查命令，结果分别提示（FR-029 共用周期） |
| 5.9 | 主题/字号/关窗收托盘/托盘检查/诊断导出/容量报告 | US10 各场景 |
| 5.10 | 配套维护 Skill：`skill import <本项目 Gitea 远端> --domain code --skills skill-manager/assets/skills/skill-manager-maintenance` 入库 → 本仓库技能源推进新版本 → `skill check`/`skill update` | 走普通上游流程入库与更新，来源记录正确；任务书引用路径可解析（FR-051、D9） |
| 5.11 | **SC-001 完整旅程**：真实库上依次 入库→维护→部署→吸收一次上游更新→上传→机 B 拉取继续使用 | 全链贯通，每步与规格场景一致 |
| 5.12 | 分别调整预设间顺序、某预设内成员顺序，使用 GUI 及 `preset reorder-members <domain> <id> --order <name,...>`，另一机拉取 | 两种显示顺序均共享；成员排序只改变数组顺序，不增减成员、不动目录、不改变已有部署（FR-025） |
| 5.13 | 为一个技能和多个技能分别生成 `skill task <ref>... --requirement <text>` | 所选技能上下文完整且可定位；保留用户要求、库/技能路径、管理资料概况与已有部署，规范只引用维护 Skill；生成不改库/部署/上传（FR-051） |

### 5.10 固定来源与实际命令（配套维护 Skill）

配套维护 Skill 的源文件维护在本项目仓库 `skill-manager/assets/skills/skill-manager-maintenance/`（技能目录名即技能名
`skill-manager-maintenance`）。按 D9/FR-051，它作为**普通上游技能**入库：本项目推送用户 Gitea 远端后，
以该项目远端为上游仓库、以 `skill-manager/assets/skills/skill-manager-maintenance` 为来源子路径入库。

本项目 `origin` = `http://192.168.200.7:3000/yfpoyuas/SkillManager.git`（Gitea，2026-10-15 核对）；
另有 `github` 远端。**远端写入（提交/推送）未获用户授权**，因此 Gitea 路径的入库/更新保持“待授权”。

实际命令（`<repo-url>` 为本项目远端地址）：

```bash
skill-manager skill preview <repo-url> --ref main
skill-manager skill import <repo-url> --domain code \
    --skills skill-manager/assets/skills/skill-manager-maintenance --ref main --category 维护
# 上游推进新版本（本项目推送后）：
skill-manager skill check code/skill-manager-maintenance
skill-manager skill update code/skill-manager-maintenance
```

- 入库后来源记录 `source.yaml` 的 `repo` 指向本项目远端、`subpath` 为上述子路径、`version` 为采用提交；
  后续更新走既有 `skill check` / `skill update`，不新增专用分发或升级机制。
- 技能**未入库**时，软件生成的维护任务如实显示“配套维护 Skill：未入库”，不编造库内路径。
- 机制验证（非 Gitea 发布验收）：以**临时本地 Git 仓库**（内容复制自本项目该目录）作为普通上游，
  完整执行 `preview → import → 推进上游新版本 → check（只展示）→ update（显式采用）`，
  来源记录与采用版本推进均正确，任务书引用路径可解析。证据见 m5 报告 5.10 与汇报。


## M5 补充 — 本机运行侧双入口（2026-10-16 规划补齐）

对应 spec.md FR-050「本机运行侧」、FR-052、FR-053 与 contracts/cli.md 的 `settings` 命令组；
实现任务见 tasks.md T093～T096。

| # | 操作 | 预期 |
|---|---|---|
| 5.14 | `settings get`（不带参数与带参数各一次） | 返回全部键的**当前生效值**（主题缺省 `system`、检查频率缺省 60、未设置的共享目录为 `null`）；带参数只返回指定键；未设置的键返回 `null`，不省略键、不用空字符串冒充 |
| 5.15 | 纯 CLI 环境（无 GUI 参与）：`settings set-shared-dir --path <临时目录>` → `deploy to shared <ref>` | 设置后共享目录部署成功；`settings set-shared-dir --clear` 后 `deploy to shared` 如实报"尚未设置共享目录路径" |
| 5.16 | `settings set-auto-check 0` 与 `settings set-auto-check 30`；再试 `settings set-auto-check 45` | 前两者写入成功并可由 `settings get` 读回；45 被拒绝并给出合法取值（0/30/60/360/1440） |
| 5.17 | `settings diagnostics`、`settings diagnostics --export`、`settings capacity` | 诊断输出含版本/平台/真实日志路径；`--export` 写出真实文件并返回其路径；容量输出库总占用与较大技能排行，空库返回 0 与空排行 |
| 5.18 | 按 FR-050 边界核对不允许的写入 | `theme`/`font_scale`/`git_auto_push` **没有** CLI 写入口（命令不存在或明确不支持），只能经 `settings get` 读取；工具安装事实回写仍只有 CLI（GUI 无入口） |
| 5.19 | 清理项核对 | 设置中不再存在自定义日志目录键；Hermes 根按 `HERMES_HOME` 与平台默认解析，`settings get` 不再返回可写的 Hermes 根键 |

## 自动化与人工分工

- M1~M3 的 CLI 场景（1.4~1.8、2.x 命令部分、3.x）纳入集成测试（`tests/`，cargo test 驱动 CLI 断言 JSON 输出与文件系统结果）；
- GUI 交互、真实 Agent 目录效果、双机模拟、大仓库实测为人工执行项，按上表逐条记录证据（截图/命令输出）；
- 每期核对表格式：`场景号 | 预期 | 实际 | 结论（通过/未过+偏差说明）`，存于本目录 `checklists/m<N>-report.md`。
