# Data Model: 库文件格式与本机状态

**Date**: 2026-10-15 | 依据：[spec.md](spec.md)、[research.md](research.md)（D1~D8）

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

- `translation.md`：中文正文译文，整份一项；仅管理器阅读，不作部署正文。
- `overlay/<agent>/<os>/<相对路径>`：覆盖层文件树；目录维度可按需省略（如仅按 agent 或仅按 os 细分，组合顺序实施时按 M2 调研定）；同路径文件一项。
- `source.yaml`：来源记录（无上游的自建技能无此文件）：

```yaml
repo: https://gitea.example/user/skills.git
subpath: tools/xxxskill       # 上游仓库内技能路径
version: v1.2.3               # 当前采用的上游版本
skipped_version: v1.3.0       # 可选：已跳过的上游版本（含删除类），随库共享
registered_at: 2026-10-15
```

- **写入规则**：任何写回以原始键值合并，**保留软件不认识的字段**（D3 格式演进约束）。
- `.manager` 整体不进入部署结果（FR-016）。

### 1.4 预设 `presets/<domain>.yaml`

```yaml
- id: rust-deps-check        # 预设内唯一标识（人写，域内不重复即可）
  name: Rust 依赖检查
  icon: shield               # 图标标识
  order: 2
  members: [cargo-audit, dep-guard, ...]   # 技能名引用（域内唯一可解析）
```

- 组合定义（含顺序与图标）随库同步；成员编辑不改变部署（FR-024）。

### 1.5 工具资料 `tools/<tool-slug>/tool.yaml`

```yaml
name: ripgrep
summary: 极快的正则搜索工具          # 卡片短摘要
intro: |                            # 概览长简介（可读、多段）
  ripgrep 是 ...
tags: [搜索, cli]
category: 命令行工具
check_command: rg --version | head -1   # Agent 编写的检查命令（FR-046）
upstream_command: git ls-remote --tags https://github.com/BurntSushi/ripgrep   # 取上游最新版本
skills:                             # 配套 Skill 关联（技能名引用）
  - domain: code
    name: ripgrep-usage
```

- `guide.md`：维护指南（用途与入口、安装方式、安装与配置位置、检查命令、更新步骤、验证、注意事项）。
- 工具 slug（目录名）即工具身份；本机安装记录不在库内（见 2.4）。

## 二、本机状态（SQLite，`skill-manager.db`）

> 只存本机状态，无外键技能表——技能引用一律存 `(domain, name)` 字符串（D1/D2）。丢失后果：部署可重部署、启用可重选、设置可重填。

### 2.1 `deployments` — 部署记录

| 列 | 说明 |
|---|---|
| domain, name | 技能引用（联合键一部分） |
| target | 目标标识：`agent:<key>` / `shared` / `hermes:default` |
| status | `ok` / `paused_local_change`（副本被外部修改，暂停覆盖） |
| source_hash | 部署时库内容指纹（供变更检测） |
| synced_at | 最近同步时间 |

- 生命周期：部署 → `ok`；同步发现副本外部变更 → `paused_local_change`（其余技能继续）；用户经 Agent 整理回库后再同步恢复 `ok`；撤销/删库技能 → 记录删除并清理副本（FR-020/022）。
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

### 2.4 `tool_installs` — 工具本机安装记录

| 列 | 说明 |
|---|---|
| tool_slug | 对应 `tools/<slug>` |
| version, method, path, config_path | 本机安装事实（Agent 回写） |
| check_status | `unchecked` / `checking` / `updatable` / `latest` / `failed` |
| checked_at, check_output | 最近检查结果 |

- 检查状态机：`unchecked → checking → (updatable | latest | failed)`；应用更新后 Agent 回写重置为 `unchecked`（FR-046/047）。
- 未安装工具：无此表行，界面显示"未安装"（FR-045）。

### 2.5 `upstream_checks` — 上游检查状态（本机）

`(repo, domain, name)` → `remote_version, checked_at, result`。采用版本与跳过决定在库内 `source.yaml`（共享）；本机只记"最近查到什么、何时查的"。

### 2.6 `settings` — 应用设置（KV）

`theme`（light/dark/system，默认 system）、`font_scale`、`auto_check_minutes`（0=关闭/30/60/360/1440，默认 60，三项共用 D 之 FR-029）、`shared_dir`（共享部署目录路径）、`last_sync_at`、诊断与日志路径配置。

## 三、身份引用与完整性规则汇总

| 引用位置 | 引用形式 | 解析失败时 |
|---|---|---|
| 部署记录 / 档案启用 | `(domain, name)` | 技能已删 → 按删除清理规则处理 |
| 预设成员 | 技能名（同文件所属域） | 界面标注"缺失成员"，不阻塞其他成员 |
| 工具配套 Skill | `(domain, name)` | 显示"未入库"，可前往入库 |
| 指令源 | `<agent-key>.md` 文件名 | 无专属文件 → 仅全局内容 |

- 所有共享文件的写入者（GUI、CLI、Agent 手改）地位平等：文件监听触发索引刷新，无"谁是权威来源"的区分（宪章 V）。
- 库同步冲突 = Git 层冲突，暂停同步交 Agent/用户（FR-039），应用层不做合并。
