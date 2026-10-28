# Agent 接入资料

## 可直接使用的数据

[agent-catalog.json](agent-catalog.json) 保存 53 项参考记录：47 项 code、6 项 lobster。
这是参考项目登记的数量，不代表已在当前电脑检测到 53 项安装，也不限制以后扩展。
名称、技能／检测目录与项目路径已逐项对照参考源码中的字面量。

| 字段 | 含义 |
|---|---|
| key / displayName | 参考标识与原文名称；新程序的标识命名由自身设计确定 |
| referenceDomain | 旧登记中的管理域，用于接入线索，不是按名称猜测的支持声明 |
| skillPath | 相对用户 home 的技能目录 |
| detectPath | 相对 home 的安装检测线索；与技能目录分开 |
| projectSkillPath | 原项目级路径覆盖资料；null 表示没有专门覆盖，不授权项目专属管理功能 |
| additionalScanPaths | 旧实现额外发现目录，不自动作为部署地址 |
| recursiveScan | 旧目录扫描方式的观察资料 |
| icon / invertInDarkTheme | 本目录内的图标路径及旧图标深色显示信息 |

图标位于 [assets/agent-icons](assets/agent-icons/)，共 50 个；QClaw、EasyClaw、AutoClaw 没有对应图标文件，记录为 null，不伪造图标。
Codex、Roo Code 的旧单色图标使用深色反相；其他图标可能自带主题适配，避免一概反相。

## 实施时值得注意的差异

- 检测目录不等于技能目录。例如 Replit 的检测线索是 `.replit`，技能目录是 `.config/agents/skills`。
- OMP 与 Pi 的用户级路径含 `agent` 段，项目级记录不含该段，不能靠统一字符串替换推出全部路径。
- Cline 与 Warp 登记到 `.agents/skills`；Amp 与 Replit 登记到 `.config/agents/skills`。
  不同 Agent 可能指向同一物理目录，不能从名称不同就推断为两个目录。
- WorkBuddy 登记的是 `.workbuddy/skills`，不把供应方 marketplace 缓存当成用户部署目录。
- DeepSeek Harness 的参考实现还提到 `DSH_HOME`、`DSH_AGENTS_HOME`；静态表不是环境变量解析器。
- `.config` 路径在 Windows 的候选解析需结合真实 Agent 行为；macOS 不能直接将所有 CLI 路径替换成
  `Library/Application Support`。静态数据不替代逐平台验证。
- Hermes 的静态 `.hermes` 记录不适合作为所有平台的最终路径，见 [Hermes 说明](hermes-integration.md)。
- Antigravity 官方已把 App／IDE 全局技能目录改为 `~/.gemini/config/skills`（legacy `~/.gemini/antigravity/skills` 仍兼容）；CLI 全局目录 `~/.gemini/antigravity-cli/skills` 未纳入部署目标。目录中该条目按官方现行路径登记，不再对应参考源码字面量（2026-10 核对官方文档）。

共享技能目录可单独表示为部署目标，不混入 53 个真实 Agent 的安装数量。
设置中的启用状态、用户自定目录及实际检测结果属于程序运行资料，不写死到这份参考清单。

## 来源

参考源码：`src-tauri/src/core/tool_adapters.rs`、`src/lib/agentIcons.ts`、`public/agent-icons/`。
抽取日期及版本见[索引](README.md)。没有保留旧适配器的数据库键、域推导、默认启用或自定义注册实现。
