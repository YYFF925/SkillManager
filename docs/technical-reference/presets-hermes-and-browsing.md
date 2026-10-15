# 预设、Hermes 档案与库浏览呈现技术参考

本资料整理旧项目 FeiSkillDock v1.44.0（HEAD `372a223414dc773f7aaefc887c75ab0e1b335be8`）
中「预设」「Hermes 档案（管理器侧）」「库浏览与技能详情」三块的**实际实现**，供本项目 Skill Manager
按需求重写时对照与取舍。行号指向 `/Users/fei/Projects/workspace/FeiSkillDock/`。

**验证状态**：全文为静态源码阅读 + 测试断言核对，**未运行旧应用、未执行真实部署、未连接 Hermes**。
凡标「测试预期」「本次推断」的单独注出。

已有资料不复写：Hermes 外部结构见 [hermes-integration.md](hermes-integration.md)；CLI 命令清单见
[cli-capability-map.md](cli-capability-map.md)；预设部署与 targets 记录见
[deployment-pipeline.md](deployment-pipeline.md)；标签/支持声明的存储见
[manager-layer-data-model.md](manager-layer-data-model.md)；管理流程与产品决策见 [需求记录](../../docs/requirements.md)、
[ADR-0001](../../docs/adr/0001-library-and-local-state.md)。

## 需求关联

| 需求单元 | 本项目需求位置 | 旧能力编号 |
|---|---|---|
| 预设：两库各自维护、组合随 SkillVault 同步 | 需求「预设」 | E01 |
| 预设成员与排序、图标 | 需求「预设」；「旧界面交互细节的沿用」 | E02/E03/B11 |
| 按组增加部署 / 按成员撤销 | 需求「预设」 | E05 |
| 不引入唯一当前预设槽 | 需求「预设」 | E04 |
| Hermes 档案：查看与管理启用、default 来源、名单留本机、批量启停 | 需求「Hermes 档案：重点功能」 | H01/H02 |
| 档案独有技能收编入龙虾库 + default | 需求「Hermes 档案」「收编时补充管理资料」 | H02 |
| 技能详情与文件清单 | 需求「技能详情与文件清单」 | A04 |
| 全库标签治理 / 批量标签 | 需求「全库标签管理」 | A05/A06 |
| 搜索与组合筛选 | 需求「搜索与组合筛选」 | A07 |
| 列表/网格与状态、查看预设归属 | 需求「技能列表展示」 | A08 |
| 分类与标签联动、卡片菜单、单技能更新、跳过版本 | 需求「分类与标签联动」「卡片菜单与单技能更新」「外部 Agent 维护任务与跳过上游版本」 | — / U04 |

---

## 一、预设（Preset）

### 1. 结论速览

1. 旧预设的真值是 **SQLite**（`scenarios` + `scenario_skills` 表），不是库内文件；
   `<CodeRoot>/.skills-manager/scenarios/*.json` 只是**派生镜像与跨机传输载荷**。
2. 「组合定义随 SkillVault 同步」在旧实现里**只是部分成立**：预设 sidecar 挂在 **Code 根的 sidecar**
   之下，而旧的 git 备份仓库根恰好也是 Code 根（`skills_dir()`），因此预设**确实随 Code 根的 git 备份走**；
   但龙虾根不在同一备份里，`lobster` 预设存的是同一份 Code 根 sidecar（不是各根各一份）。
3. 旧名要求**跨域唯一**（`scenarios.name UNIQUE`），与新需求「两库各自维护」冲突，必须剥离。
4. 旧同时存在**互斥当前槽**（E04，`active_scenario` 表 + `apply/deactivate`）与**增量部署**（E05，
   `deploy/undeploy`）；新需求只要后者的语义，前者整套剥离。

### 2. 存储：真值、镜像与同步

旧预设定义在 SQLite，库内另写 `.skills-manager` 镜像；预设及成员镜像都位于 Code 根。
旧 Git 备份也仅覆盖 Code 根，所以两类预设依附这套备份传输；未实际执行同步/恢复。
证据：`sync_metadata.rs:165/623/643`、`git_backup.rs:282-303`。

