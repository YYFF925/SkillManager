# Requirements Readiness Checklist: 本机 Host、统一管理站与内置管理 Agent

**Purpose**: 实施前审视 003 需求是否完整、清楚、一致、可验收；对象是规格与规划文字，不是软件是否已经跑通。
**Created**: 2026-11-11
**Feature**: [spec.md](../spec.md)
**Depth**: 标准；覆盖 US1–US3（Host 接入、管理站切换、内置管理 Agent），不扩到冲突合并或主动维护。
**Audience / Timing**: 需求作者；实现规划已完成、任务拆分前。
**Input**: [plan.md](../plan.md)、[research.md](../research.md)、[data-model.md](../data-model.md)、[contracts/](../contracts/)、[quickstart.md](../quickstart.md)、[ADR 0002](../../../docs/adr/0002-host-management-station-and-agent.md)、[宪章](../../../.specify/memory/constitution.md) 2.1.0。

**Note**: 由 `$speckit-checklist` 根据当前功能上下文生成。本清单审视需求文字质量。
**Review Ownership**: 勾选状态由评审者维护，只有评审确认需求质量标准已满足才标记 `[x]`。
**Marker Semantics**: `[x]` 只表示该项需求质量已评审通过，不表示功能开发完成。`[Gap]` / `[Ambiguity]` / `[Conflict]` / `[Assumption]` 是待核对标记，不是评审结论。

## Requirement Completeness

- [ ] CHK001 是否完整说明 Host、管理站、SkillManagerVault 各自持有什么、不持有什么，且管理站不得成为第二份技能库？[Completeness, Spec §FR-001/011, Spec §Key Entities]
- [ ] CHK002 无窗口接入是否同时写明 clone 命令、开机服务、启动命令、安装 Skill 四条路径，并排除图形克隆向导与应用内建库？[Completeness, Spec §US1, Spec §FR-003～005]
- [ ] CHK003 管理站是否写全唯一管理员创建、密码+一次性密码、恢复码、保持登录、本机重置，且排除公网自助开新账号？[Completeness, Spec §US2-1～3, Spec §FR-007/008]
- [ ] CHK004 设备对接是否写明出站、一次性码、改名、解除、在线/离线可见性，以及离线不可操作？[Completeness, Spec §US2-4～6, Spec §FR-009/010]
- [ ] CHK005 远程界面相对 002 的覆盖与排除是否成对出现（技能管理全套 vs 托盘/关窗/桌面更新；路径属于 Host）？[Completeness, Spec §FR-012, Spec §US2-7～8]
- [ ] CHK006 内置管理 Agent 是否写明：跑在当前 Host、覆盖 GUI 同类动作、页面上下文、任务钉死对象、不主动维护、对话不进 Vault？[Completeness, Spec §US3, Spec §FR-014～018]
- [ ] CHK007 破坏性确认与上传授权是否分别定义：哪些必须计划确认、哪些禁止逐步确认、外部编码 Agent 的每次询问是否仍有效？[Completeness, Spec §FR-019/020]
- [ ] CHK008 安装 Skill 与配套维护 Skill 的职责分界是否写进规格，而不是只出现在术语表？[Completeness, Spec §FR-005, Spec §US1-4]

## Requirement Clarity

- [ ] CHK009 「内置管理 Agent」与设置页「内置 Agent 名单」是否在规格正文中有不可互换的定义？[Clarity, Spec §FR-014, GLOSSARY]
- [ ] CHK010 「完整管理界面」是否具体到 002 的能力清单，避免被读成「看起来差不多即可」？[Clarity, Spec §FR-012]
- [ ] CHK011 「一键命令」是否在契约中落到可执行的命令与失败分段（clone 成功但对接失败不得说成库没 clone）？[Clarity, Contracts §cli.md, Spec §FR-004]
- [ ] CHK012 「任务已生成」与「已经改库」的禁止混淆是否同时覆盖模型未登录、确认未完成、上传未授权三类？[Clarity, Spec §FR-022, Spec §Edge Cases]
- [ ] CHK013 远程网页算 GUI（可写自动上传授权）与内置管理 Agent 不得写该授权，是否写清而未混在「都不逐步确认」一句话里？[Clarity, Spec §FR-020, Research D6, Agent-bridge]
- [ ] CHK014 「随系统开机启动、关机即停」是否排除唤醒休眠和额外保活，避免被读成必须常驻成功？[Clarity, Spec §FR-003, Spec §FR-021]

## Requirement Consistency

- [ ] CHK015 本规格取代 002 US8 场景 6 的表述，是否与「不进入没有 Host 的机器」同时成立、没有互相取消？[Consistency, Spec §Input, Spec §FR-021, ADR 0002]
- [ ] CHK016 `git_auto_push` 不得 CLI 可写（002）与内置管理 Agent 经 Host 调 CLI、远程网页可写该设置，三者是否无冲突？[Consistency, Spec §FR-020, Host-http, 002 FR-038]
- [ ] CHK017 「CLI 在 Host 在场时走 Host、不在场直连 core」是否与「同一 state-dir 只有一个 Host」及桌面内嵌 Host 一致？[Consistency, Spec §FR-001/006, Research D5]
- [ ] CHK018 管理站独立 SQLite、Host 对话表、Vault 文件三者的存放约定是否与宪章 II / ADR 0001 一致？[Consistency, Data-model §一/二, Constitution VII/II]
- [ ] CHK019 契约把破坏性操作放在 Host 发票，规格把确认绑在界面会话，是否同一套 session 语义？[Consistency, Spec §FR-013/019, Host-http, Data-model §1.2]
- [ ] CHK020 ACP 默认命令行「规划阶段锁定」与 research D9「探测后锁定」是否被读成同一门控，而不是规格未决？[Consistency, Spec §Assumptions, Research D9]

