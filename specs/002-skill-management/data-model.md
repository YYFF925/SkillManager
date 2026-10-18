# Data Model: 库文件格式与本机状态

**Date**: 2026-10-15 | 依据：[spec.md](spec.md)、[research.md](research.md)（D1~D9）；经需求质检修正检查记录与安装事实的边界。

原则：**共享资产全部是 SkillManagerVault 内可直接读写的文件**（用户与 Agent 可维护，随库 Git 同步）；**本机状态只在 SQLite**（丢失仅影响本机，可恢复）。技能身份 = 管理域 + 技能名（D1）。

## 一、SkillManagerVault 内文件（共享资产）

### 1.1 目录总览

```text
SkillManagerVault/                     # 用户已有 Git 仓库，首次接入时选择
├── code/                       # 编码技能库（管理域）
│   ├── [分类/]技能名/           # 分类最多一级；未分类直接在域根下
│   │   ├── SKILL.md 及技能文件  # 原始内容（上游原名保留）
│   │   └── .manager/           # 该技能的管理层
│   └── ...
├── lobster/                    # 龙虾技能库（管理域），结构同上
├── presets/                    # 预设定义
│   ├── code.yaml
│   └── lobster.yaml
├── instructions/               # Agent 指令源
│   ├── global.md               # 全局内容
│   └── <agent-key>.md          # 逐 Agent 专属内容（仅有专属时存在）
├── upstream-decisions.yaml     # 按仓库/管理域记录明确不入库的上游子路径
└── tools/                      # 工具中心共享资料
    └── <tool-slug>/
        ├── tool.yaml           # 名称、简介、能力标签、分类、检查命令、配套 Skill 关联
        └── guide.md            # 维护指南
```

### 1.2 技能身份与目录规则

- **身份**：`(domain, name)` 二元组，`name` 即目录名。写入校验：同域内唯一（大小写不敏感比较，macOS 文件系统不区分大小写）；创建、入库、收编、复制遇同名阻止（FR-004）。
- **分类** = 目录层级（0 或 1 级）；改单个技能分类或重命名分类目录 = 移动目录，技能名不变（FR-003）。
- 技能目录名即部署技能名，入库保留上游原名。

### 1.3 `.manager/`（每技能）

```yaml
# meta.yaml —— 结构化管理资料，字段即项（D3）
tags: [代码分析, 文档]        # 标签，可多个；空=无标签
comment: 用自然语言改写代码   # 中文短注释，一般 ≤20 字，不设强制上限
agents: [codex, zcode]       # 支持的 Agent；缺省该键=管理域内全支持
oses: [macos, linux]         # 支持的 OS；缺省=全支持
translation_source_hash: 9f2a...  # 软件已知键：写入译文时原文内容哈希（T011）
```

- `agents`/`oses` 缺省表示该维度全部支持，显式 `[]` 表示不支持任何目标；恢复“全部”删除相应键。修改其他字段不能把本机不可见的已声明 Agent 清掉，不能把“全部”展开为本机当前可见名单。
- `translation.md`：中文正文译文，整份一项；仅管理器阅读，不作部署正文。
- **译文时效（T011 落定）**：原文（技能入口文件 `SKILL.md`/`skill.md` 的内容）与译文的比较依据是 `meta.yaml` 的软件已知键
  `translation_source_hash`：保存译文时记录**当前原文内容**的 SHA-256 十六进制摘要。
  读取时重新计算原文摘要，与之相等即视为最新；不相等才提示“译文待更新”。
  - `translation.md` 不存在时不提示，该键缺失或与当前原文相等都不提示。
  - 用户或 Agent 按当前原文重新保存译文（`skill translation set`、GUI 保存）即用当时的原文摘要覆盖该键，提示随之解除。
  - `skill translation remove` 删除译文时一并删除该键。
  - 原文变化不改写 `translation.md`，也不自动翻译；不比对译文自身内容，不审查翻译质量，不建翻译历史。
  - 只有原文内容参与比较：注释、标签、支持声明、overlay、`source.yaml` 的变化都不触发提示。
- `overlay/[<agent>|_]/[<os>|_]/<相对路径>`：覆盖层文件树，**固定两层维度目录 + 相对路径**。
  维度目录用字面 `_` 表示“该维度通用”：`overlay/_/_/scripts/x.sh` 对所有 Agent 与系统生效，
  `overlay/codex/_/SKILL.md` 仅对 codex 生效，`overlay/_/macos/x.sh` 仅对 macOS 生效。
  **同一相对路径的文件为一项**（一次只写一处该路径的文件）；合成优先级为“具体 > 通用”，
  最终优先顺序在 M2 T021 落定（见 research.md §四 D11）：应用顺序 `_/_` → `_/<os>` →
  `<agent>/_` → `<agent>/<os>`，后者覆盖前者；同数量时 Agent 维度优先于 OS 维度。
  冲突不合并，胜者整份文件生效；允许整份 `SKILL.md` 替换且不自动合并原文（FR-008）。