这里可借鉴的是“组合定义与本机部署分开”，不是旧 DB 主、Code 根镜像的格式。
本项目已确认预设定义在 SkillVault 中共享，本机部署记录不随库走；
具体文件路径与格式留到设计，不重新提出 DB 主与库文件真值二选一。
旧表结构、逐级迁移和完整镜像结构不在此展开。

### 3. 资料字段与图标

`PresetDto`（`commands/presets.rs:33-48`）：`id / name / description / icon / sort_order / skill_count /
domain / created_at / updated_at`。`icon` 是可空字符串键。

图标候选是**前端硬编码的 31 项** `PRESET_ICON_OPTIONS`（`src/lib/presetIcons.tsx`，字段
`key/label/icon/colorClass/activeClass`），渲染用 `lucide-react`；已抽成
[preset-icons.json](preset-icons.json)（31 项，含中文标签与颜色）。`icon` 存的是 `key` 字符串；
未选/未知 key 由 `getPresetIconOption` 回退默认项。创建（`CreatePresetDialog.tsx:78-99`）与重命名
（`RenamePresetDialog.tsx:80-102`）都用同一图标网格，选中态 `activeClass+colorClass`；重命名对话框
只在 `name` 或 `icon` 变化时才允许保存（`RenamePresetDialog.tsx:40`），**即"只改图标也能保存"**，
与分析需求一致。

### 4. 成员与排序

- **成员增删 = 纯组织操作，不部署**。`set_preset_skills_internal`（`presets.rs:563-591`）在仓库锁内
  逐 id `add_skill_to_scenario`/`remove_skill_from_scenario`；两个 Tauri 命令
  `add_skill_to_preset`/`remove_skill_from_preset`（`:514`/`:538`）显式注释"**不 sync 到磁盘**"，因为新
  模型里预设是"编排标签"而非隐式部署开关。
- **预设顺序**：`reorder_presets`（`:593`）收**全表 id 顺序**写 `scenarios.sort_order`。前端拖动
  （`Sidebar.tsx:138-160`）只重排当前域的可见分区，但提交的是"把该分区插回全表原位"后的**全表 id**，
  避免把另一库的顺序打乱。**CLI 没有 reorder 子命令**（`PresetCommand` 枚举 `:599-702` 无 reorder）。
- **成员顺序**：`reorder_preset_skills`（`:628`）按传入 id 顺序写 `scenario_skills.sort_order`；
  `get_preset_skill_order`（`:614`）读回。前端 `MySkills.tsx` 仅在"正查看某预设"时可用拖拽
  （`handleDragEnd` `:525`，`canDrag = !!viewedPreset` `:553`），且只重排**已加入该预设的成员**
  （`:531`）。

可借鉴：预设顺序与成员顺序是**两个独立的顺序字段**，都用"全量 id 列表 → 逐格 sort_order"的写法；
跨域顺序要注意"可见分区 vs 全表"的差异（旧是在前端补回全表，本项目若两库分文件存储则可避免）。

### 5. 部署 / 撤销：增量（E05）vs 互斥（E04）

**E05 增量（要保留的语义）**——核心原语 `scenario_service::apply_skills_to_tools`
（`core/scenario_service.rs:1443`，模式 `BatchApplyMode::{Add,Remove}` `:1420`）：

- Add（`apply_add` `:1459`）：对每个 `(成员 skill × 目标 agent)` 合成并写目标，维护 `skill_targets` 行。
  有**整批预检**：任一目标会被非本方内容覆盖时，整批拒绝（"nothing was changed"）。
- Remove（`apply_remove` `:1795`）：按 `(成员 skill, tool)` 定位 `skill_targets` 行并删文件；
  **共享物理路径**（多个 tool 指向同一技能目录）用"剩余行是否还引用该路径"决定是否真删。
- 两者都**不触碰 `active_scenario` 槽与成员关系**。CLI 侧 `deploy/undeploy`
  （`run_preset_deployment` `bin/skills-manager-cli.rs:4127`）就是调用它，`undeploy` 的目标 agent 集合
  来自"该预设成员的已有部署行"（`select_agent_keys_for_removal` `:3994`），成员即使属于其他预设也照撤。
  **这正是新需求「按组增加部署 + 按成员撤销、不追踪来源预设、库内技能保留」的旧实现对应物。**
