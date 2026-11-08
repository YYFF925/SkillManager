# DESIGN.md — Skill Manager

Agent 改界面前读本文。视觉画布是可操作原型，不是本文的表格。
产品前端必须与当前原型所有可见细节一致；本文提供可执行 token，避免每次从 CSS 猜。

视觉真值是本仓库当前原型，不是任何已不存在的旧项目。相对**当前原型**要求所有可见细节一致。

## 1. Identity

- 这是 Skill Manager：个人桌面技能管理工具，不是官网、仪表盘或营销页。
- 气质：紧凑、克制。浅灰底、白卡片、绿色强调、细边框、轻阴影、系统中文无衬线。
- 信息密度高：三列卡片、40px 工具栏、左侧分组导航。不要做成低密度展板。
- 类比：熟悉的桌面工具，不是 Linear / Stripe 后台。不要去找或复刻已不存在的 FeiSkillDock 仓库。

## 2. Colors

浅色默认。深色只换同一套层级，不换结构。

| 角色 | 浅色 | 深色 | 产品变量 | 原型变量 |
|---|---|---|---|---|
| 应用背景 | `#FAFAFA` | `#0C0C0F` | `--c-bg` | `--bg` |
| 侧栏／次级 | `#F4F4F5` | `#0F0F13` | `--c-bg-subtle` | `--secondary` |
| 卡片／面板 | `#FFFFFF` | `#131317` | `--c-surface` | `--surface` |
| 悬停 | `#F4F4F5` | `#17171E` | `--c-surface-hover` | — |
| 选中 | `#E4E4E7` | `#1E1E28` | `--c-surface-active` | `--active` |
| 常规边框 | `#D4D4D8` | `#222229` | `--c-line` | 部分硬编码 |
| 柔和边框 | `#E4E4E7` | `#1C1C22` | `--c-line-soft` | `--border` |
| 更弱分隔 | `#EEEEF0` | `#18181E` | `--c-line-faint` | `--faint-border` |
| 主文字 | `#18181B` | `#F4F4F5` | `--c-ink` | `--text` |
| 次级文字 | `#27272A` | `#D4D4D8` | `--c-ink-2` | — |
| 三级文字 | `#52525B` | `#A1A1AA` | `--c-ink-3` | — |
| 说明 | `#71717A` | `#8E8E96` | `--c-muted` | `--muted` |
| 更弱说明 | `#8B8B94` | `#71717A` | `--c-faint` | `--faint` |
| 强调 | `#059669` | `#10B981` | `--c-accent` | `--green` |
| 成功／受管点 | `#10B981` | `#34D399` | `--c-accent-light` | `--green-light` |
| 实心主按钮 | `#047857` | `#059669` | `--c-accent-strong` | `--green-dark` |
| 危险 | `#DC2626` | `#EF4444` | `--c-danger` | — |
| 警告 | `#D97706` | `#F59E0B` | `--c-warn` | — |

卡片阴影浅色 `0 1px 2px rgba(24,24,27,.04)`，悬停 `0 4px 12px rgba(24,24,27,.07)`。
强调选中用极浅绿底，不要大面积铺绿。

## 3. Typography

- UI：`13px "SF Pro Text","PingFang SC","Noto Sans SC","Microsoft YaHei",-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif`，字距 `-0.005em`。
- 路径／代码：`SF Mono`、Menlo、系统 monospace。
- 页标题 17px / 600 / 行高 26px；技能名 14px / 600；常规 13px；状态 12px；标签 11px。
- 界面语言只有简体中文；技能名保留原名，中文注释另附，通常 ≤20 字。

## 4. Spacing

- 侧栏宽 220px；分类列 148px（工具中心 152px）；主内容最大宽 1440px；内容区内边距 20px。
- 无内嵌窗口标题栏；画布就是产品 webview。
- 侧栏顶级：技能库、龙虾技能库、Hermes 档案、工具中心。预设不在顶级导航，而在其下单独成区。
- 卡片网格三列（窄于 1301px 两列）、间距 12px；分类列与卡片区间距 16px。
- 工具栏／搜索／主按钮高 40px；紧凑按钮 30px；导航行／分类行高 35px。
- 卡片圆角 12px，按钮／输入 8px，导航行 6px，胶囊标签全圆。

## 5. Surfaces

- 窗口圆角 14px；标题栏 28px。
- 卡片：白底、1px 边框、轻阴影；悬停边框略深、阴影略抬。
- 弹窗默认宽 860px，紧凑弹窗 400px；指令编辑器更大、圆角 28px。
- 不要用厚阴影、玻璃拟态或全屏营销 Hero。

## 6. Layout

固定桌面外壳：左导航 + 右内容。内容区常见结构：页头 → 工具栏 → 标签 → 分类列+卡片网格。

