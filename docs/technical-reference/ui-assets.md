# 视觉与交互素材

现有产品视觉继续以[本项目视觉说明](../../specs/001-skill-management-prototype/visual-reference.md)、
[交互核查](../../specs/001-skill-management-prototype/ui-interaction-audit.md)和当前原型为准。
本页提取可用素材，不重建旧页面与旧路由。

## 已保存到本项目的素材

- [颜色与阴影变量](assets/theme-tokens.css)：浅色／深色的背景、面板、边框、文本、强调和危险色。
  只是样式数据，不含旧应用布局、业务选择器或构建入口。
- [Agent 图标](assets/agent-icons/)：50 个原图，与 [Agent 清单](agent-catalog.json)对应。
- [应用标志](assets/app-logo.png)：原界面使用的图形，展示名称仍为 Skill Manager。
- [预设图标选项](preset-icons.json)：当前原型已采用的 31 个 Lucide 图标 key、中文标签及颜色，
  由新实现使用 `lucide-react` 渲染，不要求复制旧预设组件。
- [源项目许可](assets/SOURCE-LICENSE.txt)：随提取素材保留的 MIT 文本。图标名称及品牌归属不因提取改变。

字体基线：SF Pro Text、PingFang SC、Noto Sans SC、Microsoft YaHei 及系统 sans-serif；
等宽内容使用 SF Mono、Menlo 或系统 monospace。具体可用字体取决于当前系统。

## 已确认的交互线索

| 位置 | 本项目使用的细节 |
|---|---|
| 应用外壳 | 紧凑左栏、分组导航、内容区滚动、白色面板与绿色强调 |
| 技能卡片 | 左上角拖动手柄、多选复选框、右上三点菜单、独立状态与部署表达 |
| 预设侧栏 | 悬停出现拖动／改名／删除；名称和图标一起编辑 |
| 分类与标签 | 左侧一级分类，顶部只显示当前分类拥有的标签 |
| 设置 Agent 卡 | 已检测／已启用状态、路径行内编辑、拖动顺序 |
| 指令编辑器 | 左列全局／工具列表、右侧单编辑区，保存拼接分发 |
| 工具中心 | 沿用卡片与标签密度，简介、指南、Skill 和版本分区按新需求组织 |

复制组件时最大的成本是旧业务耦合：不能连同旧 DTO、IPC、页面路由、唯一当前预设、
项目管理或市场等内容搬进来。先复用素材与交互，再按本项目业务实现。

## 按需定位源码

参考项目文件名：`src/index.css`、`tailwind.config.js`、`AgentIcon.tsx`、`WorkspaceSkillCard.tsx`、
`CardActionMenu.tsx`、`Sidebar.tsx`、`RenamePresetDialog.tsx`、`InstructionsDialog.tsx`、`Settings.tsx`。
这些不是新项目目录设计，也不要求继续依赖旧仓库文件。
