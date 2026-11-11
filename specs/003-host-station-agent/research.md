# Research: 本机 Host、管理站与内置管理 Agent

**Date**: 2026-11-11 | **Status**: 规划决策已确认；ACP 默认命令行的最终锁定以实施第一个可运行切片前的探测清单为准（D9），不阻塞本文件其余决策。

依据：[spec.md](spec.md)、[ADR 0002](../../docs/adr/0002-host-management-station-and-agent.md)、宪章 2.1.0、002 的 `data-model.md` / `contracts/cli.md`、当前 `AppState` 与 `client.ts`。

## 已确认的设计决策

### D1 从桌面壳抽出 Host，业务仍只在 core

- **Decision**: 把现在 `src-tauri/src/commands/mod.rs` 的 `AppState`（上下文、后台任务锁、库监听、获取取消、调度报告）升为 `crates/host`。Tauri 与 `skill-manager host serve` 共用这一层。`skill-manager-core` 不增加 Agent 运行时、不增加 HTTP。
- **Rationale**: ADR 0002 分层；宪章 V 要求入口共用规则。今日业务已在 core，缺口是进程内协调只活在 Tauri 里。
- **Alternatives**: 在 core 里起 HTTP（被拒：core 会绑死入口）；桌面与 serve 各写一套 AppState（被拒：双实例、双调度）。

### D2 同一 CLI 二进制三个角色，管理站不依赖 core

- **Decision**: 可执行文件仍是 `skill-manager`。角色由子命令区分：无窗口 Host 为 `host serve`，管理站为 `station serve`，桌面安装包内嵌 Host。`crates/station` **不得**依赖 `skill-manager-core`，因此物理上拿不到 Vault。
- **Rationale**: 规格「不为云电脑另做安装包」；「管理站不持有 Vault」。分 crate 比靠纪律更硬。
- **Alternatives**: Docker 专用镜像作为唯一形态（被拒：用户不限制形态）；管理站复用 core 只「保证不打开库」（被拒：依赖一加上就会误用）。

### D3 远程 API 与现有 invoke 同名

- **Decision**: Host 对界面的写接口为 `POST /invoke/<command>`，JSON 体与现有 Tauri `invoke` 参数一致；事件为 `GET /events` SSE，事件名沿用 `vault-changed`、`schedule-finished` 等。前端 `client.ts` 增加 `TauriTransport | HttpTransport`，业务函数不再写死 `invoke`。
- **Rationale**: 已有近百个 command，换一套 REST 资源会双写规则。
- **Alternatives**: 资源型 REST（被拒：与 002 契约一对二）；WebSocket 双向 RPC（被拒：第一版 SSE 已够推送）。

### D4 Host 出站长连接，用户不另装隧道客户端

- **Decision**: Host 用管理站地址 + 设备密钥主动建立加密长连接；管理站把已登录浏览器对「当前设备」的 invoke/SSE 转到这条连接。用户只做「连接到管理站」。不把 FRP 做成独立产品，第一版不自研完整隧道生态。
- **Rationale**: 规格禁止每台开端口；成熟隧道库若过重，单用户少设备用一条连接上的请求多路复用即可。
- **Alternatives**: 每台 Host 公网 HTTPS（被拒：体验与暴露面）；用户自建 FRP 再填端口（被拒：多一步、与「对接码」不符）；Tailscale（被拒：用户明确不用）。

### D5 跨进程锁 + Host 在场时 CLI 改走 Host

- **Decision**: `state-dir` 放咨询性文件锁。Host 在跑：CLI 的写入类命令经本机 Host 执行。Host 不在：CLI 直连 core，仍占同一把锁。
- **Rationale**: FR-006；无队列产品。
- **Alternatives**: CLI 永远直连 core（被拒：与 GUI/Agent 并发无协调）；CLI 强制要求 Host（被拒：管理站挂了、以及现在「无 GUI 也能维护」会断）。

### D6 破坏性确认只在 Host 发票

- **Decision**: 删除技能、覆盖类写入由 Host 生成带 `planId` 的操作计划，绑定 `sessionId`，过期作废。内置管理 Agent 不得口头声称已确认。`git_auto_push` 仍仅图形界面可写（含远程网页，远程算 GUI）；未授权时内置管理 Agent 的上传失败并说明「请在设置中开启自动上传」，不改为逐步确认，也不得自行打开该开关。
- **Rationale**: FR-019/020；002 禁止 CLI 写 `git_auto_push`。
- **Alternatives**: Agent 工具里放「用户已同意」布尔（被拒：可伪造）；每个 push 弹窗（被拒：用户已否决）。

