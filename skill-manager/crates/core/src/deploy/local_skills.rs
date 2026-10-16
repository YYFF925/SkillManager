//! 本机目标技能列表与删除（T068）。
//!
//! 列出某目标（内置 Agent 目录或共享目录）下的技能，标注受管/仅本地（受管判定以本机部署记录与实际路径为依据）；
//! 未受管项直接删除实际文件且不改库，受管项走 US3 的撤销与暂停规则，不把删除本机副本变成删库。
//! 删除前做路径包含性校验：名字来自用户输入，越权路径会删掉技能目录之外的无关文件。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::deploy::agent_config::is_within;
use crate::deploy::apply::DeploymentService;
use crate::deploy::compose::remove_path;
use crate::deploy::targets::TargetResolver;
use crate::domain::{validate_skill_name, Target};
use crate::error::{CoreError, Result};
use crate::output::ItemError;
use crate::state::deployments::{DeploymentRecord, DeploymentStatus};
use crate::state::StateStore;
use crate::vault::index::SkillIndex;
use crate::vault::index::file_tree;
use crate::vault::scan;

/// 本机技能目录下的一项。
#[derive(Clone, Debug, Serialize)]
pub struct LocalSkillView {
    pub deployment_blocks: std::collections::BTreeMap<String, String>,
    pub name: String,
    pub path: String,
    /// 受管：该 Agent 目标下存在指向此路径的本机部署记录。
    pub managed: bool,
    /// `managed` / `local` / `paused_local_change`。
    pub status: &'static str,
    pub target: String,
    /// 受管项对应的库内技能引用 `domain/name`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_skill: Option<String>,
    /// 库内技能是否仍存在（受管但库条目已删除时为 false）。
    pub library_exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// 本机技能的只读正文或文件；不读取库内原件来代替本机副本。
#[derive(Clone, Debug, Serialize)]
pub struct LocalSkillRead {
    pub name: String,
    pub path: String,
    pub file: String,
    pub text: Option<String>,
    pub size: u64,
    pub files: Vec<String>,
}

/// 删除本地技能的结果：成功移除项与逐项未完成原因。
#[derive(Clone, Debug, Default, Serialize)]
pub struct LocalDeleteOutcome {
    pub removed: Vec<LocalSkillView>,
    pub errors: Vec<ItemError>,
}

/// Agent 本地技能服务。
pub struct LocalSkillService<'a> {
    resolver: &'a TargetResolver<'static>,
    state: &'a StateStore,
    /// 撤销受管副本需要库根；未接入库时只能处理未受管项。
    vault_root: Option<PathBuf>,
}

impl<'a> LocalSkillService<'a> {
    pub fn new(
        resolver: &'a TargetResolver<'static>,
        state: &'a StateStore,
        vault_root: Option<PathBuf>,
    ) -> Self {
        Self {
            resolver,
            state,
            vault_root,
        }
    }

    /// 目标技能目录：内置 Agent key 用其技能目录（含本机覆盖），`shared` 用 `<home>/.agents/skills`。
    pub fn skills_dir(&self, target: &Target) -> Result<PathBuf> {
        Ok(PathBuf::from(self.resolver.resolve(target)?.path))
    }

    /// 阅读指定本机技能；仅允许读取该技能目录内的文件。
    pub fn read(&self, target: &Target, name: &str, file: Option<&str>) -> Result<LocalSkillRead> {
        validate_skill_name(name)?;
        let dir = self.skills_dir(target)?.join(name);
        read_skill_dir(&dir, file)
    }

