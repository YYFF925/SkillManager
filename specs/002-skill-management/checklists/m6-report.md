# M6 验收报告：共享目录技能浏览（2026-10-16，规划补齐）

规格依据：`spec.md` 的 FR-042（已扩到"本机目标"）与 FR-055；契约：`contracts/cli.md` 的 `agent` 命令组；
规划：`plan.md` 的"规划补齐：共享目录技能浏览"；需求来由：`docs/requirements.md` 的"共享目录技能浏览"一节。
实现任务：`tasks.md` T101～T108。

**这一批的性质**：原型「共享 Agent」页（`renders/03`）在需求链上被遗漏（见 `m5-report.md` 第六节第 9 条），
用户确认该能力必须做后按整条链补齐（需求 → 规格 → 规划 → 契约 → 任务 → 实现 → 验收）。

## 一、场景核对表（quickstart 6.1～6.4）

| # | 操作 | 实际 | 结论 |
|---|---|---|---|
| 6.1 | 未设置共享目录：CLI 与 GUI | CLI：`agent local-skills shared` 与 `agent delete-local shared <名>` 均以 **exit=1** 返回「目标无法解析：尚未设置共享目录路径（shared 是部署目标，不是第二个管理库）」；GUI：入口保留，页面显示「尚未设置共享目录路径（共享目录是编码 Agent 的部署目标），因此无法列出技能。」+「前往设置」，面包屑标"未设置"，**没有**渲染成空列表或"没有符合条件的技能" | 通过（证据 `evidence/shared-agent-unset.jpg`） |
| 6.2 | 设置共享目录后列出 | `settings set-shared-dir --path /tmp/vfy/agents-skills` → `deploy to shared code/helper` → `agent local-skills shared` 输出：`shared：/tmp/vfy/agents-skills · 2 项（受管理 1）`，`helper（已同步）` 与 `loose-helper（仅本地）`；GUI 同一页显示 `1 个受管理`、卡片分别标「已同步 · 受管 · code/helper」与「仅本地」，并给出筛选（全部／受管理／仅本地／有变更）与既有说明「仅本地：直接删除实际文件，不改 SkillManagerVault；受管理：按撤销部署规则处理」 | 通过（证据 `evidence/shared-agent-listing.jpg`） |
| 6.3 | 删除：仅本地项与受管项 | 由集成测试覆盖：`delete_local_shared_removes_unmanaged_and_undoes_managed` —— 仅本地项删除该目录实际文件且不改库；受管项按撤销部署移除副本并清理部署记录、库内技能保留 | 通过（自动化）；**GUI 内的真实点击删除未做**（见第四节） |
| 6.4 | 对照：`agent local-skills codex` | 集成测试 `local_skills_for_agent_key_is_unaffected_by_shared_support` 通过：Agent 目录行为与本次变更前一致 | 通过（自动化） |

## 二、自动化验证（事实）

| 命令 | 结果 |
|---|---|
| `cargo test --workspace` | **256 项通过、0 失败**（基线 252 + 本批 4 项；主代理独立复跑确认） |
| `cargo build --workspace` | 通过，**0 警告** |
| `npm test` | **36 项通过**（基线 33 + `AgentPage.test.tsx` 3 项），8 个文件 |
| `npx tsc --noEmit`、`npm run build` | 通过 |

测试先行证据：实施前新增的 4 个 CLI 用例全部 FAILED，失败原因是 `shared` 被当成未知 Agent key
（`目标无法解析：shared 不在内置 Agent 名单中`），即当时既无该能力、也无"未设置"语义。

前端证伪：把"未设置共享目录时的提示分支"改为恒假 → `未设置共享目录路径时保留页面，如实提示并给出前往设置入口` 用例变红；
把"未设置"错当成空目录 → 同一用例仍红。恢复后全绿，说明断言能区分错误界面。

## 三、实现方式（复用而非另造）

- 核心复用：`LocalSkillService`（目录扫描 + 按部署记录判定受管/未受管 + 与库内索引比对）入口从 **agent key** 泛化为
  **目标**；路径解析复用 `TargetResolver::resolve_with_override` 的 `Target::Shared` 分支（未设置时它本就如实报错）。
- 受管判定沿用该目标的部署记录（Agent 为 `agent:<key>`、共享为 `shared`），未自造字符串格式，未新增字段。
- CLI 复用既有目标解析（`parse_target_arg`）；GUI 复用 Agent 页既有的本机技能视图（`LocalSkillsView`），
  只换目标，未复制渲染逻辑、未新增页面。
- 未做（按需求边界）：不把共享目录变成可登记 Agent 或第二个管理库；该页不提供"从库部署"与收编。

## 四、未过与未取证项（保持未过，不静默降级）

1. **GUI 内真实点击删除未取证**：本批只验证到删除入口存在与自动化覆盖（6.3）。真实点击删除受管/仅本地项的效果
   留待下一轮 GUI 取证。
2. **导航徽标缺失**：原型该页侧栏显示技能/部署数量（`renders/03` 的 7/27 之类），当前实现的 Agent 分组各项都没有数字徽标，
   本页沿用同一处理，未新增计数接口。属与效果图的已知差异（与原型其它 Agent 页现状一致）。
3. **面包屑**：共享页顶部沿用复用视图的面包屑（`全部 Agents / 共享 Agent`），原型该页没有面包屑；为满足"复用同一视图"未加分支。
4. **共享目录的域约束**：`shared` 只对编码域成立（FR-021），但 `local-skills` 本身只列目标目录、不做域校验，
   约束由共享目录设置入口与部署准入承担；本页未额外加校验。
5. **平台**：仅 macOS（arm64）实测，未涉及其它平台。

## 五、报告与记录的更新

- `m5-report.md` 第六节第 9 条状态更新为"已按链条补齐"，指向本报告。
- `visual-comparison.md` 的 B 清单中"没有共享 Agent 页"一项在本批后不再成立，该文件第二节 03 行与本报告结论以本报告为准。

