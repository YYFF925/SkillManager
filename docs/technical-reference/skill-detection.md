# 技能识别与入口格式约定

本资料回答一个此前未被系统覆盖的问题：**如何判定一个目录是技能**。
覆盖需求「技能获取与收编」「分类与目录」「Agent 接入与本地技能」中与「识别技能目录 / 技能入口文件 / 技能名派生」有关的部分，对应旧能力 B05／B06／B08、A01。
仓库内容识别的整体候选顺序已写在[获取与发现参考](git-and-discovery.md)「仓库内容识别」一节，本文不重复；中文注释注入的合成规则见[内容合成参考](content-composition.md)，本文只引用衔接点。

验证状态：2026-10-15 对 FeiSkillDock v1.44.0（HEAD `372a223414dc773f7aaefc887c75ab0e1b335be8`）做**静态源码阅读**，未运行旧应用、CLI 或测试。外部约定一节是**有版本的历史观察**，本次未核对当前一手实现。

## 结论速览（识别谓词与候选顺序）

源码事实，识别谓词只有一个：

```
is_valid_skill_dir(dir) = dir.is_dir() && 目录下存在名为 "SKILL.md" 或 "skill.md" 的普通文件
```

- 候选文件顺序固定为 `SKILL.md` 在前、`skill.md` 在后；文件名**精确匹配**（大小写敏感、不是子串、不含其它后缀），且必须是 `file` 类型。
- `README.md`、`readme.md`、`CLAUDE.md` 在任何检测路径上**都不作为技能标记**。
- 目录内一旦命中技能标记就**停止向下递归**（"skills don't nest"）。
- 技能名派生优先级：frontmatter 的 `name:` 字段优先，缺失或清洗失败时回退到目录名。

这套约定是旧规约 `docs/skill-format-detection-spec.md`（2026-04-10）落地的结果，规约与实现**总体一致**，但实现内部有常量重复与个别错误文案漂移，见下文。

## 外部格式约定（2026-04-10 观察，本次未核对当前一手实现）

以下引自旧规约文档（`FeiSkillDock/docs/skill-format-detection-spec.md`），属**历史线索**，不代表本项目已接入这些外部规范，也不保证今天仍然成立：

- **skills.sh**（`https://skills.sh/docs`、`https://skills.sh/docs/cli`）：公开文档描述技能是经 `skills` CLI 安装的可复用能力；文档**未**把 `README.md` / `CLAUDE.md` 定义为合法技能入口文件；严格的文件级格式更多由生态示例与上游仓库体现。
- **vercel-labs/skills**：README 表述「Skills are directories containing a `SKILL.md` file with YAML frontmatter」；发现机制基于标准技能位置加显式插件清单，而非 `README.md` / `CLAUDE.md`。据此规约认为 `SKILL.md` 是**规范入口**，`skill.md` 是可作兼容的旧式变体。
- **vercel-labs/agent-skills**：README 表述每个技能包含 `SKILL.md`（给 Agent 的指令）、可选 `scripts/`、可选 `references/`；沿用 `SKILL.md`-first 约定。
- **skills.sh 生态示例**：多数公开技能以 `SKILL.md` 为可见入口，规范类创建/校验技能普遍声明 `SKILL.md` 是单一事实来源；部分指导明确要求不要在技能目录保留 `README.md`，其内容应合并进 `SKILL.md`。

规约由此给出的推荐策略是：`SKILL.md` 为规范标记，`skill.md` 作为旧式兼容标记在**所有常规检测路径**接受，`README.md` / `CLAUDE.md` 永不作为标记。规约也主张不为 `skill.md` 再拆分「严格 / 兼容」两套逻辑，避免无谓复杂度。

**后续输入格式设计**：本项目是否接受 `skill.md` 这一兼容标记、是否维持「`README.md` / `CLAUDE.md` 不作标记」的立场，由本项目自行确定；旧规约的**立场**是保留 `skill.md`、拒绝 `README.md` / `CLAUDE.md`。旧规约给出该立场的理由是可评估的经验：递归扫描时 `README.md` / `CLAUDE.md` 常属仓库文档、分类文档或 Agent 记忆/指令文件，是**误报（false positive）**的主要来源；而 `skill.md` 本身已是强标记，接受它不会实质增加误报。

## 实现与规约核对