- GUI 的 `PresetBar`（`src/components/PresetBar.tsx`）是**每预设的成员级开关**：`handleActivate`
  （`:46`）对全部成员 × agent 逐个 `onAddSkill`（已存在则 skip），`handleDeactivate`（`:79`）逐个
  `onRemoveSkill`；状态由 `computePresetStatus`（`src/lib/presetStatus.ts`）派生为
  `active|partial|inactive|empty`。这已是**叠加语义**，不含互斥。

**E04 互斥（要剥离）**——`active_scenario` 槽 + `apply_scenario_to_default`
（`scenario_service.rs:916`）：

- 切换前先 `unsync_obsolete_scenario_targets(旧槽)` 撤销"新预设以外、且不再需要"的受管部署，再写槽、
  再全量 sync。`commands/presets.rs:470/688` 的 `apply_preset_to_default` /
  `apply_preset_to_coding_agents` 是它的 Tauri 门面。
- `create_and_activate_preset_internal`（`presets.rs:253`）在 **GUI 创建预设时自动把它设为当前槽并
  unsync 旧槽**（注释说为保留桌面端旧行为）。
- `delete_preset_with_active_fallback_internal`（`:410`）删除当前槽时找**同域**候补激活。
- v1.44 已是**按域各自的槽**（`get_active_scenario_id(domain)`、`SkillDomain::{Coding,Lobster}`），
  不是全局单槽；但互斥切换本身仍是 E04。

**要剥离的具体物**：`active_scenario` 表、`set_active_scenario`/`get_active_scenario_id`/
`clear_active_scenario`、`apply_scenario_to_default`、`unsync_obsolete_scenario_targets`、
创建时自动激活、删除时同域回退、CLI `apply`/`deactivate`（`bin/...:3715`/`:3736`）与
`PresetDeactivateReport`、"当前预设"（`presets current` / `sync --preset`）语义。
可借鉴增量 Add/Remove 的语义；函数、整批预检与旧 CLI 形式不因此整体采用。

### 6. 按管理域（code / lobster）与跨域名称唯一

- 域是 `scenarios.domain` 列（`coding`/`lobster`），**读侧唯一入口** `SkillStore::scenario_domain`；
  `scenario_domain_parsed` 返回枚举，未知值当数据完整性问题而非静默回退 `coding`。
- 应用作用域**从预设行自身取域**（`apply_preset_to_coding_agents:696-706` 注释：命令名的 `coding`
  是历史命名债，语义已改为"预设自身域的全部已启用已安装 agent"）。CLI 侧 `create`/`add-skill`/
  `remove-skill` 要求显式 `--domain`，`list/current/show` 默认两域并逐行带 `domain`。
- **冲突点**：`scenarios.name` 是**表级 UNIQUE**，旧测试断言（本轮未运行）"name 必须跨域唯一"
  （`migrations.rs:724`，注释见 `:705`）。旧能力 E01 裁决明确"不沿用旧跨域名称唯一限制"。本项目两库各自维护预设，
  不应重新增加跨库名称唯一约束。具体库内名称校验与存储方式后续设计。

### 7. 剥离项（预设）

| 剥离项 | 旧位置 | 理由 |
|---|---|---|
| 全局名称唯一（跨域不许同名） | `migrations.rs:131` UNIQUE、`:705/724` 测试断言 | 需求「预设」两库各自维护；E01 明确不沿用 |
| `active_scenario` 互斥当前槽 + apply/deactivate | `presets.rs:470/688`、`scenario_service.rs:916`、CLI `:3715/3736` | 需求「预设」不引入唯一当前预设槽 |
| 创建预设即自动激活并 unsync 旧槽 | `presets.rs:253` | 同上 |
| 删除当前预设的同域回退激活 | `presets.rs:410` | 同上 |
| DB 主 + Code 根 sidecar 镜像的存储 | `sync_metadata.rs:643` | 本项目预设应是库内文件，随 SkillVault 统一 Git |
| 托盘预设入口 | 见 `legacy-skill-capabilities.md` E05 裁决 | 需求明确不放托盘 |
| 依赖互斥槽的“当前预设”界面绑定 | `active_scenario` 的界面消费者 | 不带入唯一当前预设；`PresetBar` 的叠加/部分部署状态可独立参考，不等同互斥槽 |