## 六、`hermes:default` 作为 `<target>` 取值的登记（2026-10-16 追补）

本批把 `local-skills` / `delete-local` 的 `<target>` 从"只按 Agent key"扩到"按目标"时，复用了既有目标解析
`parse_target_arg`（对 `shared` 或含冒号输入走通用目标语法），因此 `hermes:default` 被**顺带**接受，
但规格、契约、`plan.md`、`docs/requirements.md`、`README.md` 与 CLI 帮助文本当时只写了"内置 Agent key 或 `shared`"。

经用户裁决，**选择放开并登记**（而非收紧为只接受 `shared`）：`<target>` 正式接受内置 Agent key、`shared`
或 `hermes:default` 三类本机部署目标；`hermes:default` 指向 `<Hermes 根>/skills`（default 档案即 Hermes 根自身，
受管判定沿用 `target=hermes:default` 的部署记录），据此同步更新上述规格、契约、规划、需求、README 与 CLI 帮助文本。
**行为逻辑不变**（`parse_target_arg` 与 core 未改），另在 `tests/cli_agent.rs` 增加用例
`local_skills_and_delete_local_support_hermes_default_target` 把该取值固定下来。

## 七、设置页与 Agent 页按原型归位（2026-10-16）

规格依据：`spec.md` 的 FR-041（补"入口位于设置页"）、FR-056（设置页分组与只读行）、FR-057（跨目标部署清单）；
规划：`plan.md` 的"规划补齐：设置页与 Agent 页按原型归位"；实现任务：`tasks.md` T109～T112。
**这一批的性质**：用户核对原型设置页指出实现缺内容，逐区块对照后把 Agent 检测区块与设置页分区按原型归位、
把「全部 Agents」改成跨目标部署清单。

**本批范围（已实现，事实）**：

- 新建 `skill-manager/src/components/AgentSettings.tsx`：把设置页所需的 Agent 检测区块从 `AgentPage.tsx` 抽出为
  可复用组件——「支持的 Agent（已检测/内置总数）」标题、摘要「已检测 N · 已启用 M」、全部启用／全部禁用／重新检测、
  已检测 Agent 三列卡片网格（拖动排序、启停、技能目录与项目相对路径行内编辑、恢复默认）与「其他支持的 Agent」折叠区；
  由 `SettingsPage.tsx` 作为**首个区块**渲染，不再在 Agent 页承载。
- `SettingsPage.tsx` 区块顺序按原型：支持的 Agent → 全局配置 → 自己的 SkillManagerVault 同步 → 后台检查与上游更新 →
  诊断与日志；全局配置补只读展示行（部署方式=文件复制、语言=简体中文、关闭行为=托盘模式、托盘／菜单栏图标=固定开启）；
  后台检查区补「立即检查」（复用库页「检查全部」（`checkUpstream({all:true})`）与「同步」（`syncPull`），未新增组合命令）；
  「自动上传确认」并入同步区块，「技能库容量」与「诊断与运行日志」并入末尾的诊断与日志区块（均为既有分区，未删除）。
- `AgentPage.tsx` 的「全部 Agents」改为**跨目标部署清单**（标题「全部编码 Agents」、副标题「本机部署清单 · 同一个技能可部署到多个目标」）：
  数据来自 `deployStatus()` 的本机受管部署记录，按「技能 × 目标」渲染，支持搜索／筛选（全部／已部署／有变更）／刷新
  与「从库部署」（复用既有 `DeployDialog`）；「全部龙虾 Agent」同构处理。Agent 管理不再出现在该页。

**验证（事实）**：

| 命令 | 结果 |
|---|---|
| `npx tsc --noEmit` | 通过 |
| `npm test` | **42 项通过**（基线 36 + 本批 6 项，新建 `SettingsPage.test.tsx`） |
| `npm run build` | 通过 |
| `cargo test --workspace` | **257 项通过**（本批未改 Rust） |

前端证伪：把设置页首区块替换掉即 `SettingsPage.test.tsx` 相关用例变红，说明断言能区分"首区块是 Agent 检测"。

**刻意未做（附理由，与 `plan.md` 一致）**：

1. **「打开文件夹」**：原型自身是模拟控件（`ui-prototype/index.html:840` 明确"原型不调用系统文件管理器"）。
2. **「托盘／菜单栏图标」开关**：原型的开关只改模拟状态，实现也没有对应产品设置键（托盘行为固定），改为只读行。
3. **「更改目录」（更换 SkillManagerVault）**：换库后本机状态（部署记录／Hermes 启用／工具记录）语义未定，待用户裁决后再入规格。

**未取证（保持未过，不静默降级）**：GUI 实操外观未取证——本批只做类型检查、前端单测（含首区块证伪）、
前端构建与 Rust 测试，**未从 `.app` 启动逐项核对**设置页外观与「全部 Agents」清单渲染；该取证由 T113 在下一轮完成。
另：原型的「仅本地」筛选需扫描各目标目录，本批不做，`DeploymentList` 只做记录侧筛选（全部／已部署／有变更）。

## 八、GUI 实操取证：设置页与 Agent 页归位（2026-10-16，主代理，重建 .app）

从 `.app` 启动（夹具数据）实测本批两处：

- **设置页**：首个区块为「支持的 Agent (3/53)」+「已检测 3 · 已启用 3」+ 三列 Agent 卡（含启停开关、技能目录、项目路径、检测目录）+
  「其他支持的 Agent (50)」；全局配置依次为 SkillManagerVault 目录（只读路径）、共享技能目录（未设置 + 修改）、部署方式（文件复制）、
  主题、文字大小、语言（简体中文）、关闭行为（托盘模式）。证据 `evidence/settings-agent-section.jpg`。