| 规约主张 | 代码事实 | 是否一致 |
|---|---|---|
| 规范入口 `SKILL.md` | `skill_metadata.rs:83` `SKILL_MD_CANDIDATES = ["SKILL.md", "skill.md"]`；`213` `is_valid_skill_dir`；`209` 私有 `SKILL_DIR_MARKERS`（同值） | 一致 |
| 兼容入口 `skill.md` | 同上；`parse_skill_md`（`31-33`）与 `find_skill_dir`（`git_fetcher.rs:1371`）另用**内联字面量** `["SKILL.md", "skill.md"]` | 一致，但常量**重复三处** |
| `README.md` / `CLAUDE.md` 不作标记 | 无任何代码路径把它们当入口；测试 `skill_metadata.rs:444-455`（readme-only / claude-only 均 `!is_valid`）、`393-403`（`parse_skill_md` 忽略 `CLAUDE.md`） | 一致 |
| 命中后不深入 | `scanner.rs:94-98`（命中即 `push` 后 `continue`，不再递归） | 一致 |

**常量重复的漂移风险**（本次推断）：`SKILL.md`/`skill.md` 字面量散落在 `skill_metadata.rs:32`、`:83`、`:209`、`git_fetcher.rs:1371`。当前取值相同，但任一处改动不会自动同步。新项目若只需一处识别谓词，可直接消除这层重复。

**错误文案漂移**（源码事实）：`is_valid_skill_dir` 接受 `skill.md`，但 `commands/skills.rs:3081`、`:3103` 的报错文案写作「(no SKILL.md)」，未提 `skill.md`。

## 技能名派生

入口（`skill_metadata.rs:287-297`）：

```rust
pub fn infer_skill_name(dir: &Path) -> String {
    let meta = parse_skill_md(dir);
    if let Some(name) = meta.name {
        if let Some(sanitized) = sanitize_skill_name(&name) { return sanitized; }
    }
    dir.file_name().map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown-skill".to_string())
}
```

- **优先级**：frontmatter `name:` > 目录名（`infer_skill_name_from_metadata` / `infer_skill_name_falls_back_to_dirname`，`skill_metadata.rs:551-572`）。注意目录名分支**不做清洗**，只有 frontmatter 名走 `sanitize_skill_name`。
- **frontmatter 解析方式**：自写最小解析，不引专用 frontmatter 库。`parse_frontmatter`（`skill_metadata.rs:47-76`）先 `trim`，要求以 `---` 开头，取**第一个** `---` 到其后**第一个** `---` 之间的文本作为 YAML 串，交给 `serde_yaml::from_str`（`:59`）解析，再取 `name` / `description` 两个 `.as_str()`。
- **失败回退**：无 frontmatter、frontmatter 未闭合、YAML 解析失败、字段缺失或非字符串，一律返回 `SkillMeta { name: None, description: None }`，由调用方回退目录名。**读取失败与解析失败被折叠成同一「无元数据」结果**（`parse_skill_md_with_candidates` `:35-45`，`read_named_file_exact` 读失败返回 `None`）——旧实现无法区分「文件不存在」和「读取出错」。
- **与「保留上游原名」的关系**：入库/收编时名来源是 `installer::resolve_local_skill_name`（`installer.rs:122-132`）：显式传入名优先，否则 `infer_skill_name`。即**旧实现确实优先 frontmatter `name:` 而非上游目录名**。需求已裁决移除入库改名、保留上游技能原名（`requirements.md:120`，「技能获取与收编」）。旧实现的 `sanitize_skill_name` 会**静默替换**非法字符（见下），属于要剥离的行为——它会改变实际叶子名，与「不因入库改名」冲突。

### sanitize_skill_name 清洗规则（剥离项）

`skill_metadata.rs:240-285`，返回 `Option<String>`：

1. 只取最后一个路径分量（`Path::file_name()`），借此剥离 `../`；裸 `.` / `..` 返回 `None`。
2. 控制字符与 Windows 保留字符 `< > : " / \ | ? *` 替换为 `_`（替换而非删除，保持位置近似单射）。
3. 去首尾空白、去尾部 `.`。
4. 清洗后为空返回 `None`。
5. 命中 Windows 保留设备名（`CON`、`NUL`、`COM1`…，按首个 `.` 前段大写比对）时加前缀 `_`。

