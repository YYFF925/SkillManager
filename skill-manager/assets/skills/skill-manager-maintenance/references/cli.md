# Skill Manager CLI 参考

本文件是配套维护 Skill 的 CLI（命令行接口）参考，与当前实现一致。
可执行文件为 `skill-manager`（项目仓库 `skill-manager/target/debug/skill-manager` 或安装后的同名声）。

## 全局约定

所有子命令都接受：

| 选项 | 含义 |
|---|---|
| `--vault <path>` | SkillVault 根目录（共享内容真值）。未给时用本机记住的库 |
| `--state-dir <path>` | 本机状态目录（隔离 SQLite、检查缓存与日志），用于双机模拟 |
| `--json` | 输出结构化 JSON（`result` / `data` / `errors`） |
| `-h, --help` | 帮助 |
| `-V, --version` | 版本 |

**输出与退出码**

- 默认中文人类可读文本；错误以 `未完成：…` 写到标准错误。
- `--json`：统一对象 `{ "result": "success|partial|failed", "data": …, "errors": [] }`；
  批量逐项给出对象、结果与原因，`errors[]` 项含 `message` 与可选的 `skill` / `target` / `path`。
- 退出码：`0` 成功；`1` 失败；`2` 部分完成（部分成功、部分失败）。
  **参数用法错误（clap）也返回 `2`**，用提示前缀 `error:`（用法错误）与 `未完成：`（业务）区分。

**参数书写约定**

- `<domain>` = `code|lobster`；`<ref>` = `<domain>/<name>`；`<target>` = Agent key、`shared` 或 `hermes:default`。
- `a | b` 表示互斥参数；`…` 表示可重复条目；未给的编辑参数表示**保持原值**，不表示清空。
- 多个位置参数（如 `<ref>...`、`<name>...`）在 shell 中需各自作为独立参数传入。

## skill — 技能与库管理

```text
skill list [--domain <domain>] [--query <text>] [--category <c> | --uncategorized]
           [--tag <t>...] [--preset <id>] [--agent <key>] [--deployed-to <target>] [--no-tag]
skill get <ref>
skill read <ref> [--lang zh|orig]
skill set <ref> [--comment <text>] [--tags <csv>] [--agents all|none|<csv>] [--oses all|none|<csv>]
skill tag add <ref>... --tags <csv>
skill tag detach <ref>... --tags <csv>
skill tag rename <old> <new>
skill tag remove <tag>
skill move <ref> (--category <c> | --uncategorized)
skill delete <ref>...
skill translation get <ref>
skill translation set <ref> (--file <path> | --stdin)
skill translation remove <ref>
skill overlay list <ref>
skill overlay get <ref> --path <relative-path> [--agent <key>] [--os <os>]
skill overlay set <ref> --path <relative-path> (--file <path> | --stdin) [--agent <key>] [--os <os>]
skill overlay remove <ref> --path <relative-path> [--agent <key>] [--os <os>]
skill task <ref>... [--requirement <text>]
```

- `--preset` 须同时给 `--domain`（避免跨域同 id 歧义）；否则报错退出 1。
- `--deployed-to` 按本机部署记录筛选；`skill get` 的 `deployments` 列出该技能的实际部署位置。
- 需要共享目录部署时，先用 `settings set-shared-dir --path <绝对路径>` 设置（见 `settings` 一节）；
  未设置时 `deploy to shared <ref>` 报“尚未设置共享目录路径”。
- `--agents`：`all` 删除限制键（= 全支持）、`none` 写显式空集合、其余为逗号分隔名单；
  `--oses` 同规则。`--tags` 是替换集合，空字符串清空。`--comment` 空字符串清除。
- `tag add/detach` 只增减目标标签（不替换）；`tag rename/remove` 两域统一治理，`remove` 只解除关联。
- `move` 必须给 `--category` 或 `--uncategorized` 之一；不改技能名。
- `translation set` / `overlay set` 必须给 `--file` 或 `--stdin` 之一，否则报错退出 1。
- overlay 的 `--agent` / `--os` 省略即该维度通用（`_`）。
- `task` 生成可复制的维护说明（含配套维护 Skill 库内路径与“按其规范执行”）；不改库、不部署、不上传。
  `--requirement` 可省略，省略时任务文本如实写明“未填写”。

## skill — 获取与上游