---

## 二、Hermes 档案状态（管理器侧）

> 外部结构（根发现、档案判定、`skills.disabled` / `external_dirs` 的 YAML 形态）见
> [hermes-integration.md](hermes-integration.md)。本节只讲**管理器如何发现、算状态、写配置**。

### 1. 档案发现

- 根解析 `resolve_hermes_root`（`core/hermes_profile.rs:46`）：`HERMES_HOME` 解包（落在
  `.../profiles/<name>` 时回到 `<root>`；落在默认根内以默认根为准）；否则平台默认
  `~/.hermes`（`native_hermes_home:77`）或 Windows `%LOCALAPPDATA%\hermes`。
- 安装判定 `hermes_installed`（`:132`）：根目录存在**且**含 `config.yaml`/`.env`/`state.db` 任一。
  未安装 → 返回空 Vec，前端显示空状态（不是错误，`commands/hermes_profile.rs:74`）。
- `discover_profiles`（`:206`）：`default` 恒在列且置首（home = 根目录本身）；具名档案需
  名字符合 `^[a-z0-9][a-z0-9_-]{0,63}$`（`is_valid_profile_name:180`）+ 有身份标记
  （`config.yaml`/`.env`/`SOUL.md`/`profile.yaml`/`auth.json`/`state.db` 任一，`has_identity_marker:124`）
  + 非 tombstone（`profiles/.deleted/<name>` 不存在），按名字序。
- `is_active` 只读 `<root>/active_profile`（`read_active_profile:169`，剥 BOM），**只标注不切换**。
- `display_name` 只读 `profile.yaml` 的 `ui_meta.hermes-bots.title`，回退 `SOUL.md` 首行 `# 标题`
  （`:140`）；default 恒 `None`。这是只读标题回退；“不管理档案指令”不等于禁止读取发现标记，不能从范围裁决推出读取禁令。

### 2. 候选池：先部署 default（`candidate_pool`）

`candidate_pool`（`core/hermes_profile.rs:424`）：**候选 = `skill_targets` 中 `tool == "hermes"` 且
`status == "ok"` 的技能** join 中央库 skills。即"能被某档案启用的技能，必须先被部署到 Hermes
default"。这与需求"先把库内技能部署到 Hermes default，再为其他档案选择启用"一致。
`status=="ok"` 过滤是为防 sync 失败残留进池。

### 3. 启用名单与状态计算（`profile_skill_states`）

`profile_skill_states`（`:461`）对池中每个成员算：

```rust
let mounted = profile_name == "default" || is_mounted(&config_text, root);
let disabled = parse_disabled_list(&config_text)?;      // skills.disabled 名单
enabled = mounted && !disabled.contains(&name);
missing = !Path::new(&target_path).exists();            // 部署记录在、文件不在
```

- `disabled` 是**黑名单**语义（名单外即启用）——旧 Hermes 加载器行为。
- `is_mounted`（`:399`）：`skills.external_dirs` 是否含 `<root>/skills` 的规范化路径。**未挂载的具名
  档案 = 全部未启用**；default 无挂载概念，恒按名单判定。
- **这正是需求明确不沿用的行为**：已挂载档案里"不在 disabled 名单的新增技能默认启用"。本项目要求
  "新部署到 default 的技能不自动在其他档案启用"。旧的一次性初始化只把当时已有技能写入 disabled（见下），没有处理后续新增。
  这说明旧编排不满足要求，不说明 Hermes 配置协议不能继续使用；管理器可维护明确选择并由适配层写入外部支持的配置。

### 4. toggle 编排与 config 文本手术

`toggle_skill`（`:517`，进程内 `Mutex` 串行化）→ `toggle_skill_unlocked`（`:528`）：

- 拒绝 `ESSENTIAL_SKILL = "hermes-agent"`（`:535`，`EssentialSkillRefused`）。
- 目标名必须在候选池内（否则 `SkillNotInPool`）。
- **未挂载具名档案的 ON = 懒初始化**（`:562-592`）：先写 disabled 名单 = `scan_root_skill_names(root)`
  （整个 `<root>/skills` 目录扫出的技能名）− Essential − {目标技能}，**再** `append_external_dir`
  挂载 `<root>/skills`。注释说明固定顺序（名单先、挂载后）是为了第二步失败时零行为变化。