- **「全部编码 Agents」**：已改为跨目标部署清单——本机部署记录 2 项，同一技能 `helper` 分别列出「目标: Codex … 本地有变更」与
  「目标: 共享目录 … 已同步」，含搜索、筛选（全部／已部署／有变更）、刷新、从库部署，并如实说明「本清单只列本机受管部署记录
  （未受管的『仅本地』技能需在对应目标页查看）」。证据 `evidence/all-agents-deployment-list.jpg`。

**未取证（保持未过）**：在 GUI 内实际点击启停、目录行内编辑与「从库部署」；与原型效果图的像素级外观对照。

## 九、初始 SkillManagerVault 接入（2026-10-16）

- 用户将统一技能库名称改为 `SkillManagerVault`；源码、CLI 帮助、界面、配套维护 Skill 及规格资料同步更名。宪章只调整名称，版本为 2.0.3。
- 本机初始库：`/Users/fei/Projects/workspace/SkillManagerVault`；Gitea 私有仓库：`yfpoyuas/SkillManagerVault`，`main` 跟踪 `origin/main`。包含两管理域、空预设、空全局指令源、空上游决定列表及工具目录，无测试数据。
- 使用用户指定的 shared-credentials（共享凭据）技能访问 Gitea API 建库，为新库登记本机现有 SSH 公钥作为可写部署密钥；Token 不落库。远端为 `ssh://git@192.168.200.7:2222/yfpoyuas/SkillManagerVault.git`。
- 空远端尚无引用时，现有 `sync push` 的 `ls-remote --exit-code` 检查会报不可访问；初始提交由普通 Git 完成。之后用项目 CLI 接入真实本机状态，验证状态、拉取、无改动上传以及实际修改后的提交上传，均成功。最新库提交为 `2cd3831`。接入前核实部署、Hermes 启用与工具安装记录均为零，无需清理这些记录。
- GUI 实测显示新库路径、零技能及远端「已连接」。用户已允许本地网络权限，系统设置中开关为开启。原开发包只有链接器 ad hoc（临时签名），签名标识与应用标识不同且未绑定 Info.plist，GUI SSH 持续报 `No route to host`；应用包签名并统一标识、重启后连接成功。打包配置保留 `bundle.macOS.signingIdentity = "-"`。这是本机验证结果，不声明覆盖其他系统或正式发布签名。
- 自动化验证：Rust **257 项通过**、前端 **42 项通过**，类型检查、前端构建与桌面打包通过。文件监听测试在沙箱内两次超时，沙箱外全量复跑通过，未为此修改产品或测试。
- 最终签名配置重新打包并重启后，GUI 实际点击「同步」返回「远端无新提交，已是最新」。另一份临时克隆从初始提交出发，使用独立本机状态执行 `sync pull`，成功拉取 1 个新提交至 `2cd3831`；README 内容核对一致，部署刷新 0 项，指令源未变化。
- 配套维护 Skill 尚未入库：Gitea 项目上游当前没有该技能；本次没有扩大新库上传授权来推送项目源。待源推送后按 FR-051 从普通上游入库，不伪造来源或采用版本。

## 十、共享目录自动识别修订（2026-10-16）

用户要求共享目录自动识别，不再手动设置。本次修改 FR-021/042/050/055/056、数据模型、计划、CLI 契约、quickstart、需求记录与配套维护 Skill；旧的手动路径与“未设置”验收属于此前行为，以本节修订为准。

- core 按系统当前用户主目录解析 `.agents/skills`；移除持久化路径读写、CLI `set-shared-dir`、Tauri `shared_dir_set` 及设置页、部署弹窗的编辑路径。旧设置不读取，不提供兼容或迁移。
- 自动化验证：Rust **258 项通过**、前端 **42 项通过**，类型检查、前端构建、桌面打包通过。覆盖无需配置即可部署、浏览不存在的目录不创建、首次部署创建、已有技能识别与受管/未受管删除、旧设置不生效以及旧写命令移除；写入和删除测试使用临时 HOME 与独立本机状态。
- 真实 GUI：从最终 `.app` 启动，共享 Agent 页直接显示 `/Users/fei/.agents/skills`，列出已有 `shared-credentials`，标记「仅本地」，不再要求前往设置。设置页只读显示同一路径与自动识别说明，未出现共享目录修改按钮；Gitea 同步仍为「已连接」。未在真实目录执行部署或删除。
- 本机运行验证为 macOS；其他系统的实际运行未验证，路径解析仍统一使用系统用户主目录。

## 十一、本机技能交互优化（2026-10-16）

依据用户提供的 Codex 空列表、共享技能卡片截图及三项明确要求，修订 FR-014/042/052/055 与收编范围。此前“共享目录不收编”的裁决已撤销；相关旧验收记录不作为当前限制。

- 本机目标列表无技能或筛选无结果时留白，移除空状态面板及底部“仅本地／受管理”解释。
- 普通 Agent 与共享目录未受管卡片均显示可见的「纳入管理」，勾选后可批量收编。共享收编复用既有复制入库、同名检查及部署接管，进入编码库，仅接管原共享目标。
- 点击卡片打开只读详情，阅读实际本机副本的正文及目录内文件；受管项可前往技能库管理。新增同源 CLI `agent read-local <target> <name> [--file <路径>]` 与 Tauri `agent_read_local`，不以库内原件替代副本。勾选、复制、收编、删除动作不冒泡打开详情。
- 可用按钮、导航、选择控件、文件入口统一使用手型光标；本机技能卡片也显示手型及悬停反馈，禁用控件维持非手型。
- 自动化：Rust **260 项通过**、前端 **43 项通过**，TypeScript 类型检查和最终桌面打包通过。新增用例在临时 HOME 中验证共享收编进入编码库、原目标变受管且不新增 Codex 部署；本机正文与库原件不同、嵌套文件阅读、非 UTF-8 文件及目录外路径拒绝。前端验证空列表留白、卡片正文和文件阅读、收编入口以及勾选/删除不误开详情。
- 真实 GUI：Codex 空页只保留标题、路径及筛选；共享卡片显示「纳入管理」且底部无说明；打开 `shared-credentials` 详情成功阅读本机 `SKILL.md` 和 `references/cli.md`，从详情进入共享收编对话框成功，随后取消。未提交收编或修改真实技能；截图目视核对卡片布局正常。

