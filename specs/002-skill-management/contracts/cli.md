# CLI Contract: 命令表与覆盖对照

**Date**: 2026-10-15 | 原则（research.md D6）：领域子命令、全局 `--json`、无交互；决策由 Agent 对话获得用户明确指令后以参数传入。

约定：`<domain>` = `code|lobster`；`<ref>` = `<domain>/<name>`；`<target>` = Agent key、`shared` 或 `hermes:default`。所有命令支持全局 `--json`、`--vault <path>`、`--state-dir <path>`；后者隔离本机 SQLite、检查缓存与日志，用于两机模拟，不改变共享内容真值。

`a | b` 表示互斥参数，`...` 表示可重复条目；未给的编辑参数表示保持原值，不表示清空。`--category <c> | --uncategorized` 区分一级分类与明确未分类。命令和字段是待实现契约，不表示软件已经可运行。

## 安装后的调用入口（FR-050/051）

CLI 随桌面安装包交付，版本与桌面程序一致。维护 Skill 提供定位步骤；调用不依赖 GUI 运行，也不要求安装目录在 `PATH` 中。

- macOS：`<Skill Manager.app>/Contents/MacOS/skill-manager`，常规安装位置为 `/Applications/Skill Manager.app` 或 `~/Applications/Skill Manager.app`。
- Windows：`<桌面程序安装目录>\skill-manager.exe`，自定义安装时由应用快捷方式目标定位目录；PowerShell 以 `& "<绝对路径>" <参数>` 调用。
- 文中 `skill-manager` 是该可执行文件的简写。首次使用先执行绝对路径下的 `--version` 与 `--help`；找不到或执行失败如实报告，不误判为仅缺少 PATH，不把开发产物当作已安装程序。

## 命令表

### skill — 技能与库管理

```text
skill list [--domain <domain>] [--query <text>] [--category <c> | --uncategorized]
           [--tag <t>...] [--preset <id>] [--agent <key>] [--deployed-to <target>] [--no-tag]
skill get <ref>                                             # 正文/资料/来源/文件树/实际部署位置
skill read <ref> [--lang zh|orig]                            # 有译文默认中文
skill create <ref> [--category <c> | --uncategorized] [--comment <text>] [--tags <csv>]
skill set <ref> [--comment <text>] [--tags <csv>] [--agents all|none|<csv>] [--oses all|none|<csv>]
skill tag add <ref>... --tags <csv> | skill tag detach <ref>... --tags <csv>
skill tag rename <old> <new> | skill tag remove <tag>        # 两域统一治理；remove 只解除关联
skill category rename --domain <domain> <old> <new>          # 重命名一级分类目录，该分类下全部技能一并移动
skill move <ref> (--category <c> | --uncategorized)          # 移目录、不改名
skill copy <ref> --to <domain> [--category <c> | --uncategorized]
skill delete <ref>...                                       # 删库并清理；本地外改副本暂停
skill translation get <ref>
skill translation set <ref> (--file <path> | --stdin)        # 按当前原文保存译文并更新时效依据
skill translation remove <ref>
skill overlay list <ref>
skill overlay get <ref> --path <relative-path> [--agent <key>] [--os <os>]
skill overlay set <ref> --path <relative-path> (--file <path> | --stdin) [--agent <key>] [--os <os>]
skill overlay remove <ref> --path <relative-path> [--agent <key>] [--os <os>]
skill task <ref>... [--requirement <text>]                     # 单项/多选技能维护任务
```

- `--agents all` / `--oses all` 删除对应限制键，`none` 写显式空集合；未传保持原声明，不能清除本机未显示的 Agent 声明。
- `--tags` 是替换标签集合；`tag add/detach` 仅增减目标标签；空字符串清空集合。`--preset` 须同时给 `--domain`，避免跨域同 id 歧义。
- overlay 命令省略的维度表示该维度通用；同一相对路径的优先顺序与文件布局由 M2 T021 先落定，不在入口另定规则。
- `skill task` 对单个或多个明确技能生成一份维护说明；保留 FR-051 规定的用户要求、库/技能路径、管理资料概况和本机已有部署。规范仅引用配套维护 Skill，正文/完整文件由 Agent 到库内读取；生成任务不改库、不部署、不上传。
  `--requirement` 可省略（与 `tool task` 一致），省略时任务文本如实写明"未填写（按技能现有管理资料与配套维护 Skill 规范执行）"，不编造用户要求。契约不把用户要求设为必填：FR-051 只要求"保留用户要求"，省略即如实表达为未填写，比强制报错更贴近"生成可复制任务"的用途，且与工具维护任务的处理一致；实现按此为准。

