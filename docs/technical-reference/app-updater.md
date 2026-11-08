# 应用自动更新

桌面应用本体的升级，不是技能上游更新，也不是 SkillManagerVault 同步。规格见 FR-059。

## 更新源

源码仓库 [YYFF925/SkillManager](https://github.com/YYFF925/SkillManager) 公开，Issues / Pull Request / Wiki / Discussions 关闭。安装包与 updater 清单都放该仓库的 GitHub Releases：

- 版本 tag（如 `v0.1.1`）：当前平台 updater 产物（macOS `.app.tar.gz`、Windows NSIS）及其 `.sig`
- 稳定 tag `updater-manifests`：各平台清单 `latest-darwin-aarch64.json`、`latest-windows-x86_64.json`

应用按 `https://github.com/YYFF925/SkillManager/releases/download/updater-manifests/latest-{{target}}-{{arch}}.json` 取清单。macOS 与 Windows 用同一套 updater 代码，但安装包和清单分开：macOS 产出 `latest-darwin-aarch64.json`，Windows 产出 `latest-windows-x86_64.json`。两平台可错开版本。清单不写入 `main`，避免和 Gitea 双端提交互相抢历史。

GitHub Actions 在各自原生 runner 上构建，Mac 与 Windows 安装包不通用，也不能在 Mac 上交叉出可靠的 Windows 更新包。

## 签名

Tauri updater 签名不能关。公钥在 `skill-manager/src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`。私钥在 Infisical `/service/skillmanager/updater` 的 `TAURI_SIGNING_PRIVATE_KEY`，构建时注入，不进 Git。丢失私钥后，已安装用户无法再收后续更新。

这与 macOS ad hoc 代码签名（`signingIdentity = "-"`）不是同一套密钥。

## 界面入口

- 启动检查发现新版本时弹一次确认窗：版本变化、Release 说明、「稍后 / 立即更新」。
- 关闭弹窗不算忽略：更新状态保留，左下角常驻「发现新版本 x.y.z + 立即更新」，点击直接开始下载安装并显示进度，装完自动重启。
- 设置 → 全局配置 → 「软件版本」行：当前版本 + 「检查更新」，手动检查到的结果同样进入上面的弹窗与常驻入口。该行只展示标题与控件的值，不再写说明小字（FR-052 的界面纪律）。

## 发版

标准流程（双平台，GitHub Actions 构建）：

1. `node skill-manager/scripts/bump-version.mjs` 升 patch（`--minor` / `--major` 仅在用户明确要求时）。同步修改 `Cargo.toml`、`package.json`、`tauri.conf.json`。
2. 提交本次改动，推 `github` 与 `origin` 两个远端。
3. `git tag -f v<version>`，把该 tag 推到两个远端（`git push <remote> v<version> --force`）。tag 推送即触发 `.github/workflows/release.yml`；也可在 Actions 页手动 `workflow_dispatch`。
4. 等两个 job 成功，确认版本 tag 的 Release 含 macOS DMG、`.app.tar.gz` 及 `.sig`、Windows `-setup.exe` 及 `.sig`；`updater-manifests` 含对应平台的 `latest-*.json`，且 `version` 是新版本号。
5. 下载 Windows `-setup.exe`，替换 `release/` 里同平台旧包，提交并双端推送。这一步不能省：Actions 产物不在 Git 历史里，`release/` 仍要求每个平台各留一份。
6. 验证：`curl -fsSL .../latest-darwin-aarch64.json` 与 `latest-windows-x86_64.json`，核对 `version`、`url`、`signature`。

注意：

- Release 说明取 tag 指向提交的 message（`RELEASE_NOTES`），会写进清单，客户端更新提示会显示，提交信息要写清楚。
- 重复推同一 tag 会再次触发构建并覆盖 Release 资产。
- 本机单平台备选：凭据注入 `TAURI_SIGNING_PRIVATE_KEY` 后 `npm run tauri build`，替换 `release/` 安装包，再 `node skill-manager/scripts/publish-github-update.mjs` 上传当前平台产物并更新清单。不要把本机产物与 Actions 产物混用于同一版本。

尚未含 updater 的旧包（例如 0.1.0）不能被这条链路升级，需要手动安装一次 0.1.1 及以后的包。