### D7 无窗口以本机服务安装，不做保活产品

- **Decision**: `host install-service` / `host uninstall-service` 写入当前系统的用户级开机启动（macOS launchd、Linux systemd --user、Windows 登录项或等价服务）。`host serve` 即启动命令。不做唤醒休眠、不做看门狗产品。
- **Rationale**: FR-003。
- **Alternatives**: 只文档里写手动命令（被拒：安装 Skill 需要稳定入口）；独立 watchdog 进程（被拒：宪章 VI）。

### D8 新机器 clone 是 CLI，不是向导

- **Decision**: `vault clone <remote> --path <dir>` 调用本机 git（与 002 一样不内置 Git 引擎），成功后写入本机 Vault 路径并可由 `host serve` 使用。可与 `--pair` 组合成一键。失败不覆盖已有目录。
- **Rationale**: FR-004。
- **Alternatives**: GUI 克隆向导（被拒）；应用内建库（被拒：第二座中央库）。

### D9 ACP 优先；默认命令行用探测清单锁定

- **Decision**: Host 以 ACP（stdio JSON-RPC）拉起成熟 Agent CLI。工具接到 Host 的第一路径是现有 `skill-manager` JSON CLI（经 Host 转发），不先做 MCP 平行实现。默认安装哪一家：实施「可对话切片」前用同一探测清单测 **OpenCode → Codex CLI → Gemini CLI**（会话、流式、取消、工具调用、stdio 稳定性）。第一家通过者成为检测/安装默认，写入安装 Skill 的登录步骤。探测不是产品功能，失败如实换下一家，不静默降级成「已能管理」。
- **Rationale**: ADR 要求探测后锁；本环境未实测 ACP，宪章要求不足时说明。OpenCode 的 ACP 路径在既有讨论中最明确，故排第一，**不是**已经验证的结论。
- **Alternatives**: 把运行时编进 core（被拒）；打包第三方 CLI（被拒）；先做 MCP 网关（被拒：002 CLI 已覆盖业务，双协议第一版过重）。

### D10 对话与对接状态留本机 SQLite，管理站另库

- **Decision**: 对话、操作计划、设备密钥、Agent CLI 本机路径记在 Host 的 `skill-manager.db` 新表（仍是本机状态，不进 Vault）。管理站用自己的 SQLite：管理员、会话、设备名单、对接码。两边都不镜像技能文件。
- **Rationale**: 宪章 II、ADR 0001、FR-011/018。
- **Alternatives**: 对话进 Vault（被拒：跨机串会话、把闲聊当技能资产）；管理站缓存技能正文（被拒：第二份库）。

### D11 远程界面复用同一套 React，桌面专属能力按壳关掉

- **Decision**: 管理站登录后下发与桌面相同的前端。`isDesktop()` 为假时走 HttpTransport，隐藏托盘/关窗/应用更新；路径为文本输入，Host 校验存在性，第一版不做 Host 侧目录树。登录页、设备条、对话面板是新界面，必须先改 [设计层](../../design/README.md) 原型再改 React。Host 与管理站可先于对话交付。
- **Rationale**: FR-012；设计层工作流。
- **Alternatives**: 为远程另写一套页面（被拒：两套规则）；在浏览器里用 `<input type=file>` 选路径（被拒：那是笔记本磁盘）。

### D12 安装 Skill 与配套维护 Skill 分源

- **Decision**: 新技能源放 `skill-manager/assets/skills/skill-manager-install/`，入库方式与配套维护 Skill 相同（本仓库为普通上游）。内容：clone/Host/服务/对接；CLI 选型锁定后追加登录。不把维护规范抄进安装 Skill。
- **Rationale**: FR-005。
- **Alternatives**: 扩写现有维护 Skill（被拒：职责混在「管库」与「把软件装到新机器」）。

### D13 内置 Agent 的核心是自动给足两层上下文，不是禁止外部 Agent

