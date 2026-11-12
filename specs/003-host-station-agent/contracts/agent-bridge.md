# 内置管理 Agent：Host 侧桥

**Date**: 2026-11-11 | 研究决策 D6、D9。运行在当前 Host 进程内，浏览器不跑模型。

## 进程

默认管理 CLI **钉死为 OpenCode**（`opencode acp`，包 `opencode-ai`）。Host 按顺序解析当前 ACP CLI：**环境变量 `SKILL_MANAGER_AGENT_CLI`** → **用户登记或上次钉死的 `agent_runtime`** → 软件已安装到 `state-dir/agent-runtime/opencode` 的副本 → PATH 上名为 `opencode` 的命令（只认这一个名字）。都没有则 **不自动安装、不扫描厂商清单**；`GET /agent/runtime` 与对话返回 `needsInstall: true`，由用户在对话面板或 `host agent-cli --install` 安装并钉死。随后 `host agent-cli --login` / `POST /agent/runtime/login` 打开本机授权。模型由该 CLI 决定。用 ACP stdio 拉起。该 CLI 自己的登录失败时，对话只能报告无法执行并给出登录步骤，不得生成「已改库」结果。

清单外 CLI：用户执行 `host agent-cli --bin <路径> [--args …]` 或 `PUT /agent/runtime`。启动参数缺省时依次试 `acp` / `--acp` / 无参数；**进程能 spawn 不算成功**，须 ACP `initialize` 握手通过才采用该组参数，失败再试下一组。登记不只看文件存在，握手失败不得写入 `agent_runtime`。原生 Codex 不是 ACP，须登记适配器。

## 工具（主动查询与写入）

快照是推送；不够时 Agent **主动**查。查询/写入通道 = Host 经 ACP **向子进程声明的工具**（`session/new.mcpServers` stdio，带 JSON Schema 的 `skill_list` / `skill_get` / `skill_read` / `skill_set` / `skill_delete`（`references[]`）等），映射到现有 `skill-manager` JSON CLI，**经 Host 执行**（与 GUI 同一把锁、同一套确认）。不得直打 `skill-manager-core`。不得把工具清单只塞进 `initialize`。stdio MCP 只是 ACP 声明工具的协议通道，不是给用户用的平行 MCP 产品。CLI 自带 bash/edit 不得绕过 Host 锁与确认计划。同一 Host 对话复用 ACP session，并把 `agent_messages` 回灌新 session。

OpenCode 1.19.5：`session/new.mcpServers` 项若带任意 `type` 字段则当 remote HTTP（要 `url`/`headers`）。stdio 项必须省略 `type`，字段为 `name`、`command`（字符串）、`args`、`env: [{name,value}]`。不要写 `"type": "stdio"` 或 `"type": "local"`（后者是 `opencode.json` 的 `mcp` 配置，不是 ACP 字段）。不要用 `initialize.mcpServers`。

Host 启动该 CLI 时，把 `skill-manager/assets/skills/skill-manager-builtin-agent/`（内置管理说明）注入到 **该 CLI 会搜索的 skills 目录**。OpenCode：`$OPENCODE_CONFIG_DIR/skills/`（Host 写入 `state-dir/agent-runtime/opencode-config/`；实测 `OPENCODE_SKILLS_PATH` 无效）。若无此目录，改为 ACP 初始化文本并作为每条 prompt 前缀。说明两层上下文已在消息里、细节用工具、003 确认与上传规则。该资产**不入库**到用户 SkillManagerVault，不得出现在 `skill list`，不得当作可部署技能。不得注入或改写配套维护 Skill（那条路仍是每次询问上传）。

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