源码注释说明其动机是跨平台（macOS/Linux/Windows）安全作单层目录名。**新项目相关点**：本项目要求保留上游原名、同库重名提示并阻止（`requirements.md:120`、B01），因此这套「静默改名」不应在入库路径复用；真正需要的是**校验**（拒绝不能作单层目录名的名字）而非改写。旧代码在 `copy_skill_to_domain` 里已给出这一区分：`commands/skills.rs:4956-4974` 明确 `sanitize_skill_name(leaf) == leaf` 是**检查**不是改写，因为替换会把副本落到与原技能不同的叶子名，而「另一侧同名」正是拒绝对话要表达的内容。

## frontmatter 字段

- 只读取 `name` 与 `description` 两个字段（`skill_metadata.rs:47-76`），无其它字段进入 `SkillMeta`。
- 缺失／畸形一律折叠为 `None`，不报错、不回显解析问题（同上）。
- **与中文注释注入的衔接**：注入的是 frontmatter 的 `description` 头部，形态 `中文注释｜原描述`（分隔符 `DESCRIPTION_PREFIX_SEPARATOR = '｜'`，`skill_metadata.rs:86`），因为旧观察中 Agent 技能菜单渲染的是 description 而非正文。注入/清除用 `set_description_prefix_in_content`（`skill_metadata.rs:105-206`），且**必须改写解析器实际读取的那个候选文件**——所以读取与注释改写共用候选顺序（`SKILL_MD_CANDIDATES` 的注释 `:78-83` 明说：否则小写 `skill.md` 的技能会把注释写到加载器从不打开的文件）。安装时落地见 `installer.rs:192-206`，编辑注释时见 `commands/skills.rs:4261-4276`。详细合成规则见[内容合成参考](content-composition.md)，此处仅指出**读/写入口文件必须一致**这一约定。

## 递归扫描与误判防护

### 中央/仓库扫描：`scanner.rs:61-100`

- `collect_skill_dirs(dir)` 递归遍历，返回所有含标记的子目录；`visited: HashSet<PathBuf>` 用 `canonicalize` 后的路径去重，防符号链接循环（`:68-76`，测试 `recursive_survives_symlink_cycle` `:367-384`）。
- 排除目录 `RECURSIVE_SCAN_SKIP_DIRS = [".hub", ".git", "node_modules"]`（`:34`），按**目录名精确匹配**跳过。
- 命中 `is_valid_skill_dir` 即 `results.push` 并 `continue`，**不再进入技能内部**（`:94-97`；测试 `recursive_stops_descending_into_skill_dir` `:328-338`），正是为了避免把技能内的 `references/` 等当成独立技能。
- 另有一条与中央库相关的优化谓词 `is_symlink_to_central`（`:45-56`）：跳过指向中央库两个物理根的符号链接（避免重扫应用自己部署的副本）。源码注释强调它是**优化谓词而非领域分类器**，领域归属仍由 `central_repo::domain_of` 负责，不得挪用。
- 扁平扫描 `scan_flat_dir`（`:133-154`）只扫一层，同样要求 `is_valid_skill_dir`（测试 `flat_scan_requires_skill_marker` `:386-411`）。

### 项目/工作区扫描：`project_scanner.rs:127-255`

