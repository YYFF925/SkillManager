---
name: skill-manager-maintenance
description: 维护 Skill Manager 的 SkillVault：.manager 格式、目录与同名规则、原文/译文/注释/覆盖层边界、来源更新与同项合并、部署与上传约定、冲突暂停处理、CLI 参考与工具中心登记/检查/回写流程
---

# Skill Manager 库维护规范

这是 Skill Manager 桌面应用的**配套维护 Skill**：当用户要求你（外部 Agent）维护技能库、
上游更新、部署副本、跨机同步，或处理工具中心的登记/安装/检查/回写时，按本文执行，
命令语法与输出约定见 [references/cli.md](references/cli.md)。

软件只负责库管理、状态展示与版本检查，**不内置 Agent、不自动安装程序、不自动翻译**；
所有实际写入动作由你经 CLI（命令行接口）完成，`tool` 的任务文本与技能维护任务都只引用
本 Skill 的库内路径与“按其规范执行”，不内联规范全文。

## 一、基本概念

- **SkillVault**：用户自己已有的 Git 仓库，是技能内容的唯一真值源。首次接入由用户在 GUI 选择目录，
  本机记住路径；CLI 用全局 `--vault <path>` 指定。共享内容全部是库内可直接读写的文件，随库 Git 同步。
- **管理域**：`code`（编码库）与 `lobster`（龙虾库）。域的边界是硬约束：编码技能不能部署给龙虾目标，
  反之亦然；支持声明不能绕过域边界。
- **技能身份** = 管理域 + 技能名，技能名即目录名。有上游来源的技能保留上游原名。
- **分类** = 目录层级，最多一级（`code/<分类>/<技能名>/`）；未分类直接在域根下（`code/<技能名>/`）。
  改分类 = 移动目录，技能名不变。分类路径不是技能身份。
- **本机状态**只在 SQLite（`skill-manager.db`，位于本机状态目录，与库分离）：部署记录、档案启用、
  Agent 配置、设置、工具本机安装记录、检查事实。共享内容真值不写 SQLite。
- GLOSSARY 与数据模型见本项目仓库 `specs/002-skill-management/data-model.md`；命令契约见
  `specs/002-skill-management/contracts/cli.md`。

## 二、目录与同名规则

```text
SkillVault/
├── code/                    # 编码技能库
│   └── [分类/]技能名/        # 分类最多一级
│       ├── SKILL.md（或 skill.md）及技能文件   # 原始内容
│       └── .manager/        # 该技能的管理层
├── lobster/                 # 龙虾技能库，结构同上
├── presets/<domain>.yaml    # 预设定义
├── instructions/global.md, <agent-key>.md   # 指令源
├── upstream-decisions.yaml  # 明确不入库的上游子路径 + 发现基线
└── tools/<tool-slug>/tool.yaml, guide.md, 检查脚本   # 工具共享资料
```

- **同名唯一性**：同域内技能名唯一，比较**大小写不敏感**（macOS 文件系统默认不区分大小写）。
  创建、入库、收编、复制遇同域同名一律阻止，并给出可定位原因。**跨域允许同名**（`code/alpha` 与
  `lobster/alpha` 可并存，两份独立维护）。
- 技能目录识别谓词：是目录且目录下存在**精确文件名** `SKILL.md` 或 `skill.md` 的普通文件；
  命中即不向下递归，超过一级的嵌套不是技能。`README.md`/`CLAUDE.md` 不作为技能标记。
- 分类名与技能名按 CLI 校验规则（不能为空、不能含路径分隔符等）；跨域同名、同域不同名都可查询。

## 三、`.manager` 实际格式

`.manager/` 是该技能的管理层，随技能目录移动，**整体不进入部署结果**。

### 3.1 `meta.yaml`（结构化管理资料，字段即项）

```yaml
tags: [代码分析, 文档]        # 标签，可多个；空 = 无标签
comment: 用自然语言改写代码   # 中文短注释，一般 ≤20 字
agents: [codex, zcode]       # 支持的 Agent；缺省该键 = 管理域内全支持
oses: [macos, linux]         # 支持的 OS；缺省 = 全支持
translation_source_hash: 9f2a…   # 软件已知键：保存译文时记录的原文内容 SHA-256
```

- `agents` / `oses` **缺省**表示该维度全部支持；显式 `[]` 表示不支持任何目标；恢复“全部”是**删除**
  相应键（`skill set --agents all`）。修改其他字段**不能**清除本机不可见的已声明 Agent，也不能把
  “全部”展开成本机当前可见名单。
