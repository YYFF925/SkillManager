---
name: skill-manager-hermes
description: 维护 Skill Manager 的 SkillManagerVault：.manager 格式、目录与同名规则、原文/译文/注释/覆盖层边界、来源更新与同项合并、部署与上传约定、冲突暂停处理、本机 Hermes 目录的同名副本与漂移修复、CLI 参考与工具中心登记/检查/回写流程、技能与知识库的日常分流治理
---

# Skill Manager 库维护规范

这是 Skill Manager 桌面应用的**配套维护 Skill**：当用户要求你（外部 Agent）维护技能库、
上游更新、部署副本、跨机同步，或处理工具中心的登记/安装/检查/回写时，按本文执行，
命令语法与输出约定见 [references/cli.md](references/cli.md)。

要把**多台机器**接入统一管理站（station / Host 配对、设备离线、首次管理员、容器重置管理员），
读 [references/station-hosts.md](references/station-hosts.md)；其中的硬规则：Host 绝不进 Docker、
管理站不存 Vault、**配对后必须重启该机 Host**、首次管理员只能 `POST /auth/setup` 创建。

软件只负责库管理、状态展示与版本检查，**不内置 Agent、不自动安装程序、不自动翻译**；
所有实际写入动作由你经 CLI（命令行接口）完成，`tool` 的任务文本与技能维护任务都只引用
本 Skill 的库内路径与“按其规范执行”，不内联规范全文。

## 一、基本概念

- **SkillManagerVault**：用户自己已有的 Git 仓库，是技能内容的唯一真值源。首次接入由用户在 GUI 选择目录，
  本机记住路径；CLI 用全局 `--vault <path>` 指定。共享内容全部是库内可直接读写的文件，随库 Git 同步。
- **共享目录**：按系统当前用户主目录自动定位 `.agents/skills`，无需设置，不提供自定义或清除路径。浏览不创建目录，首次部署时创建。
- **本机技能收编与阅读**：`skill adopt` 接受内置 Agent key 或 `shared`；共享目录未受管技能进入编码库并接管原共享部署，不新增其他目标部署。`agent read-local <target> <name> [--file <相对路径>]` 只读实际本机副本，不以库内原件代替。
- **管理域**：`code`（编码库）与 `lobster`（龙虾库）。域的边界是硬约束：编码技能不能部署给龙虾目标，
  反之亦然；支持声明不能绕过域边界。
- **技能身份** = 管理域 + 技能名，技能名即目录名。有上游来源的技能保留上游原名。
- **分类** = 目录层级，最多一级（`code/<分类>/<技能名>/`）；未分类直接在域根下（`code/<技能名>/`）。
  改分类 = 移动目录，技能名不变。分类路径不是技能身份。
- **归档** = 把技能移到 `archive/<domain>/[<分类>/]<技能名>/`，完整保留 `.manager`。归档前须卸载全部受管部署；还原不自动重新部署。
- **本机状态**只在 SQLite（`skill-manager.db`，位于本机状态目录，与库分离）：部署记录、档案启用、
  Agent 配置、设置、工具本机安装记录、检查事实。共享内容真值不写 SQLite。
- GLOSSARY 与数据模型见本项目仓库 `specs/002-skill-management/data-model.md`；命令契约见
  `specs/002-skill-management/contracts/cli.md`。

## 二、目录与同名规则

