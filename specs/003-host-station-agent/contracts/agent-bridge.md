# 内置管理 Agent：Host 侧桥

**Date**: 2026-11-11 | 研究决策 D6、D9。运行在当前 Host 进程内，浏览器不跑模型。

## 进程

Host 检测 `agent_runtime.binary_path`；没有则按已锁定的默认 CLI 执行软件内安装（不打进 Skill Manager 安装包）。用 ACP stdio 拉起。该 CLI 自己的登录失败时，对话只能报告无法执行，不得生成「已改库」结果。

## 工具

Agent 可调用的管理工具 = 现有 `skill-manager` JSON CLI 的能力，**经 Host 执行**（与 GUI 同一把锁、同一套确认）。不得发工具直打 `skill-manager-core`。

破坏性工具（`skill delete`、覆盖部署/overwrite-local）的执行路径：

1. Host 生成操作计划并推到当前界面；
2. 用户在该 `sessionId` 确认；
3. Host 才跑 core。

Agent 文本里的「用户同意了」无效。

## 上下文

每条用户消息带发出时的快照，由 Host **自动**附上，用户不必粘贴：

- 界面：`domain`、页面、选中 `ref` 列表、分类、预设 id、Agent/Hermes/工具当前对象
- 软件已知事故（若有）：库拉取冲突的路径与 git 错误原文、`paused_local_change` 的技能引用与目标、最近一次管理命令的失败原文

缺省指示语（「这个技能」「这件事」）只解析该快照。进行中任务忽略之后的切页。上下文的目的是免去用户为 Agent 创造现场；不是新建冲突合并 UI。

## 上传

若本机 `git_auto_push` 已由 GUI（含远程网页）打开，内置管理 Agent 写入后可上传且不逐步确认。未打开则上传失败并说明去设置里打开；Agent 不能写该设置。

## 非范围

不主动发任务。不做冲突合并产品。不把对话写入 Vault。
