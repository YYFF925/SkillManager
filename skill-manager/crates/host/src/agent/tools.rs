//! ACP 工具：映射现有 JSON CLI，经 Host 执行。
//! 向子进程声明的方式是 ACP `session/new.mcpServers`（stdio，省略 `type`），不是把清单塞进 initialize。

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::invoke::{self, InvokeError, InvokeSource};
use crate::HostRuntime;

/// Agent 可用工具 = Host invoke 同名命令。原则（FR-015）：界面能做的管理动作，Agent 也能做。
/// 刻意不开放：`plans_confirm`（不能代点图形界面确认）、`vault_open`（切换整个库，路径来自界面选择器）、`agent_post_message` 等
/// Agent 运行时命令（对话基础设施自身，不是管理动作）。
const ALLOWED: &[&str] = &[
    // 技能读取与写入
    "skill_list",
    "skill_get",
    "skill_read",
    "skill_file_read",
    "skill_set",
    "skill_delete",
    "skill_create",
    "skill_copy",
    "skill_task",
    // 标签与分类
    "skill_tag_add",
    "skill_tag_detach",
    "skill_tag_rename",
    "skill_tag_remove",
    "skill_move",
    "skill_category_rename",
    // 归档
    "skill_archive_list",
    "skill_archive",
    "skill_archive_restore",
    "skill_archive_delete",
    // 译文与覆盖层
    "skill_translation_get",
    "skill_translation_set",
    "skill_translation_remove",
    "skill_overlay_list",
    "skill_overlay_get",
    "skill_overlay_set",
    "skill_overlay_remove",
    // 入库与上游
    "skill_preview",
    "skill_fetch_cancel",
    "skill_import",
    "skill_adopt",
    "skill_check",
    "skill_update",
    "skill_update_repo",
    "skill_reject_new",
    "skill_rejected",
    "skill_skip",
    "skill_unskip",
    "skill_skipped",
    // 部署与预设
    "deploy_to",
    "deploy_sync",
    "deploy_undo",
    "deploy_status",
    "preset_list",
    "preset_get",
    "preset_create",
    "preset_delete",
    "preset_rename",
    "preset_set_icon",
    "preset_reorder",
    "preset_members_add",
    "preset_members_remove",
    "preset_reorder_members",
    "deploy_preset",
    "deploy_undo_preset",
    // 库同步
    "sync_pull",
    "sync_push",
    "sync_status",
    "sync_conflicts",
    // Hermes
    "hermes_profiles",
    "hermes_list",
    "hermes_read",
    "hermes_enable",
    "hermes_disable",
    "hermes_enable_preset",
    "hermes_disable_preset",
    "hermes_adopt",
    "hermes_identity_get",
    "hermes_identity_set",
    "hermes_identity_distribute",
    // 工具中心
    "tool_list",
    "tool_get",
    "tool_check",
    "tool_task",
    "tool_skill_candidates",
    "tool_register",
    "tool_set",
    "tool_set_install",
    "tool_skill_link",
    // 本机 Agent 与指令
    "agent_list",
    "agent_detect",
    "agent_enable",
    "agent_disable",
    "agent_reorder",
    "agent_set_dir",
    "agent_reset_dir",
    "agent_local_skills",
    "agent_read_local",
    "agent_delete_local",
    "shared_dir_get",
    "instruction_get",
    "instruction_set",
    // 设置与环境
    "settings_get",
    "settings_set_theme",
    "settings_set_font_scale",
    "settings_set_auto_check",
    "settings_set_git_auto_push",
    "environment_check",
    "environment_schedule_current",
    "environment_os",
    "environment_diagnostics",
    "environment_diagnostics_export",
    "environment_capacity",
    "vault_current",
    "host_status",
];

/// 内置管理说明目录（应用资产，不入库）。
pub fn builtin_skill_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let candidate = exe
            .parent()
            .unwrap_or(Path::new("."))
            .join("assets/skills/skill-manager-builtin-agent");
        if candidate.is_dir() {
            return candidate;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/skills/skill-manager-builtin-agent")
}

pub(crate) fn canonical_name(name: &str) -> String {
    name.replace('.', "_").replace('-', "_")
}

/// 去掉 MCP 前缀后看是否在 allowlist。
pub fn is_allowed(name: &str) -> bool {
    tool_command_name(name).is_some()
}

pub fn tool_command_name(name: &str) -> Option<String> {
    basename(name)
}

fn basename(name: &str) -> Option<String> {
    let c = canonical_name(name.trim());
    if c.is_empty() {
        return None;
    }
    if ALLOWED.contains(&c.as_str()) {
        return Some(c);
    }
    for sep in ["__", "/", ":"] {
        if let Some((_, rest)) = c.rsplit_once(sep) {
            if ALLOWED.contains(&rest) {
                return Some(rest.to_string());
            }
        }
    }
    ALLOWED
        .iter()
        .find(|item| c.ends_with(&format!("_{item}")))
        .map(|item| (*item).to_string())
}