```text
SkillManagerVault/
├── code/                    # 编码技能库
│   └── [分类/]技能名/        # 分类最多一级
│       ├── SKILL.md（或 skill.md）及技能文件   # 原始内容
│       └── .manager/        # 该技能的管理层
├── lobster/                 # 龙虾技能库，结构同上
├── archive/<domain>/        # 已归档技能，保留分类目录与 .manager/
├── presets/<domain>.yaml    # 预设定义
├── instructions/global.md, <agent-key>.md   # 指令源
├── hermes-identity/global.md, <profile>.md  # Hermes 人格源（SOUL.md）
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
- 来源记录绑定技能的**上游仓库**。技能进入唯一 SkillManagerVault，来源/子路径/采用版本用于后续对上游的
  检查与更新；所有电脑同步同一个 SkillManagerVault，**另一台电脑拉取是库同步，不是重新入库**，
  不要把 SkillManagerVault 远端写成该技能的上游。

### 3.5 其他共享文件

- `presets/<domain>.yaml`：`id`（域内唯一，新建时由名称派生）、`name`、`icon`、`order`（预设间顺序）、
  `members`（技能名数组，**数组顺序即成员显示顺序**）。
- `instructions/global.md` 与 `instructions/<agent-key>.md`：指令**源**；分发目标由本机 Agent 配置决定。
- `hermes-identity/global.md` 与 `hermes-identity/<profile>.md`：Hermes 人格**源**；保存或库拉取变化后，
  按「专属在前、全局在后」无条件覆盖本机全部已发现档案的 `SOUL.md`（空合成写成空文件；不检测冲突、
  不导入现网文件、不创建档案、不改启用名单；合成约 20,000 字符以上会提示可能被截断）。
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

### 4.1 本库策略：不用覆盖层承载跨 Agent 差异

软件的覆盖层能力保留（`skill overlay set` 仍在），但**本库不用它区分不同 Agent 体系**：
同一技能在 Hermes 与编码 Agent 上需要不同正文时，用**两个管理域各持一份独立正文**
（`code/<分类>/x` 与 `lobster/<分类>/x`）承载差异，而不是用 `_/_/SKILL.md` 整份替换。

理由：`_/_` 覆盖层是第二份完整正文，它盖住库内原件，于是有两个都不会报警的盲区——①上游更新写进
`SKILL.md` 后，部署面看不到（覆盖层整份替换）；②本地写进 `SKILL.md` 的经验，会被下一次上游 `update`
整份冲掉。这不是假设：11-16 上游更新覆盖了 base，本地 11-11 写进去的两条实测经验就此消失，而部署面
也一直缺上游新增的 `archive` 命令说明。

边界：跟踪他人仓库、只能靠覆盖层做本地适配的技能（如 IsaacLab 系列）仍按上游约定保留覆盖层——
删掉它等于要么丢本地适配、要么丢上游更新。分域只适用于**我们自己维护正文**的技能。

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

## 自建技能的敏感信息管理

新建、收编或修改自建技能时，涉及密钥或私人环境信息，先使用 `shared-credentials`：

- 密码、API Key（接口密钥）、Token（访问令牌）、Cookie（会话凭据）、私钥、可授权的订阅/Webhook URL，以及需要保密的账号、地址和个人路径，只保存在 Infisical。技能原文、脚本、模板、译文、覆盖层、注释和共享工具资料均不能保存真实值或片段，也不另建 Credential Map（凭据索引）。通用示例用明确占位符。
- 先执行 `credentials.py list --path / --recursive`，按目标与 metadata（元数据）核对现有记录。非秘密目标信息可保存在 `_CREDENTIAL_INFO`，秘密值不得进入 metadata。不能把记录存在或 metadata 完整等同于字段齐全、凭据有效。同一服务、账号和权限范围的 API Key（接口密钥）复用同一条记录，不按技能、应用或机器重复录入；轮换时更新原记录。只有用户明确需要独立凭据时才另建实例。
- 需要新值或迁移时，先明确用途、目标、身份、认证方式和实际字段，再按 `shared-credentials` 规则入库。真实值只经桌面隐藏输入或用户授权的源文件/受保护消费者传递，不进入对话、命令参数或 Git；缺少来源时先询问，不编造、不从无关目录搜寻。
- 运行时仅通过 `run` / `run-json` 把所需值注入实际消费者；人工粘贴用 `copy`。JSON（结构化数据）字段必须与记录一致，普通文本不能当作 JSON；缺记录、字段或认证失败时说明原因并停止该操作，不回退旧脚本、Obsidian、`.env` 或其他密码库。
- 部署端可以保留程序必需、由凭据库派生的本地配置、私钥文件或会话；这些不是技能的备用凭据来源，不回写共享库。Infisical 自身的 bootstrap（初始认证）配置留在各机本地，不能要求首次连接依赖从同一实例取值。
- 写入或部署前检查本次涉及的正文、脚本、模板、`.manager` 和工具资料是否有真实值，验证消费者确实从注入字段取值且不回显。确认入库成功后再移除原位置的敏感值；清理当前文件不代表 Git 历史已脱敏，禁止据此直接宣称可公开。

沿用原有上传授权边界；凭据入库与技能修复不构成 Git 提交或推送授权。

## 六、修改后同步部署与上传

你修改库内容后，按固定顺序处理：

1. **同步已有部署，不新增目标**：`deploy sync` 只更新本机已有部署与已启用指令目标，
   不产生首次部署、不改启用名单或路径。
2. **上传**：接着执行 `sync push`（提交并推送**整个工作区**改动，不按改动归属拆分）。
   用户已经在用本 Skill 维护库，不要每次再问「要不要上传」，也不要让用户去 GUI。
   用户明确说不要上传时才跳过。
   - 打开/关闭 GUI 保存后自动上传：`settings set-git-auto-push true|false`。
   - CLI 写库、`deploy sync`、文件监听、启动/定时同步单独执行时仍不自动上传；本维护任务收尾的 `sync push` 才上传。
   - 上传失败时保留可读原因，可再次 `sync push`；后台路径不重试推送。
3. **部署与库同步分开**：`sync pull` 只让库内容一致；拉取到的**新增技能不会自动部署**，
   已启用指令目标会按“全局在前、专属在后”覆盖规则刷新，但不新增启用目标、不改本机路径；
   `hermes-identity/` 变化时也会按「专属在前、全局在后」无条件覆盖刷新本机全部已发现 Hermes 档案的 `SOUL.md`。

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

CLI 由 **Skill Manager 桌面安装包**提供，技能目录只包含操作规范与参考，不包含程序。首次使用须按
[CLI 定位与调用](references/cli.md#定位与调用安装后的-cli)找到实际安装的可执行文件，并验证 `--version`、
`--help`；后续示例中的 `skill-manager` 都替换为该绝对路径。不要因为 `command -v`／`Get-Command`
找不到就断言程序未安装，也不要要求用户安装 Rust 或从项目源码编译。GUI 无需运行。
若只安装了本 Skill，或应用包内程序缺失／无法执行，应明确报告需要安装包含 CLI 的桌面包及具体错误。

全局选项（所有子命令可用）：`--vault <path>`、`--state-dir <path>`、`--json`。
在本机模拟两台电脑时，用同一库的两个克隆分别配不同 `--state-dir`。

定位安装目录时，**从运行中的进程反查比搜盘可靠**：`Get-Process | Where-Object {$_.ProcessName -like "*skill*"} | Select-Object Path`（macOS 用 `ps` + `ls /Applications`）。
只搜 `%USERPROFILE%` 和 `C:\Program Files` 会把已装的机器误判成“未安装”，而 vault 常在另一块盘而不是用户目录下。

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
skill-manager skill archive add <ref>…
skill-manager skill archive list [--domain code|lobster]
skill-manager skill archive restore <ref>…
skill-manager skill archive delete <ref>…
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
skill-manager deploy to <target> <ref>… [--also-target <target>…]
skill-manager deploy sync [--ref <ref>] [--target <target>] [--overwrite-local]
skill-manager deploy status [--target <target>] [--paused]
skill-manager sync pull | sync push | sync status | sync conflicts

# Hermes
skill-manager hermes profiles | hermes list <profile> [--query <text>]
skill-manager hermes identity get global|<profile>
skill-manager hermes identity set global|<profile> (--file <path> | --stdin)
skill-manager hermes identity distribute

# 工具中心
skill-manager tool list [--query <text>] [--category <c>] [--status <status>]
skill-manager tool check (<slug> | --all)
skill-manager tool register --dir <path> | skill-manager tool set <slug> --dir <path>
skill-manager tool set-install <slug> --version <实际版本> [--method <text>] [--path <path>] [--config-path <path>]
```

