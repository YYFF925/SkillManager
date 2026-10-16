//! 工具维护任务文本生成（T078，FR-047 / FR-051）。
//!
//! 只生成**可复制的任务说明**，不启动 Agent、不建任务队列、不安装、不改库、不上传；
//! 生成任务不代表已开始或已完成。任务保留 FR-047 要求的用户要求、简介、本机资料、
//! 检查结果与维护指南，并引用配套维护 Skill 的**真实库内路径**与“按其规范执行”——
//! 引用规范不取消这些事实上下文，也不内联规范全文。配套 Skill 未入库时如实提示“未入库”。

use std::path::Path;

use crate::error::{CoreError, Result};
use crate::state::StateStore;
use crate::toolcenter::catalog::{self, ToolEntry};
use crate::vault::index::{MAINTENANCE_SKILL_NAME, SkillIndex};

/// 维护任务的动作。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolTaskAction {
    /// 登记新工具（允许尚无 slug）。
    Register,
    /// 修改已有共享资料。
    Edit,
    /// 本机安装。
    Install,
    /// 本机更新。
    Update,
    /// 补写或修订维护指南。
    Guide,
}

impl ToolTaskAction {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "register" => Ok(Self::Register),
            "edit" => Ok(Self::Edit),
            "install" => Ok(Self::Install),
            "update" => Ok(Self::Update),
            "guide" => Ok(Self::Guide),
            other => Err(CoreError::other(format!(
                "未知工具任务动作 {other}（应为 register|edit|install|update|guide）"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Register => "register",
            Self::Edit => "edit",
            Self::Install => "install",
            Self::Update => "update",
            Self::Guide => "guide",
        }
    }

    /// 任务正文中的动作描述。
    pub fn label(self) -> &'static str {
        match self {
            Self::Register => "登记工具、编写维护资料与版本检查命令",
            Self::Edit => "修改工具共享资料（简介、维护指南、检查命令、配套 Skill 来源）",
            Self::Install => "在本机安装该工具，并按 FR-048 补写维护指南",
            Self::Update => "在本机更新该工具，并核实回写安装结果",
            Self::Guide => "补写或修订该工具的维护指南",
        }
    }

    /// 是否需要给出已登记 slug。
    pub fn requires_slug(self) -> bool {
        !matches!(self, Self::Register)
    }
}

/// 配套维护 Skill 的引用行；未入库时如实提示，不编造路径。
pub fn maintenance_skill_line(index: Option<&SkillIndex>) -> String {
    match index.and_then(|index| index.maintenance_skill()) {
        Some(entry) => format!(
            "配套维护 Skill：{}（技能 {}/{}；按其规范执行，规范全文到库内阅读）",
            entry.dir.display(),
            entry.domain.as_str(),
            entry.name
        ),
        None => format!(
            "配套维护 Skill：未入库（库内未找到 {MAINTENANCE_SKILL_NAME}；请先按普通技能入库流程取得该技能。此处不编造库内路径）"
        ),
    }
}

fn line(out: &mut String, label: &str, value: impl AsRef<str>) {
    out.push_str(&format!("{label}：{}\n", value.as_ref()));
}

fn optional_line(out: &mut String, label: &str, value: Option<&String>) {
    if let Some(value) = value {
        line(out, label, value);
    }
}

fn install_block(out: &mut String, entry: &ToolEntry) {
    match &entry.install {
        Some(install) => {
            line(out, "本机安装状态", "已安装（Agent 已核实回写）");
            line(out, "当前安装版本", &install.version);
            optional_line(out, "安装方式", install.method.as_ref());
            optional_line(out, "安装位置", install.path.as_ref());
            optional_line(out, "配置位置", install.config_path.as_ref());
        }
        None => {
            line(
                out,
                "本机安装状态",
                "未安装（无 Agent 核实回写的安装记录；未安装不等于已获知上游版本）",
            );
        }
    }
}

