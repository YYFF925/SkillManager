//! 预设管理（T063）：新建/删除/重命名/图标、预设排序、成员增删与成员显示排序。
//!
//! 所有操作只改库内组合定义，**不触发部署**（FR-024）；成员限定同域（按同域技能索引解析）。
//! 两种排序（预设间 `order`、成员 `members` 数组顺序）都不改变成员集合、分类目录或实际部署；
//! 不另设成员排序字段（FR-025、data-model §1.4）。

use std::path::Path;

use serde::Serialize;

use crate::domain::{Domain, SkillRef, names_equal};
use crate::error::{CoreError, Result};
use crate::vault::index::SkillIndex;

use super::files::{Preset, PresetFile};

/// 预设成员在界面上的表示；缺失成员如实标注，不阻塞其他成员。
#[derive(Clone, Debug, Serialize)]
pub struct PresetMemberView {
    pub name: String,
    /// 库内已无同名技能（“缺失成员”）。
    pub missing: bool,
}

/// 预设对外表示。
#[derive(Clone, Debug, Serialize)]
pub struct PresetView {
    pub domain: &'static str,
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub order: i64,
    /// 成员（含缺失成员），顺序即显示顺序。
    pub members: Vec<PresetMemberView>,
    /// 缺失成员名汇总。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing_members: Vec<String>,
}

/// 列出预设；不给域时两域分别读取并合并（各自文件内顺序）。
pub fn list(root: &Path, domain: Option<Domain>) -> Result<Vec<PresetView>> {
    let index = SkillIndex::build(root)?;
    let domains: Vec<Domain> = match domain {
        Some(domain) => vec![domain],
        None => Domain::ALL.to_vec(),
    };
    let mut out = Vec::new();
    for domain in domains {
        let file = PresetFile::load(root, domain)?;
        for preset in file.presets()? {
            out.push(view(domain, &preset, &index));
        }
    }
    Ok(out)
}

/// 读取一个预设；完整引用为 `(domain, id)`。
pub fn get(root: &Path, domain: Domain, id: &str) -> Result<PresetView> {
    let index = SkillIndex::build(root)?;
    let file = PresetFile::load(root, domain)?;
    let preset = file.find(id)?.ok_or_else(|| not_found(domain, id))?;
    Ok(view(domain, &preset, &index))
}

/// 新建预设；id 由名称派生并在域内去重。
pub fn create(
    root: &Path,
    domain: Domain,
    name: &str,
    icon: Option<&str>,
) -> Result<PresetView> {
    let mut file = PresetFile::load(root, domain)?;
    let preset = file.create(name, icon)?;
    file.save()?;
    Ok(view(domain, &preset, &SkillIndex::build(root)?))
}

/// 删除预设；只删组合定义，不影响任何实际部署。
pub fn delete(root: &Path, domain: Domain, id: &str) -> Result<PresetView> {
    let mut file = PresetFile::load(root, domain)?;
    let preset = file.delete(id)?;
    file.save()?;
    Ok(view(domain, &preset, &SkillIndex::build(root)?))
}

/// 重命名预设；只改名称，不改变成员与部署。
pub fn rename(root: &Path, domain: Domain, id: &str, name: &str) -> Result<PresetView> {
    let mut file = PresetFile::load(root, domain)?;
    let preset = file.rename(id, name)?;
    file.save()?;
    Ok(view(domain, &preset, &SkillIndex::build(root)?))
}

/// 设置预设图标；传 `None` 清除。
pub fn set_icon(
    root: &Path,
    domain: Domain,
    id: &str,
    icon: Option<&str>,
) -> Result<PresetView> {
    let mut file = PresetFile::load(root, domain)?;
    let preset = file.set_icon(id, icon)?;
    file.save()?;
    Ok(view(domain, &preset, &SkillIndex::build(root)?))
}

/// 调整预设间显示顺序；`order` 必须是当前域全部预设 id 的一个排列。
pub fn reorder(root: &Path, domain: Domain, order: &[String]) -> Result<Vec<PresetView>> {
    let mut file = PresetFile::load(root, domain)?;
    let current: Vec<String> = file.presets()?.into_iter().map(|preset| preset.id).collect();
    let ordered = permutation(&current, order, "预设")?;
    file.set_order(&ordered)?;
    file.save()?;

    let index = SkillIndex::build(root)?;
    let mut out = Vec::new();
    for preset in PresetFile::load(root, domain)?.presets()? {
        out.push(view(domain, &preset, &index));
    }
    Ok(out)
}