### skill — 获取与上游维护

```text
skill preview <repo-url> [--ref <branch-or-tag>]             # 获取/列出候选，不入库；可取消
skill import <repo-url> --domain <domain> --skills <subpath,...>
             [--ref <branch-or-tag>] [--category <c> | --uncategorized]
skill adopt <target> <name>... [--category <c> | --uncategorized]
            [--comment <text>] [--tags <csv>] [--metadata-file <path>]
skill adopt-hermes <profile> <name>...
skill check (<ref> | --repo <url> | --all)                    # 检查 B；不拉取自己的 SkillManagerVault
skill update <ref> [--to <commit-oid>]                       # 单项显式采用
skill update-repo <repo-url> [--include-new <subpath,...> --domain <domain>]
                  [--category <c> | --uncategorized]
skill reject-new <repo-url> --domain <domain> --skills <subpath,...>
skill rejected [--repo <url>] [--domain <domain>]
skill skip <ref> [--version <commit-oid>] | skill unskip <ref> | skill skipped
```

- 来源候选按上游子路径选择，保留其技能原名；`preview` / `import` / 上游新增列出的候选均已跳过相对仓库根 `.agents/skills/`（FR-058），该路径下的技能不出现在输出中。多技能仓库默认分类按**跳过之后**的候选数量决定，CLI 由 Agent 与用户确认分类后传入；只剩一个候选时默认不分类。`--uncategorized` 可显式选择不分类。指定 `--skills` 若落在该排除路径下，按不是技能报错，不回退仓库根。
- `preview/import` 显示真实 Git 进度，Ctrl-C 取消本次获取；`--json` 的最终结果在标准输出，进度在标准错误，Tauri 使用同一核心的进度事件。
- `adopt` 的 `<target>` 接受内置 Agent key 或 `shared`；管理域由目标所属域决定，共享目标进入编码库并接管原共享部署。批量不同资料使用 `--metadata-file`，按技能名给各自管理资料，公共参数只能表示用户明确要求的共用值。收编不强制来源，不新增其他目标的首次部署。
- 来源/采用版本记录随唯一 SkillManagerVault 跨机共享。另一机 `sync pull` 不重新执行 import；`skill check/update` 依据技能来源检查和获取上游 B。
- `--to` / `--version` 缺省采用最近检查所得的具体远端提交；尚无检查结果则提示先检查，不在应用时静默选择另一个版本。`update-repo` 作用于两域该仓库未采用、未跳过的已有条目，新增仅处理显式选择及所给域。
- `reject-new` 是明确“不入库并记住”，记录位置见 data-model.md §1.6；不选择、返回或取消均不调用它。`update --to <oid>` 可明确采用已经跳过的版本；普通仓库批量更新尊重跳过决定。
- 新增候选的“后来出现”含义及首次未选条目的例子见 spec.md“业务关系与阅读示例”；检查或更新已有技能不等于拒绝未决定、当前仍存在的新增候选，发现基线由 T039/T043 落定，不与本机检查时间混同。

### deploy — 部署与同步

```text
deploy to <target> <ref>... [--also-target <target>...]
deploy preset <domain> <preset-id> --to <target> [--also-target <target>...]
deploy undo <target> <ref>... [--also-target <target>...] [--discard-local-changes]
deploy undo-preset <domain> <preset-id> --from <target> [--also-target <target>...]
deploy sync [--ref <ref>] [--target <target>] [--overwrite-local]
deploy status [--target <target>] [--paused]
```