- **default 或已挂载档案 = 纯名单编辑**（`:595-613`）：ON=从 disabled 移除，OFF=写回 disabled；
  内容无变化则不落盘。
- 写入统一走 `sync_metadata::atomic_write_bytes`（同目录临时文件 + rename + 父目录 flush）。

`hermes_config_surgery.rs` 是**纯文本结构手术**（无 IO、无领域知识，`:1-19` 头注释）：

- `edit_disabled(text, name, Insert|Remove)`（`:30`）：定位 `skills.disabled` 键，只插入/删除目标行，
  **其余字节逐字保留**（保 CRLF、保注释）。
- `append_external_dir(text, abs_path)`（`:44`）：在 `skills.external_dirs` 下追加一行，去重等价路径，
  只接受带根的绝对路径（拒绝 `~`/相对/换行）。
- 支持的键值形态：块列表 / 空键 / 引号字符串（JSON、Python 字面量，编辑时该键转写为标准 YAML 列表）/
  空 flow `[]`；**其余形态（块标量、行内 flow、锚点）一律 Err 拒绝，绝不静默改写**。
- 已记档边界：文件无 `skills` 段且以块标量结尾时追加新段会被并入块标量——只在用户手工构造的极端
  文件上出现且可从 diff 看出，旧实现明确不为它加块标量状态机。

> 需求「Hermes 档案」引用的正是这些**管理器侧**原语。`hermes-integration.md` 第 3 条已指出
> "整目录挂载 + 禁用名单可能让以后新增的技能自动可见"，与需求冲突；本条给出该行为的具体产出点
> （`scan_root_skill_names` + 懒初始化 disabled 名单）。

### 5. 批量启停（H02）

后端**只有单技能 `hermes_toggle_skill`**（`commands/hermes_profile.rs:152`）。批量是**前端循环**：
`HermesProfiles.tsx:184` `handleBatch(enabled)` 对选中且非 `missing` 的技能逐个
`api.toggleHermesSkill(...)`，统计成功/失败。这与需求"手动多选技能，一次启用或禁用，不依赖预设分配"
一致（操作粒度仍是单技能，只是界面聚合）。

### 6. 档案页浏览：分类/标签/搜索（前端）

`HermesProfiles.tsx` 复用了与技能库相近的浏览结构：`search` / `status(all|enabled|disabled)` /
`category(all|uncategorized|具名)` / `selectedTags`。纯逻辑在 `src/lib/hermesProfiles.ts`：

- `filterHermesSkills`（`:18`）：status、category、tags、name 组合过滤；**标签是 AND**
  （`filter.tags.every(t => s.tags.includes(t))` `:33`）——与技能库的 OR（见下）**不同**。
- `hermesTagChips`（`:42`）/ `hermesCategoryRows`（`:63`）/ `hermesAllTags`（`:78`）：都由传入的
  技能清单聚合。
- **关键：`tagChips` 由 `currentSkills`（当前档案全部技能）算，不看 `category`**
  （`HermesProfiles.tsx:122`）。即旧档案页**没有"分类→标签候选收窄"联动**。
- 技能卡片数据（`category`/`tags`）由后端 `hermes_profile_skills` 从中央库派生
  （`commands/hermes_profile.rs:139-143`：`category_of(central_path)` + `get_tags_map`），
  **不新增档案专属元数据**，与需求"沿用技能已有的管理资料"一致。

### 7. 档案独有技能收编（H02）——**旧无对应实现**

旧候选池严格 = "已部署到 hermes 的受管技能"。档案目录中**尚未受管**的独有技能（不经管理器部署的）
**不进候选池**，界面也没有"收编档案独有技能并同时部署 default"的路径。
grep `adopt` 在 `src/` 只命中 `AppContext.tsx` / `installFilter.ts` / `error.ts` / `fixture-design/data.ts`，
均为通用未受管技能收编，**不含 Hermes 档案语义**。