`<target>` = Agent key、`shared`（共享目录）或 `hermes:default`。
**`deploy sync`/`undo` 的目标串要逐字照抄 `deploy status` / `skill get` 里的部署记录**（常见形如 `agent:hermes`）。
`--ref` 技能不存在、或带了 `--target` 却没有任何匹配记录时命令失败，不再返回空的 `success`。仅 `--ref` 且技能存在但还没有部署记录时仍可空成功。
同步后仍须复查 `deploy status --paused` 确实变少。
`HERMES_HOME` 必须指向 Hermes **根**（里面有 `skills/` 和 `profiles/`），不要设成某个 `profiles/<档案>`；
指错时 `hermes list/enable` 会报档案不存在并提示应改成根目录，不会自动去上一级。
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

## 十、Hermes 本机技能目录：同名副本与版本漂移

部署目标 `hermes:default` 落在 `~/.hermes/skills`。库同步、手工拷贝或早期安装会在这里留下同一技能名的多份副本，
处理方式如下（跨域同名 `code/x` 与 `lobster/x` 是库的合法设计，不算冲突；冲突只发生在本机这一层）：

- **症状**：`skill_view('<name>')` 报 `Ambiguous skill name … refusing to guess` 并列出 `load_names`。
  说明同名技能同时存在于域根 `~/.hermes/skills/<name>/` 与分类子目录 `~/.hermes/skills/<category>/<name>/`。
  临时可按相对路径 `<category>/<name>` 加载，但必须先消解冲突，否则该技能在列表与检索里长期不可用。
