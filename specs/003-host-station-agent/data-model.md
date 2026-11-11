# Data Model: Host、管理站与对话状态

**Date**: 2026-11-11 | 依据：[spec.md](spec.md)、[research.md](research.md) D1–D15。

技能文件、`.manager`、预设、指令源、工具资料与 002 部署/Agent/Hermes 表**不在此重复**。本文件只增加入口层实体。共享资产仍只在 SkillManagerVault；下列全部为**本机状态**或**管理站状态**，丢失不删库内技能。

## 一、Host 本机（`state-dir/skill-manager.db` 增表）

与 002 同一数据库，不加技能镜像。`state-dir` 另有咨询性锁文件 `host.lock`（D5）。

### 1.1 `host_identity` — 本机作为设备

| 列 | 说明 |
|---|---|
| device_id | 本机稳定标识，对接时生成，解除后可重新生成 |
| display_name | 用户可改的名称，默认主机名 |
| station_url | 已保存的管理站地址；空表示未对接 |
| device_key | 与管理站共享的设备密钥（只存本机与管理站，不进 Vault） |
| paired_at | 最近一次成功对接时间 |

同一 `state-dir` 一行。解除对接清除 station_url 与 device_key。

### 1.2 `operation_plans` — 破坏性确认

| 列 | 说明 |
|---|---|
| plan_id | Host 签发 |
| session_id | 签发时的界面会话；确认必须带同一 session_id |
| action | `skill.delete` 或 `deploy.overwrite`（含丢弃副本独有改动） |
| payload | 对象预览与参数（技能引用、目标路径、将清理的部署） |
| created_at / expires_at | 过期未确认作废 |
| confirmed_at | 空=未确认；确认后 Host 才执行 |

非删除、非覆盖的写入不建计划。过期或 session 不匹配的确认必须拒绝。

### 1.3 `agent_sessions` / `agent_messages` — 内置管理 Agent 对话

| 表 | 说明 |
|---|---|
| agent_sessions | session_id、界面会话、创建时间；钉死的上下文快照 JSON：`global`（摘要 + 计数 + 引用，含待更新/译文待更新/部署/事故及预设·Hermes·工具·本机 Agent 目录摘要）+ `view`（Host 按引用补齐的当前页详情）。不存整库正文。客户端上报的原始 `view` 引用可另存，但详情以发消息时 core 为准。 |
| agent_messages | session_id、角色（用户/助手/系统）、正文、关联 task_id、时间 |

不写入 Vault，不随库同步。任务对象以该条用户消息发出时 Host 组装的快照为准。任务状态含进行中 / 成功 / 部分失败 / 失败 / **已取消**。

### 1.4 `agent_runtime` — 本机管理 CLI

| 列 | 说明 |
|---|---|
| cli_id | 当前 ACP 宿主标识（用户指定或探测到的名字，如 `opencode`、`custom-host`）。不是三家许可名单 |
| binary_path | 本机可执行文件，不进安装包 |
| acp_args | JSON 数组，ACP 启动参数；空则按常见约定尝试 `acp` / `--acp` |
| user_selected | 用户经 `host agent-cli` / `PUT /agent/runtime` / 环境变量登记则为 1 |
| detected_at / installed_by_app | 检测或由软件安装引导宿主 |

模型密钥不出现在此表，不出现在管理站。

## 二、管理站（`station-dir/station.db`）

独立 SQLite，**无**技能、部署、对话正文。

### 2.1 `admin` — 唯一管理员

| 列 | 说明 |
|---|---|
| username | 单一账号 |
| password_hash | 密码哈希 |
| totp_secret | 绑定后保存；绑定过程向用户展示二维码、密钥 |
| recovery_code_hashes | 一次性恢复码只存哈希 |
| session_ttl_seconds | 保持登录时长，可配置 |

一行。首次 `station serve` 无此行则走创建流程。公网不得自助再开账号。本地 `station reset-admin` 清此行并要求重新创建。

### 2.2 `sessions` — 管理员登录

| 列 | 说明 |
|---|---|
| session_id | 发给浏览器 |
| created_at / expires_at | 按 session_ttl |
| last_device_id | 当前切换到的设备，可空 |
| pending_rebind | 恢复码登录后为 1；完成 TOTP 重绑前不是完整管理员会话，不得转发/管设备 |
| pending_totp_secret | 待写入的新 TOTP；`POST /auth/totp/rebind` 确认前不进 `admin` |
| pending_recovery_hashes | 待写入的新恢复码哈希；确认前不进 `admin`。明文恢复码不落库 |

### 2.3 `devices` — 已对接 Host

| 列 | 说明 |
|---|---|
| device_id | 与 Host `host_identity.device_id` 相同 |
| display_name | 可在管理站改 |
| device_key_hash | 只存哈希 |
| last_seen_at | 长连接心跳；超时视为离线 |
| revoked | 解除后为真，原密钥失效 |

离线可见不可操作。不存 Vault 路径、不存技能列表。

### 2.4 `pairing_codes` — 一次性对接码

| 列 | 说明 |
|---|---|
| code_hash | 明文只展示一次 |
| expires_at | 过期作废 |
| consumed_at | 使用后作废 |

填错/过期/已用：该码失败，已对接设备不变。

## 三、安装 Skill（共享资产，但是本仓库的技能源）

路径：`skill-manager/assets/skills/skill-manager-install/`。随本项目仓库维护，按普通上游技能入库到用户 Vault。**不是** `station.db` 或 Host 表。与 `skill-manager-maintenance` 并列，不得合并。

## 三之一、内置管理说明（应用资产，不入库）

路径：`skill-manager/assets/skills/skill-manager-builtin-agent/`。随本项目仓库维护，由 Host 注入到所用 Agent CLI 的 skills 搜索目录（或 ACP 初始化文本）。**不是**用户 SkillManagerVault 里的技能，不出现在 `skill list`，不可部署，不按普通上游入库。与安装 Skill、配套维护 Skill 都分开。

## 四、校验与状态

- Host：同一 `state-dir` 只能有一个进程持有 `host.lock`。
- 设备在线 = 管理站上该 `device_id` 的长连接仍在。
- 操作计划：未确认 / 已确认 / 已过期 / 会话不匹配（拒绝）。
- 对话任务：进行中 / 成功 / 部分失败 / 失败 / 已取消；「任务已生成」≠ 已写库；已取消 ≠ 成功。
- `git_auto_push` 仍是 002 设置表字段，本模型不新增上传授权表。