/// OpenCode 1.19.5 默认 `permission.bash * = allow`。
/// Host 写入 `$OPENCODE_CONFIG_DIR/opencode.json`：直接 `deny` bash/edit/write，
/// 避免触发 `session/request_permission` 被拒后导致 OpenCode 立刻 `end_turn` 中止整轮对话。
pub fn opencode_host_config() -> Value {
    json!({
        "$schema": "https://opencode.ai/config.json",
        "tools": {
            "bash": false,
            "edit": false,
            "write": false
        },
        "permission": {
            "bash": { "*": "deny" },
            "edit": { "*": "deny" },
            "write": { "*": "deny" },
            "external_directory": {
                "*/.local/share/opencode/*": "allow",
                "*/opencode/tool-output/*": "allow"
            }
        }
    })
}

/// OpenCode 会搜索 `$OPENCODE_CONFIG_DIR/skills/<id>/SKILL.md`。把内置说明拷进 Host 本机目录，不入库。
pub fn prepare_opencode_config(state_dir: &Path) -> Result<PathBuf, String> {
    let config = state_dir.join("agent-runtime").join("opencode-config");
    let dest = config.join("skills").join("skill-manager-builtin-agent");
    std::fs::create_dir_all(&dest).map_err(|err| err.to_string())?;
    let src = builtin_skill_dir().join("SKILL.md");
    if src.is_file() {
        std::fs::copy(&src, dest.join("SKILL.md")).map_err(|err| err.to_string())?;
    }
    let cfg = serde_json::to_vec_pretty(&opencode_host_config()).map_err(|err| err.to_string())?;
    std::fs::write(config.join("opencode.json"), cfg).map_err(|err| err.to_string())?;
    Ok(config)
}

/// 把工具调用转到 Host invoke / 现有 JSON CLI。
pub fn run_tool(
    runtime: &HostRuntime,
    name: &str,
    args: Value,
    session_id: Option<&str>,
) -> Result<Value, String> {
    let raw = canonical_name(name);
    if raw.contains("plans_confirm") || name.contains("plans.confirm") {
        return Err("内置管理 Agent 不能确认图形界面的操作计划。".to_string());
    }
    let command = basename(name).ok_or_else(|| format!("未知或不允许的管理工具?{name}"))?;
    let args = normalize_args(&command, args);
    match invoke::invoke(runtime, &command, args, InvokeSource::Agent, session_id) {
        Ok(value) => Ok(value),
        Err(InvokeError::NeedsPlan(plan)) => Err(format!(
            "需要在界面确认后才能继续（确认前不会改库）。planId={}",
            plan.get("planId").and_then(Value::as_str).unwrap_or("")
        )),
        Err(err) => {
            let (_, body) = err.status_and_body();
            Err(body
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| body.to_string()))
        }
    }
}

/// 快照里常用 `ref` / 单数 `reference`，删除命令要的是 `references[]`。
pub fn normalize_args(command: &str, args: Value) -> Value {
    let mut obj = match args {
        Value::Object(map) => map,
        other => {
            let mut map = Map::new();
            if other.is_string() && command != "skill_list" {
                map.insert("reference".into(), other);
                return Value::Object(map);
            }
            return other;
        }
    };
    if matches!(
        command,
        "skill_get"
            | "skill_read"
            | "skill_file_read"
            | "skill_set"
            | "skill_create"
            | "skill_copy"
            | "skill_move"
            | "skill_update"
            | "skill_skip"
            | "skill_unskip"
            | "skill_translation_get"
            | "skill_translation_set"
            | "skill_translation_remove"
            | "skill_overlay_list"
            | "skill_overlay_get"
    ) && !obj.contains_key("reference")
    {
        if let Some(r) = obj.get("ref").cloned() {
            obj.insert("reference".into(), r);
        } else if let Some(Value::Array(items)) = obj.get("references").cloned() {
            if let Some(first) = items.first() {
                obj.insert("reference".into(), first.clone());
            }
        }
    }
    if matches!(
        command,
        "skill_delete"
            | "skill_tag_add"
            | "skill_tag_detach"
            | "skill_archive"
            | "skill_archive_restore"
            | "skill_archive_delete"
            | "skill_task"
    ) && !obj.contains_key("references")
    {
        if let Some(r) = obj
            .get("reference")
            .cloned()
            .or_else(|| obj.get("ref").cloned())
        {
            match r {
                Value::Array(_) => {
                    obj.insert("references".into(), r);
                }
                Value::String(_) => {
                    obj.insert("references".into(), json!([r]));
                }
                _ => {}
            }
        }
    }
    // 上游/入库命令用 job_id 关联进度与取消；模型不该编造，缺省由 Host 生成（同 CLI 的 cli-<nanos>）。
    // skill_fetch_cancel 不注入：它要复用发起 preview/import 时的 job_id，编新 id 等于取消空任务。
    if matches!(
        command,
        "skill_check" | "skill_update" | "skill_update_repo" | "skill_preview" | "skill_import"
    ) && !obj.contains_key("job_id")
        && !obj.contains_key("jobId")
    {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        obj.insert("job_id".into(), json!(format!("agent-{nanos}")));
    }
    Value::Object(obj)
}