    /// 列出该目标目录下的技能；目录不存在时如实为空（该 Agent 可能未安装）。
    pub fn list(&self, target: &Target) -> Result<Vec<LocalSkillView>> {
        let dir = self.skills_dir(target)?;
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let records = self.records_for(target)?;
        let library = self.vault_root.as_ref().map(SkillIndex::build).transpose()?;
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(|err| CoreError::io(dir.clone(), err))? {
            let entry = entry.map_err(|err| CoreError::io(dir.clone(), err))?;
            let file_type = entry
                .file_type()
                .map_err(|err| CoreError::io(entry.path(), err))?;
            if !file_type.is_dir() {
                continue;
            }
            let candidate = entry.path();
            if !scan::is_skill_dir(&candidate) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let record = find_record(&records, &candidate);
            out.push(self.view(target, name, candidate, record, library.as_ref()));
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// 单项/多选删除：未受管直接删除文件且不改库；受管走撤销与暂停规则。
    pub fn delete(&self, target: &Target, names: &[String]) -> Result<LocalDeleteOutcome> {
        let dir = self.skills_dir(target)?;
        let records = self.records_for(target)?;
        let mut outcome = LocalDeleteOutcome::default();

        for name in names {
            if let Err(err) = validate_skill_name(name) {
                outcome.errors.push(
                    ItemError::new(format!("技能名不合法：{err}")).skill(name.clone()),
                );
                continue;
            }
            let candidate = dir.join(name);
            if !candidate.exists() {
                outcome.errors.push(
                    ItemError::new("该目标的技能目录下没有这项技能")
                        .skill(name.clone())
                        .path(candidate.display().to_string()),
                );
                continue;
            }
            // 路径包含性校验：必须落在该目标的技能目录内，防止越权删除其他路径。
            if !is_within(&dir, &candidate) {
                outcome.errors.push(
                    ItemError::new("路径不在该目标的技能目录内，已拒绝删除")
                        .skill(name.clone())
                        .path(candidate.display().to_string()),
                );
                continue;
            }

            if let Some(record) = find_record(&records, &candidate) {
                if let Err(err) = self.undo_managed(record, target, &mut outcome) {
                    outcome
                        .errors
                        .push(ItemError::from_error(&err).skill(name.clone()));
                }
                continue;
            }

            if let Err(err) = remove_path(&candidate) {
                outcome.errors.push(
                    ItemError::from_error(&err).skill(name.clone()),
                );
                continue;
            }
            outcome.removed.push(LocalSkillView {
                name: name.clone(),
                path: candidate.display().to_string(),
                managed: false,
                status: "local",
                target: target.as_str(),
                library_skill: None,
                library_exists: false,
                comment: None,
                deployment_blocks: Default::default(),
            });
        }
        Ok(outcome)
    }

    /// 受管项：调用 US3 撤销（保留库内技能）；外改副本按暂停规则拒绝并如实报告。
    fn undo_managed(
        &self,
        record: &DeploymentRecord,
        target: &Target,
        outcome: &mut LocalDeleteOutcome,
    ) -> Result<()> {
        let Some(root) = &self.vault_root else {
            return Err(CoreError::other(
                "该技能是受管部署，撤销需要接入 SkillManagerVault；请先选择技能库",
            ));
        };
        let reference = record.reference();
        let service = DeploymentService::new(root.clone(), self.resolver, self.state);
        let report = service.undo(std::slice::from_ref(&reference), std::slice::from_ref(target), false)?;
        for item in report.items {
            outcome.removed.push(LocalSkillView {
                name: item_skill_name(&item.skill).to_string(),
                path: item.path,
                managed: true,
                status: "managed",
                target: item.target,
                library_skill: Some(item.skill),
                library_exists: true,
                comment: None,
                deployment_blocks: Default::default(),
            });
        }
        outcome.errors.extend(report.failures);
        Ok(())
    }

    fn records_for(&self, target: &Target) -> Result<Vec<DeploymentRecord>> {
        let target_str = target.as_str();
        Ok(self
            .state
            .deployments()?
            .into_iter()
            .filter(|record| record.target == target_str)
            .collect())
    }

    fn view(
        &self,
        target: &Target,
        name: String,
        path: PathBuf,
        record: Option<&DeploymentRecord>,
        library: Option<&SkillIndex>,
    ) -> LocalSkillView {
        match record {
            Some(record) => {
                let library_skill = record.reference().to_string();
                let entry = library.and_then(|index| index.get(&record.reference()));
                LocalSkillView {
                    name,
                    path: path.display().to_string(),
                    managed: true,
                    status: match record.status {
                        DeploymentStatus::Ok => "managed",
                        DeploymentStatus::PausedLocalChange => "paused_local_change",
                    },
                    target: target.as_str(),
                    library_skill: Some(library_skill),
                    library_exists: entry.is_some(),
                    comment: entry.and_then(|entry| entry.comment.clone()),
                    deployment_blocks: entry.map(|entry| entry.deployment_blocks()).unwrap_or_default(),
                }
            }
            None => LocalSkillView {
                name,
                path: path.display().to_string(),
                managed: false,
                status: "local",
                target: target.as_str(),
                library_skill: None,
                library_exists: false,
                comment: None,
                deployment_blocks: Default::default(),
            },
        }
    }
}

fn collect_file_paths(nodes: &[crate::vault::index::FileNode], prefix: &str, files: &mut Vec<String>) {
    for node in nodes {
        let path = if prefix.is_empty() { node.name.clone() } else { format!("{prefix}/{}", node.name) };
        if node.is_dir {
            collect_file_paths(&node.children, &path, files);
        } else {
            files.push(path);
        }
    }
}

/// 部署记录是否指向该实际路径（大小写与符号链接不敏感，按规范化路径比较）。
pub(crate) fn find_record<'r>(
    records: &'r [DeploymentRecord],
    candidate: &Path,
) -> Option<&'r DeploymentRecord> {
    let candidate = canonical_or_self(candidate);
    records.iter().find(|record| {
        canonical_or_self(Path::new(&record.target_path)) == candidate
    })
}

fn canonical_or_self(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn item_skill_name(skill: &str) -> &str {
    skill.split_once('/').map(|(_, name)| name).unwrap_or(skill)
}

/// 复用本机技能文件读取与目录边界，用于普通目标和 Hermes 档案。
pub(crate) fn read_skill_dir(dir: &Path, file: Option<&str>) -> Result<LocalSkillRead> {
        let (_, entry) = scan::entry_file(&dir)
            .ok_or_else(|| CoreError::other(format!("该目标下没有技能入口：{}", dir.display())))?;
        let file = file.unwrap_or(entry);
        let path = dir.join(file);
        if !is_within(&dir, &path) {
            return Err(CoreError::other(format!("只能阅读技能目录内的文件：{file}")));
        }
        let bytes = std::fs::read(&path).map_err(|err| CoreError::io(path.clone(), err))?;
        let size = bytes.len() as u64;
        let mut files = Vec::new();
        collect_file_paths(&file_tree(&dir)?, "", &mut files);
        Ok(LocalSkillRead {
            name: dir.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            path: dir.display().to_string(),
            file: file.to_string(),
            text: String::from_utf8(bytes).ok(),
            size,
            files,
        })
}
