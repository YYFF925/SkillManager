# Skill Manager（正式产品实现）

Skill Manager 是本地优先的技能库管理器：以用户自己的 **SkillManagerVault**（Git 仓库）为唯一内容真值源，
Rust 核心承载库管理、部署合成、上游更新、跨机同步与工具中心；GUI（Tauri 2 + React/TypeScript）与
CLI（clap，面向 Agent 的主要入口）共用同一核心；本机状态（部署记录、档案启用、Agent 配置、设置、
工具安装记录、检查事实）存 SQLite。

规格与验收依据：[../specs/002-skill-management/spec.md](../specs/002-skill-management/spec.md)；
命令契约：[../specs/002-skill-management/contracts/cli.md](../specs/002-skill-management/contracts/cli.md)；
库文件格式与本机状态模型：[../specs/002-skill-management/data-model.md](../specs/002-skill-management/data-model.md)；
按里程碑的验收场景：[../specs/002-skill-management/quickstart.md](../specs/002-skill-management/quickstart.md)。

## 目录结构

```text
skill-manager/
├── Cargo.toml                 # workspace 根（core / cli / desktop 三成员）
├── crates/core/               # 业务核心（无 UI 依赖）：库扫描、.manager、部署合成、
│   └── src/                   #   上游更新、同步、Hermes、预设、工具中心、SQLite 本机状态
├── crates/cli/                # clap 命令行入口（Agent 主要用户）
├── src-tauri/                 # Tauri 2 壳：command 层薄封装 core，无业务逻辑；托盘、诊断、调度
├── src/                       # 前端（React + TypeScript + Vite + Tailwind）
│   ├── pages/                 #   技能库（code/lobster）、预设、跨目标部署清单（「全部编码 Agents」/「全部龙虾
│   │                          #   Agent」按技能 × 目标列出本机部署记录）、Hermes、工具中心、设置（首个区块为
│   │                          #   Agent 检测与启停排序、目录行内编辑；另有「共享 Agent」页列共享目录技能）
│   ├── components/            #   详情、弹窗、部署/入库/上游、诊断与容量面板
│   └── services/client.ts     #   Tauri command 的类型化调用封装
├── assets/skills/             # 配套维护 Skill 源文件（见下）
└── tests/                     # CLI 集成测试（与 crates/cli 注册共用根 tests/）
```

业务规则只存在于 `crates/core`；CLI 与 Tauri command 都是薄入口，保证同一套规则、不形成两套体系。

## 开发与构建

前置：Node 与 npm、Rust 工具链、本机 `git`（仓库操作调用本机 git，不引入 libgit2）。

```bash
npm install                # 前端依赖（首次）

# 测试与检查（全部在 skill-manager/ 下运行）
cargo test --workspace     # Rust：core 单元测试 + CLI/核心集成测试
cargo build --workspace    # 构建 core / cli / desktop
npx tsc --noEmit           # 前端类型检查
npm test                   # 前端 vitest
npm run build              # 前端生产构建（tsc + vite build）

# 运行
npm run tauri dev          # 启动桌面应用（开发模式；默认窗口 1720×1040）
cargo run -p skill-manager-cli -- --help   # CLI
```

Tauri 打包：`npm run tauri build`（`bundle.targets` 为 `app`）。
macOS 本机打包使用 ad hoc（临时签名，`bundle.macOS.signingIdentity = "-"`），
使签名标识与应用标识 `com.fei.skillmanager` 一致。实际验证中，未绑定应用信息的链接器签名下，
系统本地网络开关虽已开启，GUI 的 Git SSH 仍报 `No route to host`；应用包签名并重启后连接成功。

## CLI 入口

```bash
# 全局选项（所有子命令可用）
skill-manager --vault <SkillManagerVault 路径> --state-dir <本机状态目录> [--json] <命令...>

# 示例
skill-manager skill list --domain code --query 依赖 --tag 检查
skill-manager skill set code/dep-guard --comment 检查依赖 --tags 依赖,安全
skill-manager skill import <repo-url> --domain code --skills <subpath>
skill-manager skill check --all
skill-manager deploy to codex code/dep-guard --also-target zcode
skill-manager deploy sync
skill-manager sync push --message "上传本周维护"
skill-manager tool check --all
skill-manager settings get
skill-manager settings get shared_dir
skill-manager settings set-auto-check 30
skill-manager settings diagnostics --export
skill-manager settings capacity
skill-manager agent local-skills shared
skill-manager agent read-local shared <name>
```

