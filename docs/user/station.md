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

只有管理站适合进容器；Host 仍装在各机系统里，**不要**写进这份 Compose。Docker 不是唯一发布形态，不用容器时用上面第二节即可。

当前公开镜像是 **linux/amd64**：

```text
ghcr.io/yyff925/skill-manager-station:latest
ghcr.io/yyff925/skill-manager-station:<版本号>
```

x86_64 的 NAS、云主机、家里的小主机可以直接拉，无需登录 GitHub。ARM 机器（常见 ARM NAS、树莓派）现在跑不了这份镜像。

### 4.1 推荐 Compose

在一台已装 Docker Compose 的 Linux 上建一个空目录，把下面存成 `docker-compose.yml`（仓库里同一份：[examples/docker-compose.station.yml](examples/docker-compose.station.yml)）：

```yaml
services:
  station:
    image: ghcr.io/yyff925/skill-manager-station:latest
    container_name: skill-manager-station
    ports:
      - "127.0.0.1:8787:8787"
    volumes:
      - station-data:/data
    restart: unless-stopped
    init: true
    logging:
      driver: json-file
      options:
        max-size: "10m"
        max-file: "3"

volumes:
  station-data:
```

要点：

- 只映射 **本机回环** `127.0.0.1:8787`，不把未加密端口暴露到局域网或公网。
- 管理员、对接设备写在 named volume `station-data`（容器内 `/data`）。重建容器不会丢；删掉这个 volume 会丢掉账号和已对接设备。
- 开机后 Docker 会按 `unless-stopped` 拉起。你自己 `docker compose stop` 过的，开机不会擅自再开。

也可以不手抄，直接下载：

```text
mkdir -p skill-manager-station && cd skill-manager-station
curl -fsSL -o docker-compose.yml \
  https://raw.githubusercontent.com/YYFF925/SkillManager/main/docs/user/examples/docker-compose.station.yml
```

### 4.2 安装步骤

1. 启动：

```text
docker compose up -d
docker compose logs -f
```

日志里出现「正在监听」即可。第一次会提示还没有管理员，这是正常的。

2. **在这台机器上**创建唯一管理员（密码至少 8 位）。把返回的 `totpSecret`、`otpauth` 和 `recoveryCodes` 存好，只出现这一次：

```text
curl -sS -X POST http://127.0.0.1:8787/auth/setup \
  -H "content-type: application/json" \
  -d "{\"username\":\"admin\",\"password\":\"你的密码\"}"
```

用手机 Authenticator 扫 `otpauth` 或手动录入密钥。不要把这段响应发到网上。

3. 用你已有的 HTTPS 反代指到本机 `127.0.0.1:8787`。管理站自己不申请证书。Caddy 示例：

```text
skill.example.com {
    reverse_proxy 127.0.0.1:8787
}
```

群晖 / 威联通的「反向代理」同样指到这台机器的 `127.0.0.1:8787`。若反代也跑在容器里：把反代和管理站放进同一 Compose 网络，让反代访问服务名 `station:8787`，不要把 `8787` 映射成 `0.0.0.0`。

4. 浏览器打开你的 HTTPS 地址，用刚才的用户名、密码和一次性密码登录。之后按第三节把各机 Host 对接过来；`host pair` 里的管理站 URL 填这个 **HTTPS 公网地址**，不要填容器里的 `127.0.0.1`。

### 4.3 日常维护

```text
# 看日志
docker compose logs -f --tail=100

# 跟着发版拉新镜像（不删 volume，管理员还在）
docker compose pull
docker compose up -d

# 停
docker compose stop
```

备份：备份 Docker volume `station-data`（或你的 Compose 项目名下的 `*_station-data`）。里面是管理站自己的库，**不含** SkillManagerVault。

重置管理员（恢复码也丢了时）：只能动跑管理站的这台机器。删 volume 会清掉管理员**和**已对接设备，然后重新走 4.2 的 setup，各机需要重新 `host pair`：

```text
docker compose down
docker volume rm skill-manager-station_station-data
docker compose up -d
```

volume 名字以 `docker volume ls | grep station-data` 为准。

改登录保持时长（秒，最小 60）：

```text
docker compose exec station skill-manager station session-ttl --data-dir /data --seconds 86400
```

### 4.4 不想用这份镜像时

在仓库根目录从源码重编：

```text
docker build -f docs/user/examples/Dockerfile.station \
  -t ghcr.io/yyff925/skill-manager-station:latest .
```

或在已有 Linux 上安装 `skill-manager` CLI，把 `SKILL_MANAGER_WEB` 指到前端 `dist/`，自己写一个只跑 `station serve --listen 0.0.0.0:8787 --data-dir /data` 的容器。