- `source.yaml`：来源记录（无上游的自建技能无此文件）：

```yaml
repo: https://gitea.example/user/skills.git
ref: main                    # 可选：所选分支/引用；缺省跟踪远端默认分支
subpath: tools/xxxskill       # 上游仓库内技能路径
version: 0123456789abcdef0123456789abcdef01234567  # 当前采用提交的完整 Git OID
skipped_version: 89abcdef0123456789abcdef0123456789abcdef  # 可选：跳过的完整 OID，含删除类
registered_at: 2026-10-15
```

- `version`、`skipped_version` 与检查所得 `remote_version` 使用解析后的完整 Git 提交 OID，以相等判断已采用/已跳过，标签仅用于显示；不假设 Git tag 遵循版本大小顺序。
- 仓库去重按保存的仓库地址字符串（去首尾空白、末尾 `/`）进行；同地址跨域只获取一次，各所选 `ref` 分别解析。不推断 SSH/HTTPS 等不同地址是否同一仓库；这是可接受的重复获取，不增建 URL 身份体系。
- 来源记录绑定技能的上游仓库：从 B 引入的技能进入唯一 SkillManagerVault，其来源/子路径/采用版本用于后续对 B 的检查和更新。所有电脑同步同一个 SkillManagerVault，来源记录随库共享；另一机拉取是库同步，不是重新入库，不把 SkillManagerVault 远端写成该技能的上游。不得把这两条链路当作待裁决的来源选择。
- **写入规则**：任何写回以原始键值合并，**保留软件不认识的字段**（D3 格式演进约束）。
- `.manager` 整体不进入部署结果（FR-016）。

### 1.4 预设 `presets/<domain>.yaml`

```yaml
- id: rust-deps-check        # 预设内唯一标识（人写，域内不重复即可）
  name: Rust 依赖检查
  icon: shield               # 图标标识
  order: 2
  members: [cargo-audit, dep-guard, ...]   # 技能名引用（域内唯一可解析）；数组顺序即成员显示顺序
```

- 组合定义（含顺序与图标）随库同步；成员编辑不改变部署（FR-024）。
- 所有预设操作的完整引用为 `(domain, id)`；CLI 的预设部署/撤销也显式传域，不在两个同 id 预设之间猜测。
- `order` 是预设间顺序，`members` 的数组顺序是该预设内成员的显示顺序；调整顺序不改变成员集合或实际部署，不再添加一份成员排序字段。

### 1.5 工具资料 `tools/<tool-slug>/tool.yaml`

```yaml
name: ripgrep
summary: 极快的正则搜索工具          # 卡片短摘要
intro: |                            # 概览长简介（可读、多段）
  ripgrep 是 ...
tags: [搜索, cli]
category: 命令行工具
check_command: sh ./check-latest.sh     # Agent 编写，示意命令；获取上游最新版本，非本机版本
skills:                             # 配套 Skill 关联（技能名引用）
  - domain: code
    name: ripgrep-usage
    repo: https://example.invalid/skills.git  # 候选来源，示例地址不可作为真实可用来源
    subpath: ripgrep-usage            # 尚未入库时可复用技能候选发现流程
```

- 只有一个 `check_command`，由 Agent 按工具实际来源编写；工作目录为该工具资料目录，所需检查脚本随该目录共享。标准输出为已提取的单个最新版本字符串，非零退出或空结果为检查失败，说明性输出放标准错误；软件负责读取和对比，不为来源类型增加专用查询器。**不**恢复“本机版本查询冒充上游检查”，**不**增加 `upstream_command` 第二条查询路径，也**不**按来源类型选用解析器。
- **非标准工具版本的比较约定（T074 落定）**：软件对“检查所得上游版本字符串”与“本机安装记录 `version`”做**同一种规范化对比**，两侧都不做来源相关解析：
  1. 去首尾空白，去掉可选的先导 `v`/`V`；
  2. 切成“非数字段 / 数字段”交替序列（`1.2.3` → `1` `.` `2` `.` `3`）；
  3. 仅当两侧**段数相同、对应位置段类型相同、且非数字段（分隔符/前缀）逐段一致**时才比较；比较时只有数字段参与大小（按数值，忽略前导零），非数字段大小写不敏感地要求相等。
  - 相等 → “已是本机安装版本”（最新）；上游更大 → “可更新”。
  - 无法比较（段数不同、对应位置类型不同、分隔符不同，例如 `7.0` 与 `rev 12ab34`、`1.2.3` 与 `2024-10-15`）→ 如实显示两侧版本字符串并说明“版本字符串无法比较”，**不得**显示“最新”或“可更新”。
  - 上游更小 → 如实显示“检查所得上游版本低于本机安装版本”，不判定为可更新。
  - 未安装工具只显示上游版本，不派生“本机最新/可更新”；检查失败、未登记命令、未检查分别表达。
  - 该约定只覆盖字符串对比，不引入版本区间、语义化版本库或按来源类型的比较器。