- `--also-target` 指定本次其他目标，支持一次多技能至多目标，以及预设至多目标；每个技能/目标组合复用同一准入、暂停和结果表达，不增加批次实体或整批回滚。未传则只处理一个目标。
- 普通部署/同步/撤销遇受管副本外改均暂停该项。Agent 整理并获得用户裁决后，可对明确的单项 `--ref` 与 `--target` 使用 `sync --overwrite-local` 按库覆盖；确认丢弃待清理副本时可显式 `undo --discard-local-changes`。这是执行裁决的参数，不是自动合并或冲突分析功能；GUI 给出暂停信息和外部维护任务，不提供内置合并界面。
- 撤销保留库技能。库技能已删除而副本暂停时，仍可按本机记录定位并继续清理；记录在副本成功移除后才删除。

### preset — 预设

```text
preset list [--domain <domain>] | preset get <domain> <id>
preset create <domain> <name> [--icon <icon>] | preset delete <domain> <id>
preset members <domain> <id> (--add <csv> | --remove <csv>)
preset rename <domain> <id> <new> | preset icon <domain> <id> <icon>
preset reorder <domain> --order <id,...>
preset reorder-members <domain> <id> --order <name,...>
```

`reorder` 调整预设间顺序，`reorder-members` 调整指定预设内现有成员的显示顺序；成员数组顺序即显示顺序，不另设 member_order 字段。两种排序不改变成员集合、分类目录或部署。定义编辑不动部署；部署叠加，撤销按成员，不追踪来源预设或引用计数，缺失成员报告后其余继续。

### hermes — 档案

```text
hermes profiles | hermes list <profile>
hermes read <profile> <name> [--file <path>]
hermes enable <profile> <name>... | hermes disable <profile> <name>...
```

只管理现有单根下的档案技能；default 部署与其他档案明确启用分开。启用沿用龙虾域/支持声明，新内容不重置已有选择。
`read` 只读该档案的实际技能正文或目录内文件；不要求先收编，不改变库、部署或启用名单。

### tool — 工具中心

```text
tool list [--query <text>] [--category <c>]
          [--status installed|not-installed|updatable|latest|failed|unchecked|no-command|no-guide]
tool get <slug>
tool check (<slug> | --all)
tool register (--dir <path> | --stdin)
tool set <slug> (--dir <path> | --stdin)
tool set-install <slug> --version <v> [--method <text>] [--path <path>] [--config-path <path>]
tool task [<slug>] --action register|edit|install|update|guide [--requirement <text>]
tool skill-link <slug> <ref>                                 # 配套 Skill 来源资料随 tool set 维护
```

- `register` 新登记，`set` 修改已有共享资料；`--dir` 提供 tool.yaml、guide.md 及其所需检查脚本；`--stdin` 接收对象 `{"tool": <字段映射>, "guide": <正文>}`，其他脚本通过资料目录维护。登记的 slug 由 `tool.yaml` 的 `name` 派生（保留 ASCII 字母数字，其余折为 `-`）；已登记时 `register` 报错，改用 `set`。`--dir` 中缺失的 `guide.md` 或其他文件表示保持原值，不表示清空；未知字段写回时保留，且值与其字符串形态（含键）不被 YAML 隐式类型改写（同 data-model.md §1.3 写入规则）。
- 工具登记、资料维护、安装与验证回写由外部 Agent 完成；GUI 生成/复制对应任务，全部实际管理动作 CLI 可达。`task --action register` 允许尚无 slug（由 `--requirement` 说明工具名称与来源/安装要求）；其他动作必须给已登记工具。任务不等于安装或更新完成。
- 检查只执行登记的单个最新版本检查命令，保存检查事实，不改 `tool_installs`。无命令、失败、未检查分别表达；未安装工具获得上游版本后仍是未安装，不显示“本机最新”。版本比较约定见 data-model.md §1.5（T074 落定）；无法比较或上游低于本机记录时如实说明，不冒充“最新/可更新”。
- `set-install` 仅在 Agent 核实实际结果后回写安装事实并重置该工具检查结果；命令本身不安装程序。`--version ""`（空字符串）表示 Agent 核实该工具已不在本机：清除安装记录并同样重置检查结果。工具版本与配套技能版本互不代替。
- `skill-link <slug> <ref>` 追加一个配套 Skill 关联（`<ref>` = `<domain>/<name>`）；来源资料（repo/subpath）随 `tool set` 维护。未入库时如实显示“未入库”，可经 `skill preview`/`skill import` 选择入库，入库不自动部署。

### sync — SkillManagerVault 跨机同步

