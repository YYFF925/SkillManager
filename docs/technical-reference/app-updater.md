# 应用自动更新

桌面应用本体的升级，不是技能上游更新，也不是 SkillManagerVault 同步。规格见 FR-059。

## 更新源

源码仓库 [YYFF925/SkillManager](https://github.com/YYFF925/SkillManager) 公开，Issues / Pull Request / Wiki / Discussions 关闭。安装包与 updater 清单都放该仓库的 GitHub Releases：

- 版本 tag（如 `v0.1.1`）：当前平台 updater 产物（macOS `.app.tar.gz`、Windows NSIS）及其 `.sig`
- 稳定 tag `updater-manifests`：各平台清单 `latest-darwin-aarch64.json`、`latest-windows-x86_64.json`

应用按 `https://github.com/YYFF925/SkillManager/releases/download/updater-manifests/latest-{{target}}-{{arch}}.json` 取清单。macOS 与 Windows 用同一套 updater 代码，但安装包和清单分开：Mac 在本机构建并发布 `latest-darwin-aarch64.json`，Windows 必须在 Windows 机器构建 NSIS 并发布 `latest-windows-x86_64.json`。两平台可错开版本。清单不写入 `main`，避免和 Gitea 双端提交互相抢历史。

## 签名

Tauri updater 签名不能关。公钥在 `skill-manager/src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`。私钥在 Infisical `/service/skillmanager/updater` 的 `TAURI_SIGNING_PRIVATE_KEY`，构建时注入，不进 Git。丢失私钥后，已安装用户无法再收后续更新。

这与 macOS ad hoc 代码签名（`signingIdentity = "-"`）不是同一套密钥。

## 发版

常规路径是 GitHub Actions：源码提交后打 `v<version>` tag 并推到 `github`，workflow 在 macOS 与 Windows runner 上注入 `TAURI_SIGNING_PRIVATE_KEY`、构建、发布。也可 `workflow_dispatch`。

本机单平台仍可用：

1. `node skill-manager/scripts/bump-version.mjs` 升 patch（或用户明确要求时 `--minor` / `--major`）。
2. 凭据注入私钥后 `npm run tauri build`（`createUpdaterArtifacts` 已开启）。
3. 替换本仓库 `release/` 中当前平台安装包。
4. `node skill-manager/scripts/publish-github-update.mjs` 上传当前平台产物（含 macOS DMG）并更新对应清单。

尚未含 updater 的旧包（例如 0.1.0）不能被这条链路升级，需要手动安装一次 0.1.1 及以后的包。