- **写回总是按原始键值合并，保留软件不认识的字段与键顺序**。用户手工加过的 `foo: bar` 之类未知字段，
  经 CLI/GUI 改注释后必须仍在。
- `translation_source_hash` 是软件已知键；不要手工伪造它，译文时效由 CLI 写入时维护。

### 3.2 `translation.md`

中文正文译文，**整份一项**，仅管理器阅读，**不作部署正文**。

译文时效：保存译文时把**当前原文内容**（入口文件 `SKILL.md`/`skill.md`）的 SHA-256 十六进制摘要
写进 `meta.yaml` 的 `translation_source_hash`。读取时重算原文摘要，与之相等即最新，不相等才提示
“译文待更新”。

- `translation.md` 不存在时不提示；该键缺失或与当前原文相等都不提示。
- 用户或你按**当前原文**重新保存译文（`skill translation set`）即覆盖该键，提示解除。
- `skill translation remove` 删除译文时一并删除该键。
- 原文变化**不**改写 `translation.md`，也**不**自动翻译；不比对译文自身内容，不审查质量，不建翻译历史。
- 只有原文内容参与比较：注释、标签、支持声明、覆盖层、`source.yaml` 的变化都不触发提示。

### 3.3 `overlay/<agent|_>/<os|_>/<相对路径>`

覆盖层文件树，**固定两层维度目录 + 相对路径**。维度目录用字面 `_` 表示“该维度通用”：

- `overlay/_/_/scripts/x.sh` 对所有 Agent 与系统生效；
- `overlay/codex/_/SKILL.md` 仅对 codex 生效；
- `overlay/_/macos/x.sh` 仅对 macOS 生效。

**同一相对路径的文件为一项**（一次只写一处该路径的文件，写入新维度会移除无法共存的旧条目）。
合成优先级为“具体 > 通用”，应用顺序：`_/_` → `_/<os>` → `<agent>/_` → `<agent>/<os>`，
后者覆盖前者；同数量时 Agent 维度优先于 OS 维度。

**冲突不合并，胜者整份文件生效**：允许整份替换 `SKILL.md`，此时部署产物用覆盖内容，
不自动合并上游正文（覆盖层整份替换，不做行级合并）。

### 3.4 `source.yaml`（来源记录）

有上游的技能才有；自建技能没有该文件。

```yaml
repo: https://gitea.example/user/skills.git   # 上游仓库地址
ref: main                                     # 可选：所选分支/引用；缺省跟踪远端默认分支
subpath: tools/xxxskill                        # 上游仓库内技能路径
version: 0123456789abcdef…（完整 Git OID）     # 当前采用的提交
skipped_version: 89abcdef…（完整 OID，可选）   # 已跳过的提交（含删除类）
registered_at: 2026-10-15
```

- `version` / `skipped_version` 与检查所得 `remote_version` 都用**解析后的完整 Git 提交 OID**；
  相等判断已采用/已跳过，标签仅用于显示，**不假设 Git tag 有版本大小顺序**。
- 仓库去重按保存的地址字符串（去首尾空白与末尾 `/`）；同地址跨域只获取一次，各 `ref` 分别解析；
  不推断 SSH/HTTPS 等不同写法是否同一仓库。
- 来源记录绑定技能的**上游仓库**。技能进入唯一 SkillVault，来源/子路径/采用版本用于后续对上游的
  检查与更新；所有电脑同步同一个 SkillVault，**另一台电脑拉取是库同步，不是重新入库**，
  不要把 SkillVault 远端写成该技能的上游。

### 3.5 其他共享文件

- `presets/<domain>.yaml`：`id`（域内唯一，新建时由名称派生）、`name`、`icon`、`order`（预设间顺序）、
  `members`（技能名数组，**数组顺序即成员显示顺序**）。
- `instructions/global.md` 与 `instructions/<agent-key>.md`：指令**源**；分发目标由本机 Agent 配置决定。
- `upstream-decisions.yaml`：按 `(repo, domain)` 记录 `rejected_subpaths`（明确不入库并记住）与
  `seen_subpaths`（发现基线，首次 `import` 时已存在的候选，**只建一次、从不自动扩充**）。
- `tools/<slug>/tool.yaml` + `guide.md` + 检查脚本：工具共享资料，随库同步；工具**程序本身**不随库同步。

