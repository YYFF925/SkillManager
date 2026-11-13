# 安装与日常使用

## 装哪一份

从 [GitHub Releases](https://github.com/YYFF925/SkillManager/releases) 或仓库根目录 `release/` 取当前平台安装包：

- macOS：Apple Silicon DMG
- Windows：x64 NSIS（`*-setup.exe`）

装好后打开 **Skill Manager**。命令行 `skill-manager` 随安装包一起提供。设置里可以 **检查更新**，确认后再安装。

没有 Linux 桌面安装包。无窗口的 Linux / 云电脑用同一套 CLI 跑 Host，见 [管理站与 Host](station.md)。

## 第一次打开

桌面只管理**这一台电脑**。

1. 准备好自己的 **SkillManagerVault**（一个已有远端的 Git 仓库；编码库 `code/` 和龙虾库 `lobster/` 在同一个库里）。
2. 欢迎页选这个本地目录。它必须已经是 Git 仓库并配置了远端。软件不会在应用里「新建一座中央库」。
3. 还没有本地克隆时，用本机 git 或：

```text
skill-manager vault clone <vault-remote-url> --path <空目录>
```

目标目录必须不存在或为空，已有内容时不会覆盖。

## 之后怎么用

侧栏进入技能库或龙虾库，维护中文注释、分类、标签，再部署到本机 Agent。预设、Hermes、工具中心、设置见 [功能说明](features.md)。

库内容跟 Git 远端走；**部署记录、Agent 开关、工具是否已安装**留在这台电脑，不会因为另一台机器部署过就当成这台也部署了。

右上角打开 **管理对话**，可以用对话做和界面同类的管理（改注释、查待更新等）。第一次会安装并登录默认管理 CLI（OpenCode）；密钥留在那个 CLI 里，不进 Vault。标题栏「关闭」左边可以切换模型。

## 命令行（本机已装好时）

写入类命令在 Host 已运行时走 Host，避免和窗口同时改库。Host 没开时仍可直连本机库。

```text
skill-manager --help
skill-manager host status
```

云电脑或不方便开窗口时，把安装说明交给该机已有的编码 Agent：仓库内 `skill-manager/assets/skills/skill-manager-install/`（与日常维护用的配套维护 Skill 不是同一份）。
