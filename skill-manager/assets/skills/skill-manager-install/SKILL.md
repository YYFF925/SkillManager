---
name: skill-manager-install
description: 在一台新电脑上接入 Skill Manager：clone SkillManagerVault、启动或安装本机 Host、对接统一管理站；不含日常库维护。登录任意 ACP 兼容 CLI（模型由该 CLI 决定）。
---

# Skill Manager 安装（接到一台电脑）

这是给**已有编码 Agent** 用的安装 Skill，与库内配套维护 Skill（`skill-manager-maintenance`）分开：
本文件只把 Skill Manager **接到这台电脑**，不维护库内技能、上游、部署或工具中心。

不要改写配套维护 Skill。不要把本说明入库成用户技能。

## 一、要达成的结果

1. 本机有一份已有远端的 SkillManagerVault 克隆（不 init 新中央库）。
2. 本机 Host 能前台跑（`host serve`）或已写入当前用户开机项（`host install-service`）。
3. Host 已用一次性码出站对接管理站（`host pair`）。第一版没有 `vault clone --pair`。
4. 管理 CLI 登录与登记在第三节。模型由该 CLI 决定，不限 OpenCode / Codex / Gemini。

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

## 三、登录所用 ACP CLI（任意兼容命令行）

内置管理 Agent 经 ACP 拉起**用户已登录的任意兼容 CLI**，模型由该 CLI 决定。不要把 OpenCode / Codex / Gemini 当成唯一选项。D9 探测只说明：本机完全没有兼容 CLI 时，Host 可以安装引导宿主 OpenCode（`opencode acp`）。用户随时可改成别的已登录 CLI。

**只在 Host 已能 `host status` 之后**做登录或登记：

1. 若本机已有要用的 CLI：在它自己的终端完成模型登录（例如 `opencode auth login`、`gemini` 的 Google / API key、其它厂商的 `login`）。密钥留在该 CLI，不得写入 Vault、Host SQLite 或本说明。
2. 若 PATH 上已有该 CLI 且它讲 ACP，Host 对话时会检测到。若检测不到、或要用清单外的 CLI，登记到本机：

```text
skill-manager --state-dir <state-dir> host agent-cli --bin <绝对路径或 PATH 名> [--args acp] [--id <标识>]
skill-manager --state-dir <state-dir> host agent-cli
skill-manager --state-dir <state-dir> host agent-cli --clear
```

也可设环境变量 `SKILL_MANAGER_AGENT_CLI`（可选 `SKILL_MANAGER_AGENT_ACP_ARGS`），或 `PUT /agent/runtime` `{ "binaryPath", "acpArgs", "cliId" }`。

该 CLI 必须能以 ACP stdio JSON-RPC 子进程运行（常见参数：`acp`、`--acp`，或二进制本身就是 ACP 宿主）。原生 `codex` 没有 ACP：要用 Codex 模型时登记 `codex-acp` 一类适配器，不要把 `codex app-server` 当成 ACP。

3. 若本机没有任何兼容 CLI：不要打进 Skill Manager 安装包。Host 首次对话时可把引导宿主 OpenCode 装到 `<state-dir>/agent-runtime/opencode`（`npm install --omit=dev --prefix … opencode-ai`）。然后 `opencode auth login`。也可按 OpenCode 官方非交互方式自行安装。
4. 回到 Skill Manager 打开「管理对话」。未登录且该 CLI 不能调用任何模型时，只应看到无法执行的说明，库文件不变。
5. 不要把配套维护 Skill 注入该 CLI；不要把内置管理说明拷进用户 `skill list`。OpenCode 的注入目录是 Host 管理的 `$OPENCODE_CONFIG_DIR/skills/skill-manager-builtin-agent/`。其它 CLI 若无 skills 目录，Host 用 ACP 初始化文本和每条消息前缀。

若用户只要无窗口 Host、暂不对话：做到第二节即可，不要强迫登录模型。
