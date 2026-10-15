# CLI Contract: 命令表与覆盖对照

**Date**: 2026-10-15 | 原则（research.md D6）：领域子命令、全局 `--json`、无交互；决策由 Agent 对话获得用户明确指令后以参数传入。

约定：`<domain>` = `code|lobster`；`<ref>` = `<domain>/<name>`；`<target>` = Agent key、`shared` 或 `hermes:default`。所有命令支持全局 `--json`、`--vault <path>`、`--state-dir <path>`；后者隔离本机 SQLite、检查缓存与日志，用于两机模拟，不改变共享内容真值。

`a | b` 表示互斥参数，`...` 表示可重复条目；未给的编辑参数表示保持原值，不表示清空。`--category <c> | --uncategorized` 区分一级分类与明确未分类。命令和字段是待实现契约，不表示软件已经可运行。

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
skill task <ref>... --requirement <text>                     # 单项/多选技能维护任务
```

- `--agents all` / `--oses all` 删除对应限制键，`none` 写显式空集合；未传保持原声明，不能清除本机未显示的 Agent 声明。
- `--tags` 是替换标签集合；`tag add/detach` 仅增减目标标签；空字符串清空集合。`--preset` 须同时给 `--domain`，避免跨域同 id 歧义。
- overlay 命令省略的维度表示该维度通用；同一相对路径的优先顺序与文件布局由 M2 T021 先落定，不在入口另定规则。
- `skill task` 对单个或多个明确技能生成一份维护说明；保留 FR-051 规定的用户要求、库/技能路径、管理资料概况和本机已有部署。规范仅引用配套维护 Skill，正文/完整文件由 Agent 到库内读取；生成任务不改库、不部署、不上传。

### skill — 获取与上游维护

```text
skill preview <repo-url> [--ref <branch-or-tag>]             # 获取/列出候选，不入库；可取消
skill import <repo-url> --domain <domain> --skills <subpath,...>
             [--ref <branch-or-tag>] [--category <c> | --uncategorized]
skill adopt <agent-key> <name>... [--category <c> | --uncategorized]
            [--comment <text>] [--tags <csv>] [--metadata-file <path>]
skill adopt-hermes <profile> <name>...
skill check (<ref> | --repo <url> | --all)                    # 检查 B；不拉取自己的 SkillVault
skill update <ref> [--to <commit-oid>]                       # 单项显式采用
skill update-repo <repo-url> [--include-new <subpath,...> --domain <domain>]
                  [--category <c> | --uncategorized]
skill reject-new <repo-url> --domain <domain> --skills <subpath,...>
skill rejected [--repo <url>] [--domain <domain>]
skill skip <ref> [--version <commit-oid>] | skill unskip <ref> | skill skipped
```

- 来源候选按上游子路径选择，保留其技能原名；多技能仓库默认分类按仓库总技能数决定，CLI 由 Agent 与用户确认分类后传入；单技能仓库默认不分类。`--uncategorized` 可显式选择不分类。
- `preview/import` 显示真实 Git 进度，Ctrl-C 取消本次获取；`--json` 的最终结果在标准输出，进度在标准错误，Tauri 使用同一核心的进度事件。
- `adopt` 管理域由 Agent 所属域决定；批量不同资料使用 `--metadata-file`，按技能名给各自管理资料，公共参数只能表示用户明确要求的共用值。收编不强制来源，不新增其他目标的首次部署。
- 来源/采用版本记录随唯一 SkillVault 跨机共享。另一机 `sync pull` 不重新执行 import；`skill check/update` 依据技能来源检查和获取上游 B。
- `--to` / `--version` 缺省采用最近检查所得的具体远端提交；尚无检查结果则提示先检查，不在应用时静默选择另一个版本。`update-repo` 作用于两域该仓库未采用、未跳过的已有条目，新增仅处理显式选择及所给域。
- `reject-new` 是明确“不入库并记住”，记录位置见 data-model.md §1.6；不选择、返回或取消均不调用它。`update --to <oid>` 可明确采用已经跳过的版本；普通仓库批量更新尊重跳过决定。
- 新增候选的“后来出现”含义及首次未选条目的例子见 spec.md“业务关系与阅读示例”；检查或更新已有技能不等于拒绝未决定、当前仍存在的新增候选，发现基线由 T039/T043 落定，不与本机检查时间混同。

### deploy — 部署与同步

```text
deploy to <target> <ref>... [--also-target <target>...] [--dry-run]
deploy preset <domain> <preset-id> --to <target> [--also-target <target>...] [--dry-run]
deploy undo <target> <ref>... [--also-target <target>...] [--dry-run] [--discard-local-changes]
deploy undo-preset <domain> <preset-id> --from <target> [--also-target <target>...] [--dry-run]
deploy sync [--ref <ref>] [--target <target>] [--overwrite-local]
deploy status [--target <target>] [--paused]
```

- `--also-target` 指定本次其他目标，支持一次多技能至多目标，以及预设至多目标；每个技能/目标组合复用同一准入、暂停和结果表达，不增加批次实体或整批回滚。未传则只处理一个目标。
- 普通部署/同步/撤销遇受管副本外改均暂停该项。Agent 整理并获得用户裁决后，可对明确的单项 `--ref` 与 `--target` 使用 `sync --overwrite-local` 按库覆盖；确认丢弃待清理副本时可显式 `undo --discard-local-changes`。这是执行裁决的参数，不是自动合并或冲突分析功能；GUI 给出暂停信息和外部维护任务，不提供内置合并界面。
- `dry-run` 报告技能/Agent/实际路径/预期结果，不修改目标、库内容或业务状态；不同于可保存检查事实的上游/工具检查。
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
hermes enable <profile> <name>... | hermes disable <profile> <name>...
```

