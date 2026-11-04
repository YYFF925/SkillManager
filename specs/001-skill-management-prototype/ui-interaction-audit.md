# 旧版界面细节核查与原型对照

本次核查覆盖技能库、预设、Agent 清单、Hermes 档案、详情、指令、设置，以及共用的菜单、多选、弹窗和状态反馈。依据用户截图与 FeiSkillDock 源码，没有启动旧应用；源码中的占位回调不视为已实现功能，也不把旧业务规则直接转成新产品要求。

用户本轮补充的原始截图已保留：[预设悬停操作](references/07-preset-row-hover.png)、[技能卡片拖动手柄](references/08-skill-card-drag.png)。

“已调整”表示本轮原型已处理；“已有”表示当前原型已表达；“待对齐”表示已发现差异，尚未据此改动。后者是评审清单，不是功能移除结论，更不是强制实现规格。

## 侧栏与预设

事实依据：[Sidebar.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/components/Sidebar.tsx:556)、[RenamePresetDialog.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/components/RenamePresetDialog.tsx:57)。

| 旧版界面事实 | 原型处理 |
|---|---|
| 预设行平时显示名称和数量，悬停时数量让位给拖动、铅笔、删除图标 | 已调整；键盘焦点进入时也显示操作 |
| 行内改名／删除与点击预设导航是不同动作 | 已调整；操作控件不触发导航 |
| 预设通过侧栏手柄直接拖动排序，不需要另开排序页 | 已调整；编码与龙虾域各自排序 |
| 重命名使用小弹窗，输入框自动获得焦点，Enter 可以提交 | 已调整；页内编辑入口移除 |
| 创建／重命名预设可以选择图标，选中图标有独立色彩反馈 | 已沿用旧版 31 个图标及网格；仅改图标也可保存，侧栏、详情及总览显示所选图标 |
| 删除组合有确认，删除预设不是删除其技能 | 已有；改为侧栏直接打开确认，删除非当前预设不会强制离开当前页面 |
| 分组标题可折叠，组内内容独立于选中高亮 | 待对齐；当前预设标题承担总览入口，不直接用折叠动作替换 |
| 侧栏可调整 Agent 顺序，当前组和数量反馈紧凑 | 已在设置卡片补回拖动排序，侧栏随本机启用状态及顺序展示；用户另已明确恢复 Agent 检测，不扩展项目管理或多实例 |

## 技能卡片、列表与排序

事实依据：[MySkills.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/views/MySkills.tsx:1830)、[WorkspaceSkillCard.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/components/WorkspaceSkillCard.tsx:53)、[SyncDots.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/components/SyncDots.tsx:106)。

| 旧版界面事实 | 原型处理 |
|---|---|
| 左上角使用固定位置：平时状态点，悬停拖动手柄，多选复选框 | 已调整库页和预设页；没有给不支持拖动的 Agent 清单添加假手柄 |
| 悬停时技能名变强调色，卡片边框及阴影轻微增强 | 已调整／已有；不依靠放大卡片制造效果 |
| 开关、更多按钮等操作不应同时触发卡片详情 | 已有；拖动手柄也不打开详情 |
| 更多按钮悬停出现，低频操作收进菜单，危险操作用红色 | 已沿用三点展开菜单，包含复制、移动分类、添加标签和删除；预设开关独立，外部点击／Escape 关闭，删除使用危险色 |
| 网格和列表表达同一批数据与选择状态，但各自使用适合扫描的排布 | 已有同一状态；列表的列宽、标签压缩和尾部操作密度仍待进一步对齐 |
| 窄窗口两列，宽窗口三列，工具栏允许换行 | 已调整，避免在窄窗口硬挤三列 |
| 长名称截断但可以通过完整名称提示辨认 | 已有截断；完整名称提示需继续补齐，不应靠新中文显示名解决 |
| 目标图标同时表达已部署／可部署，支持点击安装或移除 | 重要差异：原型只显示已部署图标，点击跳到目标页。是否沿用旧版直接切换动作需结合显式部署与预览流程评审，不能假称已还原 |
| 状态点、文字状态、文件数、来源与底部操作各有固定位置 | 部分沿用；原型的各页文件数及状态文字密度仍有差异 |

旧版排序存在一个容易被外观掩盖的限制：[MySkills.tsx:525](/Users/fei/Projects/workspace/FeiSkillDock/src/views/MySkills.tsx:525) 的实际处理只移动预设内已加入成员，普通库页没有同样的排序效果。新原型按用户要求让两个库及预设页都可以拖动；拖动只改变显示顺序，不移动分类目录、不改变成员归属或部署。

