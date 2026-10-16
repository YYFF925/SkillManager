//! 库内技能的内存索引：域、名、分类、路径、描述、注释、标签、声明与来源。
//!
//! 索引是**内存派生量**，不作为第二套中央库，也不落 SQLite；每次从库文件重建，
//! 因此 GUI、CLI 与外部 Agent 的改动地位平等（FR-005、宪章 V）。

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::domain::{Domain, SkillRef, names_equal};
use crate::error::{CoreError, Result};
use crate::manager::{
    self, MetaFile, SourceRecord, SkillMeta, read_source, read_translation,
};
use crate::preset::PresetFile;
use crate::vault::scan;

/// 配套维护 Skill 的技能名（FR-051）：随库同步的普通技能，任务说明引用其库内路径。
///
/// 库内未入库时不得编造路径，须如实提示“未入库”。
pub const MAINTENANCE_SKILL_NAME: &str = "skill-manager-maintenance";

/// 分类筛选。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CategoryFilter {
    /// 全部分类。
    #[default]
    All,
    /// 明确未分类（域根下）。
    Uncategorized,
    /// 指定一级分类。
    Named(String),
}

/// Agent 支持声明筛选。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentSupport {
    pub key: String,
    /// 该 Agent 所属的管理域，用于判断“缺省=域内全支持”。
    pub domain: Domain,
}

/// 组合搜索条件。
#[derive(Clone, Debug, Default)]
pub struct SearchQuery {
    pub domain: Option<Domain>,
    pub query: Option<String>,
    pub category: CategoryFilter,
    /// 标签筛选：命中任一标签即可（原型与旧版一致）。
    pub tags: Vec<String>,
    /// 只保留无标签技能。
    pub require_untagged: bool,
    pub agent: Option<AgentSupport>,
    /// 按预设筛选（预设 id）；须同时给 `domain`，避免跨域同 id 歧义。
    pub preset: Option<String>,
}

/// 技能所属预设的最小引用（用于归属展示与筛选）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PresetRef {
    pub id: String,
    pub name: String,
}

/// 索引中的一条技能。
#[derive(Clone, Debug)]
pub struct SkillEntry {
    pub domain: Domain,
    pub name: String,
    pub category: Option<String>,
    pub dir: PathBuf,
    /// 库内相对目录，`/` 分隔，含域。
    pub vault_rel_dir: String,
    pub entry_file: String,
    pub description: Option<String>,
    pub comment: Option<String>,
    pub tags: Vec<String>,
    /// `None`=缺省（该域内全支持）；`Some([])`=显式不支持任何目标。
    pub agents: Option<Vec<String>>,
    pub oses: Option<Vec<String>>,
    pub source: Option<SourceRecord>,
    pub has_translation: bool,
    pub translation_stale: bool,
    /// `.manager` 读取失败原因；不为空时资料的展示不作有效结论。
    pub manager_error: Option<String>,
    /// 该技能所属预设（同域，按预设顺序）。
    pub presets: Vec<PresetRef>,
}

impl SkillEntry {
    /// 展示新增部署限制，与部署执行使用同一准入规则；已有部署是否可撤销由入口单独表达。
    pub fn deployment_blocks(&self) -> BTreeMap<String, String> {
        use crate::deploy::{
            current_os,
            policy::{support_admission, Admission},
            targets::AgentCatalog,
        };
        use crate::domain::Target;

        let mut targets: Vec<_> = AgentCatalog::embedded().all().iter()
            .filter(|agent| agent.reference_domain == self.domain)
            .map(|agent| Target::Agent { key: agent.key.clone() })
            .collect();
        if self.domain == Domain::Code {
            targets.push(Target::Shared);
        } else {
            targets.push(Target::HermesDefault);
        }
        targets.into_iter().filter_map(|target| {
            let reason = if let Some(error) = &self.manager_error {
                Some(error.clone())
            } else {
                match support_admission(self, &target, current_os()) {
                    Admission::Allowed => None,
                    Admission::Unsupported(reason) => Some(reason),
                }
            };
            reason.map(|reason| (target.as_str(), reason))
        }).collect()
    }
    pub fn reference(&self) -> SkillRef {
        SkillRef {
            domain: self.domain,
            name: self.name.clone(),
        }
    }