## 四、原文 / 译文 / 注释 / 覆盖层的边界

部署产物按“库内原始文件 → 命中目标的覆盖层 → 中文注释”合成，**永远以库内原件为输入**，
从不以已注入注释的产物为输入：

| 内容 | 存放位置 | 是否进部署 | 说明 |
|---|---|---|---|
| 原文/技能文件 | 技能目录（`.manager` 之外） | 是 | 上游原名保留，库内原件不因注释或译文改变 |
| 中文注释 `comment` | `.manager/meta.yaml` | 以注入形式进入 | 作为前缀写进 frontmatter 的 `description`：`<注释>｜<原 description>`；没有 frontmatter 时新建最小块承载 |
| 覆盖层 | `.manager/overlay/…` | 命中的整份覆盖 | 不合并；`_` 为该维度通用 |
| 译文 `translation.md` | `.manager/translation.md` | **否** | 只供管理器阅读，绝不进入部署正文 |
| `.manager/` 整体 | `.manager/` | **否** | 部署结果不含 `.manager` |

- 注释只进产物，**不回写库内原件**。核对部署结果时应看到注释已在 `description`，而库内 `SKILL.md` 未变。
- 若入口文件 frontmatter 无法解析、`description` 非字符串或不是有效 UTF-8，注释会**跳过**并如实说明
  原因，不静默伪造。

## 五、来源更新与同项合并规则

上游检查（`skill check`）**只展示、不应用**：可以保存本机检查事实、时间与获取缓存，
**不应用技能变化、不改采用版本/部署、不改工具安装事实**。

应用更新（`skill update` 单项 / `skill update-repo` 仓库批量）时：

- **管理资料按项合并，上游同项值优先**：上游 `.manager` 提供的项（标签、注释、支持声明、译文、
  覆盖层同路径文件）覆盖本地同项，**本地改过该同项也是上游优先**；上游**未提供**的项本地保留。
- **分类保留本地**（分类是目录位置，非上游项）。
- **原文变化**保留已有译文并更新“译文待更新”提示（见 §3.2）。
- **技能内文件删除**：只移除对应文件，其他变化照常应用，技能与已有部署继续保留并刷新。
- **整个技能被上游删除**：批量更新不自动删除，报告待显式处理；经确认的单技能删除才清库并清理部署
  （受管副本被外改则暂停，见 §7）。**技能改名**呈现为“删除旧技能 + 新增候选”。
- **新增候选**：首次 `import` 时已存在的候选记入发现基线（`seen_subpaths`），**不冒充“后来新增”**、
  也不被记作明确拒绝。基线之后新出现且仍存在的候选保持“后来新增”提示；更新其他技能不等于处理它。
  明确“不入库并记住”用 `skill reject-new`（写 `rejected_subpaths`，随库共享）；
  未勾选/返回/取消**都不**写拒绝记录。
- **跳过版本**：`skill skip` 跳过当前上游版本（存 `source.yaml` 的 `skipped_version`，与“拒绝新增”
  不同）；仓库批量更新尊重跳过决定；`skill skipped` 查看，`skill unskip` 恢复提醒，
  `skill update <ref> --to <oid>` 可明确采用已跳过的具体版本。
- `--to` / `--version` 缺省采用**最近检查所得的具体远端提交**；尚无检查结果时应先检查，
  不在应用时静默选择另一个版本。

## 六、修改后同步部署与“询问后才上传”

你修改库内容后，按固定顺序处理：

1. **同步已有部署，不新增目标**：`deploy sync` 只更新本机已有部署与已启用指令目标，
   不产生首次部署、不改启用名单或路径。
2. **上传必须先获得用户明确同意**：库内改动**不会**自动上传。通过对话得到用户明确同意后，
   才执行 `sync push`（提交并推送**整个工作区**改动，不按改动归属拆分）。
   - 软件侧：只有“GUI 保存按已确认的自动上传策略”与“Agent 获明确同意后显式 `sync push`”两条路径
     会提交推送。CLI 写库、`deploy sync`、文件监听、启动/定时同步**都**不构成上传授权。
   - 已同意的上传失败时保留可读原因，可提示用户主动重试（再次 `sync push`）；后台路径不重试推送。
3. **部署与库同步分开**：`sync pull` 只让库内容一致；拉取到的**新增技能不会自动部署**，
   已启用指令目标会按“全局在前、专属在后”覆盖规则刷新，但不新增启用目标、不改本机路径。