- `<ref>` = `<domain>/<name>`（**不含分类路径**；技能名在域内唯一，分类只影响目录位置）。
- `<target>` = 内置 Agent key、`shared`（共享目录，编码 Agent 的本机部署目标）或 `hermes:default`（Hermes default 档案）；
  `agent local-skills <target>` / `agent delete-local <target> <name>...` 接受以上三类取值，
  受管项走撤销规则、未受管项直接删本机文件；共享目录自动定位为当前用户主目录下的 `.agents/skills`。
- 本机技能卡片可点击阅读实际副本的正文与文件；未受管项可「纳入管理」，共享目录收编使用
  `skill adopt shared <name>`，进入编码库并接管原共享部署，不新增其他目标部署。
- 退出码：`0` 成功；`1` 失败；`2` 部分完成（参数用法错误也返回 `2`）。
- `--json` 输出统一对象 `{result, data, errors}`。
- `settings` 命令组只读写本机状态：`get` 读全部运行配置与调度事实（未设置的键为 `null`）；
  **只有一个写入口**——`set-auto-check`（FR-029 三项共用周期）；共享目录自动识别且只读，不保存路径设置；
  `theme`/`font_scale` 仅 GUI 写（界面偏好，FR-052）、`git_auto_push` 仅 GUI 写（长期上传授权，FR-038），
  三者经 CLI 只读；`diagnostics`/`capacity` 与设置页共用同一 core，`--export` 返回真实文件路径。
- 完整命令表、参数与语义以 [contracts/cli.md](../specs/002-skill-management/contracts/cli.md) 为准。

## SkillManagerVault 与本机状态位置

- **SkillManagerVault（共享内容真值）**：用户已有的 Git 仓库，含 `code/`、`lobster/` 两管理域、
  `presets/`、`instructions/`、`tools/`、`upstream-decisions.yaml`；首次接入在 GUI 选择目录，
  之后本机记住路径；CLI 用 `--vault` 指定。所有共享资产都是库内可直接读写的文件，随库 Git 同步。
- **本机状态（不随库同步）**：SQLite `skill-manager.db`，默认位于系统的本地数据目录下的
  `SkillManager/`（macOS：`~/Library/Application Support/SkillManager/`）；
  运行日志与崩溃记录在其 `logs/` 子目录。CLI 用 `--state-dir` 覆盖该目录，
  配合同一库的两个克隆即可在一台机器上模拟两台电脑。

## 配套维护 Skill

`assets/skills/skill-manager-maintenance/` 是**单一**配套维护 Skill（库维护规范 + CLI 使用参考 +
工具安装登记流程合一，见 research.md D9）。它按 FR-051 作为**普通上游技能**入库：
本项目推送到用户 Gitea 远端后，以该项目远端为上游仓库、以
`skill-manager/assets/skills/skill-manager-maintenance` 为来源子路径执行 `skill import`；
后续更新走既有 `skill check` / `skill update`，不新增专用分发机制。
**远端写入（提交/推送）未获用户授权时，Gitea 路径的入库/更新保持待授权。**
软件生成的维护任务/工具任务引用该技能的真实库内路径与"按其规范执行"，未入库时如实显示"未入库"。

## 已知缺口与未验证项

1. **新增候选入库**（`skill import` / `skill update-repo`）无 `tags`/`comment` 参数，
   GUI 与 CLI 一致地只能补充分类，标签/注释继承上游 `.manager`；入库后可用 `skill set` 补充。
2. **Hermes 档案独有技能**没有只读查看正文的入口（无对应命令）。
3. **平台兼容**：本机仅在 macOS（arm64）实际运行验证；其他平台未验证，不声明兼容。
   Hermes 启用用符号链接实现，Windows 下的链接行为未验证。
4. **未取得人工界面证据的交互**：卡片三点菜单/多选批量栏/入库与部署弹窗的部分点按流程、
   "未连接"的完整界面复核等，按各期报告如实保留。

以上与更多细节见 [contracts/cli.md 的 T090 双入口核对](../specs/002-skill-management/contracts/cli.md#t090-双入口核对fr-0500512026-10-15)
与各期报告。
