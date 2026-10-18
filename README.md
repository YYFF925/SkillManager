# Skill Manager

正式产品代码位于 [skill-manager/](skill-manager/)（Rust workspace + Tauri 2 + React/TypeScript），开发、构建、测试与 CLI 入口见
[skill-manager/README.md](skill-manager/README.md)。产品行为与验收依据是
[specs/002-skill-management/spec.md](specs/002-skill-management/spec.md)；各期结果见
`specs/002-skill-management/checklists/m1-report.md`～`m5-report.md`。

**当前状态（2026-10-17）**：T001～T129 已实施，`cargo test --workspace` 270 项、
`npm test` 54 项通过，类型检查、前端构建与 macOS `.app` 构建通过。
本轮补齐统一检查、部署/同步部分完成结果、Hermes 只读阅读、工具候选来源预览、组合筛选及诊断。
大仓库真实 GUI（图形界面）的进度、取消、失败与成功路径已验收；维护 Skill（技能）已发布到
Gitea（代码托管服务），并在隔离验收库完成真实来源入库与一次普通版本更新。
**这不等于全部验收通过**：其他平台与部分人工交互证据仍按各期报告保留“未验证”；
新增候选入库无 tags/comment 参数的现有契约边界见
[CLI 双入口核对](specs/002-skill-management/contracts/cli.md#t090-双入口核对fr-0500512026-10-15)，
本轮验收结果见 [M5 报告第十二节](specs/002-skill-management/checklists/m5-report.md#十二phase-18-查漏补缺实施与验收2026-10-17)。

**Windows 打包（2026-10-17）**：新增平台覆盖配置 `skill-manager/src-tauri/tauri.windows.conf.json`
（target 改为 `nsis`），本机按 `cargo test` 之外的全部流程打包成功，
产物 `target/release/bundle/nsis/Skill Manager_0.1.0_x64-setup.exe`，GUI 已在本机启动冒烟通过。
同时修掉本地路径来源被当 URL 切分的 Windows 缺陷（`skill import C:\...` 报“分类名不合法”），
并在 Windows 上阻止会写真实用户目录的集成测试。Windows 行为仍不作兼容声明，依据与限制见
[skill-manager/README.md](skill-manager/README.md)。

**CLI 安装交付修复（2026-10-18）**：桌面包现随附同版本 CLI（命令行工具），维护 Skill 已说明安装位置与绝对路径调用。
macOS arm64 的新 DMG 位于 `release/`，已从包内验证版本、帮助、隔离库读写与签名；Windows 的本轮打包／安装尚未实测。
对应 FR-050/051、US10-5，结果见 [M6 报告第二十节](specs/002-skill-management/checklists/m6-report.md#二十桌面安装包随附-cli2026-10-18)。

**初始统一库（2026-10-16）**：名称统一为 **SkillManagerVault（统一技能库）**；本机目录为
`/Users/fei/Projects/workspace/SkillManagerVault`，对应 [Gitea 私有仓库](http://192.168.200.7:3000/yfpoyuas/SkillManagerVault)。
按正式数据模型初始化两管理域、空预设、空指令源与工具目录，不携带测试数据；通过既有 `--vault` 接入，
后续沿用 `sync status` / `sync pull` / 经同意的 `sync push`。本机状态和认证不进入库；
配套维护 Skill 的项目上游已发布；本轮验收使用隔离临时库，初始统一库可按需要显式选择该普通来源入库。

理解需求时先读正式规格的[业务关系与阅读示例](specs/002-skill-management/spec.md#业务关系与阅读示例)及[术语表](GLOSSARY.md)：区分同一个 SkillManagerVault 的跨机同步、技能上游更新和本机部署。[需求记录](docs/requirements.md)是规格的需求依据与访谈历史；[裁决清单](docs/legacy-skill-capabilities.md)记录旧版能力盘点及当前产品实现的暂定范围，不能整表当作规范。

[需求质检](specs/002-skill-management/checklists/readiness.md)已逐项评审并完成本轮修订；跨文档一致性分析已完成，无阻断项。

当前先按[技能管理整体业务流程](docs/requirements.md#技能管理的整体业务流程访谈整理)理解产品，再阅读逐项访谈记录。该流程是已有结论的整理；工具中心需求已确认并完成澄清（资料随 SkillManagerVault 共享、安装记录本机独立、版本检查为 Agent 编写的检查命令、三项检查共用一个自动检查频率、登记与回写均由 Agent 经 CLI 完成），结论见需求记录末尾与 002 规格的 Clarifications。

设计和开发前请先阅读[裁决的解释与调整规则](docs/requirements.md#裁决的解释与调整规则)：核心管理要求是后续 Spec 的依据；实现细节可以按实际需要调整，不能为了暂定范围而绕路或妨碍核心要求的实现。


当前提供[技能管理交互原型](specs/001-skill-management-prototype/ui-prototype/index.html)，沿用旧界面风格，可直接打开并点击体验完整模拟流程。模拟状态由浏览器本地保存，不访问真实技能库或 Agent 目录；使用说明见[原型说明](specs/001-skill-management-prototype/ui-prototype/README.md)。

正式实施须按[原型页面与效果图索引](specs/001-skill-management-prototype/ui-prototype/README.md)、[视觉说明](specs/001-skill-management-prototype/visual-reference.md)与[交互细节](specs/001-skill-management-prototype/ui-interaction-audit.md)还原最终界面；各期执行[视觉与交互对照](specs/002-skill-management/quickstart.md#gui-视觉与交互对照各期适用)，记录实际截图和差异。原型的视觉与交互是验收基线，正式业务规则以 002 规格为准。


接入数据、视觉素材与平台经验已整理到本项目的[技术参考](docs/technical-reference/README.md)，按实现任务查阅；这些资料不继承旧产品规则。
