# 文档导航与维护

本文件说明项目文档的职责和阅读入口，不重复业务规则或技术方案。
按当前任务读取相关资料，不默认加载整个 `docs/`、`specs/` 或全部历史记录。

## 文档结构

| 位置 | 职责 | 何时读取 |
|---|---|---|
| [README.md](README.md) | 对外项目概况与安装/开发入口；不写本机路径、内网地址或验收过程 | 缺少项目上下文时 |
| [docs/user/](docs/user/README.md) | 给使用者的安装、功能说明、管理站（含可选 Docker）；不写本机路径或内网地址 | 改安装/使用说明、或用户问「怎么装、界面是干什么的」时 |
| [.specify/memory/constitution.md](.specify/memory/constitution.md) | 长期原则、技术栈与架构基线 | 制定需求、设计或实现方案，以及判断是否偏离原则时 |
| [GLOSSARY.md](GLOSSARY.md) | 领域术语与边界 | 对库、管理域、同步、部署等概念有疑问时 |
| [docs/requirements.md](docs/requirements.md) | 核心需求、访谈结论、修订与未决事项 | 确认用户意图、查找业务规则或继续需求讨论时，定位相关章节阅读 |
| [docs/legacy-skill-capabilities.md](docs/legacy-skill-capabilities.md) | 旧能力盘点及当前范围裁决 | 核对旧能力是否保留、调整或暂缓时；区分已确认裁决与后面的旧事实清单 |
| `docs/adr/` | ADR（架构决策记录），说明跨功能边界及原因 | 任务涉及对应决策时，例如共享库与本机状态的边界、本机 Host 与统一管理站 |
| [docs/technical-reference/README.md](docs/technical-reference/README.md) | 本项目技术参考索引 | 需要接入数据、素材、实现经验或旧行为教训时，先查索引，再读对应主题 |
| `docs/tasks/` | 提取、调查等专项任务单 | 执行或复查对应专项任务时；不是功能开发任务的统一入口 |
| [design/](design/README.md) | 前端设计层：看设计、改原型、把正式界面改到与原型一致 | 任何会改变 GUI 外观或交互的工作；先读本入口再改原型 |
| `specs/<编号-功能名>/` | 按功能组织的 Spec（规格说明）、设计、计划、开发任务与验收资料 | 开展该功能工作时，只读对应目录中与任务有关的文件 |

`.specify/templates/` 与 `.agents/skills/` 是流程模板和技能说明，不是产品需求。
使用对应流程时再读取，不为了了解产品而批量加载。

## 构建产物与提交

- 可分发安装包统一放在仓库根目录的 `release/`，安装包必须纳入 Git 提交；`skill-manager/target/` 只保留可重建的构建过程产物，不提交。
- `release/` 按平台各保留一份安装包。生成新包前删除同平台的旧包，无论旧包的版本是否相同；仓库不累积历史安装包。
- 提交前检查 `git status` 和暂存区差异，确认 `release/` 中各平台的安装包均已被 Git 跟踪。
- 推送只维护 GitHub：用户说"提交并推送"时，推 `github`（`YYFF925/SkillManager`）即可。自建 Gitea `origin` 不再推送，也不再作为交付条件。
- 制作可分发安装包或发布应用更新的提交，必须先把 SemVer 的 patch（第三位）加一，并同步 `skill-manager/Cargo.toml` 的 `workspace.package.version`、`skill-manager/package.json` 与 `skill-manager/src-tauri/tauri.conf.json`。不得因「只是重造包」而不升版。可用 `node skill-manager/scripts/bump-version.mjs`。major / minor 仅在用户明确要求时提升，并重置更低位。
- GitHub 源码仓库 `YYFF925/SkillManager` 公开，Issues / Pull Request / Wiki / Discussions 关闭，不接外部贡献。在线更新源就是该仓库的 GitHub Releases：安装包挂版本 tag，各平台 `latest-*.json` 挂稳定 tag `updater-manifests`。不是 git 里的 `release/`。`release/` 仍按平台各留一份安装包并纳入本仓库提交。
- 构建更新包前用凭据 `/service/skillmanager/updater` 注入 `TAURI_SIGNING_PRIVATE_KEY`，再执行 `tauri build`；当前平台发布用 `node skill-manager/scripts/publish-github-update.mjs`。
- 双平台发版走 GitHub Actions：推送 `v*` tag（或手动 `workflow_dispatch`）后，macOS 与 Windows 标准 runner 签名构建并发布到 GitHub Releases。私钥只放 GitHub secret `TAURI_SIGNING_PRIVATE_KEY`，不进仓库。公开仓标准 runner 不收费。构建完成后从 Release 下载 Windows 安装包替换 `release/` 旧包并推到 `github`；Actions 产物不会自动进入 Git。完整步骤见 [docs/technical-reference/app-updater.md](docs/technical-reference/app-updater.md)。
- 管理站 Docker 镜像由 `.github/workflows/station-image.yml` 构建并推到 GitHub Container Registry（`ghcr.io/yyff925/skill-manager-station`），不进 Git。同一 `v*` tag 会编桌面安装包和管理站镜像。镜像只含 `station serve`，不含 Host。第一份 Package 若是私有，在 GitHub Packages 设置里改为 Public。