> **结论（有效结论）**：需求「Hermes 档案」的"档案独有技能显示 + 收编入 SkillVault 并同时部署
> default、原档案启用状态保持"是**本项目新增能力，旧版无实现**。可借鉴的只是收编通用形态
> （见 [manager-layer-data-model.md](manager-layer-data-model.md) 与需求「技能获取与收编」B08），方向由本项目定。

### 8. 剥离项（Hermes 管理器侧）

| 剥离项 | 旧位置 | 理由 |
|---|---|---|
| 档案创建/改名/删除、切换活动档案 | `discover_profiles` 只发现；`active_profile` 只读 | 需求「Hermes 档案」不做生命周期与活动切换（U04） |
| 档案指令编辑 | 旧有只读 `SOUL.md` 标题回退 | 本轮不扩展为档案指令管理；不把只读发现标记误判为指令编辑 |
| "已挂载档案自动可见新增技能" | 懒初始化 disabled 名单（`toggle_skill_unlocked:562`） | 需求：新技能不自动在其他档案启用 |
| 一次挂载后不再协调新增技能 | `is_mounted:399`、`toggle_skill_unlocked:562` | 管理器须维持明确选择；`external_dirs` 与 `skills.disabled` 是外部配置语义，是否及如何使用留给适配设计，不能当旧产品行为一并删除 |
| GUI-only（无 CLI） | `legacy-skill-capabilities.md` H01/H02、CLI 能力参考 | 已确定核心操作 CLI 可达，包括 Hermes 档案技能管理 |

---

## 三、库浏览与详情呈现

### 1. 搜索与组合筛选（A07）

`MySkills.tsx` `filtered`（`:425-479`）逐谓词 AND：

- **文本搜索**（`:428-431`）：仅 `skill.name`、目录派生显示名、`skill.description`。
  **不含中文注释、不含来源地址/分支/子目录/版本**。
- **来源类型** `sourceFilters`（`:434`，按 `source_type` 集合）。
- **分类**（`:439-445`）：`all` / `uncategorized`（`category === null`）/ 具名精确匹配。
- **标签**（`:447-452`）：多标签 **OR**（`skill.tags.some(...)`）+ `UNTAGGED_FILTER` 哨兵。
- **预设成员**（`:454-459`）：`filterMode = all|enabled|available` 三态。
- 无"部署到指定 Agent"筛选在库页；该筛选项在 **CLI `skills list`**（`--deployed-to` 等）。
  CLI `SkillSummary` 另带 `preset_ids`/`presets`（`bin/skills-manager-cli.rs:837-838`）。

与需求差距：需求 A07 要求**补中文注释、来源、分类、标签**并支持"筛无标签、部署到某 Agent、按预设"。
旧 GUI 文本搜索覆盖 name/display/description；中文注释与来源字段**未进搜索**。

### 2. 列表/网格与状态展示（A08）

- 切换：`viewMode: "grid"|"list"`（`:187`），分段按钮 `:1505-1517`；网格用
  `rectSortingStrategy`，列表用 `verticalListSortingStrategy`（`:1803-1810`）。
- 卡片右上三点菜单（`CardActionMenu`）+ 独立预设成员开关 `ToggleSwitch`（`:1965`/`:2344`）。
- 状态：部署状态点、更新标记、分类、标签、`deep_path` 异常、冲突 badge 均在卡片渲染。
- **预设归属查看**：旧 GUI **没有**"列出某技能属于哪些预设"的界面——`preset_ids` 只用于筛选
  （`grep -rn preset_ids src/` 只命中 `PresetBar`/`presetStatus`/`MySkills` 的过滤与排序）。
  **可查归属的是 CLI**：`skills list`/`skills status` 的 `SkillSummary.presets`。
  → 需求「技能列表展示」的"查看技能的预设归属"在 GUI 属**新增**，旧只有在预设上下文里筛选。

### 3. 分类与标签联动——**旧无先例**

需求「分类与标签联动」：选中分类时标签栏**只显示该分类下技能实际拥有的标签**；选"全部分类"时显示
当前页面候选技能的全部标签。

旧实现的收窄维度是**管理域（库边）**，**不是分类**：

