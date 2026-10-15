# Hermes 对接技术参考

以下是参考项目在 2026-10 对接时记录的外部结构与行为，**本次没有对当前 Hermes 版本作运行验证**。
用于减少重复摸索，实施前应核对用户的实际安装和当前 Hermes 加载器；不能直接当成新版协议。

## 根目录与档案结构线索

参考观察：非 Windows 默认根为 `~/.hermes`；Windows 为 `%LOCALAPPDATA%/hermes`。
`HERMES_HOME` 可以改变定位，且启动于档案情境时可能指向 `<root>/profiles/<name>`，
因此“进程环境中的 home”与“包含全部档案的根”需要区分。

```text
<HermesRoot>/
  config.yaml
  skills/                 default 的技能目录
  active_profile          当前活动档案的标记线索
  profiles/
    <name>/
      config.yaml
      profile.yaml
      skills/
```

default 是根目录自身，不是 `profiles/default`。参考实现以 `config.yaml`、`.env`、`state.db`
作为安装标记线索；具名档案的身份标记还包括 `SOUL.md`、`profile.yaml`、`auth.json`。
`profiles/.deleted/<name>` 是旧观察中的删除标记位置。不能仅凭任意子目录就认作档案。

## 与技能使用有关的配置

```yaml
skills:
  disabled:
    - example-skill
  external_dirs:
    - /absolute/path/to/shared-skills
```

- 旧加载器的 `disabled` 是禁用名单，不是启用白名单；不存在于名单中的可扫描技能会进入加载范围。
- `external_dirs` 挂载整个目录，不是只挂载当前点击的一个 Skill。具名档案有自己的配置，不天然继承 default。
- 旧读取路径出现 YAML 列表、JSON 字符串、Python 风格列表字符串、单个字符串等形态。
  应先核对当前实际输入，不能将无法读取的配置误表示为“全未启用”。
- 参考加载器存在 essential 技能集合的线索；是否有不可禁用项以当前 Hermes 实现为准。
- 查看某档案与切换 `active_profile` 是不同动作。本项目当前不管理活动档案切换，也不编辑 SOUL。

## 已发现的技术问题与实际后果

1. 旧静态 adapter 在 Windows 使用 `.hermes/skills`，另一条根解析链却使用 `%LOCALAPPDATA%/hermes`。
   原样复制会使“部署位置”和“Hermes 实际读取位置”不一致；实现时以同一解析结果贯通发现和部署。
2. 整目录挂载配合禁用名单可能让以后新增的技能自动可见。
   这与本项目“新增候选不自动启用其他档案”的要求不同，旧 toggle 编排不能原样复用。
3. 配置文件还包含模型、网关及用户自挂目录。修改技能使用状态时若覆盖整份配置，可能破坏用户其他设置；
   设计应针对实际格式选择适当编辑方式。旧自制文本手术器拒绝部分行内 YAML，不作为必须复刻的方案。
4. “文件不存在”和“文件存在但读不了”不能混为一谈；否则可能用新空配置覆盖原有设置。

## 可参考的源码位置

参考项目：`src-tauri/src/core/hermes_profile.rs`、`hermes_config_surgery.rs`。
其对照的 Hermes 侧文件名：`hermes_constants.py`、`agent/skill_utils.py`、`profiles.py`、`web_routers/skills.py`。
这些是核查入口，不携带旧产品的 GUI-only、候选池限制或自动启用裁决。
