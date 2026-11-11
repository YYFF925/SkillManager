# 内置管理 Agent：Host 侧桥

**Date**: 2026-11-11 | 研究决策 D6、D9。运行在当前 Host 进程内，浏览器不跑模型。

## 进程

Host 检测 `agent_runtime.binary_path`；没有则按已锁定的默认 CLI 执行软件内安装（不打进 Skill Manager 安装包）。用 ACP stdio 拉起。该 CLI 自己的登录失败时，对话只能报告无法执行，不得生成「已改库」结果。

## 工具（主动查询与写入）

快照是推送；不够时 Agent **主动**查。查询/写入通道 = Host 经 ACP **向子进程声明的工具**（`session/new.mcpServers` stdio，带 JSON Schema 的 `skill_list` / `skill_get` / `skill_read` / `skill_set` 等），映射到现有 `skill-manager` JSON CLI，**经 Host 执行**（与 GUI 同一把锁、同一套确认）。不得直打 `skill-manager-core`。不得把工具清单只塞进 `initialize`。stdio MCP 只是 ACP 声明工具的协议通道，不是给用户用的平行 MCP 产品。CLI 自带 bash/edit 不得绕过 Host 锁与确认计划。同一 Host 对话复用 ACP session，并把 `agent_messages` 回灌新 session。

Host 启动该 CLI 时，把 `skill-manager/assets/skills/skill-manager-builtin-agent/`（内置管理说明）注入到 **该 CLI 会搜索的 skills 目录**（路径由 D9 探测记录；若无此目录，改为 ACP 初始化文本）。说明两层上下文已在消息里、细节用工具、003 确认与上传规则。该资产**不入库**到用户 SkillManagerVault，不得出现在 `skill list`，不得当作可部署技能。不得注入或改写配套维护 Skill（那条路仍是每次询问上传）。

Agent 可调用的管理工具 = 现有 JSON CLI 的能力（含 `skill list` 已有的 `translation_stale` 等字段）。不为对话新增 002 筛选开关。工具经 Host，与 GUI 串行于同一把锁。调用失败必须把错误交回对话，不得编造正文。

破坏性工具（`skill delete`、覆盖部署/overwrite-local）的执行路径：

1. Host 生成操作计划并推到当前界面；
2. 用户在该 `sessionId` 确认；
3. Host 才跑 core。

Agent 文本里的「用户同意了」无效。

## 上下文

每条用户消息带发出时的两层快照，由 Host **自动**附上，用户不必粘贴。客户端 `PUT /agent/context` 只报页面与选中引用；**详情由 Host 在 `POST /agent/messages` 时从 core 补齐**（`global` 与 `view` 皆然）。字段举例不是穷尽清单；原则是 GUI 当时能回答的管理事实，对话里不必再由用户复述。快照只含当前这台 Host，不含其它设备。

**全局 `global`**（库级 / 本机状态，发消息时从 core 查询；形态 = 摘要 + 计数 + 引用，无文件正文）：

- 技能清单摘要：域、名、分类、标签
- 待办类状态：上游待更新、可入库新增、译文待更新（计数 + `ref` 列表）
- 部署摘要：哪些技能部署到哪些目标
- 事故：拉取冲突路径与 git 原文、`paused_local_change`、最近一次管理命令失败原文
- 其它 GUI 已知摘要：预设、Hermes 档案、工具资料、本机 Agent 目录（同样是摘要与引用，不是全文）

**当前界面 `view`**（缺省指示语只解析这一层；上报只有引用，详情 Host 补齐）：

- 页面、管理域、选中 `ref`、分类、预设、Agent/Hermes/工具当前对象
- 若在技能详情：该技能注释/分类/标签/支持声明、文件树、上游来源、已有部署位置

不得把整库 `SKILL.md` 正文塞进每一条消息。摘要与引用足够定位后，细节经 Host 调既有 JSON CLI 读取。进行中任务忽略之后的切页。用户可取消进行中任务：停止继续写库，不得把该任务标成成功。不是新建冲突合并 UI。

快照与工具结果会进入当前 CLI 所连接的模型；Host 第一版不脱敏。

## 上传

若本机 `git_auto_push` 已由 GUI（含远程网页）打开，内置管理 Agent 写入后可上传且不逐步确认。未打开则上传失败并说明去设置里打开；Agent 不能写该设置。

## 非范围

不主动发任务。不做冲突合并产品。不把对话写入 Vault。不把内置管理说明入库。不做多会话工作区。不为对话新增 002 CLI 筛选开关。不另做给用户用的 MCP 产品（ACP `mcpServers` 只用来声明 Host 工具）。