## 十二、卡片快捷部署与界面精简（2026-10-16）

按用户最新六项要求修订 FR-014/021/052、需求、规划、CLI 契约和维护 Skill；此前预演及文字收编入口的验收以本节修订为准。

- 本机技能卡片与批量栏的收编入口只显示图标，保留悬停说明和无障碍名称；未受管技能需先收编才能从库部署。
- 技能库、预设、本机目标/跨目标部署清单及 Hermes 档案的技能卡片右下角显示本管理域已检测且启用的 Agent 图标。按本机记录亮暗显示，点击即时部署或撤销，完成后刷新真实状态；实际失败继续展示原因。
- 部署与预设弹窗共用按状态选择、点击「保存」应用差量的交互，移除部署/撤销模式切换。已有部署初始化亮显，多技能部分部署显示数量；未改项保持原样。部分失败保留弹窗和具体原因，成功项更新基线，重试不重复执行成功项。隐藏未检测、未启用与特殊伪 Agent 选项；已有隐藏目标记录不受保存影响，显式共享目录与 Hermes default CLI 目标保留。
- 预演功能从 core、CLI、Tauri、前端调用及界面全部移除；旧 `--dry-run` 参数被拒绝，不继续提供兼容接口。
- 清理页面副标题、部署说明/脚注、技能库与上游检查的重复说明、收编/入库/工具详情和设置页中的实现规则。保留字段、路径、状态、真实错误、暂停定位复制、删除/跨预设撤销影响与自动上传范围等必要信息。
- 验证：Rust **260 项通过**；前端 **47 项通过**；TypeScript 类型检查、生产前端构建和带现有临时签名的 macOS 桌面打包通过。界面用例覆盖真实记录初始化、只列有效 Agent、保存前无副作用、同时部署与撤销、多技能部分部署、不支持时允许撤销、部分失败重试不重复成功项、读取失败阻止保存、预设撤销提示，以及快捷图标的即时操作/错误/事件不冒泡。CLI 验证四种部署命令拒绝旧预演参数。
- 最终 GUI 实测：编码库卡片显示 7 个已启用 Agent 图标；弹窗点击 Codex 后亮显且保存可用，再次点击暗显且无变化时保存禁用，随后关闭。龙虾库卡片与弹窗只显示 Hermes Agent，不出现原来的其他 5 项、Hermes default 伪 Agent、预演或长篇说明；设置页保留自动共享目录、连接状态及必要配置。测试未点击真实保存或快捷部署，真实目录的写入/删除由独立临时环境中的测试验证；本机部署仍为 `code/shared-credentials → shared` 一项。

## 十三、技能详情与入库界面优化（2026-10-16）

按用户最新六项界面要求修订 FR-021 与需求记录；本轮只调整已确认流程的呈现和点击路径，不改变管理域、支持声明、部署准入、预演移除或上传确认规则。

1. **Agent 图标密度**：技能卡片右下角快捷部署图标由 28px/gap 4px 收敛为 24px/gap 2px，并保持整组右对齐。实现：`QuickDeploy.tsx`。
2. **详情内直接部署**：「文件与部署」中的目标项直接可点击部署/撤销，已部署与未部署状态沿用真实记录；移除「本机已有部署」分组标题，目标名称前显示对应 Agent 图标，「文件与部署」页不再显示页脚“部署”按钮。实现：`DeploymentStatus.tsx`、`SkillDetail.tsx`。
3. **移除管理专用目录**：技能详情不再单列 All skills 共用的 `.manager/` 说明栏；原始文件树仍排除 `.manager`。实现：`SkillDetail.tsx`。
4. **详情副标题精简**：技能名下方只显示 `自建` 或上游仓库地址，不再显示“技能原名保持不变 …”。实现：`SkillDetail.tsx`。
5. **来源页去重**：来源页移除与「文件与部署」重复的“库内位置”字段，保留来源地址、引用、子目录、版本及已有登记资料。实现：`SkillDetail.tsx`。
6. **入库管理域下拉**：将原生 select 改为锚定在触发框正下方的自定义下拉，避免原生菜单位置漂移；仍使用同一 `targetDomain` 状态和重新预览规则。实现：`ImportDialog.tsx`。

**验证（事实）**：

| 命令 / 操作 | 结果 |
|---|---|
| `npx tsc --noEmit` | 通过 |
| `npm test` | **47 项通过**（9 个测试文件） |
| `npm run tauri build -- --debug` | 通过，macOS `.app` 重建并完成现有临时签名 |
| CUA 实测技能卡片 | 图标组已收紧并保持右侧对齐 |
| CUA 实测技能详情 | 副标题仅显示“自建”；「文件与部署」无管理专用目录、无“本机已有部署”标题、无页脚部署按钮；目标项带图标且可点击；来源页无库内位置字段 |
| CUA 实测入库弹窗 | 管理域选项展开在触发框正下方，显示选中态且未改变预览/入库规则 |

未执行真实点击部署/撤销；本轮没有改写本机部署记录或 SkillManagerVault 内容。

## 十四、技能库横向空间与图标密度优化（2026-10-16）

对照旧版软件截图后，确认技能库左侧分类列被固定为 152px、页面内容区最大宽度 1200px，宽窗口下技能卡片没有充分利用横向空间；快捷部署图标仍保留额外间隔。

1. **分类列与卡片宽度**：技能库、Hermes 档案、预设详情的分类列由 152px 收窄为 132px，分类列与内容间距由 20px 收窄为 16px；页面内容区最大宽度由 1200px 放宽为 1440px。宽窗口下三列卡片获得更大宽度，窄窗口仍按现有响应式规则降为两列。实现：`AppShell.tsx`、`LibraryPage.tsx`、`HermesPage.tsx`、`PresetPage.tsx`。
2. **图标继续收紧**：技能卡片快捷部署图标的点击区由 24px 调整为 22px，图标间距由 2px 调整为 0；仍保持整组右对齐与原有部署/撤销行为。实现：`QuickDeploy.tsx`。

