---
name: skill-manager-install
description: 在一台新电脑上接入 Skill Manager：clone SkillManagerVault、启动或安装本机 Host、对接统一管理站；不含日常库维护。锁定管理 CLI 后补登录该 CLI。
---

# Skill Manager 安装（接到一台电脑）

这是给**已有编码 Agent** 用的安装 Skill，与库内配套维护 Skill（`skill-manager-maintenance`）分开：
本文件只把 Skill Manager **接到这台电脑**，不维护库内技能、上游、部署或工具中心。

不要改写配套维护 Skill。不要把本说明入库成用户技能。

## 一、要达成的结果

1. 本机有一份已有远端的 SkillManagerVault 克隆（不 init 新中央库）。
2. 本机 Host 能前台跑（`host serve`）或已写入当前用户开机项（`host install-service`）。
3. Host 已用一次性码出站对接管理站（`host pair`）。第一版没有 `vault clone --pair`。
4. 管理 CLI 登录只在第三节；**本节不含管理 CLI 登录**。

## 二、命令（相对安装包内同版本 CLI）

先确认 `skill-manager --help` 有 `vault` / `host` / `station`。不要猜命令。

### 1. Clone 已有 Vault

目标目录必须不存在或为空。已有非空目录则失败，不得覆盖。

```text
skill-manager vault clone <vault-remote-url> --path <empty-dir>
```

成功后再用 `--vault <empty-dir>`。失败时如实报告 git 错误，不要说成 Host 已在跑。

### 2. 启动 Host

```text
skill-manager --vault <empty-dir> --state-dir <state-dir> host serve
skill-manager --state-dir <state-dir> host status
```

同一 `state-dir` 再 `host serve` 必须失败（咨询锁）。关掉后再执行同一条启动命令应能再起来。

### 3. 开机服务（当前用户，不唤醒休眠）

```text
skill-manager --state-dir <state-dir> host install-service
skill-manager --state-dir <state-dir> host uninstall-service
```

macOS launchd / Linux systemd --user / Windows 登录项或等价。`host serve` 就是启动命令。不要加看门狗。

### 4. 对接管理站

用户从管理站拿到一次性码和 URL 后再执行：

```text
skill-manager --state-dir <state-dir> host pair --station <url> --code <code> [--name <显示名>]
skill-manager --state-dir <state-dir> host unpair
```

码过期、已用或填错则失败，且不得改已对接设备。对接失败不得说成「库没 clone」。解除后清本机 `station_url` 与 `device_key`。

## 三、配置已锁定管理 CLI 登录

内置管理 Agent 按 OpenCode → Codex CLI → Gemini CLI 探测，锁定**第一个可用者**。全部失败则对话入口如实不可用。

安装或登录该 CLI **只在 Host 已能 `host status` 之后**做：

1. 若本机没有该 CLI：按其官方方式装到本机用户目录，不要打进 Skill Manager 安装包。
2. 按该 CLI 文档完成模型登录（API key / 订阅 / `login`）。密钥不得写入 Vault、Host SQLite 或本说明。
3. 回到 Skill Manager 打开「管理对话」。未登录模型时只应看到无法执行的说明，库文件不变。
4. 不要把配套维护 Skill 注入该 CLI；不要把内置管理说明拷进用户 `skill list`。

若用户只要无窗口 Host、暂不对话：做到第二节即可，不要强迫登录模型。
