# 管理站与 Host

三件东西、同一套软件：

| 角色 | 干什么 | 跑在哪 |
|---|---|---|
| 本机桌面 | 只管这一台 | 你的 Mac / Windows |
| Host | 这台机器的本机管理服务 | **每台要被管的电脑上**（不要放进 Docker） |
| 管理站 | 一个 HTTPS 入口，登录后切换设备，一次只管当前那一台 | 一台常开的机器或容器，前面加反代 |

管理站**不保存** SkillManagerVault。技能内容始终在当前设备的本地克隆上。

没有管理站时，本机桌面和命令行仍然能用。

## 1. 在每台要管的电脑上开 Host

桌面应用打开时已经带了本机 Host。无窗口机器（云电脑、家里的 NAS 旁的 Linux）用安装包里的 CLI：

```text
skill-manager vault clone <vault-remote-url> --path <空目录>
skill-manager --vault <空目录> --state-dir <本机状态目录> host serve
skill-manager --state-dir <本机状态目录> host install-service
skill-manager --state-dir <本机状态目录> host status
```

`host serve` 是启动命令。同一状态目录不能起两份 Host。关机即停，第一版不会唤醒休眠电脑。

**不要把 Host 放进 Docker。** Host 要读写这台机器上的 Agent 目录、Hermes、Vault 克隆；容器里看不到那些路径。

## 2. 启动管理站（不用 Docker）

在准备当入口的那台机器上：

```text
skill-manager station serve --listen 127.0.0.1:8787
```

首次会在本机终端创建唯一管理员（密码 + TOTP，并给出恢复码）。之后用你已有的 HTTPS 反代指到这个端口。管理站自己不申请证书。

数据默认在本机「SkillManager / station」目录；需要固定位置时加 `--data-dir`。

丢失手机且恢复码也没了：只能在**跑管理站的那台机器**上执行 `skill-manager station reset-admin`，不能在网上自助开新账号。

## 3. 把各机接到管理站

1. 浏览器打开管理站，登录后生成一次性对接码。
2. 在**那台电脑**上（有 Host 的那台）执行：

```text
skill-manager --state-dir <本机状态目录> host pair --station <管理站 URL> --code <对接码> --name <显示名>
```

3. 在管理站选这台设备，填写该机上已有的 Vault 路径（远程网页不能用你浏览器电脑上的文件选择器）。

码用过或过期就作废，不会改掉已经对接好的设备。`host unpair` 解除本机对接。

## 4. 用 Docker 跑管理站

第一版**没有**官方镜像，也不把 Docker 当成唯一发布形态。只有管理站适合进容器；Host 仍装在各机系统里。

仓库里有一份从源码构建的例子：

- 镜像：[examples/Dockerfile.station](examples/Dockerfile.station)
- 编排：[examples/docker-compose.station.yml](examples/docker-compose.station.yml)

在仓库根目录：

```text
docker compose -f docs/user/examples/docker-compose.station.yml up --build
```

容器监听 `8787`。先创建管理员（密码至少 8 位），把返回的 TOTP 密钥和恢复码存好：

```text
curl -sS -X POST http://127.0.0.1:8787/auth/setup \
  -H "content-type: application/json" \
  -d "{\"username\":\"admin\",\"password\":\"你的密码\"}"
```

然后用 Caddy / nginx 等把公网 HTTPS 反代到本机 `8787`。不要把未加密的 8787 直接暴露到公网。

管理员数据在 Docker volume `station-data` 里。重建容器不会丢掉账号，删 volume 会。

也可以不用这份 Dockerfile：在你已有 Linux 上装好 `skill-manager` CLI 和前端静态文件（`SKILL_MANAGER_WEB` 指到 `dist/`），自己写一个只跑 `station serve --listen 0.0.0.0:8787` 的容器即可。
