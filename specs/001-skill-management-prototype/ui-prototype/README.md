# Skill Manager 交互原型

一次性的交互原型，验证来源入库、库内维护、部署、上游更新与多电脑同步是否能串成完整流程。
沿用旧版视觉风格，直接在当前项目根据操作反馈迭代，不需要先交给 openDesign。

双击 [index.html](index.html) 即可使用，无需启动服务。图标已内嵌，单 HTML 文件可以独立分享。
左侧导航进入各页面；底部“原型试玩”提供引导演练、自由操作与当前状态。
只使用浏览器本地保存的模拟资料，不读取或写入真实 SkillVault，不访问来源仓库或执行部署。
主要按钮、筛选、网格／列表、多选和表单共用模拟状态，操作后可以到相关页面查看结果。
刷新或重新打开网页会保留本浏览器中的模拟状态；在“原型试玩”点击“重置原型”恢复初始资料。

两个技能库、预设和 Hermes 档案的标签栏随当前分类收窄，只显示该分类下技能拥有的标签。回到全部分类恢复完整标签；切换分类保留有效的标签选择，失效的标签筛选自动清除。

## 页面与效果图

| 页面 | 直接打开参数 | 效果图 |
|---|---|---|
| 技能库 | ?view=code | [01-code-library.png](renders/01-code-library.png) |
| 龙虾技能库 | ?view=lobster | [02-lobster-library.png](renders/02-lobster-library.png) |
| 共享 Agent | ?view=shared | [03-shared-agent.png](renders/03-shared-agent.png) |
| Codex | ?view=codex | [04-codex-agent.png](renders/04-codex-agent.png) |
| Hermes 档案 | ?view=hermes | [05-hermes-profiles.png](renders/05-hermes-profiles.png) |
| 档案选择展开 | ?view=hermes&picker=1 | [06-hermes-profile-picker.png](renders/06-hermes-profile-picker.png) |
| 上游变化预览 | ?view=code&modal=updates | [07-upstream-changes.png](renders/07-upstream-changes.png) |
| 预设管理 | ?view=presets | [08-preset-overview.png](renders/08-preset-overview.png) |
| Matt 预设详情 | ?view=preset-detail&preset=matt | [09-preset-detail.png](renders/09-preset-detail.png) |
| 预设已加入筛选 | ?view=preset-detail&preset=matt&filter=member | [10-preset-members-filter.png](renders/10-preset-members-filter.png) |
| 预设部署预览 | ?view=preset-detail&preset=matt&modal=preset-deploy | [11-preset-deployment.png](renders/11-preset-deployment.png) |
| 新建预设 | ?view=presets&modal=preset-create | [12-preset-create.png](renders/12-preset-create.png) |
| 全局预设全库浏览 | ?view=preset-detail&preset=global | [13-global-preset.png](renders/13-global-preset.png) |
| 全局指令 | ?view=code&modal=instructions | [14-global-instructions.png](renders/14-global-instructions.png) |
| Cursor 专属指令 | ?view=code&modal=instructions&instruction=cursor | [15-cursor-instructions.png](renders/15-cursor-instructions.png) |
| 侧栏预设悬停操作 | ?view=preset-detail&domain=lobster&preset=home | [16-preset-sidebar-actions.png](renders/16-preset-sidebar-actions.png) |
| 技能卡片拖动手柄 | ?view=preset-detail&domain=lobster&preset=home | [17-skill-drag-handle.png](renders/17-skill-drag-handle.png) |
| 预设名称与图标修改 | ?view=preset-detail&domain=lobster&preset=auto&modal=preset-rename | [18-preset-icon-picker.png](renders/18-preset-icon-picker.png) |
| 设置与 Agent 检测 | ?view=settings | [19-settings.png](renders/19-settings.png) |
| 技能目录行内编辑 | ?view=settings | [20-settings-edit-path.png](renders/20-settings-edit-path.png) |
| Hermes 技能统一详情 | ?view=hermes，点击技能 | [21-hermes-skill-detail.png](renders/21-hermes-skill-detail.png) |
| 技能文件目录树 | 技能详情 → 文件与部署 | [22-skill-file-tree.png](renders/22-skill-file-tree.png) |
| 技能卡片三点菜单 | 悬停卡片 → 三点 | [23-skill-card-menu.png](renders/23-skill-card-menu.png) |
| 添加标签 | 三点 → 添加标签 | [24-skill-add-tags.png](renders/24-skill-add-tags.png) |
| 单技能更新预览 | 点击技能卡片的更新 | [25-single-skill-update.png](renders/25-single-skill-update.png) |
| 部署目标亮暗状态 | 技能详情 → 文件与部署 | [26-deployment-indicators.png](renders/26-deployment-indicators.png) |
| Windows 下 Mac 专用技能灰显 | 原型试玩 → 切换 Windows | [27-windows-unsupported.png](renders/27-windows-unsupported.png) |
| 已有部署不适用警告 | 收窄支持声明 → 文件与部署 | [28-existing-deployment-warning.png](renders/28-existing-deployment-warning.png) |
| Agent 询问上传 | 原型试玩 → Agent 修改与 CLI 同步 | [29-agent-upload-question.png](renders/29-agent-upload-question.png) |
| Hermes 只读管理详情 | 档案技能 → 管理资料 | [30-hermes-readonly-manager.png](renders/30-hermes-readonly-manager.png) |
| 紧凑管理资料 | 技能库 → 技能详情 → 管理资料 | [31-manager-compact-layout.png](renders/31-manager-compact-layout.png) |
| 入库单页 | 入库 | [32-intake-single-page.png](renders/32-intake-single-page.png) |
| 原位预览仓库 | 入库 → 预览仓库 | [33-intake-inline-preview.png](renders/33-intake-inline-preview.png) |
| 仓库变化决定页 | 更新／新增 → 查看并决定 | [34-repository-update-review.png](renders/34-repository-update-review.png) |
| 上游新增列表 | 新增 | [35-upstream-additions-list.png](renders/35-upstream-additions-list.png) |
| 收编管理资料 | Agent 技能 → 收编 | [36-adopt-metadata.png](renders/36-adopt-metadata.png) |
| 批量收编逐项配置 | 多选本地技能 → 收编 | [37-batch-adopt-metadata.png](renders/37-batch-adopt-metadata.png) |