```ts
// src/views/MySkills.tsx:287-290
// "Tags are ONE vocabulary across both domains (VIEW-02)... The *displayed* chip
//  set is per side: narrowed to the tags actually carried by this domain's rows"
const chipTags = useMemo(
  () => mergeChipTags(skills.flatMap(s => s.tags), [...tagFilters].filter(t => t !== UNTAGGED_FILTER)),
  [skills, tagFilters],   // 依赖里没有 categoryFilter
);
```

`chipTags` 的依赖只有 `skills`（当前域全部技能）与 `tagFilters`，**不含 `categoryFilter`**；
分类选择（`categoryFilter:206`）只进 `filtered` 谓词，**不影响标签候选**。Hermes 档案页同理
（`hermesTagChips(currentSkills)`，`HermesProfiles.tsx:122`，不看 `category`）。

> **结论**：需求「分类与标签联动」是**新交互，旧实现无先例**。旧的"分类与标签"只是并存的筛选谓词，
> 标签候选从不随分类变化。（注意 [ui-assets.md](ui-assets.md) 的"分类与标签"一行写的是**新原型**
> 确认的交互，不是旧实现行为。）
> 旧可借鉴的仅是"标签候选收窄"这一机制的**存在**（先例是收窄到域），机制本身可复用到"收窄到分类"，
> 但收窄口径、无标签分类的处理、切分类后失效标签的清除均为新增设计（需求已给交互方案）。

### 4. 技能详情与文件清单（A04）

- GUI 详情 `SkillDetailPanel.tsx` 展示来源字段：`sourceType / sourceRef / sourceResolved /
  sourceBranch / sourceSubpath / sourceRevision`（`:278-285`），另有部署目标开关列表。
  **GUI 无技能文件清单**。
- **文件清单只在 CLI** `show`（`bin/skills-manager-cli.rs:1884` `show_skill`）：
  `files: collect_files(&skill_dir)`。`collect_files`（`:2459`）**递归收集相对路径并排序**，是**扁平
  路径列表**：不分文件夹/文件，不建树，且**包含 `.manager` 下的管理文件**（未排除）。
- 与需求差距：需求 A04 要求详情页呈现**目录层级、区分文件/文件夹**的可折叠目录树，且文件可点击阅读、
  返回保留技能上下文。旧**无树、无目录层级**，需自行设计（需求「首版范围与技能详情补充」）。

### 5. 卡片菜单与单技能更新（`CardActionMenu.tsx`）

`CardActionMenu`（`src/components/CardActionMenu.tsx`）是通用三点溢出菜单：点击外部/`Escape` 关闭
（`Escape` 会 `stopPropagation`，避免顺带退出多选 `:44-49`），支持 `danger` 色，`onOpenChange` 供宿主
抬升层级（`z-30` 面板 `:83`）。

旧卡片菜单动作（`MySkills.tsx` 网格 `:1910-1963`、列表 `:2307-2342`）：

| 动作 | 网格 | 列表 | 说明 |
|---|---|---|---|
| `check`（检查更新） | 有（非 skillvault） | 有 | vault 来源无单技能更新路径 |
| `refresh`（更新/重导入） | 条件 | 条件 | 由 `canRefresh` 决定 |
| `copy`（复制到另一库） | 有 | 有 | `mySkills.copyToOtherSide` |
| `move`（移动到分类） | 有 | **无** | 列表模式缺此动作 |
| `delete` | 有（danger） | 有（danger） | |

- **旧卡片菜单没有"添加标签"**。标签编辑走多选栏 `MultiSelectToolbar`（`BulkAction`，
  `src/components/MultiSelectToolbar.tsx:84` 用 `CardActionMenu` 承载 overflow）与 `BatchTagDialog`。
- **旧无"跳过当前上游版本"**：grep `skip_version|skipped_version|skipVersion` 在 `src-tauri/` **零命中**；
  只有部署/差异结果里的 `skipped`/`skipped_conflicts` 计数，语义无关。需求「跳过上游版本」为**新增**。
- 需求「卡片菜单与单技能更新」要求菜单含**复制到另一库、移动到分类、添加标签、删除、更新、跳过**，
  且预设成员开关单独保留、菜单操作不触发详情/切换成员。旧菜单缺"添加标签"与"跳过"，需补齐；
  旧的 `ToggleSwitch` 与菜单并列（`:1965`），与新要求"成员开关仍单独保留"方向一致。