```text
skill preview <repo-url> [--ref <branch-or-tag>]
skill import <repo-url> --domain <domain> --skills <subpath>... [--ref <branch-or-tag>]
             [--category <c> | --uncategorized]
skill adopt <agent-key> <name>... [--category <c> | --uncategorized]
            [--comment <text>] [--tags <csv>] [--metadata-file <path>]
skill adopt-hermes <profile> <name>...
skill create <ref> [--category <c> | --uncategorized] [--comment <text>] [--tags <csv>]
skill copy <ref> --to <domain> [--category <c> | --uncategorized]
skill check [<ref>] [--repo <url>] [--all]
skill update <ref> [--to <commit-oid>]
skill update-repo <repo-url> [--include-new <subpath>...] [--domain <domain>]
                  [--category <c> | --uncategorized]
skill reject-new <repo-url> --domain <domain> --skills <subpath>...
skill rejected [--repo <url>] [--domain <domain>]
skill skip <ref> [--version <commit-oid>]
skill unskip <ref>
skill skipped
```

- `import --skills` 支持逗号分隔与重复给出；`--domain` 必填。
- `check` 必须给 `<ref>`、`--repo`、`--all` 之一，否则报错退出 1。
- `preview`/`import` 的真实 Git 进度写标准错误，最终结果写标准输出；Ctrl-C 取消本次获取
  （返回 `failed` 并说明“后续不继续该获取”，不误报入库）。
- `--to` / `--version` 缺省采用最近检查所得的具体远端提交；尚无检查结果会提示先检查。
- `update-repo` 只处理两域该仓库**未采用、未跳过**的已有条目；新增只处理 `--include-new` 显式选定项。
- `adopt` 管理域由 Agent 所属域决定；批量不同资料用 `--metadata-file` 按技能名给各自资料。
- 已知限制：`import` / `update-repo` 没有 `--tags` / `--comment` 参数，新增候选入库时标签与注释
  继承上游 `.manager`，只能补充分类（GUI 与 CLI 一致）。入库后再用 `skill set` 补充标签/注释。

## deploy — 部署与同步

```text
deploy to <target> <ref>... [--also-target <target>...] [--dry-run]
deploy preset <domain> <preset-id> --to <target> [--also-target <target>...] [--dry-run]
deploy undo <target> <ref>... [--also-target <target>...] [--dry-run] [--discard-local-changes]
deploy undo-preset <domain> <preset-id> --from <target> [--also-target <target>...] [--dry-run]
deploy sync [--ref <ref>] [--target <target>] [--overwrite-local]
deploy status [--target <target>] [--paused]
```

- `--also-target` 可重复；未给则只处理一个目标。
- `--dry-run` 只报告技能/目标/实际路径/预期结果，不修改目标、库内容或业务状态。
- 遇受管副本外改：普通部署/同步暂停该项（`paused_local_change`），其余继续。
  用户裁决后按库覆盖用 `sync --ref <ref> --target <target> --overwrite-local`；
  确认丢弃待清理副本用 `undo ... --discard-local-changes`。
- `deploy preset` 按当前成员**叠加**部署；`undo-preset` 按成员逐个撤销（缺失成员报告后其余继续），
  都不追踪来源预设或引用计数。

## preset — 预设

```text
preset list [--domain <domain>]
preset get <domain> <id>
preset create <domain> <name> [--icon <icon>]
preset delete <domain> <id>
preset members <domain> <id> (--add <csv> | --remove <csv>)
preset rename <domain> <id> <new>
preset icon <domain> <id> <icon>
preset reorder <domain> --order <id,...>
preset reorder-members <domain> <id> --order <name,...>
```

- `id` 新建时由名称派生并在域内去重；`icon` 传空字符串清除图标。
- `reorder --order` 须是该域全部预设 id 的一个排列；`reorder-members --order` 须是该预设全部成员名的一个排列。
- 定义编辑不动部署，两种排序也不改变成员集合、分类目录或部署。

## agent / instruction — 本机配置与指令

```text
agent list | agent detect
agent enable <key> | agent disable <key> | agent reorder --order <key,...>
agent set-dir <key> (--skills <path> | --project-rel <path> | --instructions <path>)
agent reset-dir <key> --kind skills|project-rel|instructions
agent local-skills <target> | agent delete-local <target> <name>...
instruction get global|<key>
instruction set global|<key> (--file <path> | --stdin)
```

- `agent` 的目录/启停/顺序只写本机状态；`detect` 只读目录存在性，不启动、不部署。
- `local-skills` / `delete-local` 的 `<target>` 接受内置 Agent key 或 `shared`（共享目录是编码 Agent 的本机部署目标）：
  按内置 Agent 的同一套语义列出与处理——受管项（存在该目标部署记录）走撤销部署规则、不删库；
  未受管项直接删除实际文件、不改库。共享目录未设置路径时如实报错“尚未设置共享目录路径”，不猜测默认路径。