pub fn mcp_tools() -> Vec<Value> {
    ALLOWED
        .iter()
        .map(|name| {
            json!({
                "name": name,
                "description": tool_description(name),
                "inputSchema": tool_input_schema(name),
            })
        })
        .collect()
}

fn str_prop(desc: &str) -> Value {
    json!({ "type": "string", "description": desc })
}

fn str_array(desc: &str) -> Value {
    json!({ "type": "array", "items": { "type": "string" }, "description": desc })
}

fn bool_prop(desc: &str) -> Value {
    json!({ "type": "boolean", "description": desc })
}

fn tool_input_schema(name: &str) -> Value {
    let (required, properties): (&[&str], Value) = match name {
        "skill_list" => (
            &[],
            json!({
                "domain": str_prop("管理域，可空"),
                "query": str_prop("搜索字"),
                "category": str_prop("分类"),
            }),
        ),
        "skill_get" | "skill_read" | "skill_set" | "skill_translation_get" => (
            &["reference"],
            json!({
                "reference": str_prop("技能引用，如 code/foo"),
                "ref": str_prop("reference 的别名"),
                "comment": str_prop("skill_set：中文注释"),
                "tags": str_array("skill_set：标签"),
                "lang": str_prop("skill_read：zh 或 orig"),
            }),
        ),
        "skill_file_read" => (
            &["reference", "path"],
            json!({
                "reference": str_prop("技能引用"),
                "path": str_prop("技能目录内相对路径"),
            }),
        ),
        "skill_delete" => (
            &["references"],
            json!({
                "references": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 1,
                    "description": "要删除的技能引用列表 references[]"
                },
                "reference": str_prop("单个引用，与 references[] 二选一"),
                "ref": str_prop("单个引用别名，映射为 references[]"),
            }),
        ),
        "deploy_to" => (
            &["target", "references"],
            json!({
                "target": str_prop("部署目标"),
                "references": str_array("技能引用"),
                "also_targets": str_array("同时部署的目标"),
            }),
        ),
        "deploy_sync" => (
            &[],
            json!({
                "reference": str_prop("只同步这个技能"),
                "target": str_prop("只同步这个目标"),
                "overwrite_local": bool_prop("覆盖已改动的副本"),
            }),
        ),
        "deploy_undo" => (
            &["target", "references"],
            json!({
                "target": str_prop("部署目标"),
                "references": str_array("技能引用"),
                "also_targets": str_array("同时撤销的目标"),
                "discard_local_changes": bool_prop("丢弃本机改动"),
            }),
        ),
        "deploy_preset" | "deploy_undo_preset" => (
            &["domain", "preset_id", "target"],
            json!({
                "domain": str_prop("管理域"),
                "preset_id": str_prop("预设 id"),
                "target": str_prop("部署目标"),
                "also_targets": str_array("同时处理的目标"),
            }),
        ),
        "skill_create" => (
            &["reference"],
            json!({
                "reference": str_prop("新技能引用，如 code/new-skill"),
                "category": str_prop("分类"),
                "comment": str_prop("中文注释"),
                "tags": str_array("标签"),
            }),
        ),
        "skill_copy" => (
            &["reference", "to"],
            json!({
                "reference": str_prop("源技能引用"),
                "to": str_prop("目标管理域"),
                "category": str_prop("分类"),
            }),
        ),
        "skill_tag_add" | "skill_tag_detach" => (
            &["references", "tags"],
            json!({
                "references": str_array("技能引用"),
                "reference": str_prop("单个引用别名，映射为 references[]"),
                "tags": str_array("标签"),
            }),
        ),
        "skill_tag_rename" => (
            &["old", "new"],
            json!({ "old": str_prop("旧标签名"), "new": str_prop("新标签名") }),
        ),
        "skill_tag_remove" => (&["tag"], json!({ "tag": str_prop("要删除的标签") })),
        "skill_move" => (
            &["reference"],
            json!({
                "reference": str_prop("技能引用"),
                "category": str_prop("目标分类；空字符串表示移出分类"),
            }),
        ),
        "skill_category_rename" => (
            &["domain", "old", "new"],
            json!({
                "domain": str_prop("管理域"),
                "old": str_prop("旧分类名"),
                "new": str_prop("新分类名"),
            }),
        ),
        "skill_archive" | "skill_archive_restore" | "skill_archive_delete" | "skill_task" => (
            &["references"],
            json!({
                "references": str_array("技能引用"),
                "reference": str_prop("单个引用别名，映射为 references[]"),
                "requirement": str_prop("skill_task：补充要求"),
            }),
        ),
        "skill_translation_set" => (
            &["reference", "content"],
            json!({
                "reference": str_prop("技能引用"),
                "content": str_prop("中文译文全文"),
            }),
        ),
        "skill_translation_remove" => (
            &["reference"],
            json!({ "reference": str_prop("技能引用"), "ref": str_prop("reference 的别名") }),
        ),
        "skill_overlay_set" => (
            &["reference", "path", "content"],
            json!({
                "reference": str_prop("技能引用"),
                "path": str_prop("覆盖层文件相对路径"),
                "content": str_prop("覆盖层内容全文"),
                "agent": str_prop("限定 Agent"),
                "os": str_prop("限定操作系统"),
            }),
        ),
        "skill_overlay_remove" => (
            &["reference", "path"],
            json!({
                "reference": str_prop("技能引用"),
                "path": str_prop("覆盖层文件相对路径"),
                "agent": str_prop("限定 Agent"),
                "os": str_prop("限定操作系统"),
            }),
        ),
        "skill_preview" => (
            &["repo"],
            json!({
                "repo": str_prop("上游仓库地址"),
                "reference": str_prop("只预览这个技能"),
            }),
        ),
        "skill_fetch_cancel" => (
            &["job_id"],
            json!({ "job_id": str_prop("要取消的获取任务 id，来自 preview / import 的调用") }),
        ),
        "skill_import" => (
            &["repo", "domain", "skills"],
            json!({
                "repo": str_prop("上游仓库地址"),
                "domain": str_prop("入库管理域"),
                "skills": str_array("要入库的技能"),
                "category": str_prop("分类"),
                "uncategorized": bool_prop("入到未分类"),
            }),
        ),
        "skill_adopt" => (
            &["agent", "names"],
            json!({
                "agent": str_prop("本机 Agent 目标"),
                "names": str_array("要收编的本机技能名"),
                "category": str_prop("分类"),
                "comment": str_prop("中文注释"),
                "tags": str_array("标签"),
            }),
        ),
        "skill_check" => (
            &[],
            json!({
                "reference": str_prop("只检查这个技能"),
                "repo": str_prop("只检查这个上游仓库"),
                "all": bool_prop("检查全部上游仓库"),
            }),
        ),
        "skill_update" => (
            &["reference"],
            json!({
                "reference": str_prop("要更新的技能引用"),
                "ref": str_prop("reference 的别名"),
                "to": str_prop("目标 commit OID，缺省为上游最新"),
            }),
        ),
        "skill_update_repo" => (
            &["repo"],
            json!({
                "repo": str_prop("上游仓库地址"),
                "include_new": str_array("要一并入库的新增技能"),
                "domain": str_prop("新增入库的管理域"),
                "category": str_prop("新增入库的分类"),
            }),
        ),
        "skill_reject_new" => (
            &["repo", "domain", "skills"],
            json!({
                "repo": str_prop("上游仓库地址"),
                "domain": str_prop("管理域"),
                "skills": str_array("拒绝入库的新增技能"),
            }),
        ),
        "skill_skip" | "skill_unskip" => (
            &["reference"],
            json!({
                "reference": str_prop("技能引用"),
                "ref": str_prop("reference 的别名"),
                "version": str_prop("skill_skip：要跳过的版本，缺省为当前上游版本"),
            }),
        ),
        "preset_get" => (
            &["domain", "id"],
            json!({ "domain": str_prop("管理域"), "id": str_prop("预设 id") }),
        ),
        "preset_create" => (
            &["domain", "name"],
            json!({
                "domain": str_prop("管理域"),
                "name": str_prop("预设名"),
                "icon": str_prop("图标"),
            }),
        ),
        "preset_delete" => (
            &["domain", "id"],
            json!({ "domain": str_prop("管理域"), "id": str_prop("预设 id") }),
        ),
        "preset_rename" => (
            &["domain", "id", "name"],
            json!({
                "domain": str_prop("管理域"),
                "id": str_prop("预设 id"),
                "name": str_prop("新名称"),
            }),
        ),
        "preset_set_icon" => (
            &["domain", "id"],
            json!({
                "domain": str_prop("管理域"),
                "id": str_prop("预设 id"),
                "icon": str_prop("图标，空为清除"),
            }),
        ),
        "preset_reorder" => (
            &["domain", "order"],
            json!({ "domain": str_prop("管理域"), "order": str_array("预设 id 的新顺序") }),
        ),
        "preset_members_add" | "preset_members_remove" => (
            &["domain", "id", "names"],
            json!({
                "domain": str_prop("管理域"),
                "id": str_prop("预设 id"),
                "names": str_array("成员技能名"),
            }),
        ),
        "preset_reorder_members" => (
            &["domain", "id", "order"],
            json!({
                "domain": str_prop("管理域"),
                "id": str_prop("预设 id"),
                "order": str_array("成员新顺序"),
            }),
        ),
        "hermes_enable" | "hermes_disable" | "hermes_adopt" => (
            &["profile", "names"],
            json!({
                "profile": str_prop("Hermes 档案"),
                "names": str_array("技能名"),
            }),
        ),
        "hermes_enable_preset" | "hermes_disable_preset" => (
            &["profile", "preset_id"],
            json!({
                "profile": str_prop("Hermes 具名档案"),
                "preset_id": str_prop("龙虾预设 id"),
            }),
        ),
        "hermes_identity_get" => (&["scope"], json!({ "scope": str_prop("global 或档案 id") })),
        "hermes_identity_set" => (
            &["scope", "content"],
            json!({
                "scope": str_prop("global 或档案 id"),
                "content": str_prop("人格源 Markdown"),
            }),
        ),
        "hermes_identity_distribute" => (&[], json!({})),
        "tool_check" => (
            &[],
            json!({
                "slug": str_prop("只检查这个工具"),
                "all": bool_prop("检查全部"),
            }),
        ),
        "tool_task" => (
            &["action"],
            json!({
                "action": str_prop("任务类型，见 tool_get 返回的维护任务"),
                "slug": str_prop("工具 slug"),
                "requirement": str_prop("补充要求"),
            }),
        ),
        "tool_register" => (&["dir"], json!({ "dir": str_prop("工具目录绝对路径") })),
        "tool_set" => (
            &["slug", "dir"],
            json!({
                "slug": str_prop("工具 slug"),
                "dir": str_prop("工具目录绝对路径"),
            }),
        ),
        "tool_set_install" => (
            &["slug", "version"],
            json!({
                "slug": str_prop("工具 slug"),
                "version": str_prop("版本"),
                "method": str_prop("安装方式"),
                "path": str_prop("本机可执行路径"),
                "config_path": str_prop("配置文件路径"),
            }),
        ),
        "tool_skill_link" => (
            &["slug", "reference"],
            json!({
                "slug": str_prop("工具 slug"),
                "reference": str_prop("库内技能引用"),
            }),
        ),
        "agent_enable" | "agent_disable" => (
            &["key"],
            json!({ "key": str_prop("Agent key，见 agent_list") }),
        ),
        "agent_reorder" => (
            &["order"],
            json!({ "order": str_array("Agent key 的新顺序") }),
        ),
        "agent_set_dir" => (
            &["key", "kind", "path"],
            json!({
                "key": str_prop("Agent key"),
                "kind": str_prop("目录类别：skills / project-rel / instructions"),
                "path": str_prop("新目录绝对路径"),
            }),
        ),
        "agent_reset_dir" => (
            &["key", "kind"],
            json!({
                "key": str_prop("Agent key"),
                "kind": str_prop("目录类别：skills / project-rel / instructions"),
            }),
        ),
        "agent_delete_local" => (
            &["target", "names"],
            json!({
                "target": str_prop("本机 Agent 目标"),
                "names": str_array("要删除的本机技能名"),
            }),
        ),
        "instruction_set" => (
            &["scope", "content"],
            json!({
                "scope": str_prop("指令范围，见 instruction_get"),
                "content": str_prop("指令源全文"),
            }),
        ),
        "settings_set_theme" => (
            &["theme"],
            json!({ "theme": str_prop("light / dark / system") }),
        ),
        "settings_set_font_scale" => (
            &["scale"],
            json!({ "scale": str_prop("small / default / large / xlarge") }),
        ),
        "settings_set_auto_check" => (
            &["minutes"],
            json!({
                "minutes": { "type": "integer", "description": "分钟；0 关闭，可选 0 / 30 / 60 / 360 / 1440" }
            }),
        ),
        "settings_set_git_auto_push" => (
            &["enabled"],
            json!({ "enabled": { "type": "boolean", "description": "true/false" } }),
        ),
        "environment_diagnostics_export" => (
            &[],
            json!({ "recent": { "type": "integer", "description": "最近失败条数" } }),
        ),
        "hermes_read" | "hermes_list" => (
            &["profile"],
            json!({
                "profile": str_prop("Hermes 档案"),
                "name": str_prop("hermes_read：技能名"),
            }),
        ),
        "tool_get" | "tool_skill_candidates" => {
            (&["slug"], json!({ "slug": str_prop("工具 slug") }))
        }
        "agent_local_skills" => (
            &["target"],
            json!({ "target": str_prop("本机 Agent 目标") }),
        ),
        "agent_read_local" => (
            &["target", "name"],
            json!({
                "target": str_prop("本机 Agent 目标"),
                "name": str_prop("技能名"),
            }),
        ),
        "instruction_get" => (&["scope"], json!({ "scope": str_prop("指令范围") })),
        _ => (&[], json!({})),
    };
    let mut schema = json!({
        "type": "object",
        "additionalProperties": true,
        "properties": properties,
    });
    if !required.is_empty() {
        schema["required"] = json!(required);
    }
    schema
}