## 七、冲突与暂停处理

### 7.1 部署副本被外部修改（受管副本外改）

普通部署/同步/撤销遇到受管副本被外部修改，会**暂停该项**（`paused_local_change`），其余项继续，
不要求整批回滚、不自动合并。处理方式：

1. 向用户说明暂停项、目标与实际路径，由用户裁决（保留用户改动，还是按库覆盖）。
2. 用户确认按库覆盖：`deploy sync --ref <ref> --target <target> --overwrite-local`。
3. 库内技能已删除而副本暂停：按下本机记录定位，用户确认丢弃待清理副本后执行
   `deploy undo <target> <ref> --discard-local-changes`；记录在副本成功移除后才删除。

这是执行用户裁决的参数，不是自动合并或冲突分析功能；不要新建应用内合并流程。

### 7.2 跨机同步冲突

库同步冲突是 **Git 层冲突**：

1. `sync pull` 遇冲突会**暂停本次同步**并保留 Git 冲突现场；`sync conflicts` 输出未合并路径与状态。
2. 由你按用户裁决在 Git 层完成处理，再 `sync pull` 继续。**没有**“保留本地/远端/双方”选项，
   也不提供应用内合并界面或历史恢复。

### 7.3 远端不可用

`sync pull`/`sync push` 明确报“未连接”；本地库管理、部署与 `deploy sync` 继续可用。
不提供断开远端或重新克隆的修复入口。

## 八、CLI 使用

全局选项（所有子命令可用）：`--vault <path>`、`--state-dir <path>`、`--json`。
在本机模拟两台电脑时，用同一库的两个克隆分别配不同 `--state-dir`。

**退出码**：`0` 成功；`1` 失败；`2` 部分完成（部分成功、部分失败）。注意 **clap 的参数用法错误也返回 `2`**，
需按提示中的 `error:`/`未完成：` 与实际输出区分：`--json` 时 `result` 为 `success|partial|failed`。

高频命令（完整命令表、参数与语义见 [references/cli.md](references/cli.md)）：

```bash
# 查询与阅读
skill-manager skill list [--domain code|lobster] [--query <text>] [--category <c>|--uncategorized] [--tag <t>…] [--no-tag]
skill-manager skill get <domain>/<name>
skill-manager skill read <domain>/<name> [--lang zh|orig]

# 管理资料
skill-manager skill set <ref> [--comment <text>] [--tags <csv>] [--agents all|none|<csv>] [--oses all|none|<csv>]
skill-manager skill tag add|detach <ref>… --tags <csv>
skill-manager skill tag rename <old> <new> | skill-manager skill tag remove <tag>
skill-manager skill move <ref> (--category <c> | --uncategorized)
skill-manager skill translation set <ref> (--file <path> | --stdin)
skill-manager skill overlay set <ref> --path <relative-path> (--file <path> | --stdin) [--agent <key>] [--os <os>]

# 上游
skill-manager skill preview <repo-url> [--ref <branch-or-tag>]
skill-manager skill import <repo-url> --domain <domain> --skills <subpath,…> [--ref <ref>] [--category <c>|--uncategorized]
skill-manager skill check (<ref> | --repo <url> | --all)
skill-manager skill update <ref> [--to <commit-oid>]
skill-manager skill update-repo <repo-url> [--include-new <subpath,…> --domain <domain>]
skill-manager skill skip|unskip <ref> | skill-manager skill skipped

# 部署与同步
skill-manager deploy to <target> <ref>… [--also-target <target>…] [--dry-run]
skill-manager deploy sync [--ref <ref>] [--target <target>] [--overwrite-local]
skill-manager deploy status [--target <target>] [--paused]
skill-manager sync pull | sync push | sync status | sync conflicts

# 工具中心
skill-manager tool list [--query <text>] [--category <c>] [--status <status>]
skill-manager tool check (<slug> | --all)
skill-manager tool register --dir <path> | skill-manager tool set <slug> --dir <path>
skill-manager tool set-install <slug> --version <实际版本> [--method <text>] [--path <path>] [--config-path <path>]
```

`<target>` = Agent key、`shared`（共享目录）或 `hermes:default`。
`--agents all` 删除限制键、`none` 写显式空集合；`--tags` 是**替换**集合，空字符串清空。
`--also-target` 表示本次同时处理的其他目标（可重复）；未给则只处理一个目标。