**验证（事实）**：`npx tsc --noEmit`、`npm test`（**47 项通过**）与 `git diff --check` 通过；调试服务运行中已由 Vite HMR（热更新）加载改动，CUA 实测技能库卡片与分类列布局更新，应用窗口保持打开。未执行真实部署/撤销，也未修改 SkillManagerVault。

## 十五、管理资料标签与技能正文渲染优化（2026-10-16）

1. **移除管理资料括号说明**：「中文注释（通常不超过 20 字）」「标签（可选已有标签，也可输入新标签）」「分类目录（最多一级，不改部署名）」改为只保留字段名；未分类值由「未分类（位于管理域根下）」简化为「未分类」。实现：`ManagerEditor.tsx`。
2. **正文 Markdown 结构化展示**：技能详情正文不再使用 `pre` 直接显示原始 Markdown，新增 `MarkdownView`，渲染 frontmatter 元信息、标题、段落、列表、引用、分隔线、行内链接/代码与围栏代码块，并提供独立阅读样式。代码块跟随应用浅色/深色主题，不固定为黑色背景。实现：`MarkdownView.tsx`、`SkillDetail.tsx`、`styles.css`。

**验证（事实）**：新增 `MarkdownView.test.tsx`，确认 frontmatter、标题、链接和代码块被结构化渲染，且不再出现原始 `---` 或 `# 标题`；`npx tsc --noEmit`、`npm test`（**48 项通过**）与 `git diff --check` 通过。Vite 调试服务已重启并运行，确认其实际提供的 `SkillDetail` 已引用 `MarkdownView`。未修改技能正文或管理资料。

## 十六、详情标题锁定、图标部署与即时提示优化（2026-10-16）

1. **详情顶部锁定**：技能详情的页签栏与正文语言/译文状态控件改为吸顶区域；正文滚动时保持可见。实现：`SkillDetail.tsx`。
2. **部署目标图标化**：技能详情「文件与部署」的目标项放入独立边框区域，只显示图标；点击图标仍直接部署或撤销，Agent 名称通过悬浮提示显示。实现：`DeploymentStatus.tsx`。
3. **即时悬浮提示**：新增 `FastTooltip`，替代依赖系统 `title` 的慢速提示；技能卡片快捷部署图标和详情目标图标在鼠标进入后立即显示名称，且只显示 `共享 Agent`、`Codex`、`Cursor` 等目标名，不显示“部署到/撤销部署”动作词。实现：`FastTooltip.tsx`、`QuickDeploy.tsx`、`DeploymentStatus.tsx`、`styles.css`。
4. **共享 Agent 快捷部署**：编码库技能卡片的快捷部署目标补齐「共享 Agent」图标；共享目录仍是本机部署目标，不成为第二个管理库。实现：`QuickDeploy.tsx`。

**验证（事实）**：新增 `QuickDeploy.test.tsx`，验证编码库快捷部署同时提供共享 Agent 与普通 Agent，并存在即时 tooltip；`npx tsc --noEmit`、`npm test`（**49 项通过**）与 `git diff --check` 通过。Vite/Tauri 调试服务与最新应用窗口已重新启动。未执行真实部署/撤销。

## 十七、系统应用图标改用老项目图标（2026-10-17）

用户指出当前 `.app` 系统图标模糊，要求改用老项目 `FeiSkillDock` 的软件图标。

- 从老项目 `FeiSkillDock/src-tauri/icons/` 引入其当前 Tauri 图标集：`32x32.png`、`128x128.png`、`128x128@2x.png`、`icon.icns`、`icon.ico`、`icon.png`。
- 当前 `src-tauri/tauri.conf.json` 的 bundle icon 列表改为与老项目相同，macOS 打包直接使用老项目 `icon.icns`，不再从 32×32 PNG 放大生成。
- 构建后核对 `.app/Contents/Resources/icon.icns` 与老项目源 `icon.icns` 的 SHA-256 均为 `816d8ccc06f60be7283f86ce919bd3f6ff924d17de4fd280edf2622c873e8b58`，确认系统图标已一致替换。

**未改动**：侧栏左上角当前使用的轻量 `app-logo.png` 保持现状；本轮只替换系统应用图标资源。

## 十八、Hermes 档案显示名与下拉说明（2026-10-17）

对照老项目 `FeiSkillDock` 的实现，档案列表应显示 Hermes 面板配置的 bot 显示名，而不是内部目录 id。

1. **显示名读取**：core 的 `ProfileInfo` 新增 `display_name`，优先读取 `<档案 home>/profile.yaml` 的 `ui_meta.hermes-bots.title`，缺失时回退 `SOUL.md` 首个 `# ` 标题；default 档案没有显示名，回退目录名。实现：`crates/core/src/hermes/profiles.rs`。
2. **API 与界面**：`ProfileView`、前端 `HermesProfileView` 增加 `display_name`；档案按钮和下拉选项显示 `display_name ?? name`。实现：`crates/core/src/hermes/enables.rs`、`src/services/client.ts`、`src/pages/HermesPage.tsx`。
3. **移除说明**：档案下拉底部的“本机已发现的档案，不管理档案生命周期。”说明已删除。

**验证（事实）**：新增 core 测试 `named_profile_uses_hermes_panel_title`，从 `profile.yaml` 读取到“绘画助手”；前端测试确认下拉显示“绘画助手”、不显示内部 id，也不显示被删除的底部说明。`npx tsc --noEmit`、`npm test`（**49 项通过**）、`cargo test -p skill-manager-core hermes::profiles`（4 项通过）与 `git diff --check` 通过。