预设页可切换编码／龙虾域；左侧预设名称可打开对应库的全部技能，已加入成员优先显示。
与技能库共用分类栏、标签、搜索和网格／列表，提供“全部／已加入／未加入”筛选。
卡片右上角开关直接加入／移出当前预设，多选也可批量调整；不再通过独立的添加成员弹窗操作。
新建预设先填写名称和管理域，再进入全库页面用开关选择成员。
组合及选中目标的部署／撤销可在内存中模拟，重新载入后继续保留，明确重置才恢复预置状态。
组合编辑不自动改变模拟部署；目标操作按当前成员执行，受管副本有变更的项暂停。
预设行悬停显示拖动、重命名和删除；预设排序直接拖动左侧手柄。两个库和预设页的技能卡片悬停时，左上角状态点变为拖动手柄；多选时改为复选框。不再提供页内预设编辑或独立排序弹窗。
新建及重命名预设可选择旧版的 31 个图标，仅修改图标也可保存；所选图标显示在侧栏、预设总览及详情标题中，并随模拟预设定义保存。
预设默认已加入成员优先，手动拖动后使用该预设的显示顺序；筛选中拖动只调整可见条目的相对位置，不移动分类目录或改变部署。独立的顶级预设导航已移除；点击左侧预设分组标题进入总览，名称进入详情。

设置页复用旧版 53 个内置 Agent 的目录与图标资料，初始电脑 A 模拟检测 8 个、启用 6 个，其余项可展开。支持重新检测、全部／逐项启停、拖动排序，以及技能目录和项目相对路径的行内编辑、取消及恢复默认。侧栏随本机启用状态和顺序变化，路径修改反映到 Agent 页面；不清空已有部署。主题、字号、检查频率使用分段按钮，SkillVault 新配置采用同样的分区行。检测、目录选择、打开文件夹和托盘开关仅改变模拟状态。

## 沿用与调整

沿用浅灰背景、白色卡片、绿色强调色、紧凑字号、左侧分组导航、分类列、三列网格、
柔和彩色标签、受管状态点、卡片底部状态与操作。
使用旧版已有应用图标与 Agent 图标作为视觉参考，品牌名称显示为 Skill Manager。

依据已确认的新业务调整：

- 库页不保留旧版唯一当前预设和全局技能开关，展示部署目标；预设仍是可复用组合。
- 启停开关用于 Hermes 档案技能，不扩展为全局技能启用机制。
- 上游检查／更新／新增与自己的 SkillVault 同步区分。
- 卡片提供原名及中文短注释；来源标签表示获取来源，自己的库是维护中心。
- 本机 Agent 页面区分受管、仅本地及本地有变更；收编仅针对未受管项。
- 预设页面分开显示组合定义和本机部署，提供成员选择、新建和目标预览；不显示唯一当前预设。
- 不恢复旧首页、在线市场或项目管理；工具中心按新增专题提供可讨论的原型。