- **保留哪一份以部署记录为准，不要凭 `.manager` 猜**：部署产物**永远不含** `.manager/`，而手工拷贝的散落副本反而常带着它——按「有 `.manager` 就是受管」判断会删错。看 `deploy status` / `skill get <domain>/<name>` 的 `deployments[].target_path`，那条路径才是受管副本，其余同名目录都是散落副本。
- 不要给受管副本改目录名来消歧——技能名即身份，改名会连带弄乱库内条目与部署记录。
- **先取证再动手**：`diff -r` 两份目录；把 `git show HEAD:<库内路径> | md5`、库工作区文件、本机副本三者的 md5
  一起比。出现三四个互不相同的哈希说明是多处分叉，而不是一处未提交编辑。删除前把两份都拷到 scratch 备份，
  并说明 scratch 约 24 小时后清理，需长期保留要另存。
- **以上游或库内最新版为合并底稿，逐条回补本机独有内容**：整份覆盖会静默删掉只写在本机的经验条目——
  那些条目之所以只在本机，正因为它从未回写库。合并后还要核对脚本与 references 是否与正文同版本，
  正文描述而脚本没有的参数属于换新不彻底。
- **部署产物反写库内原件前，剥离 frontmatter `description` 里的 `<注释>｜` 注入前缀**：那是产物形态，
  库内原件必须保持无注释（注释真值在 `.manager/meta.yaml`）；照抄产物 description 进库会把注释烧成原文，
  下次部署叠出双前缀。
- **过时的本机 tests**：库内没有 `tests/`，或测试依赖已被移除的常量/接口时，删掉本机测试并如实说明失败原因；
  常年红灯的测试比没有测试更误导。
- **验证**：修复后必须 `skill_view('<裸名>')` 成功，再跑技能自带的 check／探针命令确认功能没被换坏。
- **回写**：合并结果补进库内后按 §六 收尾 `sync push` 上传；用户明确说不要上传时才跳过。

### 10.1 先定位「哪个库内条目拥有这条部署」