## 九、工具中心：登记 / 检查命令 / 真实验证回写

工具中心的共享资料随库同步；工具**程序本身**与**本机安装记录**不随库同步。

### 9.1 登记与修改共享资料

`tools/<slug>/tool.yaml`：

```yaml
name: ripgrep                  # 登记时由 name 派生 slug（保留 ASCII 字母数字，其余折为 -）
summary: 极快的正则搜索工具     # 卡片短摘要
intro: |                       # 概览长简介（可读、多段）
  第一段……
tags: [搜索, cli]
category: 命令行工具
check_command: sh ./check-latest.sh   # 版本检查命令；工作目录为该工具资料目录
skills:                        # 配套 Skill 关联
  - domain: code
    name: ripgrep-usage
    repo: <技能候选来源地址>
    subpath: ripgrep-usage
```

- `guide.md` 必须覆盖：用途与调用入口、实际安装方式、安装与配置位置、版本检查命令、更新步骤、
  验证方式、注意事项。
- 新增：`tool register --dir <含 tool.yaml/guide.md 及检查脚本的目录>`；
  修改：`tool set <slug> --dir <同结构目录>`。也支持 `--stdin` 传 `{"tool": {...}, "guide": "..."}`，
  其他脚本通过资料目录维护。已登记时 `register` 报错，应改用 `set`。
- `--dir` 中缺失的 `guide.md` 或其他文件表示**保持原值**，不表示清空；未知字段写回时保留。
- slug 由 `tool.yaml` 的 `name` 派生；`tool skill-link <slug> <ref>` 追加配套 Skill 关联。

### 9.2 编写版本检查命令

- 只有**一个** `check_command`，由你按工具实际来源编写；**工作目录为工具资料目录**，
  所需脚本随该目录共享（如 `check-latest.sh` 放在 `tools/<slug>/`）。
- **标准输出必须是已提取的单个上游最新版本字符串**；说明性输出放**标准错误**；
  非零退出或空结果为检查失败。软件负责读取与对比，**不**为来源类型增加专用查询器。
- 不要用“本机版本查询”冒充上游检查，也不要增加第二条查询路径或按来源类型选用解析器。

### 9.3 检查与真实验证回写

- `tool check (<slug> | --all)` 只执行登记的单个最新版本检查命令并保存**检查事实**
  （`checked`/`failed`/`no_command`/`unchecked`），**不**安装、不更新、不改本机版本。
- **版本对比约定**（软件对“检查所得上游版本”与“本机安装记录 version”做同一种规范化对比，
  两侧都不做来源相关解析）：去首尾空白、去掉可选先导 `v`/`V`；切成“非数字段/数字段”交替序列；
  仅当两侧段数相同、对应位置段类型相同且非数字段逐段一致时才比较，数字段按数值比大小。
  - 相等 → 已是本机安装版本；上游更大 → 可更新。
  - 无法比较（如 `7.0` 与 `rev 12ab34`、`1.2.3` 与 `2024-10-15`）→ **如实显示两侧版本字符串并说明
    “版本字符串无法比较”**，不得显示“最新/可更新”。
  - 上游更小 → 如实显示“检查所得上游版本低于本机安装版本”，不判定为可更新。
  - 未安装工具只显示上游版本，**不**派生“本机最新/可更新”。
- **安装/更新由你实际执行后再回写**：安装或更新工具后，核实真实结果，执行
  `tool set-install <slug> --version <实际版本> [--method <方式> --path <安装位置> --config-path <配置位置>]`。
  回写会重置该工具的检查结果（状态回到未检查）。已核实该工具**已不在本机**时用 `--version ""`
  清除安装记录并同样重置检查结果。
- **任务不等于完成**：`tool task <slug> --action register|edit|install|update|guide` 只生成可复制的任务说明，
  生成任务不代表已开始或已完成；软件**不自动安装**，也不以直接改版本字段代替真实核实。

## 十、状态表达诚实（硬要求）

- 未检查、检查失败、未登记命令、待更新、任务已生成，**都不得**冒充“最新、已安装、已更新、已完成”。
- 未入库的配套 Skill 如实显示“未入库”，不编造库内路径；无法比较的版本如实说明原因。
- 生成任务、检查上游、拉取同步都不等于已部署、已安装或已上传。
- 无法完成的动作要如实报告失败原因与可定位路径（技能/目标/实际路径），不静默降级。