    /// 该技能是否支持某 Agent：缺省按所属域内全支持解释（FR-007）。
    pub fn supports_agent(&self, key: &str, agent_domain: Domain) -> bool {
        match &self.agents {
            None => agent_domain == self.domain,
            Some(list) => list.iter().any(|declared| declared == key),
        }
    }

    /// 组合搜索的文本命中：原名 + 描述 + 注释 + 来源 + 分类 + 标签。
    pub fn matches_text(&self, needle_lower: &str) -> bool {
        let mut haystack = String::new();
        haystack.push_str(&self.name);
        haystack.push('\n');
        if let Some(description) = &self.description {
            haystack.push_str(description);
            haystack.push('\n');
        }
        if let Some(comment) = &self.comment {
            haystack.push_str(comment);
            haystack.push('\n');
        }
        if let Some(category) = &self.category {
            haystack.push_str(category);
            haystack.push('\n');
        }
        for tag in &self.tags {
            haystack.push_str(tag);
            haystack.push('\n');
        }
        if let Some(source) = &self.source {
            haystack.push_str(&source.repo);
            haystack.push('\n');
            haystack.push_str(&source.subpath);
            haystack.push('\n');
            if let Some(reference) = &source.reference {
                haystack.push_str(reference);
                haystack.push('\n');
            }
        }
        haystack.to_lowercase().contains(needle_lower)
    }

    /// 稳定的对外表示（GUI 与 Agent 共用）。
    pub fn to_summary(&self) -> SkillSummary {
        SkillSummary {
            deployment_blocks: self.deployment_blocks(),
            domain: self.domain.as_str(),
            name: self.name.clone(),
            category: self.category.clone(),
            comment: self.comment.clone(),
            tags: self.tags.clone(),
            description: self.description.clone(),
            agents: self.agents.clone(),
            oses: self.oses.clone(),
            has_translation: self.has_translation,
            translation_stale: self.translation_stale,
            vault_rel_dir: self.vault_rel_dir.clone(),
            path: self.dir.display().to_string(),
            entry_file: self.entry_file.clone(),
            source: self.source.as_ref().map(SourceView::from_record),
            manager_error: self.manager_error.clone(),
            presets: self.presets.clone(),
        }
    }
}

/// 技能摘要（列表项）。
#[derive(Clone, Debug, Serialize)]
pub struct SkillSummary {
    pub deployment_blocks: BTreeMap<String, String>,
    pub domain: &'static str,
    pub name: String,
    pub category: Option<String>,
    pub comment: Option<String>,
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub agents: Option<Vec<String>>,
    pub oses: Option<Vec<String>>,
    pub has_translation: bool,
    pub translation_stale: bool,
    pub vault_rel_dir: String,
    pub path: String,
    pub entry_file: String,
    pub source: Option<SourceView>,
    pub manager_error: Option<String>,
    /// 该技能所属预设（同域，按预设顺序）。
    pub presets: Vec<PresetRef>,
}

/// 来源记录对外表示。
#[derive(Clone, Debug, Serialize)]
pub struct SourceView {
    pub repo: String,
    #[serde(rename = "ref")]
    pub reference: Option<String>,
    pub subpath: String,
    pub version: String,
    pub skipped_version: Option<String>,
    pub registered_at: String,
}

impl SourceView {
    pub fn from_record(record: &SourceRecord) -> Self {
        Self {
            repo: record.repo.clone(),
            reference: record.reference.clone(),
            subpath: record.subpath.clone(),
            version: record.version.clone(),
            skipped_version: record.skipped_version.clone(),
            registered_at: record.registered_at.clone(),
        }
    }
}

/// 内存索引。
#[derive(Clone, Debug)]
pub struct SkillIndex {
    root: PathBuf,
    skills: Vec<SkillEntry>,
    /// 预设定义读取失败原因；不为空时预设归属不作有效结论（技能本身仍可用）。
    preset_error: Option<String>,
}

impl SkillIndex {
    /// 从库文件重建索引。
    pub fn build(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let mut index = Self {
            root,
            skills: Vec::new(),
            preset_error: None,
        };
        index.refresh()?;
        Ok(index)
    }