- `<ref>` 只有 `<domain>/<name>` 两段，**分类不是 ref 的一部分**：`skill get 家庭运维/x` 直接报
  「技能名不合法（家庭运维/x）：不能包含路径分隔符」，`deploy sync --ref` 同理。
- 同一技能名可同时存在于 `code/` 与 `lobster/`（跨域同名是合法设计），两份内容**可以不同**，部署目标也不同：
  常见 `code/x` → `shared`（`~/.agents/skills/x`）、`lobster/x` → `agent:hermes`（`~/.hermes/skills/x`）。
- 所以**动手前先读部署记录**：`skill get <domain>/<name>` 看 `deployments[].target_path`，确认要修的是哪个域的条目。
  按名字随便挑一个条目改，会写进没有部署的那份：白做一趟，还凭空多出一份漂移，后续排查更乱。
- 排查顺序：`deploy status --paused` 拿到暂停的 `<domain>/<name>` → `skill get` 确认目标路径 → 再决定改哪个文件。

### 10.2 库内与部署侧各有独有内容时：联合并，不整份覆盖

`sync push` 前 git 干净、部署副本里有本机独有经验、库内又有 GUI 侧更新时，按三类合并
（比较前先把部署侧 frontmatter 的 `<注释>｜` 前缀剥掉）：

| 差异类型 | 处理 |
|---|---|
| 只在部署侧出现的行 | 补进库内——那是从未回写的经验 |
| 只在库内出现的行 | 保留——不是旧版残留，而是库侧更新过 |
| `replace`（同一处两种措辞） | 取事实更新的一侧；两侧都在讲事实时**改写成一句话同时容纳**（如「写入值只从桌面弹窗 / 隐藏输入或 stdin（自动化）/ 用户指定文件进入」） |
| 整份文件部署侧明显陈旧（旧平台路径、旧逻辑脚本） | 整文件跳过，保留库内 |

合并后必查：入口文件 `description` 无残留 `<注释>｜`；部署侧独有的 `references/`、`scripts/` 已补进库
（正文引用了它们）；逐个 `deploy sync --ref <domain>/<name> --target <target> --overwrite-local`；
最后 `deploy status --paused` 必须归零，再 `sync push`。

上面的三类合并可直接用 `scripts/merge-drift.py` 做（默认 dry-run，`--write` 才落盘并先备份）：

```bash
python scripts/merge-drift.py --vault-root <SkillManagerVault> \
  --vault-skill lobster/必备/hermes-memory --deploy ~/.hermes/skills/hermes-memory \
  --keep-vault references/cli.md --copy-new-files --write
```

它按 §10.2 的规则做 opcode 级联合并、自动剥 `<注释>｜`、可选补入部署独有文件，并在落盘前把两侧拷到
scratch 备份；`--keep-vault` 用来指定「部署侧陈旧、整文件取库内」的那几个文件。

### 10.3 档案（profile）目录的部署冲突：未受管实体目录挡掉启用

桌面应用的「档案」就是 Hermes profile（`~/.hermes/profiles/<id>/`）。为具名档案启用技能时，
管理器把 default 池（`~/.hermes/skills/<name>`）**复制**成该档案下的独立实体目录
（`profiles/<id>/skills/<name>`），**不再使用符号链接**——这样 `hermes curator lock` 才能锁住该档案会话里的技能。
若那里已经有一个**未由本管理器登记的实体目录**（手工 copy、或其他 bot 放进去的），启用会被跳过并提示：

```text
已启用 0 项；1 项未完成: <skill>（档案目录已有同名技能（非本管理器安装的副本），未改动：
/Users/<user>/.hermes/profiles/<id>/skills/<skill>）
```

此时该技能在 GUI 里显示「未启用」，勾选也装不上。要接管，按此顺序做（**先取证再删**）：

