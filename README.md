# Skill Manager

当前已完成规划与需求复核：正式产品规格见 [specs/002-skill-management/spec.md](specs/002-skill-management/spec.md)，实现规划与[开发任务](specs/002-skill-management/tasks.md)已就绪，[需求质检](specs/002-skill-management/checklists/readiness.md)已逐项评审并完成本轮修订；跨文档一致性分析已完成，无阻断项，下一步按 M1～M5 依赖顺序实施，全部完成验收后投入使用。[需求记录](docs/requirements.md)是规格的需求依据与访谈历史；[裁决清单](docs/legacy-skill-capabilities.md)记录旧版能力盘点及当前产品实现的暂定范围，不能整表当作规范。

理解需求时先读正式规格的[业务关系与阅读示例](specs/002-skill-management/spec.md#业务关系与阅读示例)及[术语表](GLOSSARY.md)：区分同一个 SkillVault 的跨机同步、技能上游更新和本机部署。现行资料已经主代理与两个不继承对话上下文的子代理独立建模、交叉讨论并复核修订；过程及覆盖范围见[需求质检的三方复核记录](specs/002-skill-management/checklists/readiness.md#三方独立理解复核2026-10-15)。

当前先按[技能管理整体业务流程](docs/requirements.md#技能管理的整体业务流程访谈整理)理解产品，再阅读逐项访谈记录。该流程是已有结论的整理；工具中心需求已确认并完成澄清（资料随 SkillVault 共享、安装记录本机独立、版本检查为 Agent 编写的检查命令、三项检查共用一个自动检查频率、登记与回写均由 Agent 经 CLI 完成），结论见需求记录末尾与 002 规格的 Clarifications。

设计和开发前请先阅读[裁决的解释与调整规则](docs/requirements.md#裁决的解释与调整规则)：核心管理要求是后续 Spec 的依据；实现细节可以按实际需要调整，不能为了暂定范围而绕路或妨碍核心要求的实现。


当前提供[技能管理交互原型](specs/001-skill-management-prototype/ui-prototype/index.html)，沿用旧界面风格，可直接打开并点击体验完整模拟流程。模拟状态由浏览器本地保存，不访问真实技能库或 Agent 目录；使用说明见[原型说明](specs/001-skill-management-prototype/ui-prototype/README.md)。

正式实施须按[原型页面与效果图索引](specs/001-skill-management-prototype/ui-prototype/README.md)、[视觉说明](specs/001-skill-management-prototype/visual-reference.md)与[交互细节](specs/001-skill-management-prototype/ui-interaction-audit.md)还原最终界面；各期执行[视觉与交互对照](specs/002-skill-management/quickstart.md#gui-视觉与交互对照各期适用)，记录实际截图和差异。原型的视觉与交互是验收基线，正式业务规则以 002 规格为准。


接入数据、视觉素材与平台经验已整理到本项目的[技术参考](docs/technical-reference/README.md)，按实现任务查阅；这些资料不继承旧产品规则。