    /// 重新读取库文件（文件监听与每次 CLI 调用都走这里）。
    pub fn refresh(&mut self) -> Result<()> {
        let scanned = scan::scan_vault(&self.root)?;
        let (membership, preset_error) = self.load_presets();
        let mut skills = Vec::with_capacity(scanned.len());
        for item in scanned {
            skills.push(self.load_entry(item, &membership)?);
        }
        skills.sort_by(|a, b| {
            a.domain
                .cmp(&b.domain)
                .then(a.category.cmp(&b.category))
                .then(a.name.cmp(&b.name))
        });
        self.skills = skills;
        self.preset_error = preset_error;
        Ok(())
    }

    /// 读取两域预设定义，得到 `(domain, 小写技能名) → 预设` 的归属表。
    ///
    /// 预设文件形状不合法时只记录原因，不阻断库索引（技能仍可浏览与部署）。
    fn load_presets(&self) -> (HashMap<(Domain, String), Vec<PresetRef>>, Option<String>) {
        let mut membership: HashMap<(Domain, String), Vec<PresetRef>> = HashMap::new();
        let mut error: Option<String> = None;
        for domain in Domain::ALL {
            let presets = match PresetFile::load(&self.root, domain).and_then(|file| file.presets())
            {
                Ok(presets) => presets,
                Err(err) => {
                    if error.is_none() {
                        error = Some(err.to_string());
                    }
                    continue;
                }
            };
            for preset in presets {
                for member in &preset.members {
                    membership
                        .entry((domain, member.to_lowercase()))
                        .or_default()
                        .push(PresetRef {
                            id: preset.id.clone(),
                            name: preset.name.clone(),
                        });
                }
            }
        }
        (membership, error)
    }