1. 打包备份：`tar czf ~/backups/<id>-local-skills-<ts>.tar.gz -C <profile>/skills <实体目录…>`；
2. 删掉这些**未启用登记**的实体目录；已启用的独立拷贝不要当冲突删；
3. 让用户刷新 GUI 后正常勾选启用，成功后目录里出现实体拷贝（可用 `test -L` 确认不是链接）。

```bash
# 区分实体目录与遗留链接（不要用 test -d，它两者都真）
find ~/.hermes/profiles/<id>/skills -maxdepth 1 -mindepth 1 \
  -exec sh -c 'printf "%-32s %s\n" "$(basename "$1")" "$(test -L "$1" && echo "链接→$(readlink "$1")" || echo 实体目录)"' _ {} \;
```

两个必须记住的边界：

- **`hermes-agent` 无法被接管**：它在 `ESSENTIAL_SKILLS` 里，删掉下次会话又被播种回实体目录。
  永远会剩这一个实体目录，不要反复删。
- **档案目标只在 GUI / `hermes enable` 里存在**：CLI 的 `agent local-skills` 只认 `agent:<key>`、`shared`、`hermes:default`，
  传 `hermes:<profile>` 报「目标无法解析」、传裸 profile 名报「不在内置 Agent 名单中」。
  档案启用用 `hermes enable <profile> <name>`，禁用只删该档案拷贝，不改 `~/.hermes/skills`。
- 遗留符号链接在再次启用或 `deploy sync` 时会改写成实体拷贝。

### 10.4 退役技能一律软删除

删技能（去重、转知识后退役、清理档案副本）**不要** `rm -rf`，走 `scripts/skill-trash.sh`：
它把目录移进 `~/.hermes/.skill-trash/<时间戳>-<名字>/` 并写 `manifest.json`（原路径、时间、原因），
每次删除都打印回滚命令，`restore` 原样还原，`purge <天数>` 清过期条目。

```bash
sh scripts/skill-trash.sh rm ~/.hermes/profiles/<id>/skills/<name> --reason "转知识入库"
sh scripts/skill-trash.sh list
sh scripts/skill-trash.sh restore <条目ID>
```

库本身是 Git 仓库，是第二层网；但受管副本删了之后 `deploy status` 会报缺项，
所以仍以回收区为主、Git 为兜底。

## 十一、状态表达诚实（硬要求）

- 未检查、检查失败、未登记命令、待更新、任务已生成，**都不得**冒充“最新、已安装、已更新、已完成”。
- 未入库的配套 Skill 如实显示“未入库”，不编造库内路径；无法比较的版本如实说明原因。
- 生成任务、检查上游、拉取同步都不等于已部署、已安装或已上传。
- 无法完成的动作要如实报告失败原因与可定位路径（技能/目标/实际路径），不静默降级。

## 十二、Hermes 的自动改进与库的关系（实测结论）

Hermes 有两条**独立的**自动写入通道，都会改到技能内容，必须与库的「唯一真值源」规则对齐：

| 通道 | 触发 | 默认 | 能改什么 | 配置键 |
|---|---|---|---|---|
| 背景自我改进评审（ledger 里 `actor=curator`） | 每轮对话后的辅助模型 fork | 开 | **任何可读技能**：自建、手写、pinned、bundled、hub、external 都能 patch | `auxiliary.background_review.enabled` |
| curator 定时 pass | 距上次 ≥ `interval_hours`(168) 且闲置 ≥ `min_idle_hours`(2) | 开 | ① 确定性：14d 未用→stale、30d→`.archive/`（仅 agent-created）② LLM 合并（`consolidate`，默认关） | `curator.*` |

写权限矩阵（本机 v0.23.1 源码实测，非文档转述）：