```text
sync pull                                                   # 拉取同一 SkillManagerVault，刷新已有部署/指令目标
sync push [--message <text>]                                # 获同意后显式上传工作区全部改动
sync status                                                 # 连接/未提交/未拉取事实
sync conflicts                                              # Git 冲突信息供 Agent 检查
```

- GUI 保存沿用已有自动上传确认（设置键 `git_auto_push`，见 data-model.md §2.6）：首次保存时询问一次，确认后每次保存自动 `push`；Agent 修改后每次询问得到明确同意才 `push`。CLI 写库、`deploy sync`、文件监听及后台检查/重试都不构成上传授权，也不读 `git_auto_push`。
- `sync pull` 遇到未提交改动时先暂存再合并，成功后恢复；不提交、不推送。`upstream-decisions.yaml` 按 `(repo, domain)` 并集。与远端同一文件无法自动合并才按冲突暂停，不得因 “local changes would be overwritten by merge” 整库中止。
- 已同意的上传失败可由用户主动重试（再次 `sync push`）；后台路径不提交或推送。冲突由 Agent 按用户裁决完成 Git 层处理后，再 `sync pull` 继续；不新增应用内合并流程或“保留本地/远端/双方”选项。冲突时本次同步暂停并保留 Git 冲突现场，`sync conflicts` 输出未合并路径与状态供 Agent 检查。
- `sync status` 只读事实：连接状态（远端可用/不可用）、未提交改动、相对 upstream 的未拉取提交数、进行中的冲突；不提交、不推送、不改库。
- 远端不可用时 `sync pull`/`sync push` 明确报“未连接”，本地库管理、部署与 `deploy sync` 继续可用；不提供断开远端或重新克隆修复入口。
- 库同步与技能上游更新分开；拉取新增只让库内容一致，不产生首次部署/档案启用/工具安装。拉取到指令源变化后按同一拼接规则刷新本机已有启用指令目标（不新增启用目标、不改本机路径）。

### agent / instruction — 本机配置与指令

```text
agent list | agent detect
agent enable <key> | agent disable <key> | agent reorder --order <key,...>
agent set-dir <key> (--skills <path> | --project-rel <path> | --instructions <path>)
agent reset-dir <key> --kind skills|project-rel|instructions
agent local-skills <target> | agent delete-local <target> <name>...
agent read-local <target> <name> [--file <技能内相对路径>]
instruction get global|<key>
instruction set global|<key> (--file <path> | --stdin)
```

`local-skills` / `delete-local` 的 `<target>` 接受内置 Agent key、`shared` 或 `hermes:default`：共享目录与 Hermes default 档案同为本机部署目标（FR-042/FR-055），
按内置 Agent 的同一套语义列出与处理——受管项（存在该目标部署记录）按撤销部署规则处理，
未受管项直接删除实际文件。共享目录自动定位为 `<当前用户主目录>/.agents/skills`；不存在时浏览返回空列表且不创建目录，首次部署时创建；
`shared` 只对编码域有效（FR-021）。

`read-local` 接受相同三类目标，默认阅读本机技能的 `SKILL.md` 或 `skill.md`，不使用库内原件替代。返回 `name/path/file/text/size/files`，`files` 为可阅读文件的相对路径列表，非 UTF-8 文件的 `text` 为 `null`；只允许读取所选技能目录内的文件，不修改库、部署或本机状态记录。

Agent 目录/启停/顺序只在本机；检测不启动、不部署。指令编辑沿用单保存动作，保存源并按“全局+专属”覆盖本机目标；没有独立分发勾选。指令路径与技能路径独立，不把源跨机一致视为已经完成目标分发。用户已确认：`sync pull` 拉取到指令源变化后，按同一拼接覆盖规则自动刷新本机已有启用目标，不改变启用名单/指令路径；刷新失败分别报告，不把源同步成功冒充目标刷新成功。

- `instruction get global|<key>` 返回该作用域的**源内容**（`instructions/global.md` 或 `instructions/<key>.md`；文件不存在按空内容）以及按拼接规则得到的合成预览；`instruction set` 写入源后立即按“全局在前、专属在后”覆盖本机已启用目标的 `<detect_dir>/AGENTS.md`（可被 `agent set-dir <key> --instructions` 覆盖），空内容正常保存并覆盖目标。单目标写入失败不中断其余，逐项如实报告。

