# 管理站与多机 Host 接入

面用户问“把某台机器接入管理站”“管理站部署”“设备离线”“对接码怎么用”时使用。

## 角色与不变量

| 角色 | 干什么 | 跑在哪 |
|---|---|---|
| 本机桌面 | 只管这一台机器 | 各机桌面应用 |
| Host | 本机管理服务，读写本地 Vault 克隆与 Agent 目录 | **每台被管机器系统里，绝不放进 Docker** |
| 管理站 station | 一个 HTTPS 入口，登录后在多台设备间切换，一次只管当前那台 | 常开机器或容器，前面必须有反代 |

- 管理站**不保存 SkillManagerVault**：技能内容只存在于各机本地克隆，切换设备不新建 Vault。
- “当前设备”是**按登录会话**记的：Agent 用 API 选的当前设备不等于用户浏览器里选的那台，要让用户自己在页面下拉里选。
- 设备离线只能改名/解除，不能 invoke。

## 首次管理员：前端没有设置页

新装的管理站只能登，**必须服务端创建管理员**：

```text
POST /auth/setup  {"username":"…","password":"…"}
  → {"username","totpSecret","otpauth","recoveryCodes","qr"}   # 只返回这一次
POST /auth/login  {"username","password","otp"}  → Set-Cookie + {"ok","sessionId"}
```

- TOTP 由服务端生成，**无法导入用户已有的那套**；恢复码一次性。
- `GET /auth/setup` 返回 405 是预期的（POST-only），不要当成反代坏了。
- 凭据入 Infisical（走 `shared-credentials`）；写入的值必须由用户在**自己终端**输入（Agent 工具调用无 TTY）。
- 端到端验证以“真实 `POST /auth/login` 拿到 200 + cookie”为准，不是“接口返回了成功字样”。

## 接入一台机器

1. 该机要有**本地 Vault 克隆**，且 Host 在跑（桌面应用自带 Host）。
2. 管理站 UI 生成对接码：**32 位十六进制、 10 分钟过期、一次性**（库里只存 sha256 + `expires_at` + `consumed_at`）。
3. 在该机上执行**出站**对接（被管机不需要公网可达，只要能访问管理站入口）：

```bash
<cli> --vault "<vault>" [--state-dir "<state>"] host pair \
  --station https://<station入口> --code <码> --name <显示名> --json
# → deviceId / deviceKey / displayName / pairedAt
```

  码错/过期/已用只失败当台，**不影响已配对的其他设备**；过期就重新生成一个码再来。
4. 回管理站刷新设备列表并选中该设备。Host 带 `--vault` 启动时库路径自动可见，**不必在网页手填**。
5. 验证必须打到真数据：`PUT /current {"deviceId":…}` → `POST /current/invoke/skill_list` 返回分类与技能数。只看到设备在线不算验收完成。

### 配对后必须重启该机 Host（必做，否则一直 offline）

Host **只在启动时**建立到管理站的长连接；配对前就启动的进程配对完仍显示 `online=false`。

```bash
# macOS
osascript -e 'tell application "Skill Manager" to quit'; sleep 5; open -a "Skill Manager"
# Windows
Get-Process | ? {$_.ProcessName -like "*skill-manager*"} | Stop-Process -Force
Start-Process "<安装目录>\skill-manager-desktop.exe"
```

重启后 `host status --json` 的 `paired` 应为 true，管理站 `/devices` 的 `online` 才会变 true。

## 重置管理员：容器里的 reset-admin 是假的

`station reset-admin` 只认 `--state-dir`，而容器数据挂在 `--data-dir /data`：它会在容器内
`~/.local/share/SkillManager/station/` 另建一个库并报告“已清除管理员”，线上管理员实际还在。

有效做法：`docker stop` → 删 `<data>/station.db*` → `docker start`。
这会**连带丢掉全部已配对设备**，每台都要重新走上面的 pair 流程（含重启 Host）。

## 各机路径不通用，逐机实测

- state-dir 与 vault 都别猜：`host status --json` 会回显实际 `vault` / `stateDir` / `listen` / `pid`。
- 桌面应在跑时，**持锁的 pid 就是桌面应用进程**，再起一个 `host serve` 会撞锁。
- Windows 安装目录常不在 `C:\Program Files`，vault 也可能不在用户目录而在另一块盘；只搜 `%USERPROFILE%` 会误判成“未安装”。
- CLI 非默认装法必须显式传 `--vault` / `--state-dir`，否则会写到另一个库，表现为“配对成功但管理站看不到技能库”。
- 开机自启：`host install-service`；副作用是服务起来后再开桌面应用会撞锁。

## 部署形态要点（NAS + 公网）

- 容器内固定监听 **8787**，映射到宿主空闲端口；数据目录 `--data-dir /data` 映射到宿主 `./data/station`。
- 镜像基于 debian-slim，健康检查用 `bash -c '</dev/tcp/127.0.0.1/8787'`。
- 前端资源是绝对路径 `/assets/…`，**只能挂子域名根路径**，放子路径会 404。
- 反代必须透传 `Host $http_host`（frp 按 Host 路由），并带 `Upgrade`/`Connection` 头，否则 Host 的 WebSocket 长连接建不起来、设备永远 offline。
- NAS compose 与面板注册见 `nas-docker-deployment`，域名/nginx/frpc 接入见 `frp-tunnel-management`。