fn check_block(out: &mut String, entry: &ToolEntry) {
    match entry.check.status.as_str() {
        "checked" => {
            line(
                out,
                "版本检查结果",
                format!(
                    "已检查（{}）；检查所得上游最新版本：{}",
                    entry.check.checked_at.as_deref().unwrap_or("时间未记录"),
                    entry
                        .check
                        .latest_version
                        .as_deref()
                        .unwrap_or("未记录")
                ),
            );
        }
        "failed" => {
            line(
                out,
                "版本检查结果",
                format!(
                    "检查失败：{}",
                    entry.check.output.as_deref().unwrap_or("未记录原因")
                ),
            );
        }
        "checking" => line(out, "版本检查结果", "检查进行中（尚无结论）"),
        "no_command" => line(
            out,
            "版本检查结果",
            "未登记检查命令（请按工具实际来源编写 check_command）",
        ),
        _ => line(out, "版本检查结果", "未检查（软件尚未执行检查，无上游版本）"),
    }
    match entry.comparison.as_deref() {
        Some("latest") => line(
            out,
            "版本对比",
            "本机安装版本与检查所得上游版本一致（已是最新）",
        ),
        Some("updatable") => line(out, "版本对比", "上游存在新版本（可更新）"),
        Some(_) => line(
            out,
            "版本对比",
            format!(
                "无法判定是否可更新：{}",
                entry
                    .comparison_reason
                    .as_deref()
                    .unwrap_or("版本字符串无法比较")
            ),
        ),
        None => {
            if entry.install.is_none() {
                line(
                    out,
                    "版本对比",
                    "不适用：本机未安装，只有上游版本可展示",
                );
            }
        }
    }
}

/// 生成一份工具维护任务说明。
pub fn tool_task(
    root: &Path,
    state: &StateStore,
    index: Option<&SkillIndex>,
    slug: Option<&str>,
    action: ToolTaskAction,
    requirement: Option<&str>,
) -> Result<String> {
    if action.requires_slug() && slug.is_none() {
        return Err(CoreError::other(format!(
            "tool task --action {} 需要给出已登记工具的 slug",
            action.as_str()
        )));
    }

    let mut out = String::new();
    out.push_str("请通过本项目工具中心的 CLI（命令行接口）处理以下维护工作。\n");
    out.push_str(&maintenance_skill_line(index));
    out.push('\n');
    line(&mut out, "任务", action.label());
    line(&mut out, "本机系统", crate::deploy::current_os());
    line(
        &mut out,
        "用户要求",
        requirement
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or("未填写（按工具资料与维护指南执行）"),
    );

    if let Some(slug) = slug {
        let entry = catalog::get(root, state, slug)?;
        out.push('\n');
        line(&mut out, "工具 slug", &entry.slug);
        line(&mut out, "工具名称", &entry.name);
        optional_line(&mut out, "卡片短摘要", entry.summary.as_ref());
        if let Some(intro) = &entry.intro {
            out.push_str(&format!("工具简介：\n{intro}\n"));
        }
        line(&mut out, "工具资料目录", &entry.dir);
        if entry.has_guide {
            line(&mut out, "维护指南", "（见下）");
        } else {
            line(
                &mut out,
                "维护指南",
                "尚未编写；请按 FR-048 补写用途与调用入口、实际安装方式、安装与配置位置、版本检查命令、更新步骤、验证方式与注意事项",
            );
        }
        line(
            &mut out,
            "版本检查命令",
            entry
                .check_command
                .as_deref()
                .unwrap_or("未登记（工作目录为工具资料目录，标准输出为单个上游最新版本）"),
        );
        install_block(&mut out, &entry);
        check_block(&mut out, &entry);

        if !entry.skills.is_empty() {
            out.push_str("配套 Skill：\n");
            for skill in &entry.skills {
                let library = if skill.in_vault {
                    format!(
                        "库内已存在{}",
                        skill
                            .vault_path
                            .as_ref()
                            .map(|path| format!("（{path}）"))
                            .unwrap_or_default()
                    )
                } else {
                    "未入库".to_string()
                };
                out.push_str(&format!(
                    "- {}/{}（{library}{}）\n",
                    skill.domain,
                    skill.name,
                    skill
                        .repo
                        .as_ref()
                        .map(|repo| format!("；来源：{repo}"))
                        .unwrap_or_default()
                ));
            }
        }

        if let Some(guide) = &entry.guide {
            out.push_str("\n维护指南全文：\n");
            out.push_str(guide);
            if !guide.ends_with('\n') {
                out.push('\n');
            }
        }

        out.push_str("\n回写要求（核实实际结果后执行，失败如实报告）：\n");
        out.push_str(&format!(
            "- 修改共享资料：skill-manager tool set {} --dir <含 tool.yaml/guide.md 及检查脚本的目录>\n",
            entry.slug
        ));
        out.push_str(&format!(
            "- 回写本机安装事实：skill-manager tool set-install {} --version <实际版本> [--method <方式> --path <安装位置> --config-path <配置位置>]\n",
            entry.slug
        ));
        if action == ToolTaskAction::Register {
            out.push_str(
                "- 新登记使用 skill-manager tool register --dir <同理目录>（slug 由 name 派生）\n",
            );
        }
    } else {
        out.push_str(
            "\n本任务尚无已登记工具：请先核实工具的真实名称、来源与安装方式，编写 `tool.yaml`（含 name、summary、intro、tags、category、check_command、skills）与 `guide.md`，再执行：\n- skill-manager tool register --dir <含 tool.yaml/guide.md 及检查脚本的目录>\n- skill-manager tool set-install <slug> --version <实际版本> [--method ... --path ... --config-path ...]\n",
        );
    }

    out.push_str("\n注意：本任务说明由软件生成，不代表已开始或已完成；实际安装、更新与核实由外部 Agent 执行，软件不自动安装。\n");
    Ok(out)
}