### settings — 本机运行配置与只读报告

```text
settings get [<key>...]                                  # 只读：运行配置与调度事实
settings set-auto-check <0|30|60|360|1440>
settings diagnostics [--export]
settings capacity
```

2026-10-16 按 spec.md FR-050 的“本机运行侧”条款补齐（此前这一整类只有 GUI 入口）：

- `settings get` 不带参数返回全部键的**当前生效值**（含默认值推导，例如主题缺省 `system`、检查频率缺省 60、共享目录为当前用户主目录下的 `.agents/skills`）；带参数只返回指定键。**未设置的键返回 `null`**，不省略键、不用空字符串冒充。
- 只暴露**一个写入口**：`auto_check_minutes`（FR-029 三项共用周期，值须为 0/30/60/360/1440）。其余键按 FR-050 保持单侧：
  - `theme`、`font_scale` **不提供写**（界面偏好，不影响管理结果，见 FR-052）；
  - `git_auto_push`（GUI 保存自动上传的长期授权记录）**不提供写**——该授权只能由用户在 GUI 给出，Agent 的上传仍需每次取得明确同意后显式 `sync push`（FR-038）；允许 CLI 读取以便解释当前行为。
- `shared_dir` 只读派生自当前系统用户主目录下的 `.agents/skills`，不写 SQLite，不提供修改或清除路径的命令。
- 手动检查**不新增组合命令**：CLI 以 `skill check --all`、`sync pull`、`tool check --all` 分别表达同一批动作（FR-029 的三项），不合并成一条命令。
- `settings diagnostics` 输出诊断报告内容与日志/崩溃记录的**真实文件路径**；`--export` 追加写出可交给 Agent 的文件并返回其路径。**不提供上传诊断**。
- `settings capacity` 输出库总占用与较大技能排行；空库如实返回 0 与空排行。
- 这一组命令只读写本机状态，不改 SkillManagerVault 内容、不触发上传。

## 输出与状态约定

- 默认中文人类可读；`--json` 使用统一对象 `result`（`success|partial|failed`）、`data`、`errors`；批量逐项给出对象、结果与原因；错误含技能/目标/实际路径。成功退出 0，失败或部分未完成退出非零。
- 上游/工具检查可以保存本机检查结果、时间与获取缓存；不应用技能变化、不变更采用版本/部署、不改工具安装事实。`sync status/conflicts` 只读取事实，不提交或推送。
- 管理域边界与支持声明限制新增部署/档案启用；不阻止库内维护、撤销或禁用。已有使用因范围收窄保留并警告。
- 批量不适用、同名、暂停、失败逐项报告，其余继续；不要求整批回滚。库内同名写入阻止与未受管目标同名覆盖不是同一种冲突。

## FR-050 覆盖对照

| 规格要求 | CLI 路径 | GUI / 责任边界 |
|---|---|---|
| 组合查询/正文/文件详情 | skill list/get/read | 同一核心查询与详情 |
| 入库/收编/自建/跨域复制 | skill preview/import/adopt/adopt-hermes/create/copy | 部分入库/收编/复制；自建可生成 Agent 任务 |
| 上游检查/单项与仓库更新 | skill check/update/update-repo | 查看后显式采用 |
| 新增拒绝/版本跳过与恢复 | skill reject-new/rejected/skip/unskip/skipped，update --to | 明确拒绝、查看跳过及恢复/采用 |
| 部署/撤销/同步/暂停处置 | deploy to/undo/sync/status | 同一核心；暂停后交 Agent/用户裁决 |
| 预设 | preset *（含 reorder/reorder-members），deploy preset/undo-preset | 定义/两种显示排序与部署分开 |
| 标签/分类/注释/支持范围 | skill tag/set/move、skill category rename | 同一核心资料维护 |
| 译文/覆盖层维护 | skill translation * / overlay * | 同一管理资料入口 |
| Hermes | hermes *，skill adopt-hermes | 只读详情/明确启停/收编 |
| 技能维护任务 | skill task <ref>... | 单项/多选，保留事实上下文，规范引用维护 Skill |
| 工具清单/状态/检查 | tool list/get/check | 软件执行登记检查 |
| 工具登记/修改/本机回写 | tool register/set/set-install/skill-link | GUI 交外部 Agent 的任务入口，Agent 核实后 CLI 写回 |
| 工具维护任务 | tool task | 生成/复制，不冒充执行 |
| 库同步 | sync * | 同意边界与全量提交一致 |
| Agent 配置/本地技能/指令 | agent *，instruction * | 本机配置与单保存指令入口 |
| 本机运行配置与只读报告 | settings get/set-auto-check/diagnostics/capacity | 设置页同一核心；主题/字号与上传长期授权不提供 CLI 写入 |