- 分类栏固定在工具栏／标签栏下方，与卡片区分滚动；仅溢出时出现滚动条。
- 卡片按内容高度排布，不随剩余视口被拉高或压扁。
- 最小宽约 1080px。不为移动端重做布局。
- 主侧栏「预设」是独立分区：区标题右侧是贴着文字的「编码库 / 龙虾库」，右对齐，不拉满整行。下面只列当前库的预设。主区只展示当前预设的技能，不再内嵌库切换和预设列表。

## 7. Components

沿用原型已有形态，不发明第二套：

- 导航行、分类行、搜索、分段按钮、描边按钮、绿色主按钮、图标按钮
- 技能卡片：状态点／拖动手柄、原名+注释、描述、彩色标签、底部来源／预设胶囊与 QuickDeploy 亮暗图标（无单独「部署」文字按钮）、库卡片悬停三点菜单
- 开关：预设成员 34×18，设置 Agent 38×22；受管绿点 8px
- 设置分区行、Agent 检测卡（技能目录／项目路径／检测目录、铅笔常显）、路径胶囊、分段主题／字号
- 文件树、部署亮暗目标、指令左右栏编辑器

图标：Lucide 线性 14–16px；Agent 品牌图标保留识别性。

## 8. Motion

- 只要短过渡：卡片阴影／边框约 150ms。
- 不要装饰性动画、页面转场或骨架屏秀。

## 9. Interaction

- 悬停才出现的操作：卡片三点、预设行改名／删除／小火箭、分类铅笔与「全部分类」上的加号。分类六点手柄常显，只负责排序。设置 Agent 卡的路径铅笔常显。
- 标签多选是 AND。「全部标签」清空具体标签且不能单独取消；具体标签都取消后回到「全部标签」。「无标签」与具体标签互斥。
- 「全部部署目标」「全部预设」是挂在自己按钮正下方的标准下拉，宽度贴着文字；触发器右侧是向下 chevron。
- 分类铅笔和新建都在行内编辑，文本全选，不用弹框。预设拖动手柄松开后顺序写回并保持。
- 预设行小火箭打开「安装到 Agent」。Agent 详情在搜索栏上方用轻量 pill 列出当前库预设：已装浅描边加勾，部分安装显示个数，未装偏灰；点击部署，已装再点取消。
- Agent 指令弹窗不展示标题下说明、合成预览和保存按钮旁的说明。保存仍在。
- 选中：绿描边或 `--active` 底，不要只靠颜色文字。
- 悬浮提示自绘，不用系统 `title`。
- 危险操作红色；不可用降低透明度并禁止点击，仍要能读懂原因。
- 原型底部「原型试玩」条是评审控件，产品里没有。

## 10. Voice

- 短、具体、说明后果。区分已知事实、待确认、执行结果。
- 不把「任务已生成」写成「已经安装／已经更新」。
- 按钮用动词：入库、检查、同步、收编。不要空泛的「提交」「确认一下」。

## 11. Don'ts

- 不要只抄配色另做布局。
- 不要先改 React 再补原型；视觉变更的第一现场是原型。
- 不要引入 OpenDesign 桌面应用、Figma 或新设计系统作为前置。
- 不要恢复旧首页、市场、项目管理、唯一当前预设。
- 不要把原型模拟数据、localStorage、演示 Agent 回报当成产品实现。
- 不要为 1px 抗锯齿差加兼容层；用户能看出的位置、尺寸、颜色、文案、交互不一致必须改。
- 不要把已不存在的 FeiSkillDock 当现行对照；历史沿革只在 visual-reference 等旧记录里。
- 业务以 002 规格为准。演示和正式规则冲突时改原型去对齐正式规则，并记下差异。


## 12. 对照表

改原型的哪一块，就改哪个 React 文件。完整表在 `.agents/skills/frontend-design/SKILL.md`。

- 外壳：`sidebar` / `head` → `AppShell.tsx`
- 技能库：`library` / `libraryCard` → `LibraryPage.tsx`
- Hermes：`profiles` → `HermesPage.tsx`
- 工具中心：`toolCenter` / `toolCard` → `ToolCenterPage.tsx`
- 预设：`presetPage` → `PresetPage.tsx`
- Agent：`agents` → `AgentPage.tsx`
- 设置：`settings` → `SettingsPage.tsx` + `AgentSettings` / `SyncStatus` / `DiagnosticsPanel`
- 共用控件：`.btn` `.seg` `.badge` `.dialog` → `components/ui.tsx`
- 弹窗：`skillDialog` `deployDialog` `intakeDialog` `drawInstructions` 等 → 同名职责的 `components/*.tsx`

样式源文件是 `specs/001-skill-management-prototype/ui-prototype/prototype.css`。