当前原型默认已加入成员优先；某个预设经过手动拖动后使用该预设的手动显示顺序。筛选中拖动只调整可见条目的相对位置，隐藏条目保留位置。上述是本轮可体验的排序方案，排序数据的正式存储方式仍待实现设计。

## 筛选、分类、标签与多选

事实依据：[MySkills.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/views/MySkills.tsx:1550)、[useMultiSelect.ts](/Users/fei/Projects/workspace/FeiSkillDock/src/hooks/useMultiSelect.ts:19)、[MultiSelectToolbar.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/components/MultiSelectToolbar.tsx:62)、[BatchTagDialog.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/components/BatchTagDialog.tsx:119)。

| 旧版界面事实 | 原型处理 |
|---|---|
| 分类列即使筛选无结果也保留，让用户直接换分类 | 已有 |
| 分类栏随卡片一起滚走，分类多时无法独立浏览 | 已调整：分类栏固定，独立滚动，仅溢出时显示滚动条 |
| 再点击当前分类可以回到全部分类 | 待对齐；当前明确点击“全部分类”返回 |
| 旧分类行上的新增、菜单中存在占位回调 | 不照搬无效回调；当前原型的分类操作已有模拟结果 |
| 标签支持同时选择多个；库页匹配任一选中标签 | 重要差异：原型目前只选一个筛选标签。技能具有多个标签已实现，但这不等于多标签组合筛选已实现；组合语义尚未在本轮重新确认 |
| 标签可右键重命名、删除；批量标签支持添加和移除 | 待对齐；原型目前通过标签管理弹窗操作，批量整理只追加标签 |
| 无标签使用独立的弱化筛选样式 | 已有；是否显示该项及计数细节可进一步对齐 |
| 无结果时明确显示空状态和“清除筛选” | 已调整；恢复搜索、分类、标签和状态筛选 |
| 多选时点卡片主体或名称即可选择，并显示整卡选中描边 | 已调整；不必精确点小复选框 |
| 全选只针对当前结果，支持取消全部选择 | 已调整全选状态和部分选中反馈 |
| 筛选或页面变化不应让批量操作继续作用于不可见选择 | 原型切换筛选或搜索时清空选择；比旧版保留可见选择的实现简单，当前行为可体验 |
| 退出多选有明确按钮，Escape 也可退出；弹窗先处理 Escape | 已调整；不占用输入编辑中的 Escape 来退出多选 |
| 批量栏只突出常用动作，低频动作折叠进“…” | 待对齐；原型库页批量栏仍较拥挤 |

分类限定标签是用户新增要求，已在原型实现；旧版没有满足这项联动，不能为“沿用界面”恢复原来的全标签行为。

## Hermes、详情与指令

事实依据：[HermesProfiles.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/views/HermesProfiles.tsx:300)、[DetailSheet.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/components/DetailSheet.tsx:29)、[SkillDetailPanel.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/components/SkillDetailPanel.tsx:750)、[SkillMarkdown.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/components/SkillMarkdown.tsx:104)、[InstructionsDialog.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/components/InstructionsDialog.tsx:119)。

| 旧版界面事实 | 原型处理 |
|---|---|
| 档案下拉显示当前项、路径、技能数和选中提示 | 部分沿用；选中对号和路径提示细节待对齐 |
| Hermes 标签带数量，支持多选；分类、启用状态和搜索可以组合 | 分类与联动筛选已有；标签数量和多标签选择待对齐 |
| 档案候选为空、筛选无结果、没有档案分别给出不同说明 | 待对齐；目前通用空状态没有完整区分这些情境 |
| 档案技能底部保持分类位置及启用文字，右上角开关控制当前档案 | 已有 |
| 详情展开覆盖右侧内容区，保留左侧导航，不是居中窄弹窗 | 重要差异：当前技能详情仍是居中弹窗。下一步应优先评审是否还原这个空间组织，而不是继续向窄弹窗堆内容 |
| 详情将来源信息、中文／原文、当前／差异／来源内容分层 | 原型已有部分分层；旧版三种内容视图不直接等于新产品要做三套编辑器或冲突合并器 |
| 正文渲染 Markdown，代码块和表格独立横向滚动 | 待对齐；原型正文当前是纯文本示例，尚未表达实际阅读排版 |
| 指令弹窗为左侧全局／工具列表、右侧单编辑区、单保存按钮 | 已按用户要求沿用；跨工具草稿、搜索、清空与保存自动拼接已验证 |