- `guide.md`：维护指南（用途与入口、安装方式、安装与配置位置、检查命令、更新步骤、验证、注意事项）。
- 工具 slug（目录名）即工具身份；本机安装记录不在库内（见 2.4）。

### 1.6 新增候选决定与发现基线 `upstream-decisions.yaml`

```yaml
- repo: https://gitea.example/user/skills.git
  domain: code
  rejected_subpaths: [tools/not-needed]   # 明确“不入库并记住”
  seen_subpaths: [tools/keep-a, tools/later-c]  # 发现基线：首次观察时已存在的候选
```

按 `(repo, domain)` 记录，两字段语义不同，随库共享；跨域可独立选择：

- `rejected_subpaths`：用户明确“不入库并记住”的来源子路径。返回、未勾选或取消均不写记录；不建决定历史或队列。技能当前版本的跳过仍保存在自身 `source.yaml`，与拒绝新增不同。
- `seen_subpaths`：**发现基线**（T039 落定）——该 `(repo, domain)` **首次观察**（`import`）时已存在的候选子路径集合，**只建立一次、从不自动扩充**。候选的“后来新增”判定为“基线存在且 subpath ∉ 基线”。由此落实 spec.md 的首次未选/后来新增例子：首次已存在的 C 不冒充后来新增、也不被记作拒绝；基线之后新出现且仍存在的 B 不被更新其他技能清除提示；C 仍可手动 `preview` 选择。写入基线的候选已按 FR-058 跳过 `.agents/skills/`，该路径不会进入 `seen_subpaths`，之后也不作为新增出现。
- 发现范围本身不是本文件的字段：相对仓库根 `.agents/skills/` 下的技能在扫描阶段即排除（research.md D20/D24），不新增排除实体。

`upstream_checks` 表（§2.5）按仓库去重键存库地址；该文件与本机检查时间无关。该文件不是“所有未入库技能”的清单，也不保存全仓检查历史，只保存一份一次性集合，不记录时间序列。

## 二、本机状态（SQLite，`skill-manager.db`）

> 只存本机状态，无外键技能表——技能引用一律存 `(domain, name)` 字符串（D1/D2）。丢失后果：部署可重部署、启用可重选、设置可重填。

### 2.1 `deployments` — 部署记录

| 列 | 说明 |
|---|---|
| domain, name | 技能引用（联合键一部分） |
| target | 目标标识：`agent:<key>` / `shared` / `hermes:default` |
| target_path | 实际部署副本的**绝对路径**（技能副本目录本身，不是其父技能目录），目标目录设置后来改变仍能定位原副本 |
| status | `ok` / `paused_local_change`（副本被外部修改，暂停覆盖） |
| source_hash | 部署时库内容指纹（合成输入指纹，算法见下） |
| deployed_hash | 最近成功部署的合成产物指纹，识别目标副本外部变更 |
| synced_at | 最近同步时间 |

- **语义（M2 T021 落定，算法详见 research.md §四 D13）**：
  - `source_hash` = 合成**输入**指纹：基础树（排除 `.manager`/`.git`/`.DS_Store`）+
    本目标生效的覆盖层条目 + 库内 `comment`，按“排序相对路径 + 文件字节”做 SHA-256。
  - `deployed_hash` = **合成产物**指纹（已含注释注入与覆盖层），同一哈希算法。
  - 外部变更判定：副本 `target_path` 实际内容指纹与 `deployed_hash` 不等即为外部修改；
    副本缺失不算外部修改，按记录重建。不使用 mtime 判向。
  - `deploy to` 写入**当前解析出的目标路径**并更新 `target_path`；`deploy sync` 写入记录的
    `target_path`（目标目录设置改变后仍定位原副本）。