/// 工具资料根目录（供入口层定位）。
pub fn tools_root(root: &Path) -> std::path::PathBuf {
    crate::toolcenter::files::tools_dir(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tool(root: &Path, slug: &str, body: &str) {
        let path = crate::toolcenter::files::tool_path(root, slug);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn task_keeps_required_context_and_references_maintenance_skill() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        write_tool(
            root.path(),
            "ripgrep",
            "name: ripgrep\nsummary: 极快的正则搜索工具\nintro: |\n  第一段。\n  第二段。\ntags: [搜索]\ncategory: 命令行工具\ncheck_command: sh ./check-latest.sh\n",
        );
        std::fs::write(
            crate::toolcenter::files::guide_path(root.path(), "ripgrep"),
            "# 维护指南\n用途与入口……\n",
        )
        .unwrap();
        // 配套维护 Skill 已入库时，任务引用其真实库内路径。
        let skill_dir = root.path().join("code/维护/skill-manager-maintenance");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# 维护 Skill\n").unwrap();
        let index = SkillIndex::build(root.path()).unwrap();

        let text = tool_task(
            root.path(),
            &state,
            Some(&index),
            Some("ripgrep"),
            ToolTaskAction::Install,
            Some("请安装到本机并补写指南"),
        )
        .unwrap();

        assert!(text.contains("用户要求：请安装到本机并补写指南"), "{text}");
        assert!(text.contains("卡片短摘要：极快的正则搜索工具"), "{text}");
        assert!(text.contains("第二段"), "{text}");
        assert!(text.contains("本机安装状态：未安装"), "{text}");
        assert!(text.contains("版本检查结果：未检查"), "{text}");
        assert!(text.contains("维护指南全文："), "{text}");
        assert!(
            text.contains(&skill_dir.display().to_string()),
            "任务必须引用配套维护 Skill 的真实库内路径：{text}"
        );
        assert!(text.contains("按其规范执行"), "{text}");
        // 生成任务不冒充安装完成。
        assert!(text.contains("不代表已开始或已完成"), "{text}");
        // 不改库、不写安装事实。
        assert_eq!(state.tool_install("ripgrep").unwrap(), None);
    }

    #[test]
    fn missing_maintenance_skill_is_reported_truthfully() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        write_tool(root.path(), "rg", "name: rg\n");
        let index = SkillIndex::build(root.path()).unwrap();
        let text = tool_task(
            root.path(),
            &state,
            Some(&index),
            Some("rg"),
            ToolTaskAction::Guide,
            None,
        )
        .unwrap();
        assert!(text.contains("配套维护 Skill：未入库"), "{text}");
        assert!(!text.contains("按其规范执行"), "{text}");
    }

    #[test]
    fn non_register_actions_require_a_slug() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        assert!(
            tool_task(
                root.path(),
                &state,
                None,
                None,
                ToolTaskAction::Update,
                None
            )
            .is_err()
        );
    }

    #[test]
    fn register_without_slug_is_allowed() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        let text = tool_task(
            root.path(),
            &state,
            None,
            None,
            ToolTaskAction::Register,
            Some("工具名称：自建命令行工具\n来源：内部脚本"),
        )
        .unwrap();
        assert!(text.contains("尚无已登记工具"), "{text}");
        assert!(text.contains("tool register"), "{text}");
    }
}
