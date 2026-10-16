//! 技能维护任务文本生成（T078，FR-051）。
//!
//! 支持单项或多选技能：保留用户要求、本机 SkillManagerVault 路径、所选技能的库内路径、注释/
//! 分类/标签/支持声明、译文/覆盖层概况与本机已有部署；规范只引用配套维护 Skill 的库内
//! 路径与“按其规范执行”，不内联规范全文，也不把完整仓库内容塞进任务。
//! 只生成文本：不改库、不部署、不上传，不启动 Agent 或任务队列。

use std::path::Path;

use crate::domain::SkillRef;
use crate::error::{CoreError, Result};
use crate::manager::list_overlays;
use crate::state::DeploymentRecord;
use crate::toolcenter::tasks::maintenance_skill_line;
use crate::vault::index::SkillIndex;

fn support_text(value: &Option<Vec<String>>, label: &str) -> String {
    match value {
        None => format!("{label}：全部支持（未声明）"),
        Some(list) if list.is_empty() => format!("{label}：不支持任何目标（显式空集合）"),
        Some(list) => format!("{label}：{}", list.join("、")),
    }
}

/// 生成一份技能维护任务说明。
pub fn skill_task(
    root: &Path,
    index: &SkillIndex,
    deployments: &[DeploymentRecord],
    refs: &[SkillRef],
    requirement: Option<&str>,
) -> Result<String> {
    if refs.is_empty() {
        return Err(CoreError::other("skill task 需要至少一个技能引用"));
    }

    let mut out = String::new();
    out.push_str("请通过本项目配套维护 Skill 的规范与 CLI（命令行接口）维护以下技能。\n");
    out.push_str(&maintenance_skill_line(Some(index)));
    out.push('\n');
    out.push_str(&format!("SkillManagerVault 本机目录：{}\n", root.display()));
    out.push_str(&format!(
        "我的维护要求：\n{}\n",
        requirement
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or("未填写（按技能现有管理资料与配套维护 Skill 规范执行）")
    ));
    out.push_str("\n所选技能（以下为生成任务时的管理资料快照）：\n");

    for reference in refs {
        let entry = index.get(reference).ok_or_else(|| CoreError::SkillNotFound {
            domain: reference.domain.as_str().to_string(),
            name: reference.name.clone(),
        })?;

        out.push_str(&format!(
            "\n- {}（{}）\n",
            entry.name,
            entry.domain.label()
        ));
        out.push_str(&format!(
            "  库内路径：{}（库内相对：{}）\n",
            entry.dir.display(),
            entry.vault_rel_dir
        ));
        out.push_str(&format!(
            "  中文注释：{}\n",
            entry.comment.as_deref().unwrap_or("无")
        ));
        out.push_str(&format!(
            "  分类：{}；标签：{}\n",
            entry.category.as_deref().unwrap_or("未分类"),
            if entry.tags.is_empty() {
                "无标签".to_string()
            } else {
                entry.tags.join("、")
            }
        ));
        out.push_str(&format!(
            "  {}\n  {}\n",
            support_text(&entry.agents, "支持 Agent"),
            support_text(&entry.oses, "系统")
        ));

        let overlays = list_overlays(&entry.dir).unwrap_or_default();
        let translation = match (entry.has_translation, entry.translation_stale) {
            (false, _) => "无".to_string(),
            (true, true) => "已有（原文已变化，待更新）".to_string(),
            (true, false) => "已有".to_string(),
        };
        out.push_str(&format!(
            "  中文译文：{translation}；覆盖层：{}；入口文件：{}\n",
            if overlays.is_empty() {
                "无".to_string()
            } else {
                format!("{} 项", overlays.len())
            },
            entry.entry_file
        ));

        let mut targets: Vec<String> = deployments
            .iter()
            .filter(|record| {
                record.domain == entry.domain
                    && crate::domain::names_equal(&record.name, &entry.name)
            })
            .map(|record| {
                if record.status == crate::state::DeploymentStatus::PausedLocalChange {
                    format!("{}（暂停：副本本地变更）", record.target)
                } else {
                    record.target.clone()
                }
            })
            .collect();
        targets.sort();
        out.push_str(&format!(
            "  本机已部署：{}\n",
            if targets.is_empty() {
                "无".to_string()
            } else {
                targets.join("、")
            }
        ));
    }

    out.push_str("\n说明：此处只生成任务文本，不代表已调用 Agent 或已完成维护；请到库内读取技能内容与配套维护 Skill 的规范全文后执行，修改后按“同步已有部署 + 询问是否上传”的既有约定处理。\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Domain;
    use crate::vault::index::SkillIndex;

    fn write_skill(root: &Path, rel: &str, body: &str) {
        let dir = root.join(rel);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), body).unwrap();
    }

    #[test]
    fn single_skill_task_keeps_context_and_references_maintenance_skill() {
        let root = tempfile::TempDir::new().unwrap();
        write_skill(root.path(), "code/matt/matt-pr", "# PR\n");
        let manager = root.path().join("code/matt/matt-pr/.manager");
        std::fs::create_dir_all(&manager).unwrap();
        std::fs::write(manager.join("meta.yaml"), "tags: [评审]\ncomment: 写 PR\n").unwrap();

        let index = SkillIndex::build(root.path()).unwrap();
        let reference = SkillRef::new(Domain::Code, "matt-pr").unwrap();
        let text = skill_task(
            root.path(),
            &index,
            &[],
            std::slice::from_ref(&reference),
            Some("补充中文译文"),
        )
        .unwrap();

        assert!(text.contains("SkillManagerVault 本机目录："), "{text}");
        assert!(text.contains("我的维护要求：\n补充中文译文"), "{text}");
        assert!(text.contains("库内路径："), "{text}");
        assert!(text.contains("中文注释：写 PR"), "{text}");
        assert!(text.contains("分类：matt；标签：评审"), "{text}");
        assert!(text.contains("本机已部署：无"), "{text}");
        // 配套维护 Skill 未入库时如实提示，不编造路径。
        assert!(text.contains("配套维护 Skill：未入库"), "{text}");
    }

    #[test]
    fn multiple_skills_are_listed_individually() {
        let root = tempfile::TempDir::new().unwrap();
        write_skill(root.path(), "code/a/skill-a", "# A\n");
        write_skill(root.path(), "lobster/skill-b", "# B\n");
        let index = SkillIndex::build(root.path()).unwrap();
        let refs = vec![
            SkillRef::new(Domain::Code, "skill-a").unwrap(),
            SkillRef::new(Domain::Lobster, "skill-b").unwrap(),
        ];
        let text = skill_task(root.path(), &index, &[], &refs, None).unwrap();
        assert!(text.contains("skill-a（编码库）"), "{text}");
        assert!(text.contains("skill-b（龙虾库）"), "{text}");
    }

    #[test]
    fn missing_skill_is_reported_not_fabricated() {
        let root = tempfile::TempDir::new().unwrap();
        write_skill(root.path(), "code/only/SKILL.md", "# X\n");
        let index = SkillIndex::build(root.path()).unwrap();
        let reference = SkillRef::new(Domain::Code, "absent").unwrap();
        let err = skill_task(root.path(), &index, &[], &[reference], None).unwrap_err();
        assert!(err.to_string().contains("技能不存在"), "{err}");
    }
}