## 十九、预设入口、共享部署目标与分类治理（2026-10-17～18 追补）

本节补记 `fe31c2e`（T130～T132）之后的改动。这些提交当时就地把 FR-003/021/025/037/038 与
`contracts/cli.md`、`data-model.md` 改为现行行为，但 FR-007/052 涉及的显示与托盘行为、以及任务与验收链
当时没有跟上。本节补齐实施事实、验证结果与未取证项；FR-007/052 的现行表述及 `plan.md`、
`docs/requirements.md`、`visual-reference.md` 已同时补齐。任务文件不追加任务号，改以头部
“2026-10-17～18 后续修订”说明并指向本节。

**范围与实现（事实）**

1. **预设入口简化与保存后后台流程**（`c366dad`）：主侧栏「预设」与「工具中心」同级，取消「编码预设 → 全部预设」分组与卡片总览，进入后直接显示页内列表与成员浏览；GUI 保存完成后部署刷新与获授权上传转入后台，耗时同步不再持有界面上下文锁。实现：`src/App.tsx`、`src/pages/PresetPage.tsx`、`src/lib/librarySave.tsx`、`src-tauri/src/commands/{deploy,sync,mod}.rs`、`crates/core/src/context.rs`。规格 FR-025/FR-038；视觉基线见 `specs/001-skill-management-prototype/visual-reference.md` 的预设导航调整。
2. **共享 Agent 部署目标与全局库拉取入口**（`8cd59e1`）：编码库单项、多选与预设部署可选择「共享 Agent」（无已检测 Agent 时仍可选），按支持声明过滤并支持撤销已有部署；全局页头在「Agent 指令」左侧增加库拉取按钮，后台拉取期间保留页面操作。实现：`src/components/DeployDialog.tsx`、`src/App.tsx`。规格 FR-021/FR-037。
3. **分类重命名**（`1b6e2a1`）：core 新增 `vault::manage::rename_category`，移动实际分类目录并一并移动该分类下全部技能，目标分类已存在时拒绝（不做隐式合并），同名调用返回 0 项；CLI 新增 `skill category rename --domain <code|lobster> <old> <new>`；GUI 分类栏提供重命名入口，「全部分类」「未分类」不提供。实现：`crates/core/src/vault/manage.rs`、`crates/cli/src/commands/skill.rs`、`src/pages/LibraryPage.tsx`；契约与设计见 `contracts/cli.md`、`data-model.md` 1.2。规格 FR-003（US1 场景 9）。
4. **Hermes 档案技能池判定**（`1b6e2a1`）：档案列表改为共用一份候选池——龙虾库中已安装进 Hermes 的部署记录，`hermes:default` 与 Agent 目标 `agent:hermes` 视为同一次安装并按技能名去重；为具名档案启用的前置条件由“存在 `hermes:default` 部署记录”改为 default 目录下实际存在该技能目录。实现：`crates/core/src/hermes/enables.rs`；语义登记见 `data-model.md` 2.2。规格 FR-026/027。
5. **托盘左键切换窗口**（`1b6e2a1`）：`show_menu_on_left_click(false)`，左键点击在显示与隐藏主窗口之间切换、显示时取消最小化，右键展开菜单。此前“左键即展开菜单、不另建左键快速恢复分支”的记录由本项取代，规格 FR-052 已同步。实现：`src-tauri/src/tray.rs`。
6. **库列表不支持项位置**（`765cd9d`）：当前系统不适用的技能仍整项灰显，但按原相对顺序排到受支持技能之后，避免插在可用卡片中间。实现：`src/pages/LibraryPage.tsx`；规格 FR-007 已补。
7. **悬浮提示自绘定位**（`1b6e2a1`）：`FastTooltip` 由系统 `title` 改为 portal 自绘，按视口边缘翻转与收边，触发元素上方空间不足时移到下方。实现：`src/components/FastTooltip.tsx`；视觉说明已补一行。
8. **Windows 平台修复与安装包**（`34772cf`、`d547863`、`8bd347a`、`7fc32a0`、`6b95288`、`ea225aa`）：子进程统一经 `crates/core/src/process.rs` 创建（`CREATE_NO_WINDOW`），本地路径来源按系统分隔符取仓库名，集成测试在 Windows 上阻断会改写真实用户目录的用例；打包改为同时产出 `.app` 与 DMG，Windows 产出 NSIS，macOS DMG 纳入 `release/`。属实现与打包层，记录在 `skill-manager/README.md` 与 `AGENTS.md`，不改业务规格。

**验证（事实，2026-10-18 补记时重跑）**

| 命令 / 操作 | 结果 |
|---|---|
| `cargo test --workspace` | Rust **281 项通过**，0 失败 |
| `npm test` | 前端 **73 项通过**（17 个测试文件） |
| `npx tsc --noEmit` | 通过 |

新增或更新的用例：`crates/core/tests/vault_management.rs`（分类重命名含目标已存在时拒绝）、`tests/cli_library.rs`（`skill category rename`）、`tests/cli_hermes.rs`（档案池与启用前置）、`src/components/FastTooltip.test.tsx`、`TaskDialog.test.tsx`、`src/pages/{LibraryPage,AgentPage}.test.tsx`。提交当时记录的验证为：`c366dad` 前端 60 项 / Rust 276 项；`8cd59e1` 前端 69 项 / 9 项预设 CLI 测试；`1b6e2a1` 未记录验证行，由本次重跑补齐。

**未取证（保持未过）**

- GUI 实操未见证：预设入口位置、共享 Agent 部署目标与撤销、页头库拉取按钮、分类重命名、托盘左键切换、悬浮提示定位均只有构建与自动化测试证据，未在真实 `.app` 上逐条操作核对。
- Windows 平台行为未在真实 Windows 环境运行验证，只有单元/集成测试与代码路径说明支撑（见 `skill-manager/README.md`）。
- DMG / NSIS 安装包未做安装与首次启动验证。
- 本轮未修改真实 SkillManagerVault 内容，也未执行真实部署、撤销或上传。