只管理现有单根下的档案技能；default 部署与其他档案明确启用分开。启用沿用龙虾域/支持声明，新内容不重置已有选择。

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

- `register` 新登记，`set` 修改已有共享资料；`--dir` 提供 tool.yaml、guide.md 及其所需检查脚本；`--stdin` 接收对象 `{"tool": <字段映射>, "guide": <正文>}`，其他脚本通过资料目录维护。
- 工具登记、资料维护、安装与验证回写由外部 Agent 完成；GUI 生成/复制对应任务，全部实际管理动作 CLI 可达。`task --action register` 允许尚无 slug；其他动作必须给已登记工具。任务不等于安装或更新完成。
- 检查只执行登记的单个最新版本检查命令，保存检查事实，不改 `tool_installs`。无命令、失败、未检查分别表达；未安装工具获得上游版本后仍是未安装，不显示“本机最新”。具体非标准版本比较由 T074 落定，不可比较的结果如实说明。
- `set-install` 仅在 Agent 核实实际结果后回写安装事实并重置该工具检查结果；命令本身不安装程序。工具版本与配套技能版本互不代替。

### sync — SkillVault 跨机同步

```text
sync pull                                                   # 拉取同一 SkillVault，刷新已有部署/指令目标
sync push [--message <text>]                                # 获同意后显式上传工作区全部改动
sync status                                                 # 连接/未提交/未拉取事实
sync conflicts                                              # Git 冲突信息供 Agent 检查
```

- GUI 保存沿用已有自动上传确认；Agent 修改后每次询问得到明确同意才 `push`。CLI 写库、部署同步、文件监听及后台检查/重试都不构成上传授权。
- 已同意的上传失败可由用户主动重试；后台路径不提交或推送。冲突由 Agent 按用户裁决完成 Git 层处理后，再 `sync pull` 继续；不新增应用内合并流程。
- 库同步与技能上游更新分开；拉取新增只让库内容一致，不产生首次部署/档案启用/工具安装。

### agent / instruction — 本机配置与指令

```text
agent list | agent detect
agent enable <key> | agent disable <key> | agent reorder --order <key,...>
agent set-dir <key> (--skills <path> | --project-rel <path> | --instructions <path>)
agent reset-dir <key> --kind skills|project-rel|instructions
agent local-skills <key> | agent delete-local <key> <name>...
instruction get global|<key>
instruction set global|<key> (--file <path> | --stdin)
```

Agent 目录/启停/顺序只在本机；检测不启动、不部署。指令编辑沿用单保存动作，保存源并按“全局+专属”覆盖本机目标；没有独立分发勾选。指令路径与技能路径独立，不把源跨机一致视为已经完成目标分发。用户已确认：`sync pull` 拉取到指令源变化后，按同一拼接覆盖规则自动刷新本机已有启用目标，不改变启用名单/指令路径；刷新失败分别报告，不把源同步成功冒充目标刷新成功。

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
| 标签/分类/注释/支持范围 | skill tag/set/move | 同一核心资料维护 |
| 译文/覆盖层维护 | skill translation * / overlay * | 同一管理资料入口 |
| Hermes | hermes *，skill adopt-hermes | 只读详情/明确启停/收编 |
| 技能维护任务 | skill task <ref>... | 单项/多选，保留事实上下文，规范引用维护 Skill |
| 工具清单/状态/检查 | tool list/get/check | 软件执行登记检查 |
| 工具登记/修改/本机回写 | tool register/set/set-install/skill-link | GUI 交外部 Agent 的任务入口，Agent 核实后 CLI 写回 |
| 工具维护任务 | tool task | 生成/复制，不冒充执行 |
| 库同步 | sync * | 同意边界与全量提交一致 |
| Agent 配置/本地技能/指令 | agent *，instruction * | 本机配置与单保存指令入口 |

**覆盖结论**：上表为需求与命令契约映射，不是命令已实现的证明。各入口实施后仍须按 SC-002 实际验收；CHK026 已确认拉取后自动刷新本机已有启用指令目标。
