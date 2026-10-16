# M1 验收报告：底座——库与 `.manager`

**日期**：2026-10-15 | **范围**：T001～T020（Phase 1/2 + US1） | **依据**：[quickstart.md](../quickstart.md) M1、[spec.md](../spec.md) US1、[tasks.md](../tasks.md) Phase 3

**证据说明**：CLI 与核心结论来自本机实际执行（命令与输出摘要随条目记录）；GUI 结论来自真实桌面应用（`npm run tauri dev` + 临时 Git 库 `/tmp/m1-gui/vault`）截图，截图位于 `/tmp/m1-gui/`。**未取得证据的项一律记为未验证，不按"实现完成"记通过。**

## 一、场景核对表

| 场景号 | 预期 | 实际 | 结论 |
|---|---|---|---|
| 1.1 启动 GUI 首次选择 SkillVault | 直接进入技能库；两域技能、分类、标签、注释可见 | 用本机状态预置库路径启动，应用直接进入技能库；两域技能（code/lobster）、分类列、标签栏、中文短注释均可见（截图 `21-startup-library.png`、`01-library-code.png`、`23-lobster.png`）。**原生目录选择器的人工点击未验证**（无头环境无法稳定驱动原生对话框） | 部分通过：进入库与信息展示通过；首次选择对话框未验证 |
| 1.2 搜索 / 分类+标签组合筛选；分类联动标签 | 标签候选只含该分类内实际标签；"全部分类"恢复全部 | 选中"未分类"后标签候选只剩该分类实际标签、网格收窄为 2 条（截图 `14-category-browser.png`）；分类联动标签候选、"无标签"筛选、组合搜索、切换分类后失效标签自动清除均有 CLI/核心断言（`tests/cli_library.rs` 14 项、`crates/core/tests/vault_management.rs` 16 项） | 通过（分类联动与筛选）；搜索框人工输入未取截图，行为由核心测试覆盖 |
| 1.3 打开带译文的技能详情 | 默认中文、可切原文；无译文显示原文 | 详情正文默认中文、提供"中文/原文"切换，原文变化后显示"译文待更新"且译文保留不清除（截图 `03-detail-read.png`）；无译文技能显示原文、`skill read --lang orig` 行为由 CLI 测试覆盖 | 部分通过：默认中文与待更新提示有截图；切换原文后的界面状态未取截图 |
| 1.4 改分类 A→B（GUI 与 `skill move` 各一次） | 目录实际移动、名字不变、筛选可找到 | `skill move code/beta --category 工具`：`vault_rel_dir=code/工具/beta`，旧目录消失、新目录含 `SKILL.md`，`skill list --category 工具` 可见，`skill get code/beta` 仍按原名解析；回移 `--uncategorized` 正常。**GUI 侧"移动到分类"弹窗的保存未点通验证** | 部分通过：CLI 路径通过；GUI 保存路径未验证 |
| 1.5 `skill set` 改标签/注释/支持声明 → GUI 查看 | 界面自动刷新，无需重启 | 应用运行中用 CLI 改注释，卡片自动变为"（技能路由（外部改动））"，无需重启（截图 `13-after-external.png`）；`skill set` 的 meta.yaml 写回与索引重建均有测试 | 通过 |
| 1.6 手工编辑 `meta.yaml` 加未知字段 → 保存 → 查看 | `foo: bar` 仍存在 | 手工加 `foo: bar` 与 `future_field.nested: 1` 后执行 `skill set --comment 新注释`：两个未知字段原样保留、键顺序未变、仅 `comment` 改变。**GUI"管理资料 → 保存"的落盘未验证** | 部分通过：核心写回规则通过；GUI 保存未验证 |
| 1.7 现有库写入口核对同域同名限制 | 同域阻止、跨域允许；M3 覆盖创建/入库/收编/复制 | `skill move` 移到已有同名目录的分类：退出码 1、错误含目标路径与技能引用、原目录未动；`code/工具/alpha` 与 `lobster/alpha` 跨域同名并存。`skill create/import/adopt/copy` **入口尚不存在**，clap 报 `unrecognized subcommand`（退出码 2），不返回伪成功 | 通过（M1 范围内的唯一现存写入口）；创建/入库/收编/复制的同名检查**待 M3 复核** |
| 1.8 `skill list --json` / `skill get --json` | 稳定 JSON schema | `skill list`：`{skills,total,tag_candidates,categories}`，技能摘要含 15 个固定键（缺资料为 `null`，不省略键）；`skill get`：`{content,files,deployments,deployments_note,skill,vault_path}`。错误退出码 1（技能不存在/未知域/未接入库等）、2（clap 参数冲突） | 通过 |

## 二、GUI 视觉与交互对照

对照基线：原型效果图 `specs/001-skill-management-prototype/ui-prototype/renders/01-code-library.png`，同尺寸 1720 × 1040。

