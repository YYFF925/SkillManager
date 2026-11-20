# 技能与知识的分流：每日状态机

用于「每天扫一遍各 bot 的技能变更，该转知识的转知识、该入库的入库」这类工作。
规则层（默认偏向、删除顺序、与 curator 的分工）在 SKILL.md §十三；本文件是执行细节。

## 一、六阶段

### 1. 采集变化（只把真正变过的内容送去判断）

```bash
# 各 profile 的使用记录（没有 .usage.json 说明该 profile 还没产生过技能活动）
python3 - <<'PY'
import json, glob, os
for f in glob.glob(os.path.expanduser('~/.hermes/profiles/*/skills/.usage.json')):
    prof = f.split('/profiles/')[1].split('/')[0]
    d = json.load(open(f))
    for name, m in d.items():
        print(prof, name, m.get('created_at'), m.get('last_used_at'),
              m.get('use_count'), m.get('created_by'), m.get('state'))
PY
```

- 候选 = `created_at`／`last_patched_at` 落在当天的条目；再对目录做哈希比对，只把真正变化的目录送进下一步。
- **跨设备是盲区**：本流程只读本机文件系统。其他机器上的 profile 变更需由那台机器的 agent 自查后回报，
  或在那台机器装采集任务；不要假定「本机没变 = 全机没变」。

### 2. 理解单个技能

读 `SKILL.md`、`references/`、`scripts/`，同时回答四问：

1. 有没有真的可执行步骤（命令、参数、顺序）？—— 有 → 倾向技能。
2. 是否已由 Skill Manager 管理（库内同域同名条目 + `deployments[].target_path`）？—— 无 → 走收编/入库路径。
3. 是否正被任务依赖（被 cron 引用的技能不能判为可删，curator 也这样处理）？
4. 是否含真实凭据（密码、API Key、Token、私钥）？—— 有 → 暂停并上报，不自动脱敏。

### 3. 分类（五类，不是两类）

| 判定 | 特征 | 处理 |
|---|---|---|
| 知识型 | 通篇是结论、配置事实、排障经验，没有可照做的步骤 | 整理成 Markdown 进知识库，原件软删除 |
| 技能型 | 有可复用流程、命令、参数、判断分支 | 经 Skill Manager 正式入口入库并部署 |
| 混合型 | 既有环境专属事实，又有通用流程（如某台服务器的配置经验 + 通用的规则维护步骤） | **默认保留技能**，把背景经验抽成知识文档；不整份转换 |
| 探测/脚本型 | 既非流程也非事实，而是回归检测、数据采集脚本 | 按技能型入库，但要求 SKILL.md 写清输入输出与调用前提 |
| 无法确定 | 证据不足，或判断会破坏当前依赖 | 保留原件 + 记账，连续挂满多日或下一步要删时才提一次 |

混合型的默认处置是整条流水线里最容易做错的一步：把可执行能力转成散文，损失不可逆。

### 4. 迁移（顺序不可颠倒）

1. 写知识：正文**全文**进文档，知识页只做索引（用户偏好，便于回查原文）；真实值一律占位符。
2. 等索引：见下节验证。
3. 技能侧：合格技能 `skill import`／收编入库；不合格的走 `skill-trash` 软删除，带 `--reason`。

### 5. 验证（三个都要过）

- 知识可检索：`chunk_count > 0`、`is_processing=false`，且用一句真实问句 `search chunks` 能命中该文档；
- 技能已记录：`skill get <domain>/<name>` 有库内条目与 `deployments[].target_path`；
- 目标 bot 真能用：目标侧 `skill_view('<name>')` 可加载（具名档案启用后应是实体拷贝，不是符号链接；可用 `test -L` 确认不是链接）。

### 6. 汇报

每日一份简报：今天动了哪些、依据是什么、回滚 ID 是什么、哪些留待观察。**不要求用户回复**；
只有高风险动作（清空某个 bot、跨域覆盖、单批删除超过阈值）才单独发消息。

## 二、WeKnora 侧调用

凭据在 Infisical 的 `/service/weknora/<空间>`，空间 Key 只能访问自己空间：

```bash
python3 <shared-credentials>/scripts/credentials.py list --path /service/weknora --recursive
python3 <weknora-search>/scripts/weknora.py --credential-path /service/weknora/<空间> -- kb list
python3 <weknora-search>/scripts/weknora.py --credential-path /service/weknora/<空间> -- search chunks "问句" --kb <知识库>
```

- **每次先 `kb list` 看真实库名**，不要沿用记忆里的空间/库清单——库会增删，空间也会新建。
- `search chunks` 与 `doc list` 都必须显式 `--kb`，**没有跨库检索**。所以「每 bot 一库 + 共享库」这种结构下，
  bot 每次“先查知识库”是多次请求：通用经验放共享库，检索顺序在 bot 的指令里写死「先共享、后自己」。
- 知识库命名要与 bot 名／主题名对齐；出现与 bot 无关的库名（例如以第三方内容命名的库）时，不要把它当 bot 库用。

## 三、与其他机制的边界

- **curator**：管旧的（14d stale / 30d `.archive/` / pinned 与 cron 引用豁免 / 只归档不删）。
  当日增量归本流程；两套各有自己的时间阈值，重叠就会互相打脸。
- **`skill-trash`**：所有删除的唯一入口，删完打印回滚 ID；库的 Git 历史是第二层网。
- **`.archive/`**：curator 的归档区，与本流程的回收区不是一回事，回滚时别走错地方。

## 四、留痕模板

```text
[YYYY-MM-DD] <profile>/<技能>
信号: created_at=… use_count=… patch_count=… 库内有无同名=… cron 引用=…
判定: 知识型/技能型/混合型/探测型/无法确定 —— 一句话理由
动作: 入库 <空间>/<库>/<文档> | skill import <domain>/<name> | skill-trash rm …
验证: chunk_count=… 检索命中=… | skill get 部署路径=…
回滚: skill-trash restore <ID> | 删除文档 <id>
```
