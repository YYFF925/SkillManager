# CLI Contract: Host、管理站与接入

**Date**: 2026-11-11 | 技能/部署等既有命令仍以 [002 contracts/cli.md](../../002-skill-management/contracts/cli.md) 为准。本文件只增加入口层命令。全局 `--json`、`--vault`、`--state-dir` 含义不变。

Host 已在对应 `--state-dir` 运行时：写入类既有命令由 CLI 转到该 Host，不另开一套 core 写入。Host 未运行：直连 core，并占用同一咨询锁。

`skill-manager` 仍是安装包内那一个可执行文件。

## host — 本机管理服务

```text
host serve
host install-service
host uninstall-service
host pair --station <url> --code <pairing-code> [--name <display>]
host unpair
host status
host agent-cli
host agent-cli --bin <path-or-name> [--args acp] [--id <cli-id>]
host agent-cli --clear
```

- `serve`：前台运行 Host（启动命令）。已有实例占锁则失败并说明。
- `install-service`：登记开机自启，不唤醒休眠。
- `pair`：出站对接管理站；码错误/过期/已用则失败，不影响其它设备。
- `status`：是否在跑、Vault 路径、是否已对接、锁是否被占。
- `agent-cli`：查看或登记内置管理 Agent 所用的 ACP CLI。任意兼容命令行，模型由该 CLI 决定，不限 OpenCode / Codex / Gemini。`--bin` 为绝对路径或 PATH 名；`--args` 为 ACP 启动参数（省略则按常见约定尝试）。`--clear` 取消用户指定。与 002 的 `agent` 子命令（部署目标名单）不是一回事。

## station — 统一管理站

```text
station serve [--listen <addr>]
station reset-admin
station session-ttl [--seconds <n>]
```

- `serve`：HTTP 入口，供已有 HTTPS 反代指过来。第一版不在进程内申请证书。`--listen` 默认本机回环或反代可达地址，不要求对公网暴露原始端口策略。
- 首次启动若无管理员，在本机完成创建（密码 + 绑定一次性密码）。不提供公网注册。
- `reset-admin`：只能在管理站这台机器执行，清除管理员并要求重新创建。
- `session-ttl`：读取或设置登录保持时长（秒，最小 60）。省略 `--seconds` 只读取。

## vault — 新机器接入

```text
vault clone <remote-url> --path <dir>
```

- 调用本机 `git clone`，成功后把该路径记为本机 Vault。
- 目标目录已存在且非空则失败，不覆盖。
- 不提供图形向导，不在应用内 init 一座新中央库。
- 可与随后的 `host serve` / `host pair` 衔接；安装 Skill 按此顺序写，不另发明第二套参数。

`--pair` 若作为 `vault clone` 的附加选项实现，语义等于 clone 成功后立刻 `host pair`，失败时 clone 已成功则如实分段报告，不得把「未对接」说成「库没 clone」。
