# Skill Manager

本地优先的技能库管理器。以用户自己的 SkillManagerVault（统一技能库，Git 仓库）为内容真值源，把 Skill（技能）部署到 Code Agent（编码代理）和 Lobster Agent（龙虾代理）。GUI（Tauri 2）与 CLI（命令行）共用同一套 Rust 核心。

本仓库公开以便桌面应用自动更新；不接受 Issue、Pull Request 或其他贡献。

## 安装

安装包见 [Releases](https://github.com/YYFF925/SkillManager/releases) 与仓库根目录 `release/`。

- macOS：Apple Silicon DMG
- Windows：x64 NSIS 安装程序

启动后检查更新，确认后再安装。CLI 随安装包交付。

## 开发

产品代码在 [skill-manager/](skill-manager/)，构建、测试与命令说明见 [skill-manager/README.md](skill-manager/README.md)。

- 行为与验收：[specs/002-skill-management/spec.md](specs/002-skill-management/spec.md)
- 术语：[GLOSSARY.md](GLOSSARY.md)
- 前端设计层：[design/](design/README.md)
- 交互原型：[specs/001-skill-management-prototype/ui-prototype/index.html](specs/001-skill-management-prototype/ui-prototype/index.html)