    fn load_entry(
        &self,
        item: scan::ScannedSkill,
        membership: &HashMap<(Domain, String), Vec<PresetRef>>,
    ) -> Result<SkillEntry> {
        let dir = item.dir;
        let mut errors: Vec<String> = Vec::new();
        let presets = membership
            .get(&(item.domain, item.name.to_lowercase()))
            .cloned()
            .unwrap_or_default();

        let meta = match MetaFile::load(&dir).and_then(|file| file.meta()) {
            Ok(meta) => meta,
            Err(err) => {
                errors.push(err.to_string());
                SkillMeta::default()
            }
        };
        let source = match read_source(&dir) {
            Ok(source) => source,
            Err(err) => {
                errors.push(err.to_string());
                None
            }
        };
        let translation = match read_translation(&dir) {
            Ok(translation) => translation,
            Err(err) => {
                errors.push(err.to_string());
                None
            }
        };

        // 译文时效：只有原文内容变化（且译文存在）才提示待更新。
        let translation_stale = match (&translation, &meta.translation_source_hash) {
            (Some(_), Some(hash)) => match manager::entry_content_hash(&dir) {
                Ok(current) => &current != hash,
                Err(err) => {
                    errors.push(err.to_string());
                    false
                }
            },
            _ => false,
        };

        Ok(SkillEntry {
            domain: item.domain,
            name: item.name,
            category: item.category,
            description: scan::read_frontmatter(&dir).description,
            comment: meta.comment,
            tags: meta.tags,
            agents: meta.agents,
            oses: meta.oses,
            source,
            has_translation: translation.is_some(),
            translation_stale,
            manager_error: if errors.is_empty() {
                None
            } else {
                Some(errors.join("；"))
            },
            vault_rel_dir: item.vault_rel_dir,
            entry_file: item.entry_file,
            dir,
            presets,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 预设定义读取失败原因；不为空时归属展示不作有效结论。
    pub fn preset_error(&self) -> Option<&str> {
        self.preset_error.as_deref()
    }

    pub fn all(&self) -> &[SkillEntry] {
        &self.skills
    }

    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// 按 `(domain, name)` 定位；同域同名（大小写不敏感）取第一个。
    pub fn get(&self, reference: &SkillRef) -> Option<&SkillEntry> {
        self.skills.iter().find(|entry| {
            entry.domain == reference.domain && names_equal(&entry.name, &reference.name)
        })
    }

    /// 按技能名跨域定位；同名时按域顺序取第一个。
    pub fn find_by_name(&self, name: &str) -> Option<&SkillEntry> {
        self.skills
            .iter()
            .find(|entry| names_equal(&entry.name, name))
    }

    /// 配套维护 Skill（FR-051）在库内的真实条目；**未入库返回 `None`**，调用方须如实提示。
    pub fn maintenance_skill(&self) -> Option<&SkillEntry> {
        self.find_by_name(MAINTENANCE_SKILL_NAME)
    }

    /// 组合筛选（域、文本、分类、标签、无标签、支持声明）。
    pub fn search(&self, query: &SearchQuery) -> Vec<&SkillEntry> {
        let needle = query
            .query
            .as_ref()
            .map(|text| text.trim().to_lowercase())
            .filter(|text| !text.is_empty());

        self.skills
            .iter()
            .filter(|entry| match query.domain {
                Some(domain) => entry.domain == domain,
                None => true,
            })
            .filter(|entry| category_matches(&query.category, entry))
            .filter(|entry| match &needle {
                Some(needle) => entry.matches_text(needle),
                None => true,
            })
            .filter(|entry| {
                if query.require_untagged {
                    return entry.tags.is_empty();
                }
                query.tags.is_empty() || entry.tags.iter().any(|tag| query.tags.contains(tag))
            })
            .filter(|entry| match &query.agent {
                Some(agent) => entry.supports_agent(&agent.key, agent.domain),
                None => true,
            })
            .filter(|entry| match &query.preset {
                Some(preset) => entry
                    .presets
                    .iter()
                    .any(|item| names_equal(&item.id, preset)),
                None => true,
            })
            .collect()
    }

    /// 分类联动的标签候选：只含所选域与分类内实际出现的标签。
    pub fn tag_candidates(&self, domain: Option<Domain>, category: &CategoryFilter) -> Vec<String> {
        let mut tags: Vec<String> = Vec::new();
        for entry in &self.skills {
            if let Some(domain) = domain {
                if entry.domain != domain {
                    continue;
                }
            }
            if !category_matches(category, entry) {
                continue;
            }
            for tag in &entry.tags {
                if !tags.iter().any(|existing| existing == tag) {
                    tags.push(tag.clone());
                }
            }
        }
        tags.sort();
        tags
    }

    /// 指定域（或两域）出现的一级分类。
    pub fn categories(&self, domain: Option<Domain>) -> Vec<String> {
        let mut categories: Vec<String> = Vec::new();
        for entry in &self.skills {
            if let Some(domain) = domain {
                if entry.domain != domain {
                    continue;
                }
            }
            if let Some(category) = &entry.category {
                if !categories.iter().any(|existing| existing == category) {
                    categories.push(category.clone());
                }
            }
        }
        categories.sort();
        categories
    }
}

fn category_matches(filter: &CategoryFilter, entry: &SkillEntry) -> bool {
    match filter {
        CategoryFilter::All => true,
        CategoryFilter::Uncategorized => entry.category.is_none(),
        CategoryFilter::Named(name) => entry.category.as_deref() == Some(name.as_str()),
    }
}

/// 技能正文（阅读用）。
#[derive(Clone, Debug, Serialize)]
pub struct SkillContent {
    /// 原文（入口文件内容）。
    pub original: String,
    /// 中文译文；无译文为 `null`。
    pub translation: Option<String>,
    pub has_translation: bool,
    /// 原文变化而译文未维护。
    pub translation_stale: bool,
    /// 默认阅读语言：有译文为 `zh`，否则 `orig`。
    pub default_lang: &'static str,
}

impl SkillContent {
    pub fn load(entry: &SkillEntry) -> Result<Self> {
        let original = read_entry_text(&entry.dir)?;
        let translation = read_translation(&entry.dir)?;
        let default_lang = if translation.is_some() { "zh" } else { "orig" };
        Ok(Self {
            original,
            has_translation: translation.is_some(),
            translation,
            translation_stale: entry.translation_stale,
            default_lang,
        })
    }
}

/// 请求的阅读语言。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadLang {
    Zh,
    Orig,
}

impl ReadLang {
    pub fn as_str(self) -> &'static str {
        match self {
            ReadLang::Zh => "zh",
            ReadLang::Orig => "orig",
        }
    }
}

/// 一次阅读的正文。
#[derive(Clone, Debug, Serialize)]
pub struct SkillBody {
    pub skill: String,
    pub lang: &'static str,
    pub text: String,
    pub translation_stale: bool,
}

/// 按请求语言读取正文；有译文默认中文，无译文显示原文。
pub fn read_body(entry: &SkillEntry, requested: Option<ReadLang>) -> Result<SkillBody> {
    let translation = read_translation(&entry.dir)?;
    let want_zh = matches!(requested, None | Some(ReadLang::Zh));
    let use_zh = want_zh && translation.is_some();

    let (lang, text) = if use_zh {
        (ReadLang::Zh, translation.unwrap_or_default())
    } else {
        (ReadLang::Orig, read_entry_text(&entry.dir)?)
    };

    Ok(SkillBody {
        skill: entry.reference().to_string(),
        lang: lang.as_str(),
        text,
        translation_stale: entry.translation_stale,
    })
}

/// 文件树节点（带目录层级，路径相对技能目录，`/` 分隔）。
#[derive(Clone, Debug, Serialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub children: Vec<FileNode>,
}