/// 添加成员：只接受同域已存在的技能；已存在的成员跳过（保持原顺序，新成员依次追加）。
pub fn add_members(
    root: &Path,
    domain: Domain,
    id: &str,
    names: &[String],
) -> Result<PresetView> {
    let mut file = PresetFile::load(root, domain)?;
    let mut preset = file.find(id)?.ok_or_else(|| not_found(domain, id))?;

    let index = SkillIndex::build(root)?;
    let mut unknown: Vec<String> = Vec::new();
    let mut additions: Vec<String> = Vec::new();
    for raw in names {
        let name = raw.trim();
        if name.is_empty() {
            continue;
        }
        if index.get(&SkillRef { domain, name: name.to_string() }).is_none() {
            if !unknown.iter().any(|item| names_equal(item, name)) {
                unknown.push(name.to_string());
            }
            continue;
        }
        if preset.members.iter().any(|item| names_equal(item, name))
            || additions.iter().any(|item| names_equal(item, name))
        {
            continue;
        }
        additions.push(name.to_string());
    }
    if !unknown.is_empty() {
        return Err(CoreError::other(format!(
            "{}中不存在以下技能，未加入预设：{}",
            domain.label(),
            unknown.join("、")
        )));
    }

    preset.members.extend(additions);
    let preset = file.set_members(id, &preset.members)?;
    file.save()?;
    Ok(view(domain, &preset, &index))
}

/// 移除成员；不在成员列表中的名字按无操作处理。
pub fn remove_members(
    root: &Path,
    domain: Domain,
    id: &str,
    names: &[String],
) -> Result<PresetView> {
    let mut file = PresetFile::load(root, domain)?;
    let mut preset = file.find(id)?.ok_or_else(|| not_found(domain, id))?;
    preset
        .members
        .retain(|member| !names.iter().any(|name| names_equal(member, name.trim())));
    let preset = file.set_members(id, &preset.members)?;
    file.save()?;
    Ok(view(domain, &preset, &SkillIndex::build(root)?))
}

/// 调整成员显示顺序；`order` 必须是当前成员（含缺失成员）的一个排列。
/// 复用 `members` 数组顺序，不另建字段；不增减成员、不动目录、不改变部署。
pub fn reorder_members(
    root: &Path,
    domain: Domain,
    id: &str,
    order: &[String],
) -> Result<PresetView> {
    let mut file = PresetFile::load(root, domain)?;
    let preset = file.find(id)?.ok_or_else(|| not_found(domain, id))?;
    let ordered = permutation(&preset.members, order, "成员")?;
    let preset = file.set_members(id, &ordered)?;
    file.save()?;
    Ok(view(domain, &preset, &SkillIndex::build(root)?))
}

/// 某预设的成员技能名（含缺失成员），供部署/撤销与筛选使用。
pub fn member_names(root: &Path, domain: Domain, id: &str) -> Result<Vec<String>> {
    let file = PresetFile::load(root, domain)?;
    let preset = file.find(id)?.ok_or_else(|| not_found(domain, id))?;
    Ok(preset.members)
}

// ---------------------------------------------------------------------------
// 内部
// ---------------------------------------------------------------------------

fn not_found(domain: Domain, id: &str) -> CoreError {
    CoreError::other(format!("预设不存在：{}/{}", domain.as_str(), id))
}

fn view(domain: Domain, preset: &Preset, index: &SkillIndex) -> PresetView {
    let mut members = Vec::with_capacity(preset.members.len());
    let mut missing = Vec::new();
    for name in &preset.members {
        let exists = index
            .get(&SkillRef {
                domain,
                name: name.clone(),
            })
            .is_some();
        if !exists && !missing.iter().any(|item: &String| names_equal(item, name)) {
            missing.push(name.clone());
        }
        members.push(PresetMemberView {
            name: name.clone(),
            missing: !exists,
        });
    }
    PresetView {
        domain: domain.as_str(),
        id: preset.id.clone(),
        name: preset.name.clone(),
        icon: preset.icon.clone(),
        order: preset.order,
        members,
        missing_members: missing,
    }
}