## 二十、桌面安装包随附 CLI（2026-10-18）

**问题与范围**：本机维护 Skill 只有规范与 CLI 参考，原桌面包也未携带独立 CLI；开发机有编译产物不代表新电脑可执行管理任务。本轮补齐 FR-050 安装交付与 US10 场景 5，维护 Skill 仍按 FR-051 从普通上游入库／更新，未另建技能分发机制。

**实现**：Tauri `externalBin` 随桌面包安装 `skill-manager`；开发／打包钩子从当前 workspace 编译对应目标 CLI，失败即停止。macOS 在应用包 `Contents/MacOS/`，Windows 在桌面主程序旁。维护 Skill 和 CLI 契约明确绝对路径调用、首次版本／帮助检查及缺失时如实报告，无运行时复制或 PATH 修改。项目 `AGENTS.md` 新增每次修改按适用 Spec 同步维护相关文档的要求。

**验收（macOS arm64，实际 DMG）**

| 场景号 | 预期 | 实际 | 结论 |
|---|---|---|---|
| 6.13 | 真实安装包中包含同版本 CLI，外部进程无需 PATH／源码／GUI 即可执行帮助与版本 | 只读挂载 DMG，以 `ditto` 复制到项目外带空格的临时安装目录；未启动该 GUI 副本，`PATH` 为空时 `--version` 返回 `skill-manager 0.1.0`（与应用 Info.plist 一致），`--help` 正常且退出码为 0 | 通过 |
| 6.14 | 包内 CLI 在隔离库实际查询／修改，签名有效 | 从复制后的包内绝对路径调用，PATH 仅保留系统 Git 目录 `/usr/bin:/bin`，隔离 HOME／vault／state；`skill list`、`skill set code/smoke --comment`、`skill get` 均返回成功 JSON，磁盘 meta.yaml 与独立 SQLite 真实写入；应用与 CLI 的 `codesign --verify --strict` 均通过（应用另带 `--deep`） | 通过 |
| 6.15 | 技能明确入口与缺失原因，release 唯一安装包受 Git 跟踪 | SKILL.md 与 references/cli.md 已补 macOS／Windows 定位、绝对路径及缺失说明；release 仅保留已跟踪的 `Skill Manager_0.1.0_aarch64.dmg`，其中同时包含桌面程序与 CLI | 通过 |

**构建与回归**：`npm run tauri build` 已产出带签名 `.app` 与 DMG（前端类型检查、生产构建均通过）。沙箱内 `hdiutil` 报“设备未配置”，切换到获准的沙箱外构建后成功；真实 DMG 只读挂载、复制与卸载均成功。Rust workspace 首次在沙箱内执行，已有文件监听测试 `external_edits_refresh_the_index_and_notify` 等待通知超时，其余已执行用例通过；在沙箱外重跑同一 `cargo test --workspace` 后 **281 项全部通过**，包含该监听测试；未改业务代码或该测试。开发模式的 `npm run prepare:cli` 也构建成功。

**安装包**：`release/Skill Manager_0.1.0_aarch64.dmg`，7,691,081 字节，SHA-256 `196b390b64c47eb592b92b13a639097c4ad3b2bbae4cd1926f3cd0c3616a2cc2`。

**平台与交付限制**：Windows 的共用打包配置与技能调用说明已调整，但尚未在真实 Windows 环境执行本轮构建／安装／调用验证；不得据此声称 Windows 验收通过。本轮不改真实 SkillManagerVault 或部署副本，不自动上传项目或技能更新。

## 二十一、YAML 写回保型（2026-10-28）

**问题与范围**：部署合成与 YAML 写回把整块结构经 `yaml_serde::to_string` 重序列化，emitter 按 YAML 1.2 core 判定 plain 标量安全，`"2026-10-08"`、`"12:30"`、`"no"`、`"on"` 等形状被无引号输出；PyYAML 等下游按 YAML 1.1 解析为 date/int/bool，键 `on:` 也会变成布尔键，行内注释丢失。实际事故：Hermes `skill_view` 对 ego-browser 报 `Object of type date is not JSON serializable`。全仓共 7 处写回点（frontmatter 注入、meta.yaml、tool.yaml ×2、source.yaml、presets、决定记录）。

**实现**（tasks.md Phase 21 / T136）：新增 `crates/core/src/yaml_safe.rs`——按 PyYAML resolver 的 1.1 隐式类型谓词 `needs_quotes` 与保守发射器，统一入口 `to_yaml_string`（无危险字符串时与 `yaml_serde` 输出逐字节一致，含 `Value::Tagged` 回退）；`deploy/compose.rs` 注入改为文本级最小编辑：定位唯一顶层 `description:` 行（块标量前缀进首条内容行、引号值前缀进引号内、plain 值原位拼接），编辑后整块重解析校验，不符即「未注入」安全降级；`--stdin` 注册经 JSON 文本转 Value（JSON 即带引号 YAML，无隐式类型化窗口）。修订 data-model.md §1.3 写入规则、spec.md US3/FR-016、contracts/cli.md tool register 条款。

**验收（macOS arm64，实际 DMG 内 CLI）**

