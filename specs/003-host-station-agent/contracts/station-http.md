# 管理站 HTTP：登录、设备、转到当前 Host

**Date**: 2026-11-11 | 管理站不提供技能业务 API。技能读写只发生在当前设备的 Host 上。

第一版监听在反代后面的明文或内网 TLS 由部署决定；对用户表现为一个 HTTPS 源站。

## 管理员

```text
POST /auth/setup          # 仅当尚无管理员：用户名、密码、开始绑定 TOTP
GET  /auth/totp/setup     # 二维码与密钥、一次性恢复码（只展示一次）
POST /auth/totp/verify
POST /auth/login          # 用户名、密码、TOTP 或恢复码；恢复码成功时须带 mustRebindTotp 与新密钥/恢复码。此时不得改写 admin.totp_secret，新密钥只挂在该会话
POST /auth/logout
GET  /auth/session        # 待重绑会话返回 mustRebindTotp 与密钥/二维码（明文恢复码仍只在登录响应展示一次）
GET  /auth/session-ttl    # 当前保持登录秒数
PUT  /auth/session-ttl    # 保持登录秒数（最小 60）
POST /auth/totp/rebind    # 用本会话待定密钥校验 OTP 后才写入 admin.totp_secret 与新恢复码哈希；此前该会话不是完整管理员会话
```

无管理员时除 `setup` 外一律拒绝。不得公网注册第二人。

## 设备

```text
POST /devices/pairing-codes    → 一次性码明文 + 过期时间
GET  /devices
PATCH /devices/:id             { displayName }
POST /devices/:id/revoke
```

列表含 `online`。离线设备可改名、可解除，不可对其 `/current` 发 invoke。

Host 出站对接不走浏览器：Host 使用配对码调用管理站的设备登记（实现可与长连接握手合并），浏览器不持有设备密钥。

## 当前设备与转发

```text
PUT  /current { deviceId }     # 本管理员会话切换
POST /current/invoke/<command> # 转到当前 Host 的 /invoke/<command>；Host 409 / needsConfirmation 原样返回 409
POST /current/plans/:id/confirm # 转到当前 Host 确认计划（须带签发该计划的 sessionId）
GET  /current/events           # 转到当前 Host SSE
```

未选设备或设备离线：转发失败，说明离线，不得用缓存技能内容顶上。切换设备不新建 Vault。

静态前端（同一套 React）由管理站在登录后提供。登录页不加载技能数据。
