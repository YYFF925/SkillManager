# CLI Contract: 命令表与覆盖对照

**Date**: 2026-10-15 | 原则（research.md D6）：领域子命令、全局 `--json`、无交互（决策由 Agent 对话确认后以参数传入；检查类只展示、应用类显式执行）。

约定：`<domain>` = `code|lobster`；`<ref>` = `<domain>/<name>`（技能引用）；`<target>` = Agent key、`shared`（共享目录）或 `hermes:default`。所有命令支持 `--json`（结构化输出）与 `--vault <path>`（覆盖默认库位置）。

## 命令表

### skill — 技能与库管理

```text
skill list [--domain] [--category <c>] [--tag <t>] [--preset <id>] [--agent <key>]
           [--deployed-to <target>] [--no-tag] [--json]      # 清单与组合筛选 (FR-009)
skill get <ref> [--json]                                      # 详情：正文/译文/来源/文件清单/部署 (FR-010)
skill read <ref> [--lang zh|orig]                             # 正文阅读（有译文默认中文）
skill create <ref> [--category] [--comment] [--tags t1,t2]    # 自建创建，不部署 (FR-015)
skill set <ref> [--comment ...] [--tags ...] [--agents ...] [--oses ...]  # 管理资料 (FR-005)
skill tag rename <old> <new> | skill tag remove <tag>         # 全库标签（两库生效）(FR-009)
skill move <ref> --category <c|--none>                        # 改分类=移目录 (FR-003)
skill copy <ref> --to <domain> [--category]                   # 库间复制，保留原件 (US2-7)
skill delete <ref>                                            # 删除并清理受管部署 (FR-022)
skill import <repo-url> [--domain] [--category] [--skills s1,s2]   # 上游入库，流式进度，可 Ctrl-C (FR-011~013)
skill adopt <agent-key> <name>... [--domain] [--category] [--comment] [--tags]  # 收编并接管 (FR-014)
skill adopt-hermes <profile> <name>...                        # 档案独有收编→龙虾库+default (FR-028)
skill check [--repo <url>] | --all [--json]                   # 上游检查，只展示 (FR-029/030)
skill update <ref> [--to <version>]                           # 单项更新 (FR-031)
skill update-repo <repo-url> [--include-new s1,s2]            # 仓库批量更新+选定新增 (FR-031/033)
skill skip <ref> [--version <v>] | skill unskip <ref> | skill skipped [--json]   # 跳过管理 (FR-034)
```

### deploy — 部署与同步

```text
deploy to <target> <ref>... [--dry-run]                       # 部署（合成副本）(FR-016~018, FR-021)
deploy preset <preset-id> --to <target> [--dry-run]           # 预设叠加部署 (FR-024)
deploy undo <target> <ref>... [--dry-run]                     # 撤销，库内保留 (FR-021)
deploy undo-preset <preset-id> --from <target>                # 按成员撤销 (FR-024)
deploy sync [--ref <ref>]                                     # 库变化→已有部署同步（Agent 改库后必调）(FR-019)
deploy status [--target] [--paused] [--json]                  # 部署记录/paused 本地变更 (FR-020)
```

### preset — 预设

```text
preset list [--domain] [--json] | preset get <domain> <id>
preset create <domain> <name> [--icon] | preset delete <domain> <id>
preset members <domain> <id> --add n1,n2 | --remove n1,n2     # 只改定义，不动部署 (FR-024)
preset rename <domain> <id> <new> | preset icon <domain> <id> <icon>
preset reorder <domain> --order id1,id2
```

### hermes — 档案

```text
hermes profiles [--json]                                      # 档案列表 (FR-026)
hermes list <profile> [--json]                                # 档案技能与启用状态
hermes enable <profile> <name>... | hermes disable <profile> <name>...   # 明确启用/禁用+批量 (FR-026/028)
```

### tool — 工具中心

```text
tool list [--status installed|not-installed|updatable|failed|unchecked|no-guide] [--category] [--json]  # (FR-045)
tool get <slug> [--json]                                      # 详情四区数据 (FR-045)
tool check <slug> | tool check --all [--json]                 # 执行登记的检查命令并对比 (FR-046)
tool register --file <tool.yaml+guide.md 目录|--stdin         # Agent 登记共享资料 (FR-047)
tool set-install <slug> --version <v> [--method] [--path] [--config-path]   # Agent 回写本机记录 (FR-047)
tool task <slug> [--action install|update|guide] [--requirement <text>]     # 生成维护任务文本 (FR-047)
tool skill-link <slug> <ref>                                  # 登记配套 Skill 关联 (FR-049)
```

### sync — SkillVault 跨机同步

```text
sync pull [--json]                                            # 拉取；无冲突静默应用并刷新部署 (FR-037)
sync push [--message <m>]                                     # 提交推送工作区全部改动（获同意后）(FR-038)
sync status [--json]                                          # 远端连接、未提交改动、未拉取提交 (FR-040)
sync conflicts [--json]                                       # 冲突详情（Agent 检查用）(FR-039)
```

### agent — 本机 Agent 与指令

```text
agent list [--json] | agent detect [--json]                   # 检测，不启动不部署 (FR-041)
agent enable <key> | agent disable <key> | agent reorder --order k1,k2
agent set-dir <key> --skills <path> | --project-rel <path> | --instructions <path>   # (FR-041/043)
agent local-skills <key> [--json]                             # 未受管本地技能浏览 (FR-042)
agent delete-local <key> <name>...                            # 删除未受管本地技能 (FR-042)
instruction get [global|<key>] [--json]                       # 指令源读取 (FR-043)
instruction set [global|<key>] --file <f> | --stdin           # 保存源并按"全局+专属"拼接分发 (FR-043)
```

### 输出约定

- 默认人类可读（表格/要点）；`--json` 输出稳定 schema（结果、数据、错误码），Agent 优先使用；
- 检查类命令（`skill check`、`tool check`、`sync status`、`deploy --dry-run`）绝不修改状态；
- 所有"应用类"命令先校验管理域边界与支持声明（FR-017），不适用项在输出中列明并跳过，不静默失败；
- 错误输出含可读原因与涉及的技能/目标，退出码非零。

## FR-050 覆盖对照

| 规格要求 | 命令 |
|---|---|
| 入库 | `skill import` |
| 收编（Agent/档案） | `skill adopt` / `skill adopt-hermes` |
| 上游检查与更新 | `skill check` / `skill update` / `skill update-repo` / `skill skip` |
| 部署同步 | `deploy sync` / `deploy to/undo` / `deploy status` |
| 预设 | `preset *` / `deploy preset/undo-preset` |
| 标签 | `skill set --tags` / `skill tag rename/remove` / `skill list --tag` |
| 分类 | `skill move` / `skill create/import --category` |
| 注释 | `skill set --comment` |
| 支持范围 | `skill set --agents/--oses` |
| Hermes 管理 | `hermes *` |
| 工具：查看清单与检查状态 | `tool list/get` |
| 工具：单项/全部检查 | `tool check` |
| 工具：登记与修改资料 | `tool register` / `tool skill-link` |
| 工具：回写本机安装记录 | `tool set-install` |
| 工具：生成维护任务 | `tool task` |
| 库同步 | `sync *` |
| Agent 配置与指令 | `agent *` / `instruction *` |

**覆盖结论**：FR-050 列举的全部操作均有对应命令，无 GUI 独占操作。