## 按任务阅读

- 先判断任务属于哪个功能及哪类工作，再用搜索定位相关章节；需要理解边界时补读上下文。
- 功能目录中，`spec.md` 描述行为与验收，`plan.md` 描述实现设计，`tasks.md` 描述开发工作。
  后两者存在且与任务相关时再读，不假设每个目录都已具备完整开发资料。
- 任何会改变 GUI 外观或交互的工作，先走 [design/](design/README.md) 设计层：改 `ui-prototype/prototype.css` 与 `index.html`，同一轮按 `.agents/skills/frontend-design/SKILL.md` 对照表把 `skill-manager/` 前端改到与原型所有可见细节一致。不要按页面停下来确认。
- 修改原型时，按需读取该功能的 `visual-reference.md`、`ui-interaction-audit.md` 及
  `ui-prototype/README.md`，再定位原型实现。
- `specs/002-skill-management/` 是正式产品规格（spec.md 为行为与验收，checklists/ 为需求质检），
  `specs/001-skill-management-prototype/` 定义交互原型；原型中模拟数据和演示实现
  不代表正式产品已经实现，也不自动决定正式接口、存储或同步方案。正式界面的视觉真值是设计层当前原型，不是旧版 FeiSkillDock 截图。
- 技术参考足够回答问题时，无需回读旧项目。确有证据缺口时才按来源定位核查，
  不因参考来自旧项目就继承旧架构、旧数据格式或旧业务规则。

## 长期记忆库（Hindsight）

本项目的跨会话记忆存放在 Hindsight 记忆库（自托管于 NAS，Vectorize AI）。
记忆库是仓库文档的补充，不是替代：与仓库文档冲突时以文档为准，并修正记忆库中的过时条目。

- 项目库：`bank_id = coding-agent::SkillManager`（旧库 `skillmanager` 已于 2026-11-08 并入该库并删除）；个人通用偏好存 `Fei` 库，不混入本项目库。
- 访问方式：仓库内 `.cursor/skills/hindsight-memory/SKILL.md` 与 always-on 规则 `.cursor/rules/hindsight-memory.mdc`（本机还可有 `~/.agents/skills/hindsight-memory/SKILL.md`）。
  优先 MCP 工具；已验证的 HTTP 端点是根 multi-bank `https://hindsight.feiron.top/mcp/`，调用时显式传 `bank_id: "coding-agent::SkillManager"`。
  Cloud Agents 不读仓库 `.cursor/mcp.json`，须在 cursor.com/agents MCP 面板登记同一 URL；Bearer 来自 Infisical `/service/hindsight` 的 `MCP_BEARER_TOKEN`，不得写入仓库。
  MCP 未覆盖的操作走技能自带的 REST 脚本（如知识库导出）。
- 开工前：用 `recall` 检索当前任务相关的历史事实——验收方式与实操经验、跨机同步踩坑、已定裁决、
  用户对本项目的工作方式要求，避免重复踩坑。检索为空不等于事实不存在，换 query 再试或如实报告未命中。
- 收工时：把新出现的稳定结论用 `sync_retain` 写回 `coding-agent::SkillManager` 库，标签带 `skillmanager`
  及主题标签（现用：`acceptance`、`gui-acceptance`、`computer-use`、`git-sync`、`vault`、`feedback`）。
  存入范围：需求裁决、架构与数据边界决定、实施与验收经验、部署与跨机同步约定、用户反馈。
- 不入库：代码与文档中已可查到的实现细节、一次性的任务过程数据、可从 Git 推导的内容。

## 解释与维护规则

- 每次修改都必须按对应 Spec（规格说明）的规范同步维护文档：先确认适用规格与范围，行为变化更新 `spec.md`，实现方案更新已有 `plan.md`／契约，开发工作与验证结果更新 `tasks.md`、`quickstart.md` 及对应验收记录。纯实现修复不虚构新需求，不为普通改动另建文档层级；交付时说明修改对应的规格及实际验证结果。
- 用户最新明确指令优先。文档冲突时核对适用范围、状态和后续修订，指出冲突并修订对应记录，
  不用旧记录回避用户意图。
- 原则约束长期方向；已确认的功能规格指导对应功能；访谈记录提供需求依据；
  技术参考、旧能力盘点和任务回执不替代业务规格。不能仅凭文件位于 `specs/` 就把草案当成正式定案。
- 当前“暂不做”与实现细节不是永久禁令；需要调整时按实际场景说明，不擅自升级为项目原则。
- 新结论写到对应职责的文档中，优先更新已有定义，其他位置引用即可，避免多份现行规则独立维护。
  原型调整若引入新业务需求，应同步记录到需求文档或对应功能规格；纯视觉调整写入相关设计资料。
- 不为一次普通改动新增文档层级、重复总结或额外流程；任务需要哪些资料，就读取和维护哪些资料。
- 根 README 面向公开仓库：只保留产品说明、安装入口与开发索引。本机绝对路径、内网地址、验收日志和访谈过程写在 `specs/` 或 `docs/`，不写进根 README。