**覆盖结论**：上表为需求与命令契约映射，不是命令已实现的证明。各入口实施后仍须按 SC-002 实际验收；CHK026 已确认拉取后自动刷新本机已有启用指令目标。

## T090 双入口核对（FR-050/051，2026-10-15）

核对方式：`skill-manager <命令> --help` 逐条与本节命令表比对（参数、子命令、互斥关系），再在**临时库 + 临时 HOME + 独立 `--state-dir`** 中实际执行每条命令，记录退出码与输出；GUI 侧用同一核心的 Tauri 命令，逐项对照其 CLI 等价路径。汇总：53 条调用、91 个参数与实现一致；102 条子命令实际执行，除下列有意用例（缺共享目录、已登记重复登记、删除不存在的本机副本）外全部退出 0。

**核对项目（每项：CLI 路径 | 验证方式 | 结论）**

| FR-050 操作 | CLI 路径 | 验证方式 | 结论 |
|---|---|---|---|
| 组合查询/正文/文件详情 | `skill list`（含 `--deployed-to` 按本机部署记录筛选）/`get`/`read` | 实际执行并核对 JSON 键 | 通过 |
| 入库/收编/自建/跨域复制 | `skill preview/import/adopt/adopt-hermes/create/copy` | 实际执行；入库含显式子路径与来源记录 | 通过 |
| 上游检查/单项与仓库更新 | `skill check`（`<ref>`/`--repo`/`--all` 三者之一必给）/`update`/`update-repo` | 实际执行；检查只写事实不应用 | 通过 |
| 新增拒绝/跳过与恢复 | `skill reject-new/rejected/skip/unskip/skipped`，`update --to` | 实际执行 | 通过 |
| 部署/撤销/同步/暂停处置 | `deploy to/preset/undo/undo-preset/sync/status` | 实际执行 | 通过 |
| 预设 | `preset list/get/create/delete/members/rename/icon/reorder/reorder-members` | 实际执行；两种排序校验全量排列 | 通过 |
| 标签/分类/注释/支持范围 | `skill tag add/detach/rename/remove`、`skill set`、`skill move`、`skill category rename` | 实际执行；`--agents all/none/csv` 写回核对 | 通过 |
| 译文/覆盖层维护 | `skill translation get/set/remove`、`skill overlay list/get/set/remove` | 实际执行；`set` 缺 `--file/--stdin` 报错退出 1 | 通过 |
| Hermes | `hermes profiles/list/enable/disable`，`skill adopt-hermes` | 实际执行（假根 + 具名档案） | 通过 |
| 技能维护任务 | `skill task <ref>... [--requirement <text>]` | 实际执行；任务引用维护 Skill **真实库内路径** | 通过 |
| 工具清单/状态/检查 | `tool list`（`--status` 八态）/`get`/`check`（单项/`--all`） | 实际执行 | 通过 |
| 工具登记/修改/本机回写 | `tool register/set`（`--dir`/`--stdin`）/`set-install`/`skill-link` | 实际执行；`--version ""` 清记录并重置检查 | 通过 |
| 工具维护任务 | `tool task --action register/edit/install/update/guide` | 实际执行；`register` 可无 slug | 通过 |
| 库同步 | `sync pull/push/status/conflicts` | 实际执行（临时裸远端）；后台路径不推送 | 通过 |
| Agent 配置/本地技能/指令 | `agent list/detect/enable/disable/reorder/set-dir/reset-dir/local-skills/delete-local`，`instruction get/set` | 实际执行（临时 HOME 假目录） | 通过 |
| 本机运行配置与只读报告 | `settings get/set-auto-check/diagnostics/capacity` | 实际执行（临时库 + 临时 HOME + 独立 `--state-dir`）：`get` 全键/指定键与 `null` 语义、`shared_dir` 自动定位，未配置即可 `deploy to shared`，目录不存在时浏览不创建而首次部署创建，旧路径设置不再生效且已删除写入命令、`set-auto-check` 接受 0/30/60/360/1440 并拒绝 45、`theme`/`font_scale`/`git_auto_push` 无 CLI 写入口（`--help` 无对应子命令，调用退出 2）、`diagnostics --export` 返回真实存在文件、`capacity` 空库返回 0 与空排行；并与 `--help` 逐项比对子命令与互斥关系。自动化见 `skill-manager/tests/cli_settings.rs`（8 项） | 通过 |

