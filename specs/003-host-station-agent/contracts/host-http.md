# Host HTTP：与 Tauri invoke 同名的本机入口

**Date**: 2026-11-11 | 研究决策 D3。本契约给桌面以外的界面和「Host 在场时的 CLI」使用。业务结果与现有 Tauri command / 002 CLI 相同。

Host 默认只接受本机回环与已认证的管理站长连接，不对公网提供登录页。

## invoke

```text
POST /invoke/<command>
Content-Type: application/json

{ ... 与当前 Tauri invoke 第二个参数相同的字段 ... }
```

- `<command>` 与 `src-tauri` 已注册名一致（`skill_set`、`deploy_to`、`sync_push` 等）。
- 响应 JSON 与现有 command 返回值一致；错误不改写为「成功」。
- 删除技能、覆盖类 command：未带有效 `planId` 时不得执行，返回需要确认的操作计划（见下）。其它写入直接执行。
- `git_auto_push` 设置项：仅当请求来自图形界面会话时可写；内置管理 Agent 与 CLI 不可写。

## 事件

```text
GET /events
```

SSE。事件名至少包括现有 `vault-changed`、`schedule-finished`。远程网页与桌面订阅同一类事件。

## 会话与确认

```text
POST /session          → { sessionId }
POST /plans            → 对给定 destructive command 生成计划 { planId, action, preview, expiresAt }
POST /plans/:id/confirm  { sessionId }  → 仅签发该计划的 sessionId 可确认
```

`POST /plans` 也可由 Host 在 invoke 缺 planId 时内联返回，不必强迫前端多打一次，但确认必须是显式第二步。

## 对话上下文（给内置管理 Agent）

```text
PUT /agent/context     { sessionId, view }
POST /agent/messages   { sessionId, text }
POST /agent/tasks/:id/cancel   { sessionId }
GET  /agent/sessions
GET  /agent/sessions/:id/messages
```

`view` 只含页面身份与选中引用，例如 `{ page, domain, selection: { refs, category, presetId, profile, toolId, agentKey } }`，**不含**技能正文、注释全文、文件树或报错原文。

Host 在 `POST /agent/messages` 时从 core 组装两层并钉死到该条任务：

- `global`：当前 Host 的摘要 + 计数 + 引用（见 `agent-bridge.md`），不由浏览器拼；
- `view`：按刚上报的引用补齐详情。引用已失效则如实标空，不得沿用过期正文。

后续 `PUT /agent/context` 只更新界面层引用，只影响之后的新消息。`POST /agent/tasks/:id/cancel` 停止该任务继续写库，已取消不得标成成功。不得要求用户粘贴报错、清单或技能正文。

## 本机状态

```text
GET /host/status
```

与 `host status` CLI 相同事实：是否接入 Vault、对接、调度是否在跑。不得把未检查写成已更新。