- `instruction get` 返回源内容与按“全局在前、专属在后”的合成预览；
  `set` 写入源后立即覆盖本机已启用目标的 `<detect_dir>/AGENTS.md`（可用 `--instructions` 覆盖路径），
  空内容可正常保存并覆盖；单目标写入失败不中断其余并逐项报告。

## hermes — 档案

```text
hermes profiles
hermes list <profile> [--query <text>] [--category <c> | --uncategorized] [--tag <t>...] [--no-tag]
hermes enable <profile> <name>...
hermes disable <profile> <name>...
```

`default` 档案即部署记录（`hermes:default`）；其他档案的启用是本机状态，内容更新不重置。

## tool — 工具中心

```text
tool list [--query <text>] [--category <c>]
          [--status installed|not-installed|updatable|latest|failed|unchecked|no-command|no-guide]
tool get <slug>
tool check [<slug>] [--all]
tool register (--dir <path> | --stdin)
tool set <slug> (--dir <path> | --stdin)
tool set-install <slug> --version <v> [--method <text>] [--path <path>] [--config-path <path>]
tool task [<slug>] --action register|edit|install|update|guide [--requirement <text>]
tool skill-link <slug> <ref>
```

- `register` 新登记、`set` 修改已有共享资料；已登记时 `register` 报错。
- `set-install --version ""`（空字符串）表示已核实该工具不在本机：清除安装记录并重置检查结果；
  正常回写也会重置该工具检查结果。
- `tool check` 只写检查结果，不改安装事实；未安装工具取得上游版本后仍显示未安装。
- `tool task --action register` 允许省略 `<slug>`（用 `--requirement` 说明名称与来源）；其他动作必须给已登记 slug。

## sync — SkillVault 跨机同步

```text
sync pull
sync push [--message <text>]
sync status
sync conflicts
```

- `sync pull` 无冲突静默应用并刷新已有部署/已启用指令目标；**新增技能不自动部署**；冲突时暂停本次同步并保留现场。
- `sync push` 提交并推送**整个工作区**改动；须先获得用户明确同意，CLI 写库/`deploy sync`/文件监听/定时同步都不构成授权。
- `sync status` 只读事实（连接/未提交/未拉取/冲突），不提交、不推送、不改库。
- `sync conflicts` 输出未合并路径与状态供你在 Git 层处理，之后再次 `sync pull` 继续。

## settings — 本机运行配置与只读报告

```text
settings get [<key>...]
settings set-shared-dir (--path <绝对路径> | --clear)
settings set-auto-check <0|30|60|360|1440>
settings diagnostics [--export]
settings capacity
```

- 只读写**本机状态**，不改 SkillVault 内容、不触发上传。
- `settings get` 不带参数返回全部键的当前生效值：`theme`、`font_scale`、`auto_check_minutes`、
  `shared_dir`、`git_auto_push`、`vault_path`、`last_sync_at`、`last_check_at`；带参数只返回指定键。
  **未设置的键返回 `null`**（不省略键、不用空字符串冒充）；已套用默认值：主题 `system`、字号 `default`、
  自动检查频率 `60`。
- **只有两个写入口**：
  - `settings set-shared-dir --path <绝对路径>` 设置共享目录（`shared` 部署目标前置）；`--clear` 清除。
  - `settings set-auto-check <0|30|60|360|1440>` 设置 FR-029 三项共用的自动检查周期（`0` = 关闭）。
    其他取值被拒绝并列出合法值。
- **不提供写入口**（只能经 `settings get` 读取）：`theme`、`font_scale`（界面偏好，在 GUI 设置页调整）、
  `git_auto_push`（GUI 保存自动上传的长期授权记录，Agent 不得自行开启；上传仍需每次明确同意后 `sync push`）。
- **无组合调度命令**：手动检查分别用 `skill check --all`、`sync pull`、`tool check --all` 表达，不合并成一条。
- `settings diagnostics` 输出诊断报告与运行日志/崩溃记录的**真实文件路径**；`--export` 追加写出文件并返回其路径；
  不提供上传诊断。
- `settings capacity` 输出库总占用与较大技能排行；空库返回 0 与空排行。与 GUI 设置页共用同一 core。
- Hermes 根不在这里设置：按 `HERMES_HOME` 环境变量与平台默认解析（`hermes profiles` 报出实际根）。
