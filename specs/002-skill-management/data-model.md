# Data Model: 库文件格式与本机状态

**Date**: 2026-10-15 | 依据：[spec.md](spec.md)、[research.md](research.md)（D1~D9）；经需求质检修正检查记录与安装事实的边界。

原则：**共享资产全部是 SkillVault 内可直接读写的文件**（用户与 Agent 可维护，随库 Git 同步）；**本机状态只在 SQLite**（丢失仅影响本机，可恢复）。技能身份 = 管理域 + 技能名（D1）。

## 一、SkillVault 内文件（共享资产）

### 1.1 目录总览

```text
SkillVault/                     # 用户已有 Git 仓库，首次接入时选择
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
- **分类** = 目录层级（0 或 1 级）；改分类 = 移动目录，名字不变（FR-003）。
- 技能目录名即部署技能名，入库保留上游原名。

### 1.3 `.manager/`（每技能）

```yaml
# meta.yaml —— 结构化管理资料，字段即项（D3）
tags: [代码分析, 文档]        # 标签，可多个；空=无标签
comment: 用自然语言改写代码   # 中文短注释，一般 ≤20 字，不设强制上限
agents: [codex, zcode]       # 支持的 Agent；缺省该键=管理域内全支持
oses: [macos, linux]         # 支持的 OS；缺省=全支持
```

- `agents`/`oses` 缺省表示该维度全部支持，显式 `[]` 表示不支持任何目标；恢复“全部”删除相应键。修改其他字段不能把本机不可见的已声明 Agent 清掉，不能把“全部”展开为本机当前可见名单。
- `translation.md`：中文正文译文，整份一项；仅管理器阅读，不作部署正文。
- 译文时效：按 spec.md 的“译文时效”规则表达；原文变化而未维护译文才提示待更新。具体比较依据和存储形式在 M1 的 T011 落定；不把未完成该设计写成“译文仍最新”，不建翻译历史。
- `overlay/<agent>/<os>/<相对路径>`：覆盖层文件树；目录维度可按需省略（如仅按 agent 或仅按 os 细分，组合顺序实施时按 M2 调研定）；同路径文件一项。
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
- 来源记录绑定技能的上游仓库：从 B 引入的技能进入唯一 SkillVault，其来源/子路径/采用版本用于后续对 B 的检查和更新。所有电脑同步同一个 SkillVault，来源记录随库共享；另一机拉取是库同步，不是重新入库，不把 SkillVault 远端写成该技能的上游。不得把这两条链路当作待裁决的来源选择。
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

- 只有一个 `check_command`，由 Agent 按工具实际来源编写；工作目录为该工具资料目录，所需检查脚本随该目录共享。标准输出为已提取的单个最新版本字符串，非零退出或空结果为检查失败，说明性输出放标准错误；软件负责读取和对比，不为来源类型增加专用查询器。具体非标准版本比较约定在 T074 落定，无法比较时不得冒充“最新”或“可更新”。
- `guide.md`：维护指南（用途与入口、安装方式、安装与配置位置、检查命令、更新步骤、验证、注意事项）。
- 工具 slug（目录名）即工具身份；本机安装记录不在库内（见 2.4）。

### 1.6 新增候选决定 `upstream-decisions.yaml`

```yaml
- repo: https://gitea.example/user/skills.git
  domain: code
  rejected_subpaths: [tools/not-needed]
```

只保存用户明确“不入库并记住”的来源子路径，按 `(repo, domain, subpath)` 生效，随库共享；跨域可独立选择。返回、未勾选或取消均不写记录；不建决定历史或队列。技能当前版本的跳过仍保存在自身 `source.yaml`，与拒绝新增不同。

该文件不是“所有未入库技能”的清单，也不保存全仓检查历史。新增发现基线须落实 spec.md 的首次未选/后来新增例子，由 T039/T043 在实现前设计；不因更新其他技能而把未决定且仍存在的新增候选自动当作已处理。

## 二、本机状态（SQLite，`skill-manager.db`）

> 只存本机状态，无外键技能表——技能引用一律存 `(domain, name)` 字符串（D1/D2）。丢失后果：部署可重部署、启用可重选、设置可重填。

### 2.1 `deployments` — 部署记录

| 列 | 说明 |
|---|---|
| domain, name | 技能引用（联合键一部分） |
| target | 目标标识：`agent:<key>` / `shared` / `hermes:default` |
| target_path | 实际部署副本的绝对路径，目标目录设置后来改变仍能定位原副本 |
| status | `ok` / `paused_local_change`（副本被外部修改，暂停覆盖） |
| source_hash | 部署时库内容指纹（供变更检测） |
| deployed_hash | 最近成功部署的合成产物指纹，识别目标副本外部变更 |
| synced_at | 最近同步时间 |

- 生命周期：部署 → `ok`；同步发现副本外部变更 → `paused_local_change`（其余技能继续）；用户经 Agent 整理后显式确认该项按库覆盖，再同步恢复 `ok`；副本删除成功才移除记录（FR-020/022）。
- 库内条目删除而副本暂停时，保留该行及 `target_path`，报告“库已删除、副本清理暂停”；Agent 经用户裁决继续清理后删记录。不新增待删任务队列，也不自动丢弃本地副本。新增 `deployed_hash`/`target_path` 分别防止覆盖外改副本和删除库后失去其定位，这是两项直接影响用户内容的后果。
- 受管身份以本机部署记录及实际路径为依据；不额外向技能产物写受管标记文件。数据库丢失时按未受管目标处理，可由用户明确部署重新接管，不建设自动恢复机制。
- 未受管同名目标：直接覆盖并插入记录（接管，FR-018）。

### 2.2 `hermes_enables` — 档案启用名单

`(profile, domain, name)` → 启用关系。default 档案即部署记录（`hermes:default`）；其他档案启用是本机状态，内容更新不重置（FR-027）。

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

`(repo, domain, name)` → `remote_version, checked_at, result`。采用版本与跳过决定在库内 `source.yaml`（共享）；本机只记"最近查到什么、何时查的"。

### 2.6 `settings` — 应用设置（KV）

`theme`（light/dark/system，默认 system）、`font_scale`、`auto_check_minutes`（0=关闭/30/60/360/1440，默认 60，三项共用 FR-029）、`shared_dir`（共享部署目录路径）、`last_sync_at`、诊断与日志路径配置。GUI 自动上传沿用用户既有确认；具体确认记录形式由 M4 T050 落定，不为 Agent 操作增加授权账本。

## 三、身份引用与完整性规则汇总

| 引用位置 | 引用形式 | 解析失败时 |
|---|---|---|
| 部署记录 / 档案启用 | `(domain, name)` | 技能已删 → 按删除清理规则处理 |
| 预设成员 | 技能名（同文件所属域） | 界面标注"缺失成员"，不阻塞其他成员 |
| 工具配套 Skill | `(domain, name)` | 显示"未入库"，可前往入库 |
| 指令源 | `<agent-key>.md` 文件名 | 无专属文件 → 仅全局内容 |

- 所有共享文件的写入者（GUI、CLI、Agent 手改）地位平等：文件监听触发索引刷新，无"谁是权威来源"的区分（宪章 V）。
- 库同步冲突 = Git 层冲突，暂停同步交 Agent/用户（FR-039），应用层不做合并。