原始参考截图保存在 [references](../references/)。
业务约束参见 [spec.md](../spec.md)，视觉来源参见 [visual-reference.md](../visual-reference.md)。
本原型用于用户直接操作评审，不替代正式软件实现或规格中的完整验收。

旧界面的广泛细节核查、已调整项与仍有差异的部分见[交互细节核查](../ui-interaction-audit.md)，包括详情展开方式、目标图标动作、多标签筛选、菜单和反馈等；这些差异没有被默认为已还原。

## 重新导出

本机已有 Chrome，使用隔离的无头浏览器渲染本地页面，不读取用户浏览器资料。
效果图尺寸为 1720 × 1040。

运行：

    /Users/fei/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node render-prototype.cjs

该脚本只写本目录 renders 下的效果图；没有自动安装依赖或联网下载。

## 建议体验顺序

1. 在技能库点击“入库”，选择部分示例技能；确认后查看来源与分类，再部署到本机目标。
2. 点击技能名称，修改中文注释、标签或支持范围；保存后在对应 Agent 页面查看已有部署结果。
3. 到 Codex 收编“仅本地”技能；对“本地有变更”项模拟 Agent 检查并作出裁决。
4. 点击“更新”或“新增”，按仓库查看文件变化、选择是否入库；未勾选项确认后不再提示。
5. 新建预设，在全库页面按分类浏览，用卡片开关加入／移出成员，再体验排序、部署和撤销；编辑组合本身不自动改变部署。
6. 在 Hermes 档案按分类、标签和启用状态组合筛选，切换到绘画助手，收编档案独有技能并切换启用。页面沿用技能库的标签栏、左侧分类栏与卡片网格。
7. 在“原型试玩”模拟其他电脑提交并同步，切换模拟电脑查看独立的部署和档案选择。
8. 点击右上角指令图标，左侧切换全局和各工具，右侧编辑。一次“保存”保留全部草稿并自动模拟分发全局＋各工具专属指令，弹窗保持打开。

## 模拟边界与暂定交互

- Git 下载、认证、提交上传、CLI（命令行）、目录搬动与部署均为内存模拟，不执行命令或访问网络。入库预览使用虚构技能，不分析输入网址的真实内容；下载进度和失败流程仍待实现。
- 检查间隔可以设置，但没有实际后台定时器；浏览器原型不具备系统托盘行为。
- 原始正文和脚本提供示例阅读；中文译文编辑与覆盖文件示例添加／移除用于评审，不据此决定正式编辑器需求。没有一键翻译。
- overlay（覆盖层）暂按列表顺序替换同名文件，实际格式和合成顺序尚未裁决。
- 同步冲突情境中的“采用远端”是一项明确的模拟用户裁决，不是自动覆盖策略，也没有合并引擎。
- 系统不支持时卡片灰显、不能部署，但仍可查看。支持声明改变不自动撤销既有部署，保留副本并显示不适用警告；这是用户已确认要求。
- 电脑 A／B、系统切换、重置和情境按钮仅用于评审，不引入正式产品的远程设备管理能力。
- 指令编辑器沿用旧版样式，不再使用分发目标勾选和双保存按钮。指令源内容放在模拟 SkillVault 中；模拟结果使用 `AGENTS.md`，示例指令目录复用旧版工具目录资料，与可修改的技能路径独立；不实际写入文件或验证工具读取能力。

本轮模拟操作的结果可在对应页面及操作记录中查看。真实端到端验收在正式实现后进行。

Hermes 档案的技能名称与内容区直接打开统一详情。受管项与技能库共用正文、管理资料、覆盖层及文件与部署，档案中只读，需明确前往技能库编辑统一内容；未收编项使用相同分区查看本机内容，提供收编入口。文件清单使用可折叠目录树，显示图标、缩进与分支线，点击文件阅读后可返回目录树。

第一版不内置 Agent，外部 Agent 的 CLI 维护链路仍属于既定产品范围。


库页和预设页卡片悬停出现三点菜单：复制到另一库、移动到分类、添加标签、删除；预设开关独立保留。添加标签支持选择已有及输入多个新标签，保留原有标签。菜单点外部／Escape 关闭。有上游变化的技能显示独立“更新”，先预览再确认，只更新当前条目与已有部署，不处理其他技能及新增候选；仓库批量更新可继续完成剩余变化，不重复应用已更新的版本。最新单项更新要求替代此前仅支持仓库整批更新的限制，所有操作仍为内存模拟。


