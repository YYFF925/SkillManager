# Skill Manager

当前处于规划阶段：正式产品规格见 [specs/002-skill-management/spec.md](specs/002-skill-management/spec.md)，实现规划（plan/research/data-model/contracts/quickstart）已就绪，下一步拆分开发任务（speckit-tasks）。[需求记录](docs/requirements.md)是规格的需求依据与访谈历史；[裁决清单](docs/legacy-skill-capabilities.md)记录旧版能力盘点及当前产品实现的暂定范围，不能整表当作规范。

当前先按[技能管理整体业务流程](docs/requirements.md#技能管理的整体业务流程访谈整理)理解产品，再阅读逐项访谈记录。该流程是已有结论的整理；工具中心需求已确认并完成澄清（资料随 SkillVault 共享、安装记录本机独立、版本检查为 Agent 编写的检查命令、三项检查共用一个自动检查频率、登记与回写均由 Agent 经 CLI 完成），结论见需求记录末尾与 002 规格的 Clarifications。

设计和开发前请先阅读[裁决的解释与调整规则](docs/requirements.md#裁决的解释与调整规则)：核心管理要求是后续 Spec 的依据；实现细节可以按实际需要调整，不能为了暂定范围而绕路或妨碍核心要求的实现。


当前提供[技能管理交互原型](specs/001-skill-management-prototype/ui-prototype/index.html)，沿用旧界面风格，可直接打开并点击体验完整模拟流程。模拟状态由浏览器本地保存，不访问真实技能库或 Agent 目录；使用说明见[原型说明](specs/001-skill-management-prototype/ui-prototype/README.md)。


接入数据、视觉素材与平台经验已整理到本项目的[技术参考](docs/technical-reference/README.md)，按实现任务查阅；这些资料不继承旧产品规则。