/// 收集技能目录的文件树（跳过 `.git`，不跟随符号链接）。
pub fn file_tree(skill_dir: &Path) -> Result<Vec<FileNode>> {
    let mut nodes = Vec::new();
    let entries =
        std::fs::read_dir(skill_dir).map_err(|err| CoreError::io(skill_dir.to_path_buf(), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| CoreError::io(skill_dir.to_path_buf(), err))?;
        let file_type = entry
            .file_type()
            .map_err(|err| CoreError::io(entry.path(), err))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if scan::SKIP_DIRS.contains(&name.as_str()) {
            continue;
        }
        let is_dir = file_type.is_dir();
        let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        let children = if is_dir {
            file_tree(&entry.path())?
        } else {
            Vec::new()
        };
        nodes.push(FileNode {
            name: name.clone(),
            path: name,
            is_dir,
            size: if is_dir { 0 } else { size },
            children,
        });
    }
    // 目录在前，同类按名称排序；子节点的 path 补上父前缀。
    nodes.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(nodes)
}

/// 给文件树节点补全相对路径前缀（`file_tree` 只给叶子名）。
fn prefix_paths(nodes: &mut [FileNode], prefix: &str) {
    for node in nodes {
        node.path = if prefix.is_empty() {
            node.name.clone()
        } else {
            format!("{prefix}/{}", node.name)
        };
        let child_prefix = node.path.clone();
        prefix_paths(&mut node.children, &child_prefix);
    }
}

/// 技能详情：正文、资料、来源、文件树、库内路径与实际部署位置。
#[derive(Clone, Debug, Serialize)]
pub struct SkillDetail {
    pub skill: SkillSummary,
    pub content: SkillContent,
    pub vault_path: String,
    pub files: Vec<FileNode>,
    /// 本机部署位置；由调用方从本机状态库填入。
    pub deployments: Vec<DeploymentView>,
    pub deployments_note: String,
}

/// 部署位置记录（data-model §2.1）。
#[derive(Clone, Debug, Serialize)]
pub struct DeploymentView {
    pub target: String,
    pub target_path: String,
    pub status: String,
}

impl SkillDetail {
    pub fn load(entry: &SkillEntry, vault_root: &Path) -> Result<Self> {
        let mut files = file_tree(&entry.dir)?;
        prefix_paths(&mut files, "");
        Ok(Self {
            skill: entry.to_summary(),
            content: SkillContent::load(entry)?,
            vault_path: vault_root.display().to_string(),
            files,
            deployments: Vec::new(),
            deployments_note: "部署位置尚未接入".to_string(),
        })
    }

    /// 填入本机部署记录（M2 起由调用方从状态库读取）。
    pub fn with_deployments(
        mut self,
        deployments: Vec<DeploymentView>,
        note: impl Into<String>,
    ) -> Self {
        self.deployments = deployments;
        self.deployments_note = note.into();
        self
    }
}

fn read_entry_text(skill_dir: &Path) -> Result<String> {
    let Some((path, _)) = scan::entry_file(skill_dir) else {
        return Err(CoreError::other(format!(
            "技能目录缺少 SKILL.md/skill.md：{}",
            skill_dir.display()
        )));
    };
    let bytes = std::fs::read(&path).map_err(|err| CoreError::io(path, err))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