最新维护规则：文件与部署显示本机已启用或已有部署的目标，亮显表示已部署，灰暗表示未部署；不在此管理 Hermes 档案。Hermes 详情只读并可前往库中编辑。修改支持范围后不撤销旧副本，显示警告并禁止不适用的新部署／启用；Mac 专用声明在模拟 Windows 及重新打开后仍然生效。原型试玩可切换系统、重新打开与重置。

模拟 Agent 修改／创建后会演示维护 Skill 主动询问是否上传：不上传时保留本机修改，CLI 部署同步与上传分开，其他同步动作不会绕过确认；同意上传后才推进模拟远端版本，每次新修改重新询问。这里没有内置 Agent、真实网络、真实 Git 或部署操作。浏览器本地存储只用于原型评审。

同库重名创建、收编、入库和复制均阻止并提示；跨 Code／Lobster 域可同名。覆盖整份 SKILL.md 后部署使用覆盖内容，不自动合并上游正文。

管理资料中，分类和系统支持上下放在左列，与右列 Agent 支持区对齐。Agent 选项仅显示所属库已检测且启用的本机工具；保存不清除已有的隐藏声明，“全部支持”也不会改成只支持当前本机。

入库采用同一弹窗的单页布局：来源地址、管理域和分类始终可见，预览结果在下方展开；可直接修改并重新预览。同一来源再次预览保留分类和勾选，换来源／管理域需重新预览后确认，勾选不会重建弹窗。仍只模拟虚构候选，不访问输入网址。

更新／新增的决定页可返回原仓库列表，保留查看范围、位置和新增选择／分类草稿。返回不应用变化；确认后更新并回到剩余仓库列表。仓库、版本、技能、变化状态和文件路径分别展示，单技能更新共用新的视觉层级。

收编时可填写中文注释、标签、分类、Agent／系统支持和可选中文译文，已有资料预填。批量收编可逐项填写，确认后资料随技能入库并接管原部署，原正文和文件相对路径保留；不新增其他目标部署。


## 外部维护与版本跳过

技能卡片三点菜单、多选栏和“文件与部署”提供“生成维护任务”，填写要求后可复制路径、当前资料及维护约定给外部 Agent；原型不会调用模型。

单技能更新页可“跳过当前版本”，仓库批量更新尊重此决定；更新列表底部“已跳过版本”可查看、恢复提醒或明确更新。下一版本重新提醒，刷新保留决定；与新增技能不入库记录分开。

效果图：[维护任务](renders/38-maintenance-task.png)、[跳过当前版本](renders/39-skip-upstream-version.png)、[已跳过列表](renders/40-skipped-versions.png)。


## 工具中心（已登记需求的交互原型）

侧栏“工具中心”可查看本机工具、搜索／分类筛选、检查更新、进入详情阅读维护指南。软件模拟检查版本，实际更新交给外部 Agent，点击“交给 Agent 更新”可复制包含指南的任务说明。安装／登记入口也生成 Agent 任务；“演示 Agent 回报”仅用于体验结果，不执行安装或升级。

配套 Skill 在详情中查看，已入库可前往技能库，未入库可选管理域、分类、中文注释和标签后入库，不自动部署。工具记录在当前模拟电脑，跨电脑的工具资料同步边界等待讨论。示例的版本、来源、路径、指南均不代表真实工具情况。

效果图：[工具清单](renders/41-tool-center.png)、[概览](renders/42-tool-overview.png)、[维护指南](renders/43-tool-maintenance-guide.png)、[更新任务](renders/44-tool-update-task.png)、[配套 Skill 入库](renders/45-tool-skill-intake.png)、[安装／登记](renders/46-tool-registration.png)。

配套 Skill 入库弹窗会列出具体技能名称、中文注释、用途与勾选状态；可取消／重新勾选，已选数量联动，未选任何技能时不能确认，已填管理资料保留。

工具概览顶部提供完整“工具简介”，介绍用途与适用场景，调用入口和安装／维护资料单独展示；模拟 Agent 回报支持维护简介。

用户已确认按当前工具中心原型登记需求，见需求记录“工具中心：已确认需求”及规格 FR-035；简介、候选列表和维护流程已纳入。具体实现与未裁决边界不因原型被固定。