- `hermes curator pin`：只防归档，**不防内容改写**；且**仅对 agent-created 有效**（其它技能直接拒绝，`set_pinned` 受 curation-gate）。
- `hermes curator lock`：**唯一能冻结内容**的手段——skill_manage 全动作 + `write_file`/`patch` + terminal 写该目录全部拒绝；不受 curation-gate，任何技能都能锁。
- `skills.write_approval: true`：只拦 `skill_manage` 的写动作（staged 到 `~/.hermes/pending/skills/`，用 `/skills pending|diff|approve` 处理）；**file 工具直写不受控**。
- `skills.external_dirs`：只拦 delete/archive，**内容照样可写**（`skill_manager_guards._background_review_delete_guard` 原文：Content writes are not gated by ownership）。所以外部目录**不能**当作内容只读隔离。

**部署副本被自动改写 = `paused_local_change` 的常见来源**：2026-11-17 实测本机 9 项部署全部暂停，全局 ledger 里 `actor=curator` 的 patch 达 153 次、write_file 27 次，且写入路径正是部署副本。

### 12.1 让「库内原件 = 运行副本」：符号链接式部署（实测可行）

把部署路径从拷贝换成指向库内原件的符号链接，三项实测结论（2026-11-17，`lobster/symlink-probe` 探针）：

1. 经部署路径写入会**写穿到库内原件**（内容落在库，`git status` 立即可见）；
2. `deploy status` 仍报 `ok`（源与部署是同一份文件，哈希一致），**不再产生 `paused_local_change`**；
3. 之后跑 `deploy sync`（不带 `--overwrite-local`）会提示「受管副本已被外部修改，已暂停覆盖」——这是**记录哈希过期**，不是内容分叉；内容本身已一致，可忽略或在 GUI 点一次「按库覆盖」清零。

配套限制（实测演进）：v0.1.12 及以前 `--overwrite-local` 与 `skill delete` 这类破坏性动作 CLI 拒绝执行（报「破坏性操作需要有效的界面会话」）；v0.1.13 起 Agent/CLI 的删除与覆盖不再强制界面确认，可直接执行。

因此规划「自动改进按规则走」时：先决定部署形态（拷贝 or 符号链接），再决定自动写入范围；不要只关开关而留着拷贝式部署，否则漂移会再次累积。

### 12.2 具名档案启用必须是实体拷贝，否则 lock 看不见

`hermes curator lock` 用 `Path.rglob`（不跟随符号链接）找技能；加载器 `os.walk(followlinks=True)` 却能顺着链接写穿到 `~/.hermes/skills`。
因此具名档案启用改为独立拷贝：锁写在该档案自己的 `skills/.usage.json` / 技能目录里，锁 default 保护不了其他档案的会话。

- 从库更新 default 后跑 `deploy sync`：未改动的档案拷贝跟着刷新；档案内已改（含自动改进）的拷贝默认跳过。
- 同步时保留技能目录里的 `.usage.json`，避免把 lock 冲掉。
- 禁用 `hermes disable <profile> <name>` 只删该档案拷贝，不动 default 池。
- 不要把 `HERMES_HOME` 设成某个 `profiles/<档案>`：那会把该档案当成 Hermes 根，查找其他档案会报不存在。

## 十三、技能与知识的分流治理（技能管家的日常执行）

定位是**执行者，不是审计员**：判出来的事自己做完，不留成待用户逐日批准的清单。用户不接受每天做决定；
可追溯的留痕与可回滚的软删除，代替事前审批。

- **判定只认客观信号**：各 profile 的 `skills/.usage.json`（`created_at`、`last_used_at`、`last_viewed_at`、
  `use_count`、`view_count`、`patch_count`、`pinned`、`state`、`archived_at`）。`created_by` **可能为 null**，
  所以「某 bot 自建」用「出现在该 profile 技能目录 + 库内无同名条目 + `created_at`」组合判定，不单看该字段。
- **成本不对称决定默认偏向**：错删技能丢掉的是自动执行能力，代价高且难以复原；错留一条知识只是检索噪声。
  因此混合内容（既有背景经验又有可复用步骤）默认「**保留技能 + 抽出经验进知识库**」，不默认“转成知识”。