- 生命周期：部署 → `ok`；同步发现副本外部变更 → `paused_local_change`（其余技能继续）；
  用户经 Agent 整理后以 `deploy sync --ref <ref> --target <target> --overwrite-local`
  显式确认该项按库覆盖，再同步恢复 `ok`；副本删除成功才移除记录（FR-020/022）。
- 库内条目删除而副本暂停时，保留该行及 `target_path`，报告“库已删除、副本清理暂停”；
  Agent 经用户裁决后用 `deploy undo <target> <ref> --discard-local-changes` 继续清理后删记录
  （该撤销按记录定位，不依赖库内技能仍存在）。不新增待删任务队列，也不自动丢弃本地副本。
  新增 `deployed_hash`/`target_path` 分别防止覆盖外改副本和删除库后失去其定位，这是两项
  直接影响用户内容的后果。
- 受管身份以本机部署记录及实际路径为依据；不额外向技能产物写受管标记文件。数据库丢失时按
  未受管目标处理，可由用户明确部署重新接管，不建设自动恢复机制。
- 未受管同名目标：直接覆盖并插入记录（接管，FR-018）。

### 2.2 `hermes_enables` — 档案启用名单

`(profile, domain, name)` → 启用关系；其他档案启用是本机状态，内容更新不重置（FR-027）。

default 档案即 `<Hermes 根>/skills` 目录。表示「已安装进 Hermes」的部署记录有两种等价写法：`hermes:default`
与 Agent 目标 `agent:hermes`（`Target::HermesDefault` 在 Agent 维度就是 `hermes`，两者解析到同一目录，见
`crates/core/src/deploy/targets.rs`）。各档案共用一份候选池：龙虾库中以上述任一写法记录的技能，按技能名去重；
default 是否已安装以该目录下实际存在技能目录为准，不要求特定记录名（FR-026）。

### 2.3 `agents` — 本机 Agent 配置

| 列 | 说明 |
|---|---|
| key | 内置 Agent 标识（全量名单来自技术参考资产） |
| enabled, order | 启停与顺序 |
| skills_dir | 实际技能目录（可改，恢复默认=NULL） |
| project_rel_dir | 项目技能相对路径（仅配置保留） |
| instructions_target | 指令分发目标路径（`AGENTS.md` 位置） |

- 指令分发沿用本机 Agent 的启用配置与指令路径，不新增独立分发勾选字段；编辑器只有“保存”。用户已确认：拉取到指令源变化后，按全局+专属自动刷新本机已有启用目标，不改启用名单和指令路径；刷新失败如实表达，源同步成功不冒充目标已刷新。

### 2.4 `tool_installs` — 工具本机安装记录

| 列 | 说明 |
|---|---|
| tool_slug | 对应 `tools/<slug>` |
| version, method, path, config_path | 本机安装事实（Agent 回写） |

- 此表只表达 Agent 已核实回写的安装事实；无此表行显示“未安装”（FR-045），软件检查不能创建或改写这行。

### 2.4a `tool_checks` — 工具本机检查结果

| 列 | 说明 |
|---|---|
| tool_slug | 工具目录身份，无论是否已安装均可有检查记录 |
| status | `unchecked` / `checking` / `checked` / `failed` / `no_command` |
| latest_version | 成功获取的上游最新版本；失败/未登记时不得当作本次有效结论 |
| checked_at, check_output | 本次检查时间与命令结果 |

- 状态：无记录为 `unchecked`；检查中 `checking`；成功取得最新版本为 `checked`；失败为 `failed`；无命令为 `no_command`。对已安装工具，`checked` 再结合安装版本派生“最新/可更新”；无法比较则如实显示版本与不能判定的原因。未安装工具只显示上游版本，不派生“本机最新/可更新”。Agent 安装或更新回写后，将该工具检查结果重置为 `unchecked`。
- 分开存储仅为避免“未安装工具有检查结果”被误表示成已安装；不是共享资料镜像，也不增加按来源类型的检查器。

### 2.5 `upstream_checks` — 上游检查状态（本机）

`(repo, domain, name)` → `remote_version, checked_at, result`。`repo` 存**去重后的仓库地址**（去首尾空白与末尾 `/`，与 §1.3 一致）；`result` 取值 `up_to_date`/`update_available`/`skipped`/`upstream_missing`/`error`。采用版本与跳过决定在库内 `source.yaml`（共享）；本机只记"最近查到什么、何时查的"，检查不应用变化、不改采用版本或部署。

### 2.6 `settings` — 应用设置（KV）

键、默认值/取值与**两个入口的可达性**（2026-10-16 按 FR-050「本机运行侧」逐项清点后补齐；`settings` 命令组见 `contracts/cli.md`）：