/// 要求 `requested` 恰为 `current` 的一个排列，返回按 `requested` 顺序排列的**现有拼写**。
fn permutation(current: &[String], requested: &[String], what: &str) -> Result<Vec<String>> {
    if requested.is_empty() && current.is_empty() {
        return Ok(Vec::new());
    }
    if requested.len() != current.len() {
        return Err(CoreError::other(format!(
            "{what}顺序必须包含全部 {} 项（收到 {} 项）",
            current.len(),
            requested.len()
        )));
    }
    let mut used = vec![false; current.len()];
    let mut out = Vec::with_capacity(requested.len());
    for raw in requested {
        let wanted = raw.trim();
        let position = current
            .iter()
            .enumerate()
            .find(|(index, existing)| !used[*index] && names_equal(existing, wanted))
            .map(|(index, _)| index);
        let Some(index) = position else {
            return Err(CoreError::other(format!(
                "{what}顺序包含未知项或重复项：{wanted}"
            )));
        };
        used[index] = true;
        out.push(current[index].clone());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> tempfile::TempDir {
        let root = tempfile::TempDir::new().unwrap();
        for name in ["alpha", "beta", "gamma"] {
            let dir = root.path().join("code").join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("SKILL.md"), "x").unwrap();
        }
        root
    }

    #[test]
    fn create_add_reorder_and_missing_members() {
        let root = env();
        let created = create(root.path(), Domain::Code, "发布流程", Some("rocket")).unwrap();
        assert_eq!(created.id, "preset");
        let created = create(root.path(), Domain::Code, "Rust 检查", None).unwrap();
        assert_eq!(created.members.len(), 0);

        let updated = add_members(
            root.path(),
            Domain::Code,
            &created.id,
            &["alpha".to_string(), "beta".to_string(), "alpha".to_string()],
        )
        .unwrap();
        assert_eq!(
            updated.members.iter().map(|m| m.name.clone()).collect::<Vec<_>>(),
            vec!["alpha", "beta"]
        );

        let reordered = reorder_members(
            root.path(),
            Domain::Code,
            &created.id,
            &["beta".to_string(), "alpha".to_string()],
        )
        .unwrap();
        assert_eq!(
            reordered.members.iter().map(|m| m.name.clone()).collect::<Vec<_>>(),
            vec!["beta", "alpha"]
        );

        // 删除库内技能后，成员变为“缺失成员”，其他成员不受影响。
        std::fs::remove_dir_all(root.path().join("code/beta")).unwrap();
        let after = get(root.path(), Domain::Code, &created.id).unwrap();
        assert_eq!(after.missing_members, vec!["beta".to_string()]);
        assert!(after.members.iter().any(|m| m.name == "alpha" && !m.missing));
        assert!(after.members.iter().any(|m| m.name == "beta" && m.missing));
    }

    #[test]
    fn members_are_limited_to_the_same_domain() {
        let root = env();
        let lobster = root.path().join("lobster").join("lob-skill");
        std::fs::create_dir_all(&lobster).unwrap();
        std::fs::write(lobster.join("SKILL.md"), "x").unwrap();

        let preset = create(root.path(), Domain::Code, "编码组", None).unwrap();
        let err = add_members(
            root.path(),
            Domain::Code,
            &preset.id,
            &["lob-skill".to_string()],
        )
        .unwrap_err();
        assert!(err.to_string().contains("编码库中不存在"), "{err}");
    }

    #[test]
    fn reorder_requires_full_permutation_and_presets_stay_separate() {
        let root = env();
        let a = create(root.path(), Domain::Code, "A", None).unwrap();
        let b = create(root.path(), Domain::Code, "B", None).unwrap();
        // 同 id 的另一域预设存在时不误选：域参数显式参与。
        let lobster = create(root.path(), Domain::Lobster, "A", None).unwrap();
        assert_eq!(lobster.id, "a");

        let err = reorder(root.path(), Domain::Code, &[b.id.clone()]).unwrap_err();
        assert!(err.to_string().contains("必须包含全部 2 项"), "{err}");

        let views = reorder(root.path(), Domain::Code, &[b.id.clone(), a.id.clone()]).unwrap();
        assert_eq!(
            views.iter().map(|v| v.id.clone()).collect::<Vec<_>>(),
            vec![b.id.clone(), a.id.clone()]
        );
        assert_eq!(views[0].order, 0);
        assert_eq!(views[1].order, 1);
    }

    #[test]
    fn editing_members_does_not_touch_other_presets() {
        let root = env();
        let a = create(root.path(), Domain::Code, "A", None).unwrap();
        let b = create(root.path(), Domain::Code, "B", None).unwrap();
        add_members(root.path(), Domain::Code, &a.id, &["alpha".to_string()]).unwrap();
        let other = get(root.path(), Domain::Code, &b.id).unwrap();
        assert!(other.members.is_empty());
    }
}