- 与 scanner 的差别：递归深度由 `recursive` 开关控制，默认只扫一层（测试 `linked_workspace_flat_scan_ignores_nested_skills` `:467-492`）。
- `should_skip_dir`（`:114-125`）：跳过所有以 `.` 开头的目录；跳过嵌套的「另一个 `skills/` 子树」——即**当 `dir != root` 且 `dir/skills` 是目录时**判为嵌入式插件/缓存包布局（如 `<bundle>/<version>/skills/<skill>/SKILL.md`），从而不误判；`root` 自身可叫 `skills`，故用 `dir != root` 保护。
- 递归时也维护 `visited`（`canonicalize` 去重）防链接循环；源码注释指出**技能目录是叶子节点，允许别名到同一 canonical 目标**——`visited` 只在进入命名空间目录前检查（`:229-243`）。
- **命名空间目录不误判**：`research/` 这类分组目录本身不含标记，只有其下的 `web-search/` 命中标记，才会以相对路径 `research/web-search` 上报（测试 `reads_nested_project_skills_recursively` `:337-360`、`prefers_skill_dir_over_namespace_parent_dir` `:362-381`）。
- `relative_path` 统一用 `/` 展示；Windows 下替换 `\`，非 Windows 保留（unix 下 `\` 是合法文件名字符，测试 `unix_relative_path_keeps_backslash_in_file_names` `:497-520`）。
- 项目发现层 `scan_projects_in_dir`（`:257-313`）另有一套容器级规则：命中含任意 Agent 技能目录的项目即记录并**停止向下**，跳过隐藏目录、`node_modules`、`target`、`__pycache__`，受 `max_depth` 限制。注意本项目当前**不做项目专属技能管理**（`requirements.md:222`），这段属历史线索。

### Git 仓库解析：`git_fetcher.rs:1306-1391`

- 有 `skill_id` 时按固定顺序尝试：`repo/<id>` → `repo/skills/<id>` → `repo/.agents/skills/<id>`（优先统一位置而非 `.cursor/skills`、`.claude/skills` 等 provider 变体，见 `:1322-1331`），每步都要求 `is_valid_skill_dir`；随后 `walkdir` 深度 6 内按**目录名或 frontmatter `name`** 匹配；全部失败则**报错**而非回退容器根（`:1364`，问题 #278）。
- 无 `skill_id` 时按：根含 `SKILL.md|skill.md` → `skills/` → `skill/` → **默认返回仓库根**（`:1371-1390`）。
- 上层 `collect_git_skill_dirs`（`commands/skills.rs:3748-3755`）：若解析出的目录本身合法则返回自身，否则递归枚举子技能；**空结果不回退扫描根**（测试 `collect_git_skill_dirs_returns_empty_when_no_skills` `:6412-6421`）。
- 值得注意的**实质回退与测试表象**：无 `skill_id` 时 `find_skill_dir` 对任意仓库**总会返回一个目录**（默认根）。测试 `find_skill_dir_root_with_claude_md`（`:1736-1742`）看似「CLAUDE.md 被当作标记」，实际只是「无标记 → 默认根」的回退；误报在此层不会发生，因为下游 `collect_git_skill_dirs` 仍需对返回目录或其子目录通过 `is_valid_skill_dir`，仅含 `CLAUDE.md` 的仓库最终枚举为空。判读此测试时不要把它当作「CLAUDE.md 是入口」的证据。
- 安全约束（与识别耦合）：`resolve_skill_dir`（`commands/skills.rs:3839-3880`）与 `resolve_repoint_skill_dir`（`:3075-3107`）都对子路径做 `path_guard::is_path_safe` 包含性校验，防 `..` / 绝对路径逃出检出目录；有 locator 时子路径还必须仍是技能目录，否则交给 locator 按 id 重新定位。

## 与本项目需求的差距

需求依据：入库保留原名不改名、分类即目录（`requirements.md:120`、`397`）、同库重名提示阻止（B01，`legacy-skill-capabilities.md:25`）、收编接管部署（`requirements.md:142-148`）。

**可直接借鉴**：

1. **识别谓词**：`is_dir() && 存在 SKILL.md|skill.md`，精确文件名、忽略 `README.md`/`CLAUDE.md`。这套谓词与「分类即目录、最多一级」兼容，可作为统一识别入口的参考；新项目接受哪些文件名仍按后续输入格式设计确定。
2. **短路规则**：命中即不深入，天然避免 `references/`、命名空间目录误判（需求「分类层级不限制技能内部文件目录」，`GLOSSARY.md`）。
3. **相对路径 key**：`skill_rel_key`（`commands/skills.rs:3759-3768`）用 `/` 归一、空相对路径回退文件名，可在来源仓库扫描阶段区分路径；本项目同库同名仍须提示并阻止，不能据此放开（测试 `:6433-6449`）。
4. **来源路径取段**：`derive_source_category`（`:3783-3789`）取倒数第二段，只是旧默认值算法。它不统计技能数，也不提供仓库名默认值，不能声称已满足新分类流程。新需求多技能仓库默认仓库名、可修改/询问、单技能不自动分类。可借鉴分类名校验中拒绝路径分隔符与 `.`/`..` 的部分，避免分类输入变成跨目录路径。
5. **注释读/写入口一致**：`SKILL_MD_CANDIDATES` 的「改写文件 = 解析文件」约定，见[内容合成参考](content-composition.md)。

**必须剥离**：

1. **静默改名 / 自动目录后缀**。`sanitize_skill_name` 会把非法字符替换、加 `_` 前缀；`installer::unique_skill_dest`（`installer.rs:260-287`）与 `resolve_install_destination` 的 `next_candidate_leaf`（`commands/skills.rs:1514-1529`）在占用时**自动分配 `name-2`、`name-3`**。本项目同库重名要求「提示并阻止」，不复用旧条目也不覆盖（B01），所以这套自动后缀 + 静默改名都不带入。可参考的替代点：旧代码里已有「**校验而非改写**」的先例（`copy_skill_to_domain_unlocked` `commands/skills.rs:4956-4974`）。
2. **读取/解析失败折叠为「无元数据」**。旧 `parse_skill_md` 与 `read_named_file_exact` 把读失败当空结果，收编/入库时可能把「读不到」当成「无 frontmatter 名」而改用目录名。新项目若要求「保留上游原名」并需要在失败时可见，应让真实读失败区别于「字段缺失」。
3. **文件级兼容旧模型**：`find_skill_dir` 无 id 时默认回退仓库根、按 `.agents/skills` 优先等规则，属多 Agent 生态的经验；是否采用由本项目对「入口文件」「仓库布局」的定义决定，不因旧代码存在而继承。

**旧项目未覆盖之处**：旧规约只讨论中央库、项目/工作区、本地导入、Git 发现四条路径，**没有**「同一库内重名阻止」「保留上游原名、不改名」的策略（这正是要剥离的旧行为）。本项目需要的新增点是**识别后不再自动改名**，这在旧实现里不存在对应物。

## 未确认事项与验证状态

- 全部结论为**静态源码阅读**，未运行旧测试或应用；「命中即停」「符号链接循环」「CLAUDE.md 非标记」等均有旧单测断言支撑（`skill_metadata.rs:429-469`、`scanner.rs:311-384`、`project_scanner.rs:337-465`、`git_fetcher.rs:1729-1811`），但本次未执行。
- 外部约定为 **2026-04-10 历史观察**，**本次未核对** skills.sh、vercel-labs/skills、vercel-labs/agent-skills 的当前一手实现。
- `path_guard::sanitize_name`（`path_guard.rs:28`）在 `src-tauri/src` 内**未发现除自身测试外的调用点**（静态 grep）；其定位与 `skill_metadata::sanitize_skill_name` 的取舍属旧内部实现细节，本文不展开。
- 后续输入格式设计：本项目是否接受 `skill.md` 兼容标记；是否维持「`README.md` / `CLAUDE.md` 永不作为技能标记」。旧规约立场为接受 `skill.md`、拒绝后两者，理由是误报防护。

## 来源定位

- `FeiSkillDock/docs/skill-format-detection-spec.md`（2026-04-10 规约，历史观察）。
- `src-tauri/src/core/skill_metadata.rs`：`parse_skill_md`（31）、`parse_frontmatter`（47）、`SKILL_MD_CANDIDATES`（83）、`DESCRIPTION_PREFIX_SEPARATOR`（86）、`SKILL_DIR_MARKERS`（209）、`is_valid_skill_dir`（213）、`sanitize_skill_name`（240）、`infer_skill_name`（287），测试 429-572。
- `src-tauri/src/core/scanner.rs`：`RECURSIVE_SCAN_SKIP_DIRS`（34）、`collect_skill_dirs`（61）、`scan_flat_dir`（133）、`scan_recursive_dir`（156），测试 311-384。
- `src-tauri/src/core/project_scanner.rs`：`should_skip_dir`（114）、`read_skills_from_dir`（127），测试 331-521。
- `src-tauri/src/core/git_fetcher.rs`：`find_skill_dir`（1306），测试 1728-1890。
- `src-tauri/src/commands/skills.rs`：`validate_category_name`（1203）、`next_candidate_leaf`（1514）、`resolve_install_destination`（1554）、`resolve_repoint_skill_dir`（3075）、`collect_git_skill_dirs`（3748）、`skill_rel_key`（3759）、`derive_source_category`（3783）、`resolve_skill_dir`（3839）、注释注入（4261）、批量导入（4743）、跨域复制叶子校验（4956）。
- `src-tauri/src/core/installer.rs`：`install_from_local`（83）、`resolve_local_skill_name`（122）、`unique_skill_dest`（267）。
- `src-tauri/src/commands/scan.rs`：`scan_local_skills`（71）、`import_existing_internal`（117）、`import_all_discovered_internal`（207）。

不保留旧常量重复、旧自动目录后缀、`sanitize_name` 静默改名、旧中央库导入链路与整套旧测试门禁。