## Acceptance Criteria Quality

- [ ] CHK021 SC-001 的「3 分钟」是否写明起止（已在线设备、含登录、不含首次 clone）？[Measurability, Spec §SC-001, Plan §Performance]
- [ ] CHK022 删除未确认不得发生、连续上传不逐步确认，是否能不依赖具体 UI 组件名称而验收？[Measurability, Spec §SC-004]
- [ ] CHK023 「不因远程管理多出第二份库内容」的判定对象是否明确（各机克隆 vs 管理站存储）？[Measurability, Spec §SC-007, Spec §FR-011]
- [ ] CHK024 对话切页不串对象（SC-006）是否同时有进行中任务与新指令两条可观察结果？[Acceptance Criteria, Spec §SC-006, Spec §US3-5]

## Scenario Coverage

- [ ] CHK025 主路径是否覆盖：无窗口 clone+serve、管理站登录切换改注释、桌面对话改注释？[Coverage, Spec §US1～3]
- [ ] CHK026 交替路径是否覆盖：管理站不可用时的本机桌面/CLI、Host 在场与不在场的 CLI？[Coverage, Spec §US1-6, Spec §SC-005, Quickstart §4]
- [ ] CHK027 两个浏览器同时管一台、破坏性确认绑会话，是否有需求而不是只在规划里？[Coverage, Spec §US2-9, Spec §FR-013]
- [ ] CHK028 内置管理 Agent 组合多步（如介绍仓库并导入）是否有规格级要求，或被明确推迟？[Coverage, Spec §FR-015, Spec §US3 引言]

## Edge Case Coverage

- [ ] CHK029 对接码过期/已用/填错、clone 失败不覆盖已有目录，是否都有规格结果？[Edge Case, Spec §Edge Cases, Spec §US1]
- [ ] CHK030 Host 中途掉线时「不得把失败显示成成功」是否也约束管理站转发层（不得用缓存正文顶上）？[Edge Case, Spec §Edge Cases, Station-http]
- [ ] CHK031 会话过期后提交破坏性确认、模型未登录就对话，是否定义为拒绝/说明而非部分成功？[Edge Case, Spec §Edge Cases, Spec §FR-022]
- [ ] CHK032 桌面窗口与远程网页同时管同一台、两套 serve 抢同一 state-dir，是否都有唯一 Host 结论？[Edge Case, Spec §US1-5, Spec §Edge Cases]

## Non-Functional & Security

- [ ] CHK033 公网暴露面是否限定为管理站登录，Host 不对公网提供登录页的需求是否可检查？[Coverage, Spec §FR-009, ADR 0002]
- [ ] CHK034 设备密钥、TOTP 密钥、恢复码、模型密钥各自允许出现的位置是否写清（含「不进 Vault / 不进管理站」）？[Completeness, Data-model §1.1/2.1, Spec §FR-017]
- [ ] CHK035 单管理员、禁止角色/租户/人际共享，是否与「多设备切换」需求同时写清以免做成多人产品？[Consistency, Constitution VII, Spec §FR-008, Spec §FR-010]
- [ ] CHK036 性能/规模是否仅按个人数台设备与 SC-001 陈述，没有把未承诺的并发指标写进规格？[Clarity, Plan §Scale/Performance, Spec §SC-001]

## Dependencies & Assumptions

- [ ] CHK037 「用户已有 Vault 远端、已有 HTTPS 反代、云电脑上常有编码 Agent」是否标为假设而非静默前提？[Assumption, Spec §Assumptions]
- [ ] CHK038 002 仍为技能管理真值、本规格只取代列明条款，是否可被实施者用来决定改不改 002 正文？[Dependency, Spec §Input, Spec §Assumptions]
- [ ] CHK039 登录页/设备条/对话的视觉是否明确交给设计层，规格只约束行为？[Assumption, Spec §Assumptions, Plan §D11]
- [ ] CHK040 冲突专题、主动维护、跨设备总览、管理站自行申请证书，是否在规格中列为排除而非遗漏？[Coverage, Spec §FR-021]

## Ambiguities & Conflicts

- [ ] CHK041 「覆盖」是否穷尽了必须走确认计划的动作（部署覆盖副本、丢弃副本独有改动），避免实现把普通部署也做成确认？[Ambiguity, Spec §FR-019]
- [ ] CHK042 默认 Agent CLI「未锁定」与安装 Skill「选型锁定后再写登录步骤」是否允许第一版先交付 Host/管理站、对话稍后，或必须同一版本一起交付？[Ambiguity, Spec §FR-005/017, Research D9]
- [ ] CHK043 远程路径「填 Host 上的路径」是否允许 Host 侧浏览目录，或仅文本输入？规格与规划是否一致？[Ambiguity, Spec §US2-8, Research D11]
- [ ] CHK044 002 US5「Agent 修改后必须询问上传」在本规格下是否仅指外部编码 Agent，避免实施者继续拦内置管理 Agent 的每次 push？[Conflict, Spec §FR-020, 002 §US5-2]

## Notes

- 勾选表示评审认为该项需求质量已满足，不表示代码已实现。
- `checklists/requirements.md` 由 specify/clarify 维护，与本清单分开。
- 本轮未生成任务文件；tasks.md 不在本次质检输入中。