**JSON/退出码核对**：`--json` 输出统一 `result`(`success|partial|failed`)/`data`/`errors`；退出码 0/1/2 = 成功/失败/部分完成。**已知碰撞**：clap 的参数用法错误也返回 2，调用方需按 `errors` 或提示前缀区分（本节如实记录，未新增机制消除该碰撞）。

**无 GUI 裁决依赖核对**：GUI 的 85 个前端调用名与 87 个 Tauri 注册命令逐一对齐（无前端调用缺失），全部命令共用 core，均只做参数映射。除下方缺口 ① 外，所有核心管理操作均有 CLI 等价路径，**Agent 无需回 GUI 补做核心操作**；缺口 ① 已由 2026-10-16 的 `settings` 命令组补齐并实现，见上表 `settings` 行结论。但 GUI 独有的界面偏好写入仍有边界：`theme`/`font_scale`/`git_auto_push` 不由 CLI 写（FR-052/FR-038）。

**维护任务 Skill 路径核对**：配套维护 Skill 未入库时，任务文本显示"配套维护 Skill：未入库（…此处不编造库内路径）"；入库后显示其**真实库内绝对路径**与技能引用，并含"按其规范执行"（FR-051）。均已实际执行验证。

**发现的缺口与处理**

1. **本机运行侧整类缺 CLI 入口**（原记为"仅共享目录路径"，2026-10-16 逐项清点后确认为整类，已在 `spec.md` FR-050/FR-052/FR-053 补齐表述并在本节补齐命令契约，见 `settings` 命令组与 tasks T093～T100）。清点结论：
   - **必须 CLI 可写**：`auto_check_minutes`（FR-029 三项共用周期）；
   - **必须 CLI 可读**：全部运行配置与调度事实（含仅由 GUI 写入的 `theme`、`font_scale`、`git_auto_push`）；
   - **不提供 CLI 写**：`theme`、`font_scale`（界面偏好）、`git_auto_push`（长期上传授权，FR-038）；
   - **有意单侧**：工具安装事实回写只有 CLI（FR-047/FR-050），不是缺口。
2. **清理两处无写入点的配置键**（T097 已实现）：自定义日志目录不再保留（日志固定 `<state-dir>/logs`）；Hermes 根改为按 `HERMES_HOME` 与平台默认解析，不设可写设置键。
3. **新增候选入库时的标签/注释**（M3 已记录）。`skill import` / `skill update-repo` 无 `tags`/`comment` 参数，故 GUI 与 CLI 一致地只能补充分类；标签/注释继承上游 `.manager`，入库后可用 `skill set` 补齐。这是契约层面的限制而非入口不对齐，**保持现状**：批量入库用 `--tags` 会把同一份资料套给全部技能，与"不把同一段注释擅自套给全部技能"冲突；若确需入库时逐项给资料，应复用 `skill adopt` 的 `--metadata-file` 形式，需用户裁决。
4. **`skill task --requirement` 原契约写必填、实现为可选** → 已按最小改动**修订契约文字**为可选（`[--requirement <text>]`），与 `tool task` 一致；理由见 `skill task` 条目注。
5. **契约文字与实现的细节修正**：`skill list --deployed-to` 已实现（按本机部署记录筛选），原实现内的帮助文字标注"尚未接入"属过时描述，已更正为"按本机部署记录"；行为本身无需改动。
6. **Hermes 档案独有技能的只读查看**：2026-10-17 已由 T125 补齐 `hermes read`、Tauri 与 GUI 阅读入口；查看不收编、不改变启用名单，CLI 与组件测试已验证。

