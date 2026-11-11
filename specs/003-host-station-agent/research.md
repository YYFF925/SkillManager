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

- **Decision**: 管理站登录后下发与桌面相同的前端。`isDesktop()` 为假时走 HttpTransport，隐藏托盘/关窗/应用更新；路径输入按文本（Host 侧校验存在性）。登录页、设备条、对话面板是新界面，必须先改 [设计层](../../design/README.md) 原型再改 React。
- **Rationale**: FR-012；设计层工作流。
- **Alternatives**: 为远程另写一套页面（被拒：两套规则）；在浏览器里用 `<input type=file>` 选路径（被拒：那是笔记本磁盘）。

### D12 安装 Skill 与配套维护 Skill 分源

- **Decision**: 新技能源放 `skill-manager/assets/skills/skill-manager-install/`，入库方式与配套维护 Skill 相同（本仓库为普通上游）。内容：clone/Host/服务/对接；CLI 选型锁定后追加登录。不把维护规范抄进安装 Skill。
- **Rationale**: FR-005。
- **Alternatives**: 扩写现有维护 Skill（被拒：职责混在「管库」与「把软件装到新机器」）。

## 探测清单（D9，实施「可对话切片」前执行）

对每个候选 CLI 记录：能否 stdio ACP 握手、流式输出、取消、声明并调用外部工具、在 macOS 与 Windows 无交互安装。通过者锁定；全部失败则停在「对话不可用」并说明，不得用假回复冒充。