- **Decision**: 用户难受的是为外部 Agent **创造上下文**，不是「必须用外面的模型」。每条消息由 Host 自动附上两层快照：`global`（当前 Host 的库级摘要：技能/分类/标签、待更新、新增、译文待更新、部署、失败与暂停、以及预设/Hermes/工具/本机 Agent 目录等 GUI 已知事实，举例非穷尽）与 `view`（当前页及打开对象的详情、上游、部署等）。**客户端只报页面与选中引用；`global` 与 `view` 详情都在发消息时从 core 查询**，不由浏览器拼、不另建技能库。快照形态是摘要 + 计数 + 引用，不把整库正文塞进每条消息。不因此做冲突合并 UI。无对话里程碑时仍用 002 提示。
- **Rationale**: FR-015/021 Clarifications 2026-11-11。
- **Alternatives**: 只给当前页（被拒：不够，用户仍要点名「待更新的那些」）；只给冲突事故（被拒：冲突只是全局事实的一类）；每条消息贴全文（被拒：体积与重复，细节走 CLI）；浏览器把详情 JSON 推给 Host（被拒：易过期，且等于让用户侧拼现场）。

### D14 主动查询走 ACP 工具接现有 CLI；短说明 Skill 只讲规则；不另做用户向 MCP 产品

- **Decision**: 快照推送不够时，Agent 用 ACP 工具拉取（读正文、列待更新、查部署等），工具 = 经 Host 转发的 002 JSON CLI（含现有 `translation_stale` 等字段）。向真 CLI 声明这些工具必须走 ACP `session/new.mcpServers`（stdio + JSON Schema），不能只塞进 `initialize`。**不为对话新增 002 筛选开关。** Host 注入与安装 Skill、配套维护 Skill 分开的 `skill-manager-builtin-agent` 短说明，不把维护 Skill 的「每次询问上传」套进来。不另做给 GUI/用户用的 MCP 网关；stdio MCP 只是 ACP 声明 Host 工具的协议通道。
- **Rationale**: 真 CLI 只认协议里的 mcpServers / MCP tools/list。查询能力要有；配套维护 Skill 面向外部编码 Agent，规则与内置路径冲突。`skill list` JSON 已能表达译文时效，不必为 003 改 002 命令表。
- **Alternatives**: 只靠快照、不准再查（被拒：不可能把全部正文放进每条消息）；只把 descriptors 塞进 initialize（被拒：真 CLI 调不到）；把维护 Skill 原文塞给内置 Agent（被拒：FR-020 与 FR-038 不同）；新增 `--translation-stale` 一类 002 flag（被拒：现有 JSON 字段足够，003 不重写 002 契约）。

### D15 注入路径、快照边界、取消与模型外发（查漏补缺）

- **Decision**:
  1. `skill-manager-builtin-agent` 是应用资产，由 Host 注入到 **D9 锁定的那家 CLI 会搜索的 skills 目录**；若该 CLI 无此目录，改为 ACP 初始化/系统文本。**不入库**、不出现在用户技能列表、不可部署。与安装 Skill（按普通上游入库）不同。
  2. 探测清单须记录该 CLI 的注入路径（或「无目录、走初始化文本」）。
  3. 快照与工具结果会进入该 CLI 所连接的模型；第一版不脱敏。Agent 只看见**当前 Host**，看不见其它设备的库。
  4. 用户可取消进行中的对话任务；取消后不得写成成功，也不得继续写库。
  5. 第一版对话是该机上可继续的时间线，不要求多会话工作区（分叉/文件夹）。
  6. 工具失败必须如实说明，不得假装读到正文。
- **Rationale**: 2026-11-11 上下文讨论查漏补缺。安装 Skill 入库是给外部编码 Agent 用的；内置说明若入库会污染用户库并与「不是查询 API」冲突。
- **Alternatives**: 把内置说明当库内技能（被拒：用户会看到/误部署，且与维护 Skill 混淆）；为每家 CLI 写不同正文的查询 Skill（被拒：查询走工具）；产品内做脱敏网关（被拒：第一版过重，与「选用即接受」不符）；多会话工作区（被拒：超出本功能）。

## 探测清单（D9，实施「可对话切片」前执行）

对每个候选 CLI 记录：能否 stdio ACP 握手、流式输出、取消、声明并调用外部工具、在 macOS 与 Windows 无交互安装、**内置管理说明应写入的 skills 目录路径**（没有则记「初始化文本」）。通过者锁定；全部失败则停在「对话不可用」并说明，不得用假回复冒充。