fn tool_description(name: &str) -> &'static str {
    match name {
        "skill_list" => "列出当前 Host 的技能摘要（含 translation_stale 等已有字段）",
        "skill_get" => "读取一个技能的详情（含文件树与部署位置；正文请用 skill_read）",
        "skill_read" => "读取技能正文片段",
        "skill_file_read" => "读取技能目录内指定相对路径文件",
        "skill_set" => {
            "修改技能中文注释、分类、标签或支持声明。完善中文注释必须用本工具的 comment 字段，禁止 bash/edit"
        }
        "skill_delete" => "删除技能（直接执行，不经图形界面确认）",
        "skill_create" => "创建自建技能骨架",
        "skill_copy" => "把技能复制到另一管理域",
        "skill_task" => "生成技能维护任务说明（不改库）",
        "skill_tag_add" => "给技能加标签（可批量）",
        "skill_tag_detach" => "移除技能标签（可批量）",
        "skill_tag_rename" => "全库重命名标签",
        "skill_tag_remove" => "全库删除标签",
        "skill_move" => "移动技能到分类",
        "skill_category_rename" => "重命名一级分类",
        "skill_archive_list" => "列出已归档技能",
        "skill_archive" => "归档技能（可批量）",
        "skill_archive_restore" => "恢复归档技能",
        "skill_archive_delete" => "删除归档技能（不可恢复）",
        "skill_translation_get" => "读取中文译文",
        "skill_translation_set" => "写技能中文译文全文",
        "skill_translation_remove" => "删除技能中文译文",
        "skill_overlay_list" => "列出覆盖层文件",
        "skill_overlay_get" => "读取一项覆盖层",
        "skill_overlay_set" => "写技能覆盖层文件",
        "skill_overlay_remove" => "删除技能覆盖层文件",
        "skill_preview" => "真实获取上游仓库，列出可入库技能（不改库）",
        "skill_fetch_cancel" => "取消进行中的上游获取任务",
        "skill_import" => "把上游仓库技能入库",
        "skill_adopt" => "收编本机 Agent 已有技能入库",
        "skill_check" => "检查上游更新：reference 单技能 / repo 单仓库 / all 全部",
        "skill_update" => "把一个技能采用上游更新；用户明确要求时执行",
        "skill_update_repo" => "按仓库批量更新已入库技能，include_new 可一并入库新增",
        "skill_reject_new" => "把上游新增技能记为拒绝入库",
        "skill_rejected" => "已拒绝入库的新增候选",
        "skill_skip" => "跳过技能当前上游版本，不再提醒",
        "skill_unskip" => "恢复技能的上游更新提醒",
        "skill_skipped" => "已跳过的上游版本",
        "deploy_to" => "普通部署，不逐步确认",
        "deploy_sync" => "同步已有部署；overwrite_local 直接按库覆盖",
        "deploy_undo" => "撤销部署；discard_local_changes 直接丢弃副本改动",
        "deploy_status" => "本机部署记录；paused 只列出 paused_local_change",
        "deploy_preset" => "按预设整体部署到目标",
        "deploy_undo_preset" => "撤销预设在目标的部署",
        "preset_list" => "列出预设摘要",
        "preset_get" => "读取一个预设的成员",
        "preset_create" => "新建预设",
        "preset_delete" => "删除预设",
        "preset_rename" => "重命名预设",
        "preset_set_icon" => "设置或清除预设图标",
        "preset_reorder" => "调整预设顺序",
        "preset_members_add" => "预设添加成员",
        "preset_members_remove" => "预设移除成员",
        "preset_reorder_members" => "调整预设成员顺序",
        "sync_pull" => "拉取库远端更新；冲突会暂停并列出",
        "sync_status" => "库同步状态",
        "sync_push" => "上传库；用户要求时直接执行",
        "sync_conflicts" => "拉取冲突路径与 git status 原文",
        "hermes_profiles" => "本机 Hermes 档案列表",
        "hermes_list" => "列出某档案技能",
        "hermes_read" => "读取档案技能文件",
        "hermes_enable" => "启用 Hermes 档案技能",
        "hermes_disable" => "停用 Hermes 档案技能",
        "hermes_enable_preset" => "按龙虾预设为 Hermes 具名档案一键启用（未装进 default 则自动安装）",
        "hermes_disable_preset" => "仅在当前 Hermes 具名档案禁用龙虾预设成员",
        "hermes_adopt" => "收编 Hermes 档案技能入库",
        "hermes_identity_get" => "读取 Hermes 人格源与合成预览",
        "hermes_identity_set" => "写人格源并覆盖各档案 SOUL.md",
        "hermes_identity_distribute" => "按当前人格源覆盖各档案 SOUL.md",
        "tool_list" => "工具中心清单（不含指南正文）",
        "tool_get" => "工具详情",
        "tool_check" => "检查工具版本与安装状态",
        "tool_task" => "生成工具维护任务说明（不改库）",
        "tool_skill_candidates" => "工具配套 Skill 候选",
        "tool_register" => "登记本机工具目录到工具中心",
        "tool_set" => "更新工具登记的目录",
        "tool_set_install" => "登记工具安装版本与方式",
        "tool_skill_link" => "把库内技能登记为工具配套 Skill",
        "agent_list" => "本机 Agent 检测与启用状态",
        "agent_detect" => "重新检测本机 Agent",
        "agent_enable" => "启用本机 Agent",
        "agent_disable" => "停用本机 Agent",
        "agent_reorder" => "调整本机 Agent 顺序",
        "agent_set_dir" => "设置本机 Agent 目录（skills / project-rel / instructions）",
        "agent_reset_dir" => "恢复本机 Agent 默认目录",
        "agent_local_skills" => "某目标目录下的本机技能（含 paused_local_change）",
        "agent_read_local" => "读取本机技能副本文件",
        "agent_delete_local" => "删除目标下的本机技能副本（不可恢复）",
        "shared_dir_get" => "共享 Agent 目录",
        "instruction_get" => "读取指令源与合成预览",
        "instruction_set" => "写指令源（AGENTS.md）",
        "settings_get" => "主题、检查频率、自动上传等本机设置",
        "settings_set_theme" => "设置主题 light / dark / system",
        "settings_set_font_scale" => "设置字号 small / default / large / xlarge",
        "settings_set_auto_check" => "设置自动检查频率分钟数；0 关闭",
        "settings_set_git_auto_push" => "打开或关闭自动上传",
        "environment_check" => "立即执行一次上游与工具检查",
        "environment_schedule_current" => "最近一次调度报告（含可入库新增）",
        "environment_os" => "本机系统信息",
        "environment_diagnostics" => "最近命令失败原文与诊断路径",
        "environment_diagnostics_export" => "导出诊断文件并返回路径",
        "environment_capacity" => "库容量排行",
        "vault_current" => "当前接入的 Vault 路径",
        "host_status" => "Host 状态",
        _ => "现有 JSON CLI 经 Host 转发",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invoke::InvokeSource;
    use crate::HostRuntime;
    use serde_json::json;
    use skill_manager_core::AppContext;

    #[test]
    fn agent_cannot_confirm_or_call_unknown_tools() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        let denied = run_tool(
            &runtime,
            "plans_confirm",
            json!({ "planId": "x" }),
            Some("agent"),
        );
        assert!(denied.unwrap_err().contains("不能确认图形界面"));
        let unknown = run_tool(
            &runtime,
            "vault_open",
            json!({ "path": "/tmp" }),
            Some("agent"),
        );
        assert!(unknown.unwrap_err().contains("未知或不允许"));
        let invoke_denied = invoke::invoke(
            &runtime,
            "plans_confirm",
            json!({ "planId": "x" }),
            InvokeSource::Agent,
            Some("agent"),
        );
        assert!(invoke_denied.is_err());
    }

    #[test]
    fn descriptors_include_input_schema_for_core_tools() {
        let tools = mcp_tools();
        for name in [
            "skill_list",
            "skill_get",
            "skill_read",
            "skill_set",
            "skill_delete",
        ] {
            let tool = tools
                .iter()
                .find(|item| item["name"] == name)
                .unwrap_or_else(|| panic!("missing {name}"));
            assert!(
                tool["inputSchema"]["type"] == "object",
                "{name} must declare JSON Schema"
            );
        }
        let delete = tools
            .iter()
            .find(|item| item["name"] == "skill_delete")
            .unwrap();
        assert!(delete["inputSchema"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "references"));
        assert_eq!(
            delete["inputSchema"]["properties"]["references"]["type"],
            "array"
        );
        assert_eq!(
            delete["inputSchema"]["properties"]["references"]["items"]["type"],
            "string"
        );
    }

    #[test]
    fn skill_delete_accepts_ref_alias() {
        let out = normalize_args("skill_delete", json!({ "ref": "code/demo" }));
        assert_eq!(out["references"][0], "code/demo");
        let out = normalize_args("skill_get", json!({ "ref": "code/demo" }));
        assert_eq!(out["reference"], "code/demo");
    }

    #[test]
    fn agent_tools_cover_gui_parity() {
        // FR-015：界面能做的管理动作，Agent 也能做（上游更新、入库、标签、归档、
        // 译文/覆盖层、预设、Hermes、工具、同步、本机 Agent、设置、环境）。
        for name in [
            "skill_check",
            "skill_update",
            "skill_update_repo",
            "skill_skip",
            "skill_unskip",
            "skill_reject_new",
            "skill_preview",
            "skill_import",
            "skill_adopt",
            "skill_create",
            "skill_copy",
            "skill_tag_add",
            "skill_move",
            "skill_archive",
            "skill_translation_set",
            "skill_overlay_set",
            "deploy_undo",
            "deploy_preset",
            "preset_create",
            "hermes_enable",
            "hermes_identity_set",
            "tool_check",
            "tool_register",
            "sync_pull",
            "agent_enable",
            "instruction_set",
            "settings_set_theme",
            "settings_set_git_auto_push",
            "environment_check",
        ] {
            assert!(is_allowed(name), "{name} 应对 Agent 开放");
        }
        // 刻意不开放：图形界面确认计划、库切换、Agent 运行时自身。
        for name in [
            "plans_confirm",
            "vault_open",
            "agent_post_message",
        ] {
            assert!(!is_allowed(name), "{name} 不应对 Agent 开放");
        }
        let listed = mcp_tools();
        let update = listed
            .iter()
            .find(|item| item["name"] == "skill_update")
            .unwrap();
        assert!(update["inputSchema"]["required"]
            .as_array()
            .unwrap()
            .contains(&json!("reference")));
    }

    #[test]
    fn upstream_jobs_get_host_generated_job_id() {
        let out = normalize_args("skill_check", json!({ "reference": "code/demo" }));
        assert!(
            out["job_id"].as_str().unwrap_or("").starts_with("agent-"),
            "缺省 job_id 应由 Host 生成：{out}"
        );
        let kept = normalize_args(
            "skill_update",
            json!({ "reference": "code/demo", "job_id": "j1" }),
        );
        assert_eq!(kept["job_id"], "j1");
        // fetch_cancel 必须复用原任务 id，不注入新 id。
        let cancel = normalize_args("skill_fetch_cancel", json!({}));
        assert!(cancel.get("job_id").is_none());
    }

    #[test]
    fn upstream_reference_aliases_are_normalized() {
        let out = normalize_args("skill_update", json!({ "ref": "code/demo" }));
        assert_eq!(out["reference"], "code/demo");
        let tags = normalize_args(
            "skill_tag_add",
            json!({ "reference": "code/demo", "tags": ["a"] }),
        );
        assert_eq!(tags["references"][0], "code/demo");
        let archive = normalize_args("skill_archive", json!({ "ref": "code/demo" }));
        assert_eq!(archive["references"][0], "code/demo");
    }

    #[test]
    fn mcp_prefixed_names_are_allowed() {
        assert!(is_allowed("skill_list"));
        assert!(is_allowed("skill-manager__skill_set"));
        assert!(is_allowed("mcp_skill-manager_skill_read"));
        assert!(!is_allowed("bash"));
        assert!(!is_allowed("plans_confirm"));
    }

    #[test]
    fn opencode_config_denies_native_bash_and_edit() {
        let tmp = tempfile::tempdir().unwrap();
        let config = prepare_opencode_config(tmp.path()).unwrap();
        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(config.join("opencode.json")).unwrap())
                .unwrap();
        assert_eq!(written["permission"]["bash"]["*"], "deny");
        assert_eq!(written["permission"]["edit"]["*"], "deny");
        assert_eq!(written["permission"]["write"]["*"], "deny");
        assert_eq!(
            written["permission"]["external_directory"]["*/.local/share/opencode/*"],
            "allow"
        );
        assert_eq!(written["tools"]["bash"], false);
        assert_eq!(written["tools"]["edit"], false);
        assert!(config
            .join("skills/skill-manager-builtin-agent/SKILL.md")
            .is_file());
    }
}