## 弹窗、反馈、设置与桌面外壳

事实依据：[ConfirmDialog.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/components/ConfirmDialog.tsx:68)、[CardActionMenu.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/components/CardActionMenu.tsx:49)、[InstallToast.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/components/InstallToast.tsx:10)、[Layout.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/components/Layout.tsx:65)、[Settings.tsx](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Settings.tsx:1980)。

| 旧版界面事实 | 原型处理 |
|---|---|
| 简单改名、删除使用紧凑弹窗，长详情列表内部滚动 | 改名已调整；部分删除弹窗尺寸和危险色仍待统一 |
| 菜单点外部关闭，Escape 先关闭最上层交互 | 弹窗与卡片菜单已有；菜单先消费 Escape，不退出底层多选状态 |
| 保存、下载、同步等进行中有忙碌状态及对应文案 | 真实任务未接入；不能把原型的即时成功当作已完成加载／失败反馈设计 |
| 安装反馈区分获取、安装、同步和完成阶段 | 可借用阶段表达，但不恢复“入库即部署”；真实下载百分比和大仓库失败处理仍按新需求设计 |
| 成功、部分失败、失败使用不同反馈，不只弹统一成功 | 待对齐；原型事件记录不能替代正式操作结果反馈 |
| 设置按主题分区，路径、开关和说明靠近相应操作 | 已沿用分区配置行、三列 Agent 卡片、检测分组、启停、拖动、路径悬停铅笔与行内保存／取消／恢复默认；主题、字号、检查频率使用分段按钮，新同步设置采用相同配置行。检测与桌面行为仅模拟，不恢复代理、内置 Git 或历史恢复 |
| 顶部拖窗条、固定侧栏、滚动内容区和右上角指令入口各自独立 | 视觉已有；真实拖窗和托盘行为需桌面实现，浏览器不能冒充完成 |

## 后续评审的优先顺序

先看三个会改变使用感受的差异：详情占用区域、部署目标图标的点击行为、多标签筛选；再对齐批量栏密度、空状态和标签计数；卡片“…”菜单已补回。它们比继续微调颜色更值得下一轮操作评审。

现有首页、在线市场、项目管理、命令面板、旧备份恢复等已退出或暂缓范围，研究其通用样式不代表恢复其功能。本文作为后续界面改动的核对基线；不要求用户自行发现并逐一报告所有差异。

### 设置页补充核查（2026-10-15）

参考 [旧版 Agent 检测](references/09-settings-agents.png)、[目录悬停编辑](references/10-settings-path-hover.png)、[全局配置行](references/11-settings-global.png)，以及旧版 `Settings.tsx`、`tool_adapters.rs` 与 `agentIcons.ts`。原型保留完整 53 项内置目录资料，未检测项折叠展示；真实安装情况未探测。路径调整只修改本机模拟配置，指令目录独立保留。


### 技能统一详情与文件层级（2026-10-15）

用户最新补充：Hermes 档案的受管技能直接打开库内统一详情，未收编项沿用相同布局查看及收编；文件区改为可折叠目录树，支持多层文件夹和文件归属展示。此处不改变前述仍待评审的详情空间组织与 Markdown 阅读排版。


### 最新维护与部署状态裁决（2026-10-15）

用户确认：Hermes 档案不编辑技能，统一详情在档案上下文中只读，编辑需前往技能库；文件与部署只展示 Agent 目标的亮暗与警告，不增加档案分配管理。系统不支持的技能保留灰显，声明重新打开后仍生效；已有不适用部署不自动撤销。Agent 维护后的上传由外部维护 Skill 每次主动询问，网页弹窗只模拟这个对话。模拟状态现存浏览器本地，重置入口明确恢复初始数据。


### 工具中心专题恢复（2026-10-15）

已查看旧版 ToolCard.tsx、ToolDetail.tsx、DetailSections.tsx、HomeHeader.tsx、FilterBar.tsx 及冻结数据契约：沿用名称／用途／能力标签／Skill／版本卡片与文档阅读层级；按新需求增加软件版本检查、交给 Agent 的维护任务及配套 Skill 入库。旧版只读限制、多机器聚合、健康枚举与文档文件名仅为旧实现事实，未直接继承为新规范。当前截图 41—46 用于首轮讨论。