| 原型页面/状态 | 实际截图（`/tmp/m1-gui/`） | 结论与差异 |
|---|---|---|
| 技能库（01-code-library） | `01-library-code.png`、`21-startup-library.png` | 通过：左栏 220px 分组导航、页标题+数量徽标+副标题+分隔线、搜索框/分段筛选/网格列表切换/多选的工具形态、标签栏、一级分类列、三列卡片网格、卡片"状态点→原名+注释→描述→标签→底部来源"分层与密度一致。**差异**：原型工具栏的"检查全部/更新(n)/新增(n)/+入库"与卡片底部 Agent 部署图标属 US2/US3/US4，本期不出现（不伪造未实现入口）；部署状态分段灰显并标注 M2 |
| 龙虾技能库（02-lobster-library） | `23-lobster.png` | 通过：与编码库共用同一视觉体系，跨域同名技能并存 |
| 列表密度 | `24-list-layout.png` | 通过：状态点/原名/描述/标签/状态与操作的位置与原型一致 |
| 技能详情与文件目录树（22-skill-file-tree） | `03-detail-read.png`、`05-detail-files.png` | 通过：弹窗外壳与分区、可折叠目录树含缩进与分支线、点击文件阅读后可返回目录树。**差异**：详情增加"来源"标签页承载 FR-010 的来源地址/引用/子目录/采用版本/库内位置（原型把来源放在副标题与独立弹窗）；其余四个分区与原型一致 |
| 管理资料紧凑布局（31-manager-compact-layout） | `04-detail-manager.png` | 通过：分类与系统支持在左列、标签与 Agent 支持在右列、底部固定保存行。"全部支持（未声明）"与显式名单区分正确 |
| 覆盖层维护 | `06-detail-overlay.png` | 通过：列表 + 维度徽标 + 阅读/移除/添加 |
| 系统不适用整项灰显（27-windows-unsupported） | `14-category-browser.png`（"不支持当前系统"徽标） | 通过：Linux 专用技能在 macOS 下整项灰显、原因可读、仍可维护资料 |
| 外部改动实时刷新（US1-5） | `13-after-external.png` | 通过：应用运行中 CLI 改注释，卡片自动刷新 |

**未取得界面证据的交互（记为未验证，不得视为通过）**：卡片三点菜单、多选批量栏、添加标签/移动到分类/管理标签弹窗、详情"来源"标签页、文件点击阅读流程、**GUI 保存（管理资料/译文/覆盖层/分类移动）**。原因：本环境用 System Events 合成点击在 webview 上投递不稳定（AX 能命中元素但 press 无效）。其中"GUI 保存"属功能性断言，未取得证据即不通过。

**前端与后端接线核对（补充证据）**：`src/services/client.ts` 中 21 个 `invoke` 名称与 `src-tauri/src/lib.rs` 注册的 21 个 command 逐一对齐（脚本比对，无差集）。这证明调用契约一致，**不代替**上述界面点击验证。

## 三、自动化验证

| 命令 | 结果 |
|---|---|
| `cargo test --workspace` | 通过：CLI 集成 14 + core 单元 39 + core 集成 16 = 69 项，0 失败 |
| `cargo build --workspace` | 通过，0 警告 |
| `npx tsc --noEmit` / `npm run build` | 通过 |
| `npm run test`（vitest） | 通过：1 文件 / 7 用例（支持声明合并：保留本机未显示的已声明 Agent、显式空集合、系统不适用判定） |
| `cargo fmt --check` | **未执行**：本机 toolchain 未安装 rustfmt；代码按现有风格手写 |

## 四、遗留与待后续里程碑复核

1. **T019 的界面点击验证**（上节"未取得界面证据的交互"）需人工复核，尤其 GUI 保存的落盘与刷新。
2. **1.7 的创建/入库/收编/复制同名检查**：已于 M3 复核通过（见 [m3-report.md](m3-report.md) 第四节：入库大小写不同同名阻止、跨域同名允许、自建同名阻止，均本机实测）。
3. **`--preset` / `--deployed-to` 筛选**在当前入口返回"尚未接入（M2/M5）"错误；`skill get` 的 `deployments` 为空数组并附 `deployments_note` 说明。部署位置、部署刷新与上传分别在 M2/US3、M5/US5 接通后复验。
4. **文件监听**已接成 Tauri 事件（250ms 去抖，`vault-changed`）并有单元测试与一张外部改动截图；不因监听事件自动提交、推送或部署。
5. **Agent key 未校验**：`--agents/--oses` 接受任意字符串（不校验是否在内置 53 项名单内）。后果可接受：回显即暴露，用户可立即修正；未增加校验机制。
6. **依赖变更**：为对齐 `@tauri-apps/api` 2.12.4，`Cargo.lock` 中 tauri 2.11.6→2.12.4、tauri-plugin-dialog 2.7.3→2.8.3、wry→0.57.0（同主版本），构建与测试通过。
7. **本机副作用**：`~/Library/Application Support/SkillManager` 与 `/tmp/m1-gui/`（临时库、状态目录、截图、dev 日志）为本次验证产物，未清理以便复核。
8. 未执行 `git commit` / `git push`。