| 场景号 | 预期 | 实际 | 结论 |
|---|---|---|---|
| 6.16 | 注释注入只改 description 行，引号/行内注释逐字节保留 | 隔离库含 `date: "2026-10-08"`、`time: "12:30"`、`flag: "no" # 上游注释` 的技能经包内 CLI `skill set --comment` + `deploy to cursor`，副本除 description 行外与原件逐字节一致；`cargo test --workspace` 覆盖同断言（compose 单测 10 例、deployment/cli_deploy 集成各 1 例、vault_management/toolcenter 未知字段与 stdin 各 1 例） | 通过 |
| 6.17 | 同输入重复合成字节一致 | compose 单测与 cli_deploy 端到端均断言两次产物逐字节相等；真实库 `deploy sync` 二次执行全部 unchanged | 通过 |
| 6.18 | 受污染既有副本恢复，严格 1.1 消费者无损解析 | 包内同源 release CLI 对真实库执行 `deploy sync`：8 项 updated、42 项 unchanged、0 错误；`~/.hermes/skills/ego-browser/SKILL.md` 的 `date: "2026-10-08"` 引号恢复，PyYAML 全部解析为 str（`bootstrap: true` 保持 bool，原件即布尔），`json.dumps` 通过；扫描 `~/.hermes/skills` 与 `~/.agents/skills` 共 51 个 SKILL.md 无残留类型化 | 通过 |
| 6.19 | release/ 每平台一份安装包 | macOS 新包替换旧包（hdiutil UDZO）；Windows NSIS 包保持旧版（含修复前 CLI，待 Windows 机器重建），不得据此声称 Windows 已含修复 | 部分（如实记录） |

**构建与回归**：`cargo test --workspace` **300 项全部通过、0 失败**（含本轮新增的 yaml_safe/compose 单测、deployment 与 cli_deploy 字节保真、vault_management/toolcenter 未知字段类型保真用例）。`npm run tauri build` 编译与 `.app` 打包成功（ad hoc 签名 `codesign --verify --strict` 通过），但 create-dmg 的 Finder AppleScript 在自动化会话持续超时（-1712，沙箱内外一致；另清理过一只残留挂载的旧镜像 `/dev/disk9`），按最小充分原则改用 `hdiutil create -format UDZO` 直接封装：功能等价（同样只读挂载、拖拽安装、含已签名 `.app`），仅无 Finder 窗口美化（背景图/图标布局）。

**安装包**：`release/Skill Manager_0.1.0_aarch64.dmg`，8,612,058 字节，SHA-256 `0d94195dffd4685320df2c9deddf6cb7df95d217bfd6b44450fd17547fd5f007`（按提交 `a7f80d1` 内的实际文件校正；原记录 8,943,032 字节与 SHA `a390da30…` 与提交内容不符）。版本号沿用 0.1.0（仓库惯例不因重造升版）。

**平台与交付限制**：Windows NSIS 安装包未在本轮重建（macOS 无法产出），包内 CLI 仍含本缺陷，待 Windows 实机重建后一并恢复；DMG 改用 hdiutil 封装，与既往 create-dmg 产物相比无 Finder 视觉布局。本轮不自动提交或推送，交付提交由用户指示后按 AGENTS.md 双端规则执行。

## 二十二、分类栏固定独立滚动（2026-10-29）

**问题**：技能库（及沿用同一布局的 Hermes 档案、预设详情、工具中心）左侧分类栏原先随整页内容滚动；分类较多时滚走卡片也会把分类入口带走，且没有独立滚动条。

**规格**：不新增业务 FR。交互以原型为验收基线（002 spec 视觉约束、001 `visual-reference.md`「分类栏滚动（2026-10-29）」）。行为：分类栏固定在工具栏／标签栏下方，与右侧卡片区分滚动；仅分类内容溢出时显示滚动条。

**实现**：原型 `.library-layout` 占满剩余高度，`.category-rail` 与卡片网格各自 `overflow-y: auto`。产品 `AppShell` 内容区改为定高，`LibraryPage`／`HermesPage`／`PresetPage`／`ToolCenterPage` 的分类栏使用 `.category-rail`。

**验证（事实）**

| 命令 / 操作 | 结果 |
|---|---|
| `npx tsc --noEmit` | 通过 |
| `npx vitest run` | 前端 **75 项通过**（17 个测试文件）；含分类栏布局断言与合并入的 chips 间距测试 |
| `cargo test --workspace` | **300 项全部通过、0 失败**（与 YAML 写回保型合并后回归） |

同日将技能库／档案／预设详情分类列由 132px 加宽为 148px，避免五个汉字分类名被计数徽标挤成省略号。工具中心分类列仍为 152px，未改。

GUI 实操未见证：分类溢出时出现滚动条、未溢出时不出现、滚动卡片时分类栏仍固定、五个汉字分类名完整显示。未启动真实 `.app`，也未改 SkillManagerVault。

**合并与安装包**：本改动与远端 `a7f80d1`（YAML 写回保型）、`cb94490`（chips 间距收紧）合并后重造 DMG：`npm run tauri build -- --bundles app` 加 `hdiutil create -format UDZO`，包内 `.app` 通过 `codesign --verify --strict`，CLI 输出版本 `skill-manager 0.1.0` 与完整帮助。`release/Skill Manager_0.1.0_aarch64.dmg`，9,413,675 字节，SHA-256 `9d83ad80daa056ac0ca89ff4deab5736bfcfe2d657b4ea6d6087b601466889c3`。

## 二十三、DMG 安装入口补齐（2026-11-05）

**问题**：此前 DMG 只包含 `Skill Manager.app`，没有 `Applications` 快捷方式；用户只能从只读挂载卷直接运行应用，无法按 macOS 常见方式拖拽安装。该问题属于打包交付缺陷，不改变业务规格。

**实现**：在 DMG 暂存目录中同时放入应用与指向 `/Applications` 的 `Applications` 快捷方式，再用 `hdiutil create -format UDZO` 封装；包内应用与 CLI 仍来自同一次 `npm run tauri build -- --bundles app`。

**验证（事实）**：只读挂载新 DMG，确认根目录同时存在 `Skill Manager.app` 与 `Applications -> /Applications`；应用 `codesign --verify --strict` 通过，包内 CLI `--version` 返回 `skill-manager 0.1.0`。

**安装包**：`release/Skill Manager_0.1.0_aarch64.dmg`，8,955,440 字节，SHA-256 `b182856a807823abd2c469a1ca57fecaf9afa93d0f14b3ab3a2cad2dbd978f64`。本记录替代第二十二节中的 macOS 包哈希；Windows NSIS 包未重建。
