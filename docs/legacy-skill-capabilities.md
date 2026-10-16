# FeiSkillDock 旧版管理能力与新产品裁决清单

2026-10-13，只读核查旧仓库 `/Users/fei/Projects/workspace/FeiSkillDock`，HEAD 为 `372a223414dc773f7aaefc887c75ab0e1b335be8`（v1.44.0），工作区干净。GPT-6-Luna、max 推理强度子代理核查了 Tauri 命令注册、CLI（命令行）分组、GUI（图形界面）及核心服务；主代理对易混淆入口和行为进行了补查。没有运行应用、CLI、测试或部署，因此这是静态源码能力核查，不是运行验收。

本项目是新产品。以下 86 个旧能力条目在最初盘点时默认待裁决，后续现行结论记录在下表；旧参考包的“保留/废除”也不代替本次裁决。用户可对每项决定是否保留，以及保留时如何改变管理形式。条目按产品能力拆分，同一能力的读取/写入命令合并列出；技术机制如锁、数据库表、哈希算法及内部兼容读路径不是独立功能。

## 当前产品实现的暂定范围与访谈结论

本表不是 Spec（规格说明），也不能因条目标为“已确认”就整表转成强制规范。核心技能管理要求另见[核心要求概览](requirements.md#核心管理要求后续-spec-的依据)；本表保留这些要求与旧能力的关联，以及按钮、入口、技术机制等当前暂定实现细节。“移除／不保留”表示当前不纳入产品范围，“暂不保留”表示尚无明确实际场景、暂不安排实现。实现细节可按实际需要调整，不能为了暂定裁决而绕路或妨碍核心要求的实现，见[解释与调整规则](requirements.md#裁决的解释与调整规则)。

| 编号 | 裁决 | 新产品行为与理由 |
|---|---|---|
| A01 | 移除 | 不设独立中文显示名；保留原名，用中文短注释解释用途，避免中文名称妨碍查找技能。 |
| A02 | 移除 | 不设独立中文简介；中文短注释与中文正文翻译仍保留。 |
| A03 | 保留并调整 | 仅正文支持中文/原文切换；有译文时默认中文，无译文时显示原文，不切换名称和简介。 |
| A04 | 保留并调整 | 保留来源地址、分支、来源子目录、版本、库内位置和部署位置；详情页提供技能文件清单。 |
| A05 | 保留，共用两库标签名称 | 两库共用标签名称，各库分别筛选；查看全库标签列表，全库重命名两库都生效，删除仅移除标签关联，不删除技能。 |
| A06 | 保留 | 多选技能后批量添加或移除标签，其他标签保持原样。 |
| A07 | 保留并调整 | 按名称、描述、中文注释、来源、分类和标签查找并组合筛选；可筛无标签、部署到指定 Agent 的技能，并按预设筛选。 |
| A08 | 保留 | 支持列表/网格切换，展示分类、标签、部署状态和更新标记，并查看技能的预设归属；布局后续设计。 |
| A09 | 移除独立首页 | 启动默认直接进入技能库；用户通常直接查看技能，极少使用首页。未要求将旧统计和最近更新条目迁入其他页面。 |
| A10 | 移除 | 不提供独立命令面板；用户从未使用该功能，技能库搜索仍保留。 |
| A11 | 保留 | 外部修改技能内容、管理资料或目录后自动刷新库内显示，无需重启；部署生效另已确认 GUI 保存自动同步、Agent 编辑后调用 CLI 同步。 |
| A12 | 移除 | 用户没有单独导出技能的需求，不提供技能目录导出功能。 |
| B01 | 双库独立，同库名称唯一 | Code／Lobster 分域，可跨库同名；同库内创建、入库、收编、复制遇到同名应提示并阻止，不因分类不同允许重名。 |
| B02 | 调整 | 选择已有本地 SkillManagerVault 仓库，只管理这一根目录，不另存中央库内容，不提供远端克隆首次接入。code/ 与 lobster/ 是同一库内两个管理域，现行布局与职责见 002 spec/data-model。 |
| B03 | 移除 | 不提供普通本地技能文件夹、ZIP 或 .skill 压缩包添加入口；自建技能由 Agent 按规范写入 SkillManagerVault。 |
| B04 | 移除 | 不提供扫描普通目录批量导入的独立入口；Agent 已有技能的单项/批量收编仍保留。 |
| B05 | 普通目录入库，SkillManagerVault 统一管理版本 | 技能及 .manager 以普通目录入库；来源按仓库检查，2026-10-15 最新要求同时支持单技能与仓库批量更新，各技能记录采用版本。分类、新增选择、改名按删除＋新增及只提交自己库等边界保留。 |
| B06 | 保留分类与资料配置，移除入库改名 | 多技能仓库默认以仓库名分类，可修改；Agent 与用户确认后传入 CLI，单技能仓库不自动分类。保留原名；资料继承与后续维护按 002 FR-005/011/012 及入库单页要求处理，不把早期“待讨论”当作禁止补充资料。 |
| B07 | 移除 | 不保留内置在线技能市场，包括市场浏览、排行榜、在线搜索和市场获取入口。 |
| B08 | 保留并调整 | 支持单项/批量收编并接管原部署，补充各技能资料；GUI 与 CLI 同规则。正式修改在 SkillManagerVault，保存后 Agent 调 CLI 同步；不强制补来源，当前无专用来源绑定/重绑入口，来源和后续检查按 002 现行规则。 |
| B10 | 保留并改进实际获取体验 | 获取时显示实际下载进度并允许取消。用户反馈旧版看不到进度条、较大仓库经常失败；大仓库获取需验证，失败根因未诊断，不照搬旧实现作为已验收方案。 |
| B11 | 已有本地 Git 库接入，复用本机认证 | SkillManagerVault 须已有本地仓库及远端，沿用本地 Git 远端/分支及既有认证，不另存应用专用远端/分支，不提供独立登录授权。 |
| C01 | 保留并调整为仓库级检查及新增发现 | GUI 显示更新/新增数量和更新标记；新增专指已登记来源仓库后来出现的技能，点击按仓库展示供用户选择。决定不入库的下次不再提示，决定随 SkillManagerVault 同步。CLI 支持全库检查供 Agent 汇总、用户裁决；报告格式待设计。 |
| C02 | 保留单技能及仓库批量更新 | 最新要求替代此前仅做整批更新：单项确认只更新所选条目，其他技能和新增候选保留；仓库确认批量处理剩余变化及选定新增，GUI／CLI 同规则，已更新版本不重复应用。 |
| C03 | 保留技能与文件变化清单 | 更新前展示新增、删除与内容修改，并列出变化文件；改名按删除与新增展示，不单独识别。不提供专门的逐行差异界面。 |
| C05 | 当前不提供来源绑定／重绑 | 来源迁至 Gitea 等仓库后重新从该仓库导入，不通过修改既有条目的来源完成。 |
| C07 | 三项检查共用已确认周期 | 上游检查、SkillManagerVault 拉取及工具版本检查共用一个自动检查频率：关闭、30分钟、1小时、6小时、24小时，默认1小时；上游只提示、库无冲突静默拉取，见 002 FR-029/037/046。 |
| C08 | 上游显式采用，库同步静默拉取 | 全部技能上游（包括用户自维护 Gitea）先展示变化并经用户确认采用；同一 SkillManagerVault 的跨机拉取无冲突静默应用，按与库的关系区分，不按平台或作者身份决定。 |
| D01 | 保留按 Agent 部署及撤销部署 | 库内技能可提供给指定 Agent，也可撤销受管部署；撤销部署不删除库内技能。GUI 与 CLI 均支持。 |
| D02 | 保留多技能、多 Agent 批量操作 | 可一次选择多个技能部署到多个指定 Agent，也可批量撤销指定 Agent 上的受管部署，库内技能保留；GUI 与 CLI 均支持。具体交互与命令形式后续设计。 |
| D03 | 当前独立副本，部署不带 .manager | 不提供复制／符号链接切换；副本包含实际文件、Agent/系统覆盖结果及中文注释，不携带 .manager。库内修改后按既定规则同步。属于当前技术方案，不作为永久规格限制。 |
| D04 | 未受管同名目标直接覆盖 | 部署遇到 Agent 中未受管的同名技能时直接覆盖并接管目标，不做同名冲突检查；不采用暂停同名项、检查和裁决的建议。受管副本修改后的同步处理另见 F01。 |
| D05 | 取消部署预演 | 2026-10-16 按用户最新要求移除 GUI、CLI 与底层预演功能；实际部署与撤销保留。 |
| D06 | 更新后同步部署已确认，旧链接修正不沿用 | 库内容更新后同步本机已有部署，按 GUI 保存/Agent 调用 CLI 的已确认规则执行。当前采用独立副本，旧版分类移动后修正符号链接的机制不沿用；移动分类时的具体部署记录维护留到技术设计。 |
| D07 | 保留共享技能目录部署，要求内容通用 | 共享目标供支持读取该目录的编码 Agent 使用，所有读取它的 Agent 均应支持该技能且可使用相同内容；有 Agent 内容差异时分别部署。SkillManagerVault 是唯一管理库，具体路径及通用性判断待设计，不按存在 overlay 目录机械拒绝。 |
| D08 | 保留管理域部署边界 | 编码库技能不能直接部署给龙虾 Agent，即使声明支持也须先明确放入 SkillManagerVault 的龙虾技能库，再从该库部署。分库决定部署管理域，不只是展示分类。 |
| D09 | 删除与部署清理规则保留 | 删除库内技能时一并撤销受管部署，包括已批准的上游删除；副本有本地变更时暂停，由 Agent 检查、用户裁决。具体删除入口和批量操作形式待讨论。 |
| D10 | 保留库间独立复制，保留上游来源 | 内容与 .manager 复制后两份独立维护，不自动部署，保留来源。来源只检查一次；单项更新只影响所选副本，仓库批量更新处理两库剩余相关变化，管理资料遵守上游同项值优先规则。 |
| E01 | 两库预设分开，组合定义随库同步 | 两库各自维护预设，成员来自对应库，按库区分预设。定义随 SkillManagerVault 跨电脑同步，部署各机独立；具体资料与格式待设计，不沿用旧跨域名称唯一限制。 |
| E02 | 保留成员管理，编辑组合不自动改变部署 | 预设不移动技能目录，同一技能可加入多个预设。加入成员不自动部署，移出成员不自动撤销部署，明确执行部署/撤销才改变 Agent 中的技能。展示与筛选形式后续设计。 |
| E03 | 保留预设与成员的手动排序 | 可调整预设及预设内技能的显示顺序，具体交互与保存形式后续设计。 |
| E04 | 当前不采用互斥预设切换 | 部署预设只增加该组技能，保留已有其他技能；不以整套切换方式撤销新预设以外的受管技能，不引入唯一当前预设槽。 |
| E05 | 保留 GUI／CLI 按组部署与撤销，不放托盘 | 部署增加成员并保留其他技能；撤销按成员执行，即使成员属于其他预设或单独部署，也撤销，库内技能保留。不追踪部署来源预设。编辑成员只改组合；GUI 主窗口与 CLI 提供操作，不提供托盘预设入口。 |
| F01 | 保留列表、状态与简单变更处理；专用差异功能不纳入 | 保留 Agent 技能列表、受管绿点、“已同步”“仅本地”和“本地有变更”提示。遇到本地变更暂停该技能覆盖，Agent 自行检查，用户决策；其他无本地变更的技能正常同步。不建设专用 GUI 差异查看、CLI 详细差异命令或内置冲突分析/合并。 |
| F02 | 保留并调整流程，旧受管上传入口待定 | 未受管技能通过上传图标按钮快速收编。受管技能正式修改直接在 SkillManagerVault 完成；副本改动须经 Agent 检查、用户裁决，由 Agent 将保留改动整理入库并维护相关 .manager，再调用 CLI 同步部署。旧版受管副本独立上传界面入口尚未裁决。 |
| F04 | 保留未受管技能删除与受管撤销部署 | Agent 中未受管的本地技能可单项／多选删除实际文件；受管技能撤销部署，保留库内技能。 |
| F05 | 当前不管理项目专属技能 | 不安排手动项目登记或项目批量扫描；后续有实际需求再讨论。 |
| F06 | 当前不保留独立目录登记 | 不提供 linked workspace（关联工作区）入口；已知 Agent 的实际技能目录配置、浏览与收编仍保留。 |
| F07 | 当前暂不纳入 | 最新结论为暂不管理项目专属技能，替代此前保留项目范围部署的结论。 |
| F08 | 当前暂不纳入 | 项目技能启停、删除及标签入口随项目专属管理一起暂不安排；库与 Agent 全局管理仍按既定规则执行。 |
| G01 | 全量内置 Agent，保留本机检测 | 2026-10-15 最新要求替代此前移除检测的结论；沿用已检测／其他支持 Agent 分组、启停、重新检测和拖动排序。原型复用旧版 53 个 Agent 资料，检测结果是模拟；不因检测自动部署或启动 Agent。 |
| G02 | 当前不保留自定义 Agent | 不提供自定义 Agent 登记入口；内置 Agent 名单全量支持，路径修改能力独立保留。 |
| G03 | 保留实际技能目录配置 | Agent 使用非默认技能路径时，可在管理器指定其实际目录。设置卡片内悬停显示铅笔，支持行内保存、取消和恢复默认；已有部署记录协调留到正式实现。 |
| G04 | 保留 Agent 项目技能相对路径配置 | 按 Agent 配置，不按单个项目分别配置；沿用设置卡片的项目相对路径行内编辑，项目技能管理仍后置。 |
| G05 | 不保留读取声明 | 管理器默认编码 Agent 都读取共享目录，不提供逐 Agent 声明；这是产品处理假设，不等于实际读取能力已验证。技能支持声明仍保留。 |
| G06 | 单保存指令编辑，按本机配置刷新 | 全局/专属指令源放 SkillManagerVault 共享；单编辑区、单保存，按全局前专属后覆盖本机目标，无独立分发勾选。拉取源变化也刷新本机已有启用目标，路径与启用状态本机保存；技能路径不隐式改变指令路径，见 FR-043。 |
| H01 | 保留本机多档案技能管理，不做档案生命周期 | 重点功能，按一个 Hermes 根目录下多个档案管理，只管本机、不考虑多实例。当前不做档案创建/改名/删除/活动切换，界面选择只用于查看和管理技能。 |
| H02 | default 候选池、明确启用、收编与普通批量操作 | 保留先部署 default 再按档案启用；名单留本机，新技能不自动在其他档案启用。档案独有技能可收编入库并同时部署 default，原档案启用状态不变；保留普通多选批量启停。暂不做预设分配及档案 SOUL.md 等指令管理。 |
| I01 | 调整为接入已准备的本地库及远端 | 接入必须已有本地 SkillManagerVault 仓库且已配置远端，软件不提供远端克隆首次接入，沿用本地 Git 配置进行同步。旧独立备份初始化/远端配置流程不作为新接入方案。 |
| I02 | 移除独立授权，复用本机 Git 认证 | 用户或 Agent 提前配置认证，管理器复用本机已有的密钥或凭据，不提供独立登录授权界面。 |
| I03 | Agent 维护后询问上传，保留库同步及已授权上传重试 | 最新裁决：配套维护 Skill 在每次 Agent 修改后主动询问是否上传，未同意不提交推送；CLI 同步部署不等于上传。不是新增常驻 Git 操作面板；已同意的上传失败可重试。 |
| I04 | 静默拉取；已确认上传提交全工作区 | 库无冲突拉取后刷新本机已有部署/指令目标，新增不首次部署。Agent 每次维护后询问上传；一旦经 GUI 保存既有确认或 Agent 获同意显式上传，提交工作区全部改动，允许混入尚未单独上传的改动，不做归属账本。此为 2026-10-15 后续裁决，替代早期禁止 GUI 顺带上传的表述；后台路径不提交/推送。 |
| I05 | 移除查看历史与恢复功能，手动备份点入口待定 | 产品不提供查看历史版本和恢复历史版本的功能；自动提交仍产生 Git 历史。手动创建备份点的独立入口尚未单独裁决。 |
| I06 | 调整为提示冲突、Agent 检查与用户裁决 | SkillManagerVault 跨电脑同步冲突时暂停本次库同步并提示，由 Agent 检查、用户决策，处理完再继续。不提供专门的冲突裁决界面或旧版本地/远端/双方选择流程。 |
| I07 | 移除重新克隆修复入口 | 仓库故障显示失败原因，由 Agent 检查处理，产品不提供从远端重新克隆 SkillManagerVault 的修复按钮。 |
| I08 | 移除断开远端操作，展示未连接状态 | 远端未连接时依赖该远端的功能暂不可用，本地库管理与部署仍可使用。不提供移除远端配置、清除认证信息的断开流程。 |
| I09 | 移除设备名称设置，保留容量报告 | 不需要区分改动来自哪台电脑，提交沿用本机 Git 作者配置。保留查看库占用空间及体积较大技能的容量报告；具体展示和体积标准待定，旧版排除状态展示尚未单独裁决。 |
| I10 | 当前使用本机 Git，移除引擎切换设置 | 当前阶段不内置 Git 库，仓库操作使用本机 Git；不提供传输或合并引擎的切换设置。 |
| I11 | 暂不纳入 | 目前没有明确实际应用场景，暂不实现旧版远端同步内部引用清理命令。不构成永久禁止；后续同步设计发现实际需要时重新评估，不要求绕路替代。 |
| I12 | 调整为选择已有本地库的首次启动界面 | 当前方案为选择已准备好的本地 SkillManagerVault 目录后进入技能库，不提供创建、克隆和恢复向导；属于暂定界面范围，不作永久规格限制。 |
| J01 | 所有核心技能管理操作可由 CLI 完成 | Agent 是 CLI 的主要使用者；可通过 CLI 完成所有核心技能管理操作，无需用户回 GUI 补做。已有分类对话、更新裁决、创建与部署分离等管理规则仍适用。具体命令、输出格式及调用入口实现留到设计阶段讨论，不直接沿用旧版 JSON 与命令行桥方案。 |
| J02 | 按普通 Skill 管理，开发时维护入库 | 开发时写好并入库，后续开发同步更新 SkillManagerVault 中的管理 Skill，属于开发维护规范；按普通技能管理，不增加专门分发、升级或冲突机制，不沿用旧外部获取的专用引导。此前关于软件升级后专用自动更新的提议不采用。 |
| J03 | 工具中心已确认，Agent 登记维护 | 共享工具条目/简介/指南/检查命令随 SkillManagerVault 同步，安装事实各机独立；软件展示和检查，外部 Agent 登记、安装/更新、验证回写。未安装工具仍显示，配套 Skill 明确选择入库；详见 requirements 工具中心现行章节及 002 FR-044～050，早期“专题暂缓”已被取代。 |
| J04 | 当前移除代理设置，不专门做代理功能 | 新软件当前不提供独立代理设置、不专门实现代理功能；不据此要求干预本机 Git 或系统已有网络配置。 |
| J05 | 保留故障信息与日志获取能力 | 支持方便获取崩溃记录、诊断信息及日志，供 Agent 排查下载失败、部署异常等问题。具体展示、复制及导出形式后续设计；清除记录、问题页面等旧细节不自动作为规格条款。 |
| J06 | 只需简体中文，保留主题、字号与托盘后台运行 | 没有其他界面语言需求；支持浅色、深色、跟随系统主题，默认跟随系统；可调整文字大小。关闭窗口收起到系统托盘并继续后台检查与同步，只有托盘右键“退出”才真正退出。具体字号及托盘视觉设置待设计。 |
| J07 | 保留托盘打开、退出及检查技能更新 | 可从托盘重新打开窗口，右键退出才终止应用；可直接检查技能上游更新，无需先打开主窗口。不提供预设部署或撤销入口；更新结果的托盘展示形式待设计。 |
| J08 | 不纳入旧标签迁移与旧数据兼容 | 用户不需要搬运旧软件数据库标签，新软件按自身规则管理；不提供旧数据搬运、兼容或迁移逻辑。参考包的知识和资产可参考，但不自动沿用旧管理状态。 |

## 已确认六项与旧实现的对照

| 新需求 | 旧源码情况 | 新项目现状 |
|---|---|---|
| 多标签 | 已有单项/批量标签；全库改名删除与跨域共享词表属于额外行为，见 A05/A06。 | 多标签、全库重命名/删除、批量添加/移除及两库共用标签名称已确认。 |
| 目录分类 | 已实现目录派生、移动及未分类；独立分类菜单是占位。 | 目录即分类，最多一级；未分类放 code/ 或 lobster/ 下，有分类放对应库/分类名/技能名，改分类移动真实目录。独立分类操作形式待设计。 |
| 中文正文翻译 | 人工编辑/保存中文正文；没有自动翻译服务；另有中文显示名和简介。 | 译文用于管理器阅读，不作为部署正文；不提供内置一键翻译；上游原文变化后保留译文并提示待更新，由用户决定重译。 |
| 中文短注释 | 单行最多10字；在装入旧中央库时及编辑注释时改写库内 SKILL.md。 | 一般不超过20字；部署才注入，库中原始文件保持原样，已确认；具体注入形式与生效时机待讨论。 |
| Agent 与 OS 支持声明 | 只有 Agent 的 all/selected，后端/CLI 可修改，GUI 无编辑调用；没有 OS 管理字段。 | 未声明默认全支持，可修改范围；Agent 不适用不提供部署，预设批量过滤；当前系统不适用整项变灰、禁止部署。具体展示及共享目标条件待设计。 |
| 技能 overlay（覆盖层） | 有覆盖层格式与合成部署，但仅用于 SkillManagerVault 来源；没有覆盖层编辑 UI，普通自建技能不走合成。 | 按 Agent 和当前系统应用覆盖层并可替换文件，适用于自建及有上游的技能；有 Agent 内容差异时分别部署，通用内容可共享。维度组合及具体合成规则待设计。 |

每个技能自己的 `.manager` 存放管理资料已确认，分类由目录决定。上游自带 `.manager` 是正常输入：入库继承已有管理资料，更新采用上游提供的同项值，即使本地改过也不上管理资料冲突流程；上游不带管理资料时保留本地资料。现行管理文件格式与字段粒度见 `specs/002-skill-management/data-model.md` 与 `research.md` D3；此处旧源码情况不替代正式设计。

六项对照的核心证据：[skills.rs:4091](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:4091)、[skills.rs:4170](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:4170)、[skills.rs:4231](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:4231)、[installer.rs:153](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/installer.rs:153)、[skill_render.rs:40](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/skill_render.rs:40)。

## 旧源码已实现能力（逐项待裁决）

GUI-only 表示只有图形界面入口；CLI-only 表示只有命令行入口。仅命令注册但未接入当前界面的情况在入口栏注明。

### 资料、组织与浏览

| 编号 | 能力 | 旧版实际行为与范围 | 入口 | 源码证据 |
|---|---|---|---|---|
| A01 | 中文显示名 | 独立维护中文展示名称；详情中文视图采用它，技能原始名称仍可单独显示。 | GUI / CLI | [SkillDetailPanel.tsx:302](/Users/fei/Projects/workspace/FeiSkillDock/src/components/SkillDetailPanel.tsx:302)；[skills.rs:4091](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:4091) |
| A02 | 中文简介 | 独立维护中文简介，与正文翻译和短注释是不同字段。 | GUI / CLI | [SkillDetailPanel.tsx:305](/Users/fei/Projects/workspace/FeiSkillDock/src/components/SkillDetailPanel.tsx:305)；[skills.rs:4091](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:4091) |
| A03 | 原文与中文资料切换 | 在详情中切换原文/中文名称、简介和正文；旧实现提供人工编辑，不提供自动翻译。 | GUI | [SkillDetailPanel.tsx:748](/Users/fei/Projects/workspace/FeiSkillDock/src/components/SkillDetailPanel.tsx:748) |
| A04 | 技能详情与文件清单 | GUI 查看正文、来源类型、地址、分支、子路径、版本及部署路径；CLI show 另显示内容文件清单。 | GUI（部分）/ CLI | [skills.rs:635](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:635)；[skills-manager-cli.rs:1884](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:1884) |
| A05 | 全库标签治理 | 列出标签词表、重命名和删除标签；旧词表跨 Code / Lobster 共享，显示可按当前域收窄。 | GUI / CLI | [skills.rs:4379](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:4379)；[skills.rs:4537](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:4537) |
| A06 | 批量标签编辑 | GUI 多选技能批量添加/移除标签；CLI 标签 add/remove/set 命令作用于单个技能引用。 | GUI；CLI 为单项标签操作 | [BatchTagDialog.tsx:1](/Users/fei/Projects/workspace/FeiSkillDock/src/components/BatchTagDialog.tsx:1)；[skills-manager-cli.rs:561](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:561) |
| A07 | 本地搜索与联合筛选 | GUI 按技能名称、目录显示名、描述、来源、分类、标签及预设成员关系过滤；CLI 另可筛无标签、无预设和部署到某 Agent 的技能。当前库页不按 .manager 的中文显示名搜索。 | GUI / CLI | [MySkills.tsx:425](/Users/fei/Projects/workspace/FeiSkillDock/src/views/MySkills.tsx:425)；[skills-manager-cli.rs:212](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:212) |
| A08 | 列表/网格与状态展示 | 切换列表或网格，查看部署状态、更新标记、分类与标签，筛选预设内已加入/可加入技能。 | GUI | [MySkills.tsx:1411](/Users/fei/Projects/workspace/FeiSkillDock/src/views/MySkills.tsx:1411) |
| A09 | 首页统计与最近条目 | 统计库内技能数、已部署技能覆盖情况、已启用 Agent 数；最近活动实际是最近更新的五个技能，不是操作事件时间线。 | GUI | [Dashboard.tsx:19](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Dashboard.tsx:19)；[Dashboard.tsx:117](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Dashboard.tsx:117) |
| A10 | 命令面板 | 通过快捷键查找技能、预设、项目并跳页或进入常用操作。 | GUI | [CommandPalette.tsx:64](/Users/fei/Projects/workspace/FeiSkillDock/src/components/CommandPalette.tsx:64)；[CommandPalette.tsx:135](/Users/fei/Projects/workspace/FeiSkillDock/src/components/CommandPalette.tsx:135) |
| A11 | 外部编辑自动刷新 | 外部文件编辑或 CLI 修改触发索引与列表刷新，无需重启应用。 | 后台 / GUI | [AppContext.tsx:373](/Users/fei/Projects/workspace/FeiSkillDock/src/context/AppContext.tsx:373)；[file_watcher.rs:154](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/file_watcher.rs:154) |
| A12 | 技能目录导出 | 复制技能内容到用户指定目录，已有目标需显式覆盖；这是目录导出，不是打包导出，走 Copy 部署复制规则。 | CLI-only | [skills-manager-cli.rs:2228](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:2228) |

### 库与技能获取

| 编号 | 能力 | 旧版实际行为与范围 | 入口 | 源码证据 |
|---|---|---|---|---|
| B01 | Code / Lobster 双管理域 | 两侧库、工作区和预设分别组织，允许两侧存在同名技能；旧物理根为 skills 与 lobster-skills。 | GUI / CLI | [App.tsx:43](/Users/fei/Projects/workspace/FeiSkillDock/src/App.tsx:43)；[central_repo.rs:290](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/central_repo.rs:290) |
| B02 | 库路径配置与打开目录 | 设置/重置中央库基目录、查看当前库位置、在文件管理器打开；修改路径会提示重启。 | GUI / CLI | [settings.rs:80](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/settings.rs:80)；[skills-manager-cli.rs:162](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:162) |
| B03 | 本地目录与归档添加 | 从本地目录、ZIP 或 .skill 格式归档把技能安装进旧中央库。 | GUI / CLI | [installer.rs:26](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/installer.rs:26)；[InstallSkills.tsx:327](/Users/fei/Projects/workspace/FeiSkillDock/src/views/InstallSkills.tsx:327) |
| B04 | 本地目录批量导入 | 从所选目录批量发现并添加技能，报告导入、跳过和失败数量。 | GUI | [skills.rs:4726](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:4726)；[InstallSkills.tsx:385](/Users/fei/Projects/workspace/FeiSkillDock/src/views/InstallSkills.tsx:385) |
| B05 | Git 获取与多技能预览 | 从 Git 地址获取技能，识别分支/子路径；多技能仓库可预览并勾选子目录安装。 | GUI / CLI | [skills.rs:1962](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:1962)；[InstallSkills.tsx:1580](/Users/fei/Projects/workspace/FeiSkillDock/src/views/InstallSkills.tsx:1580) |
| B06 | 获取时配置名称与归属 | Git 预览允许逐项编辑名称、分类和短注释；CLI install 可指定 name、域和注释。普通本地选择 UI 没有名称编辑框。 | GUI（部分）/ CLI | [InstallSkills.tsx:1683](/Users/fei/Projects/workspace/FeiSkillDock/src/views/InstallSkills.tsx:1683)；[skills-manager-cli.rs:305](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:305) |
| B07 | skills.sh 市场 | 浏览排行榜、在线搜索、打开来源页面、从结果获取技能；与 A07 本地搜索不同。 | GUI / CLI | [browse.rs:26](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/browse.rs:26)；[InstallSkills.tsx:780](/Users/fei/Projects/workspace/FeiSkillDock/src/views/InstallSkills.tsx:780) |
| B08 | 扫描 Agent 已有技能并导入 | 扫描 Agent 技能目录、识别已导入项，从发现结果单项或批量导入中央库。 | GUI | [scan.rs:70](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/scan.rs:70)；[InstallSkills.tsx:1286](/Users/fei/Projects/workspace/FeiSkillDock/src/views/InstallSkills.tsx:1286) |
| B09 | CLI 收编既有目录 | adopt 可收编指定目录内技能，支持预演、来源关联和注释；与普通安装入口分开。 | CLI-only | [skills-manager-cli.rs:466](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:466)；[skills-manager-cli.rs:3178](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:3178) |
| B10 | 安装进度与取消 | 长时间获取过程显示进度，允许取消安装或取消 Git 预览。 | GUI / 后端 | [InstallSkills.tsx:448](/Users/fei/Projects/workspace/FeiSkillDock/src/views/InstallSkills.tsx:448)；[skills.rs:4606](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:4606) |
| B11 | SkillManagerVault 源连接管理 | 配置远端 Git 地址或本地目录及分支，检查连接、清除配置；CLI 还可将既有来源地址迁往新地址。 | GUI / CLI | [Settings.tsx:715](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Settings.tsx:715)；[skills-manager-cli.rs:751](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:751) |
| B12 | SkillManagerVault 清单检查与同步 | 检查新增/既有/冲突/移除条目，按外部库导入、刷新或清理；旧流程会按源库覆盖 .manager，并拒绝软件直接编辑这些来源的管理资料。 | GUI / CLI | [skill_vault.rs:16](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skill_vault.rs:16)；[skill_vault.rs:1588](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/skill_vault.rs:1588) |
| B13 | CLI 临时指定外部库 | 通过 --skills-root / --lobster-skills-root 临时操作外部目录，管理状态放在独立本机命名空间；不改变持久库路径配置。 | CLI-only | [skills-manager-cli.rs:22](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:22)；[skills-manager-cli.rs:1215](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:1215) |

### 来源维护与更新

| 编号 | 能力 | 旧版实际行为与范围 | 入口 | 源码证据 |
|---|---|---|---|---|
| C01 | 单项/全量更新检查 | 比较已登记技能与来源，展示可更新、当前版本和检查失败等状态。 | GUI / CLI | [skills.rs:2140](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:2140)；[skills.rs:2185](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:2185) |
| C02 | 单项/批量更新与本地重新导入 | Git/skills.sh 来源执行更新，本地/导入来源可重新从本地来源取内容；支持批量执行和结果呈现。 | GUI / CLI | [skills.rs:2430](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:2430)；[skills.rs:2459](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:2459)；[skills.rs:2475](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:2475) |
| C03 | 来源正文及文件差异查看 | 详情展示来源正文、整个技能目录的新增/删除/修改差异；文本、二进制及执行权限变化分别呈现。 | GUI | [skills.rs:707](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:707)；[skills.rs:904](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:904) |
| C04 | 删除性更新确认 | 更新将移除现有文件时，先列待移除路径供确认；无人值守调用可暂缓这类更新。 | GUI / CLI / 后台 | [skills.rs:2841](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:2841)；[MySkills.tsx:2468](/Users/fei/Projects/workspace/FeiSkillDock/src/views/MySkills.tsx:2468) |
| C05 | Git 来源绑定/重绑 | 把已存在技能关联到 Git 来源，保留原条目关联；可预演，不同内容替换需要 force。 | CLI-only | [skills.rs:3123](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:3123)；[skills-manager-cli.rs:441](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:441) |
| C06 | 本地来源重连/解除 | 重新选择本地来源路径，或解除来源关联并保留库内内容。 | GUI / 后端 | [MySkills.tsx:1121](/Users/fei/Projects/workspace/FeiSkillDock/src/views/MySkills.tsx:1121)；[skills.rs:2549](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:2549)；[skills.rs:2661](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:2661) |
| C07 | 启动/定时自动检查 | 启动时检查；定时检查默认关闭，可设置关闭、1小时、6小时、24小时，并显示上次运行时间。未设置周期时后端不执行定时检查。 | GUI / 后台 | [AppContext.tsx:452](/Users/fei/Projects/workspace/FeiSkillDock/src/context/AppContext.tsx:452)；[Settings.tsx:189](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Settings.tsx:189)；[skill_auto_updater.rs:83](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/skill_auto_updater.rs:83) |
| C08 | 自动应用更新 | 另行开启自动应用；周期更新器的实际写路径只走 Git updater，删除性版本会暂缓。 | GUI / 后台 | [skill_auto_updater.rs:140](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/skill_auto_updater.rs:140)；[skill_auto_updater.rs:201](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/skill_auto_updater.rs:201) |

### 部署与删除

| 编号 | 能力 | 旧版实际行为与范围 | 入口 | 源码证据 |
|---|---|---|---|---|
| D01 | 按技能×Agent 部署/取消 | 把库内技能提供给指定 Agent，或撤销受管部署；取消部署保留库内技能。 | GUI / CLI | [sync.rs:84](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/sync.rs:84)；[sync.rs:167](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/sync.rs:167) |
| D02 | 批量多 Agent 部署 | 多选技能批量部署到若干 Agent；库页批量同步只加不减，CLI 另有多引用部署/撤销。 | GUI / CLI | [BatchSyncAgentDialog.tsx:1](/Users/fei/Projects/workspace/FeiSkillDock/src/components/BatchSyncAgentDialog.tsx:1)；[skills-manager-cli.rs:381](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:381) |
| D03 | Copy / Symlink 部署模式 | 选择复制或符号链接作为全局默认模式；旧 SkillManagerVault 合成部署强制复制。 | GUI / 后端 | [Settings.tsx:1604](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Settings.tsx:1604)；[sync_engine.rs:260](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/sync_engine.rs:260) |
| D04 | 部署冲突与目标覆盖规则 | 已有同名目标按是否受管及对象类型决定能否替换；拒绝时呈现路径，避免覆盖未受管目录。旧链路不校验目标文件内容是否被外部改过。 | GUI / CLI / 后端 | [sync_engine.rs:193](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/sync_engine.rs:193)；[sync_engine.rs:232](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/sync_engine.rs:232) |
| D05 | 部署状态查询与预演 | 查看技能在各 Agent 的实际部署状态；CLI 支持指定 Agent 和 dry-run 预演。 | GUI / CLI | [skills-manager-cli.rs:408](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:408)；[skills-manager-cli.rs:2002](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:2002) |
| D06 | 更新后刷新部署与移动后修正链接 | 更新技能后重新同步受管复制目标；移动分类目录后修正已部署符号链接并报告失败。 | 后端随操作执行 | [skills.rs:3946](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:3946)；[skills.rs:1249](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:1249) |
| D07 | 共享技能目录部署 | 把共享目录作为 _global 目标管理；旧版只面向 Code 域，SkillManagerVault 来源需全 Agent 支持且没有 overlay。 | GUI / CLI | [GlobalAgentView.tsx:1](/Users/fei/Projects/workspace/FeiSkillDock/src/views/GlobalAgentView.tsx:1)；[tool_adapters.rs:239](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/tool_adapters.rs:239) |
| D08 | Code / Lobster 部署边界 | 库中技能只能部署到同域 Agent；支持声明不能绕过跨域拒绝。 | GUI / CLI / 后端 | [scenario_service.rs:400](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/scenario_service.rs:400) |
| D09 | 删除库技能与部署清理 | 单个/批量删除库技能，同时清理已登记部署；共享目标仍有其他认领时保留，失败会呈现。 | GUI / CLI | [skills.rs:1023](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:1023)；[skills.rs:1057](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:1057) |
| D10 | 跨域复制独立技能 | 复制到另一域，保留内容和管理资料，但生成独立条目、清除来源和部署关系；同名目标拒绝覆盖。 | GUI / CLI | [skills.rs:4910](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:4910)；[skills-manager-cli.rs:271](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:271) |

### Preset（预设）

| 编号 | 能力 | 旧版实际行为与范围 | 入口 | 源码证据 |
|---|---|---|---|---|
| E01 | 预设创建与资料管理 | 创建、改名、修改描述/图标、删除预设；预设有域归属，旧名称要求跨域唯一。 | GUI / CLI | [presets.rs:150](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/presets.rs:150)；[skills-manager-cli.rs:600](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:600) |
| E02 | 预设成员增删与查看 | 把技能加入或移出某预设，按正在查看的预设筛选库；查看/成员编辑不等于部署。 | GUI / CLI | [presets.rs:515](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/presets.rs:515)；[Sidebar.tsx:597](/Users/fei/Projects/workspace/FeiSkillDock/src/components/Sidebar.tsx:597) |
| E03 | 预设与成员排序 | 拖动预设顺序，以及预设内技能顺序；通过 GUI/Tauri 保存，CLI 没有 reorder 子命令。 | GUI / 后端 | [presets.rs:594](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/presets.rs:594)；[presets.rs:629](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/presets.rs:629) |
| E04 | 互斥应用/停用预设 | apply/deactivate 切换当前预设，移除旧预设中不再需要的部署；停用可回到替代预设或空槽。 | CLI；Tauri 有接口、当前 GUI 无直接 Apply 入口 | [scenario_service.rs:916](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/scenario_service.rs:916)；[skills-manager-cli.rs:3736](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:3736) |
| E05 | 增量部署/撤销预设 | 只增加或移除该预设成员的部署，不改当前预设槽与成员关系；可按工作区/Agent 范围操作。 | GUI / 托盘 / CLI | [PresetBar.tsx:46](/Users/fei/Projects/workspace/FeiSkillDock/src/components/PresetBar.tsx:46)；[skills-manager-cli.rs:661](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:661) |
| E06 | 当前预设、计划与状态 | 读取两域当前预设，预览部署计划，查看逐 Agent 状态；GUI 有简化状态，详细计划/状态在 CLI。 | GUI（部分）/ CLI | [skills-manager-cli.rs:610](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:610)；[skills-manager-cli.rs:3821](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:3821) |
| E07 | CLI skills sync 流程 | 按指定或当前预设同步，可指定 Agent 或预演；执行时会更新当前预设槽，不等同于增量 deploy。 | CLI-only | [skills-manager-cli.rs:415](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:415)；[skills-manager-cli.rs:3051](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:3051) |

### Agent 本地与项目工作区

| 编号 | 能力 | 旧版实际行为与范围 | 入口 | 源码证据 |
|---|---|---|---|---|
| F01 | Agent 本地工作区浏览 | 查看 Agent 目录中的受管/本地独有技能、正文、状态和差异；与共享 _global 页面分别组织。 | GUI | [WorkspaceView.tsx:348](/Users/fei/Projects/workspace/FeiSkillDock/src/views/WorkspaceView.tsx:348)；[agent_workspace.rs:148](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/agent_workspace.rs:148) |
| F02 | Agent 本地内容上传到库 | 显式把 Agent 目录中的技能收编/上传到中央库，可能更新匹配副本或建立新条目与托管关系；不是自动双向同步。 | GUI | [WorkspaceView.tsx:437](/Users/fei/Projects/workspace/FeiSkillDock/src/views/WorkspaceView.tsx:437)；[agent_workspace.rs:346](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/agent_workspace.rs:346) |
| F03 | 库内容拉取到 Agent 本地 | 显式以库内容更新 Agent 侧副本；某些本地更新状态会拒绝直接覆盖，由界面处理。 | GUI | [WorkspaceView.tsx:475](/Users/fei/Projects/workspace/FeiSkillDock/src/views/WorkspaceView.tsx:475)；[agent_workspace.rs:730](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/agent_workspace.rs:730) |
| F04 | Agent 本地取消同步与删除 | 受管技能撤销部署，本地非受管技能可真正删除；支持批量操作。 | GUI | [GlobalAgentView.tsx:182](/Users/fei/Projects/workspace/FeiSkillDock/src/views/GlobalAgentView.tsx:182)；[agent_workspace.rs:781](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/agent_workspace.rs:781) |
| F05 | 项目注册、扫描与排序 | 手动添加项目，扫描目录后批量注册，移除注册，排序项目列表。 | GUI | [AddProjectDialog.tsx:45](/Users/fei/Projects/workspace/FeiSkillDock/src/components/AddProjectDialog.tsx:45)；[projects.rs:567](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/projects.rs:567)；[Sidebar.tsx:166](/Users/fei/Projects/workspace/FeiSkillDock/src/components/Sidebar.tsx:166) |
| F06 | 关联已有技能工作区 | 把已有 Agent 技能目录登记为 linked workspace；后端可处理同级 disabled 目录，与普通项目不同。 | GUI / 后端 | [AddProjectDialog.tsx:108](/Users/fei/Projects/workspace/FeiSkillDock/src/components/AddProjectDialog.tsx:108)；[projects.rs:613](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/projects.rs:613) |
| F07 | 项目级技能分发与双向更新 | 按项目中的 Agent 目标分发库技能、从项目收编或更新库内容、从库更新项目副本，并查看差异；支持批量。 | GUI | [projects.rs:864](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/projects.rs:864)；[projects.rs:973](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/projects.rs:973)；[projects.rs:988](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/projects.rs:988)；[projects.rs:1093](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/projects.rs:1093) |
| F08 | 项目技能启停/删除/标签 | 启停项目技能副本、删除副本，有多选批量操作；项目页标签编辑作用于关联的库技能，界面有作用域提示。 | GUI | [projects.rs:1167](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/projects.rs:1167)；[projects.rs:1195](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/projects.rs:1195)；[ProjectDetail.tsx:678](/Users/fei/Projects/workspace/FeiSkillDock/src/views/ProjectDetail.tsx:678) |

### Agent 配置与指令文件

| 编号 | 能力 | 旧版实际行为与范围 | 入口 | 源码证据 |
|---|---|---|---|---|
| G01 | Agent 安装发现、启停与排序 | 探测已知 Agent、刷新、选择参与管理的 Agent、全部启停及排序；旧内置目录表有 53 个 Agent。 | GUI / CLI（部分） | [tools.rs:45](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/tools.rs:45)；[Settings.tsx:1264](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Settings.tsx:1264) |
| G02 | 自定义 Agent | 登记名称、全局技能目录及项目相对路径，删除自定义 Agent；删除会处理其已有部署。旧创建路径固定为 Code 类。 | GUI / 后端 | [tools.rs:439](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/tools.rs:439)；[tools.rs:484](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/tools.rs:484) |
| G03 | Agent 全局技能路径覆盖 | 修改/重置内置 Agent 的全局技能目录，协调旧部署目标；自定义 Agent 可改自身路径。 | GUI / 后端 | [tools.rs:302](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/tools.rs:302)；[tools.rs:312](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/tools.rs:312) |
| G04 | Agent 项目相对路径覆盖 | 修改/重置某 Agent 的项目相对技能路径，应用于该 Agent 的项目工作区；不是按单个项目分别配置。 | GUI / 后端 | [Settings.tsx:238](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Settings.tsx:238)；[tools.rs:389](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/tools.rs:389) |
| G05 | Agent 共享目录读取声明 | 声明某 Agent 是否读取共享目录，只影响管理器侧目标评估与展示，不改变 Agent 本身行为；与技能的支持名单不同。 | GUI / CLI | [Settings.tsx:413](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Settings.tsx:413)；[skills-manager-cli.rs:191](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:191) |
| G06 | Agent 指令编辑/预览/分发 | 维护全局及逐 Agent 指令，拼接后覆盖 AGENTS.md；CLI 可 show/push。默认写入范围以已检测且启用的 Agent 集合为准。 | GUI / CLI | [InstructionsDialog.tsx:73](/Users/fei/Projects/workspace/FeiSkillDock/src/components/InstructionsDialog.tsx:73)；[agent_instructions.rs:84](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/agent_instructions.rs:84) |

### Hermes 档案

| 编号 | 能力 | 旧版实际行为与范围 | 入口 | 源码证据 |
|---|---|---|---|---|
| H01 | 档案发现与浏览 | 发现 default/具名档案，显示名称、目录、活跃标记，按选中档案查看并筛选技能；选中不切换 Hermes 活动档案。 | GUI-only | [hermes_profile.rs:69](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/hermes_profile.rs:69)；[HermesProfiles.tsx:162](/Users/fei/Projects/workspace/FeiSkillDock/src/views/HermesProfiles.tsx:162) |
| H02 | 按档案启停技能 | 单项/批量启停；旧候选池仅取管理器已部署到 Hermes default 的技能，写档案配置名单及挂载，已挂载档案会默认看到后续新增部署。 | GUI-only | [hermes_profile.rs:424](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/hermes_profile.rs:424)；[hermes_profile.rs:517](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/hermes_profile.rs:517)；[HermesProfiles.tsx:323](/Users/fei/Projects/workspace/FeiSkillDock/src/views/HermesProfiles.tsx:323) |

### Git 备份、同步与维护

| 编号 | 能力 | 旧版实际行为与范围 | 入口 | 源码证据 |
|---|---|---|---|---|
| I01 | 备份仓库连接与初始化 | 初始化本地备份仓库，配置远端、克隆已有备份。旧备份覆盖 Code 根，不覆盖 Lobster 根。 | GUI / CLI | [git_backup.rs:113](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/git_backup.rs:113)；[git_backup.rs:580](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/git_backup.rs:580) |
| I02 | GitHub 授权连接 | 通过设备授权流程或 PAT（个人访问令牌）连接 GitHub 备份；凭据由相关连接流程管理。 | GUI | [git_backup.rs:167](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/git_backup.rs:167)；[git_backup.rs:231](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/git_backup.rs:231) |
| I03 | 早期新产品需求记录（非旧源码事实） | **历史记录，现行以上方当前裁决表为准。**最新裁决：配套维护 Skill 在每次 Agent 修改后主动询问是否上传，未同意不提交推送；CLI 同步部署不等于上传。不是新增常驻 Git 操作面板；已同意的上传失败可重试。 |
| I04 | 早期新产品需求记录（非旧源码事实） | **历史记录，现行以上方当前裁决表为准。**启动、定时及主动同步无冲突拉取后更新已有部署，新增不首次部署。Agent 维护后的库上传须每次询问，不得由后台同步或后续 GUI 保存顺带上传未获同意改动。部署、档案启用与上传确认状态留本机。 |
| I05 | 快照与历史恢复 | 创建备份点、列出历史版本、恢复选定版本。 | GUI / CLI | [git_backup.rs:625](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/git_backup.rs:625)；[git_backup.rs:653](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/git_backup.rs:653) |
| I06 | 同步冲突处理 | 列出待裁决冲突，可选本地、远端或保留双方版本。 | GUI / 后端 | [git_backup.rs:537](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/git_backup.rs:537)；[git_backup.rs:549](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/git_backup.rs:549) |
| I07 | 重新克隆修复 | 提供备份重新克隆/恢复流程；旧恢复界面在特定故障场景以重新克隆为处理方式。 | GUI / 后端 | [git_backup.rs:604](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/git_backup.rs:604)；[GitRecoveryDialog.tsx:1](/Users/fei/Projects/workspace/FeiSkillDock/src/components/GitRecoveryDialog.tsx:1) |
| I08 | 断开备份远端 | 移除本机 origin、清除远端配置及本机访问凭据，保留本地仓库和远端资料。 | GUI / 后端 | [git_backup.rs:316](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/git_backup.rs:316)；[Backup.tsx:690](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Backup.tsx:690) |
| I09 | 设备名称与容量报告 | 设置备份提交使用的设备作者名；查看库体积、超大技能和排除状态。 | GUI / 后端 | [git_backup.rs:683](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/git_backup.rs:683)；[git_backup.rs:706](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/git_backup.rs:706) |
| I10 | Git 传输/合并引擎选择 | 选择系统 Git 或内置 git2 传输，选择对象级合并或系统 Git 合并。 | GUI | [Settings.tsx:1933](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Settings.tsx:1933)；[Settings.tsx:1955](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Settings.tsx:1955) |
| I11 | 远端同步引用清理 | 主动清理备份远端 refs/skills-manager/*，保留本地同步引用。 | CLI-only | [skills-manager-cli.rs:738](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:738)；[skills-manager-cli.rs:4426](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:4426) |
| I12 | 首启恢复引导 | 空库且未连备份时，可选择开始新库或从远端备份恢复。 | GUI / 启动 | [FirstRunRestoreDialog.tsx:12](/Users/fei/Projects/workspace/FeiSkillDock/src/components/FirstRunRestoreDialog.tsx:12)；[FirstRunRestoreDialog.tsx:49](/Users/fei/Projects/workspace/FeiSkillDock/src/components/FirstRunRestoreDialog.tsx:49) |

### 自动化入口与相邻产品能力

| 编号 | 能力 | 旧版实际行为与范围 | 入口 | 源码证据 |
|---|---|---|---|---|
| J01 | CLI 与 Agent 调用入口 | 命令行提供库、技能、Agent、标签、预设、Git、Vault、指令操作及 JSON 结果；应用发布一份可预测路径的 CLI bridge（命令行桥）。 | CLI / 后台 | [skills-manager-cli.rs:108](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:108)；[cli_bridge.rs:53](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/core/cli_bridge.rs:53) |
| J02 | 管理器 Skill 安装引导 | 首页引导手选 Agent，从 GitHub 获取 manage-skills 并部署；不是随应用更新自动同步。 | GUI | [AgentControlSetupCard.tsx:78](/Users/fei/Projects/workspace/FeiSkillDock/src/components/AgentControlSetupCard.tsx:78)；[Dashboard.tsx:52](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Dashboard.tsx:52) |
| J03 | Tool Center 工具中心 | 只读浏览工具研究资料、机器状态、更新与健康信息，搜索过滤并查看说明；软件页面不执行工具安装/升级。 | GUI，邻接能力 | [ToolCenterPage.tsx:27](/Users/fei/Projects/workspace/FeiSkillDock/src/tool-center/ToolCenterPage.tsx:27)；[App.tsx:62](/Users/fei/Projects/workspace/FeiSkillDock/src/App.tsx:62) |
| J04 | 网络代理配置 | 保存网络代理地址，用于技能市场及相关网络访问。 | GUI / 后端 | [Settings.tsx:919](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Settings.tsx:919)；[browse.rs:26](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/browse.rs:26) |
| J05 | 诊断、崩溃记录与日志导出 | 查看/清除崩溃记录，复制诊断与问题资料，打开问题页面，导出日志 ZIP。 | GUI / 后端 | [Settings.tsx:562](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Settings.tsx:562)；[settings.rs:424](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/settings.rs:424) |
| J06 | 界面与窗口偏好 | 简中/繁中/英文，浅色/深色/跟随系统，文字大小、托盘图标、关窗行为。 | GUI | [Settings.tsx:1632](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Settings.tsx:1632)；[Settings.tsx:1688](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Settings.tsx:1688)；[Settings.tsx:1718](/Users/fei/Projects/workspace/FeiSkillDock/src/views/Settings.tsx:1718) |
| J07 | 托盘快捷管理 | 从托盘打开应用、查看/检查更新、按预设增加或移除技能部署及退出。 | 托盘 | [lib.rs:645](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/lib.rs:645)；[lib.rs:799](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/lib.rs:799) |
| J08 | 标签迁移维护命令 | 主动将旧数据库标签写入 .manager，已有管理文件不覆盖；属于旧数据维护入口，不是标签日常编辑。 | CLI / 后端 | [skills.rs:4639](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/skills.rs:4639)；[skills-manager-cli.rs:492](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:492) |

## 不能当作现成功能的项目

| 编号 | 项目 | 实际状态与证据 |
|---|---|---|
| U01 | 分类栏独立新增/重命名/删除分类 | 控件存在，但回调为空或只关闭菜单；移动技能到新分类目录是实际已实现的能力，二者不同。[MySkills.tsx:1715](/Users/fei/Projects/workspace/FeiSkillDock/src/views/MySkills.tsx:1715)、[MySkills.tsx:1753](/Users/fei/Projects/workspace/FeiSkillDock/src/views/MySkills.tsx:1753)。 |
| U02 | 原始技能正文编辑器、自动翻译器 | 当前详情支持原文查看和中文资料人工编辑，没有普通原文编辑器或内置自动翻译流程；外部改文件可触发 A11。[SkillDetailPanel.tsx:343](/Users/fei/Projects/workspace/FeiSkillDock/src/components/SkillDetailPanel.tsx:343)、[SkillDetailPanel.tsx:632](/Users/fei/Projects/workspace/FeiSkillDock/src/components/SkillDetailPanel.tsx:632)。 |
| U03 | 自带两个管理 Skill 随应用自动安装/更新 | AGENTS.md 写的是目标形态；实际只有 J02 的 manage-skills 手工引导。tool-center-manager 是仓库资产，未发现专用运行时安装入口；不能据此宣称已自动分发。[AGENTS.md:3](/Users/fei/Projects/workspace/FeiSkillDock/AGENTS.md:3)、[AgentControlSetupCard.tsx:78](/Users/fei/Projects/workspace/FeiSkillDock/src/components/AgentControlSetupCard.tsx:78)。 |
| U04 | Hermes 档案创建/重命名/删除、切换活动档案 | 当前 App 只发现与显示档案，并管理技能启停；界面选中档案不会改 Hermes 活动档案。[hermes_profile.rs:69](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/commands/hermes_profile.rs:69)、[HermesProfiles.tsx:162](/Users/fei/Projects/workspace/FeiSkillDock/src/views/HermesProfiles.tsx:162)。 |
| U05 | skills enable/disable 作为技能启停 | 兼容命令仍存在：enable 仅修复旧 enabled=false，disable 实际不操作；可用的部署/撤销是 D01。[skills-manager-cli.rs:3003](/Users/fei/Projects/workspace/FeiSkillDock/src-tauri/src/bin/skills-manager-cli.rs:3003)。 |

OS 声明、普通自建技能覆盖层合成、覆盖层编辑 UI 的旧实现缺口已在六项对照说明；不将缺口混入“已实现”条目。

## 容易误读的旧行为

- 侧栏点预设只是切换正在查看的预设；E04 互斥应用、E05 增量部署、E07 skills sync 三套行为分别裁决。
- 预设及成员排序有 GUI/Tauri 入口，旧 CLI 没有 reorder 子命令。
- 中文短注释、中文简介、中文显示名是三个不同内容，不能用同一个字段含糊合并。
- 当前库页搜索的“显示名”来自技能名或同名时的目录名，不来自 .manager 中文显示名；参考包对应描述与源码不一致，以源码为准。
- Agent 是否读取共享目录（G05）与 Skill 支持哪些 Agent 的声明是两个不同主体。
- 旧备份页侧栏入口被隐藏，但 Settings 和直接路由可达，功能并未移除；旧 Git 备份只覆盖 Code 根。
- CLI instructions push 的旧帮助写“全部 Agent”，核心默认范围实际是已检测且启用的 Agent。
- SkillManagerVault 同步、只读管理层、来源类型决定是否合成覆盖层，是旧模型的实际行为；此处列出供新产品重新决定其去留及形式。

## 裁决记录

最初盘点时以上旧能力尚待逐项裁决；本文件后半段保留当时的旧实现事实。现行结论以上方“当前产品实现的暂定范围与访谈结论”及 002 正式规格为入口，不能将这段历史状态读成当前全部未裁决，也不能仅凭旧实现生成任务。

