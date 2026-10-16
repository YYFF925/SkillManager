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
| 6.2 | 设置共享目录后列出 | `settings set-shared-dir --path /tmp/vfy/agents-skills` → `deploy to shared code/helper` → `agent local-skills shared` 输出：`shared：/tmp/vfy/agents-skills · 2 项（受管理 1）`，`helper（已同步）` 与 `loose-helper（仅本地）`；GUI 同一页显示 `1 个受管理`、卡片分别标「已同步 · 受管 · code/helper」与「仅本地」，并给出筛选（全部／受管理／仅本地／有变更）与既有说明「仅本地：直接删除实际文件，不改 SkillVault；受管理：按撤销部署规则处理」 | 通过（证据 `evidence/shared-agent-listing.jpg`） |
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