- **必须等知识可检索才动原件**：知识库入库是**异步索引**（`is_processing`/`processing_count`），接口成功
  不等于可检索。判据是 `chunk_count > 0`、索引空闲、且用一句真实问句搜到该内容；达标后原技能走 §10.4
  `skill-trash` 软删除，**不用 `rm`**。顺序颠倒会造成「知识没进库、技能已删」的空档。
- **含真实凭据的技能不自动脱敏**：命中密码／密钥／Token／私钥就暂停该技能并通知用户决定。自动脱敏是整条
  流水线里唯一会**静默泄密**的动作，不能自动化。
- **与 curator 分工，不重叠**：curator 管「旧的」（14d stale / 30d `.archive/`、pinned 与 cron 引用豁免、
  从不真删，见 §12），我只管「当日增量」——新产生的东西该不该存在、以什么形式存在。
  不要另做一套“长期零使用即淘汰”，两套修剪互不知情时故障最难查。
- **「无法确定」不建待确认队列**（那等于把每日决策还给用户）：保留原件 + 记账，只在同一项连续挂满多日、
  或下一步要删时才提一次。
- **知识库侧必须逐个 `--kb` 检索**（无跨库检索），所以规划多库时：通用经验进共享库、bot 专属进各自库，
  检索顺序固定「先共享、后自己」；说明“知识库在哪、怎么查”的那层薄 skill 用**一份库内条目多目标部署**，
  不要每个 bot 各写一份，否则副本必然分叉（§10 的成因）。
- **留痕格式**：每个动作一条——信号数据 → 判定理由 → 执行的命令 → 回滚 ID。这是给用户事后查的，
  不是给他事前批的；日常出简报，不要求回复。

### 13.1 自主执行的前提：审批闸门必须先打开

「要它自主干活」不是改 SOUL 就能实现——**新建/克隆的 profile 默认会把危险命令挡在审批闸门前**，
定时任务里更是直接拒绝，表现出来像「它不肯干」。

先查**生效值**，不要读 `config.yaml`：文件里常常只写了 `approvals.mode: off`，其余几档是代码默认值，
读文件会漏看。

```bash
hermes -p <profile> config get approvals
```

| 键 | 克隆档案的常见生效值 | 对自主执行的含义 |
|---|---|---|
| `mode` | `off` | 交互式会话可执行 |
| `cron_mode` | **`deny`** | 定时/后台任务里危险命令**全被拒**——每日巡检等于白跑 |
| `single_query_mode` | **`deny`** | `hermes -p <profile> -z "..."` 一次性执行同样被拒 |
| `unattended_mode` | `deny` / `approve` | 无人值守时挂起等批准，等于卡死 |
| `deny` | **`[]`（空）** | 反方向的问题：放开后连最危险的命令也没拦 |

三条通道一起打开，并把全局的灾难黑名单继承过来（**不要手写那些模式串**：在 terminal 命令里逐字出现
会命中 Hermes 自己的 deny 匹配器而被硬拦，用脚本从 `~/.hermes/config.yaml` 的 `approvals.deny`
读出来再写进 profile 的 `config.yaml`）：

```bash
hermes -p <profile> config set approvals.cron_mode off
hermes -p <profile> config set approvals.single_query_mode off
hermes -p <profile> config set approvals.unattended_mode off
```

自主权还缺两条就是假的：**可回滚的删除**（§10.4 `skill-trash`）与**留痕**（信号 → 理由 → 命令 → 回滚 ID，
替代事前审批而不是补充它）。放开审批等于把该 profile 的破坏半径交给它的判断力，所以**自主权的域要窄**：
让一个能删技能、能写知识的 bot 只做技能与知识治理，不要再把「升级 Hermes、重启网关、删 profile」
这类全局动作塞进同一个 bot；需要运维就另起一个，各自作用域不重叠。

完整状态机（采集 → 理解 → 分类 → 迁移 → 验证 → 汇报）、判定表与 WeKnora 侧调用见
[references/skill-knowledge-triage.md](references/skill-knowledge-triage.md)。