| 键 | 取值/默认 | GUI 写 | CLI 写 | CLI 读 | 说明 |
|---|---|---|---|---|---|
| `theme` | light/dark/system，默认 system | 是 | **不提供** | 是 | 界面偏好，不影响管理结果（FR-052）；主题与字号不是 FR-050 的双入口范围 |
| `font_scale` | small/default/large/xlarge，默认 default | 是 | **不提供** | 是 | 同上 |
| `auto_check_minutes` | 0=关闭/30/60/360/1440，默认 60 | 是 | **是**（`settings set-auto-check`） | 是 | FR-029 三项共用周期；缺 CLI 写会使 Agent 无法按用户指令调整自动检查 |
| `git_auto_push` | "true"=已确认；缺失/"false"=未确认 | 是 | **不提供**（只读） | 是 | GUI 保存自动上传的长期授权记录；Agent 不得自行开启，其上传仍需每次明确同意（FR-038） |
| `vault_path` | 绝对路径，无默认 | 是（`vault_open`） | 等价入口 `--vault` / 记住的库 | 是 | 入口形式不同，非缺失 |
| `last_sync_at` | 时间戳 | 由 `sync pull` 写 | 由 `sync pull` 写 | 是 | 界面显示用，不参与授权 |
| `last_check_at` | 时间戳 | 由统一调度写 | **不提供**（只读） | 是 | 定时门控用；CLI 的手动检查由三条既有命令分别表达，不写该键（FR-029） |

- `shared_dir` 是运行时派生的只读值，不是 SQLite 设置键：`settings get shared_dir` 返回按系统用户主目录解析的 `.agents/skills`；GUI 同源展示，不提供写入或清除入口，不读取旧路径设置。
- **不再保留**"自定义日志目录"配置：无写入点也无实际需要，日志固定落在 `<state-dir>/logs`。
- **Hermes 根不做可写设置键**：按 `HERMES_HOME` 环境变量与平台默认解析（`~/.hermes`）；另设一份可写配置会与 Hermes 自身的定位规则分叉。当前实现的 `hermes_root` 键随本次清理移除。
- 未设置的键读取返回 `None`，不以空字符串冒充；`settings get` 输出当前**生效值**（含默认值推导）。

**GUI 自动上传的既有确认（T050 落定）**——设置键 `git_auto_push`：

- 取值语义：`"true"` = 用户已确认“GUI 保存后自动提交并推送自己的 SkillManagerVault”；键缺失或 `"false"` = 尚未确认（默认）。
- **首次获得确认的交互**：GUI 第一次保存库内修改时询问一次是否以后自动上传；用户确认后写 `"true"`，此后每次保存沿用该确认，不再逐次询问。用户拒绝或未作答时不写键，保存只刷新已有部署、不提交推送。
- **边界**：该键只保存 GUI 保存这一条已授权来源，不是上传授权账本，也不记录逐次同意或改动范围。Agent 修改库后仍每次单独询问，得到明确同意才显式 `sync push`；CLI 写库、`deploy sync`、文件监听、启动/定时同步一律不读该键、不提交推送。
- **失败与重试**：已同意的上传失败时保留可读原因，界面提示用户可主动重试；用户经 `sync push` 重试视为该次已同意上传的继续，不改写同意记录。后台路径不重试推送。

**同步运行边界（T050 落定）**：`last_sync_at` 只记录最近一次成功的 `sync pull` 时间供界面显示，不参与授权，也不代表上传。后台（启动/定时/文件监听）与 `deploy sync` 只拉取与刷新已有部署，不提交、不推送；拉取无冲突时静默应用，冲突则暂停本次同步（Git 层冲突现场保留），交 Agent 检查、用户裁决后再次 `sync pull` 继续，不新增应用内合并流程或“保留本地/远端/双方”选项。

## 三、身份引用与完整性规则汇总

| 引用位置 | 引用形式 | 解析失败时 |
|---|---|---|
| 部署记录 / 档案启用 | `(domain, name)` | 技能已删 → 按删除清理规则处理 |
| 预设成员 | 技能名（同文件所属域） | 界面标注"缺失成员"，不阻塞其他成员 |
| 工具配套 Skill | `(domain, name)` | 显示"未入库"，可前往入库 |
| 指令源 | `<agent-key>.md` 文件名 | 无专属文件 → 仅全局内容 |

- 所有共享文件的写入者（GUI、CLI、Agent 手改）地位平等：文件监听触发索引刷新，无"谁是权威来源"的区分（宪章 V）。
- 库同步冲突 = Git 层冲突，暂停同步交 Agent/用户（FR-039），应用层不做合并。