### 6. 全库标签治理与批量标签（A05/A06）

存储细节见 [manager-layer-data-model.md](manager-layer-data-model.md)“标签治理与分类联动”。界面：

- `TagRenameDialog.tsx`：单标签重命名（输入新名，非空且与原不同才可保存）。
- `BatchTagDialog.tsx`（A06）：多选技能批量"加一组/减一组标签，其余保持原样"。
- 全库词表是**派生量**（`SELECT DISTINCT tag`），旧**没有独立标签词表表**；重命名/删除遍历受影响技能
  先改 `.manager` 再改 DB 镜像（见管理层参考“标签治理与分类联动”）。
- **两库共用标签名、各库分别筛选**：`skill_tags` 无 domain 列，重命名/删除是整个表操作；筛选是前端行为
  （见 §3 的 `chipTags`）。这与需求「全库标签管理」"两库共用标签名称，各库分别筛选，全库重命名两库都
  生效，删除只移除关联"一致；可借鉴行为，不继承旧表和三份镜像。

### 7. 差异查看器（C03）

- `SkillSourceDiffViewer.tsx`：按 `SkillSourceDiffEntry[]` 渲染逐文件卡片，
  `status` 三态 `added|removed|modified` 上色（`:11-15`），`modified` 且可执行位变化时显示 `0755/0644`；
  文本进 `DocumentDiffViewer`，二进制显示摘要。
- `DocumentDiffViewer.tsx`：**逐行 diff**（旧 GUI 的来源差异视图）。
- 与需求「卡片菜单与单技能更新」「上游更新」：需求明确单技能更新**先展示当前采用版本、目标版本和变化
  文件，不增加逐行差异界面**（见 legacy C03 裁决与需求原文）。因此旧 `DocumentDiffViewer` 的**逐行
  diff 界面属剥离项**，`SkillSourceDiffViewer` 的"变化文件清单 + 状态标记 + 执行位"呈现形式可借鉴。

### 8. 剥离项（浏览与详情）

| 剥离项 | 旧位置 | 理由 |
|---|---|---|
| 逐行 diff 界面 | `DocumentDiffViewer.tsx`、`SkillSourceDiffViewer` 内嵌用法 | 需求：只列变化文件，不做逐行差异 |
| 扁平文件清单（无目录树）作为唯一文件视图 | `collect_files` `bin/...:2459`、CLI `show` | 需求要目录层级可折叠树 |
| 多语言界面（en / zh-TW / zh）与 i18n key 间接 | `src/i18n/index.ts`、`en.json`/`zh-TW.json`/`zh.json`、`parity.test.ts` | 需求「界面与窗口偏好」只提供简体中文 |
| 标签候选按域收窄（而非按分类） | `MySkills.tsx:287` | 需求改为按分类收窄（分类与标签联动） |
| GUI 无预设归属 / 无文件清单 | `SkillDetailPanel.tsx` | 需求要求 GUI 也呈现 |

---

## 四、未验证与存疑

- **未验证**：全文静态阅读 + 测试断言核对；未运行旧应用、未执行真实部署、未连接 Hermes 实例。
  旧 Hermes 侧文件名（`agent/skill_utils.py` 等）是有版本的历史观察，**未核对当前 Hermes 实现**。
- **本次推断**：需求「分类与标签联动」判定为"旧无先例"依据是 `chipTags`/`hermesTagChips` 的依赖与
  grep `preset_ids`/`adopt` 结果；未逐行通读全部 2581 行 `MySkills.tsx`，结论以可见调用链为准。
- **本次推断**：预设随 git 备份同步的判断，基于 `git_backup.rs:282-303` 的仓库根 = Code 根、
  且 Code 根含 `.skills-manager`；未实际执行备份/恢复，未核对 merge 引擎对 `scenarios/` 的处理。
- 新需求已经排除旧跨库名称唯一限制，Hermes 核心技能管理需有 CLI；二者不再列为待裁决。
- 具体预设文件格式、Hermes 明确选择如何映射外部配置是后续设计事项，不能凭旧数据库或黑名单实现反推产品要求。
- Hermes 配置协议是有版本的历史观察，实施时须核对实际支持；本轮未连接或改写真实配置。
