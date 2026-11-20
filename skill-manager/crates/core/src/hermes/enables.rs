//! Hermes 档案技能：发现、列表、明确启用/禁用与批量操作（T058）。
//!
//! 列表与安装分开：
//! - 各档案的技能列表共用一份候选池——已从龙虾库安装进 Hermes 的技能。
//! - 安装（部署）把副本放进 default；列表不按部署记录标识过滤。
//! - 具名档案“明确启用”才把该技能装进该档案（从 default 共享副本做成独立实体拷贝，以便
//!   Hermes `curator lock` 能锁住该档案内的技能；库更新经 `deploy sync` 刷新未改动的拷贝）。
//! - 禁用 = 移除该档案内的启用拷贝并删除本机启用名单条目；不改 default 池，不改用户 `config.yaml`，只读其 `skills.disabled`。
//! - 域与支持声明沿用 US3 的 `deploy::policy`（龙虾域；`hermes` Agent 维度）。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::deploy::compose::hash_dir_excluding_sidecars;
use crate::deploy::compose::replace_dir_from;
use crate::deploy::policy::{self, Admission, HERMES_AGENT_KEY, is_hermes_pool_target};
use crate::deploy::{TargetResolver, current_os};
use crate::domain::{Domain, SkillRef, Target, names_equal, validate_skill_name};
use crate::error::{CoreError, Result};
use crate::state::StateStore;
use crate::vault::index::{CategoryFilter, SearchQuery, SkillEntry, SkillIndex};
use crate::vault::scan;

use super::disabled::{DisabledState, read_disabled};
use super::hermes_home;
use super::profiles::{ProfileInfo, discover_profiles, resolve_profile};

/// 档案目录中外来副本来历：本管理器创建的链接。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LinkState {
    Missing,
    /// 指向此路径的符号链接（可能是悬空链接）。
    Symlink(PathBuf),
    /// 真实目录或文件。
    Real,
}

pub(crate) fn link_state(path: &Path) -> LinkState {
    match std::fs::symlink_metadata(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => LinkState::Missing,
        Err(_) => LinkState::Missing,
        Ok(meta) if meta.file_type().is_symlink() => {
            let target = std::fs::read_link(path).unwrap_or_default();
            LinkState::Symlink(target)
        }
        Ok(_) => LinkState::Real,
    }
}

/// 目录是否就是我们指向 default 共享副本的链接（按规范化路径比较）。
pub(crate) fn is_our_link(link: &Path, copy: &Path) -> bool {
    match link_state(link) {
        LinkState::Symlink(target) => same_path(&target, copy),
        _ => false,
    }
}

pub(crate) fn same_path(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[cfg(all(test, unix))]
pub(crate) fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(all(test, windows))]
pub(crate) fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    // Windows 上可能因缺少权限失败；失败时如实报错，不静默降级为复制。
    std::os::windows::fs::symlink_dir(target, link)
}

/// 单个档案的一体化视图。
#[derive(Clone, Debug, Serialize)]
pub struct ProfileView {
    pub name: String,
    pub display_name: Option<String>,
    pub kind: &'static str,
    pub path: String,
    pub skills_dir: String,
    pub enabled_count: usize,
}

/// 档案技能列表项。
#[derive(Clone, Debug, Serialize)]
pub struct ProfileSkillView {
    pub deployment_blocks: std::collections::BTreeMap<String, String>,
    pub name: String,
    pub domain: Domain,
    /// 库内引用（`lobster/<名>`）；不在库内为 None。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_skill: Option<String>,
    pub in_library: bool,
    /// 已部署到 default 档案（档案共用库内容的来源）。
    pub in_default_pool: bool,
    pub enabled: bool,
    /// 档案自有、尚未收编（未受管）的技能。
    pub needs_adoption: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// 标签；空集合也必须出现，客户端按数组处理（不做缺字段猜测）。
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub agents: Option<Vec<String>>,
    pub oses: Option<Vec<String>>,
    pub has_translation: bool,
    pub translation_stale: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manager_error: Option<String>,
}

/// 某档案的技能列表结果。
#[derive(Clone, Debug, Serialize)]
pub struct ProfileSkillsView {
    pub profile: String,
    pub kind: &'static str,
    pub path: String,
    pub skills_dir: String,
    pub root: String,
    pub default_skills_dir: String,
    pub skills: Vec<ProfileSkillView>,
    pub total: usize,
    pub tag_candidates: Vec<String>,
    pub categories: Vec<String>,
    /// 档案 `config.yaml` 读取失败原因（不为空时启用状态不作有效结论）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_error: Option<String>,
}

/// 启用/禁用与收编的逐项结果。
#[derive(Clone, Debug, Serialize)]
pub struct HermesItem {
    pub name: String,
    pub ok: bool,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// 为具名档案按预设批量启用或禁用的结果。
#[derive(Clone, Debug, Serialize)]
pub struct HermesPresetOutcome {
    pub profile: String,
    pub preset_id: String,
    pub preset_name: String,
    pub enabled: bool,
    /// 本次启用前未安装而自动部署到 `hermes:default` 的数量。
    pub installed: usize,
    pub changed: usize,
    pub items: Vec<HermesItem>,
}

/// 档案服务：绑定 Hermes 根、SkillManagerVault 与本机状态。
pub struct HermesService<'a> {
    root: PathBuf,
    vault_root: Option<PathBuf>,
    resolver: &'a TargetResolver<'static>,
    state: &'a StateStore,
}

impl<'a> HermesService<'a> {
    pub fn read(
        &self,
        profile: &str,
        name: &str,
        file: Option<&str>,
    ) -> Result<crate::deploy::local_skills::LocalSkillRead> {
        validate_skill_name(name)?;
        let info = resolve_profile(self.root(), profile)?;
        let path = scan_skill_dirs(&info.skills_dir)
            .into_iter()
            .find(|(found, _)| names_equal(found, name))
            .map(|(_, path)| path)
            .ok_or_else(|| CoreError::other(format!("档案中没有该技能：{profile}/{name}")))?;
        crate::deploy::local_skills::read_skill_dir(&path, file)
    }

    pub fn new(
        root: PathBuf,
        vault_root: Option<PathBuf>,
        resolver: &'a TargetResolver<'static>,
        state: &'a StateStore,
    ) -> Self {
        Self {
            root,
            vault_root,
            resolver,
            state,
        }
    }

    /// 由本机上下文与本机状态构造服务：根按 [`hermes_home`] 解析
    /// （`HERMES_HOME` 环境变量 → 平台默认），与 `hermes:default` 部署目标同源。
    pub fn from_context(
        ctx: &'a crate::context::AppContext,
        resolver: &'a TargetResolver<'static>,
    ) -> Result<Self> {
        let root = Self::root_of(ctx, resolver)?;
        Ok(Self::new(
            root,
            ctx.vault_opt().map(|vault| vault.root().to_path_buf()),
            resolver,
            ctx.state(),
        ))
    }

    /// 解析本机 Hermes 根目录；不读任何可写设置键。
    fn root_of(
        _ctx: &'a crate::context::AppContext,
        resolver: &TargetResolver<'static>,
    ) -> Result<PathBuf> {
        Ok(hermes_home(resolver.home()))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn vault_root(&self) -> Option<&Path> {
        self.vault_root.as_deref()
    }

    pub(crate) fn resolver(&self) -> &'a TargetResolver<'static> {
        self.resolver
    }

    pub(crate) fn state(&self) -> &'a StateStore {
        self.state
    }

    /// default 档案的技能目录（共享副本所在）。
    pub fn default_skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    fn library(&self) -> Result<Option<SkillIndex>> {
        self.vault_root.as_ref().map(SkillIndex::build).transpose()
    }

    // -----------------------------------------------------------------------
    // profiles
    // -----------------------------------------------------------------------

    /// 发现本机现有档案（default 在前）。
    pub fn profiles(&self) -> Result<Vec<ProfileView>> {
        let mut out = Vec::new();
        for info in discover_profiles(&self.root) {
            let enabled_count = self.enabled_count(&info);
            out.push(ProfileView {
                name: info.name.clone(),
                display_name: info.display_name.clone(),
                kind: info.kind.as_str(),
                path: info.home.display().to_string(),
                skills_dir: info.skills_dir.display().to_string(),
                enabled_count,
            });
        }
        Ok(out)
    }

    /// 档案已启用技能数：default 按根技能目录（排除 Hermes 禁用名单）；具名按本机启用名单。
    fn enabled_count(&self, info: &ProfileInfo) -> usize {
        if !info.is_default() {
            return self.state.hermes_enable_count(&info.name).unwrap_or(0);
        }
        let disabled = read_disabled(&info.home);
        count_default_enabled(&self.root, &disabled)
    }

    // -----------------------------------------------------------------------
    // list
    // -----------------------------------------------------------------------

    /// 列出某档案的技能（组合筛选，沿用库内分类/标签规则）。
    pub fn list(&self, profile: &str, query: &SearchQuery) -> Result<ProfileSkillsView> {
        let info = resolve_profile(&self.root, profile)?;
        let library = self.library()?;
        let mut skills = self.build_views(&info, library.as_ref())?;
        skills.sort_by(|a, b| a.name.cmp(&b.name));

        let tag_candidates = tag_candidates(&skills, &query.category);
        let categories = categories(&skills);
        let filtered: Vec<ProfileSkillView> = skills
            .into_iter()
            .filter(|view| matches_query(view, query))
            .collect();

        Ok(ProfileSkillsView {
            profile: info.name.clone(),
            kind: info.kind.as_str(),
            path: info.home.display().to_string(),
            skills_dir: info.skills_dir.display().to_string(),
            root: self.root.display().to_string(),
            default_skills_dir: self.default_skills_dir().display().to_string(),
            total: filtered.len(),
            skills: filtered,
            tag_candidates,
            categories,
            config_error: read_disabled(&info.home).error().map(str::to_string),
        })
    }

    /// 构造档案技能视图（含支持收窄警告/阻止）。
    fn build_views(
        &self,
        info: &ProfileInfo,
        library: Option<&SkillIndex>,
    ) -> Result<Vec<ProfileSkillView>> {
        let disabled = read_disabled(&info.home);
        let os = current_os();
        let enables = if info.is_default() {
            Vec::new()
        } else {
            self.state.hermes_enables(&info.name)?
        };
        let is_enabled = |domain: Domain, name: &str| -> bool {
            enables
                .iter()
                .any(|(d, n)| *d == domain && names_equal(n, name))
        };

        let default_dir = self.default_skills_dir();
        let scanned = scan_skill_dirs(&info.skills_dir);
        let pool = installed_hermes_skills(self.state);

        let mut out: Vec<ProfileSkillView> = Vec::new();
        let mut covered: Vec<String> = Vec::new();

        for reference in &pool {
            let entry = library.and_then(|index| index.get(reference));
            let name = reference.name.clone();
            let enabled = if info.is_default() {
                !disabled.contains(&name)
            } else {
                is_enabled(reference.domain, &name)
            };
            let default_path = default_dir.join(&name);
            let profile_path = info.skills_dir.join(&name);
            let display_path = if !info.is_default() && enabled && profile_path.exists() {
                profile_path.display().to_string()
            } else {
                default_path.display().to_string()
            };
            let mut view = base_view(
                name.clone(),
                reference.domain,
                entry,
                Some(display_path),
                true,
                enabled,
                false,
            );
            apply_admission(&mut view, entry, enabled, os);
            covered.push(name.clone());
            out.push(view);
        }

        for (name, path) in scanned {
            // 指向 default 共享副本的链接已由候选池覆盖。
            if covered.iter().any(|existing| names_equal(existing, &name)) {
                continue;
            }
            if is_our_link(&path, &default_dir.join(&name)) {
                continue;
            }
            let entry = library.and_then(|index| {
                index.get(&SkillRef {
                    domain: Domain::Lobster,
                    name: name.clone(),
                })
            });
            let enabled = !disabled.contains(&name);
            let in_library = entry.is_some();
            let view = base_view(
                name.clone(),
                Domain::Lobster,
                entry,
                Some(path.display().to_string()),
                false,
                enabled,
                !in_library,
            );
            out.push(view);
        }

        Ok(out)
    }

    // -----------------------------------------------------------------------
    // enable / disable
    // -----------------------------------------------------------------------

    /// 为具名档案批量启用或禁用技能。
    pub fn set_enabled(
        &self,
        profile: &str,
        names: &[String],
        enabled: bool,
    ) -> Result<Vec<HermesItem>> {
        let info = resolve_profile(&self.root, profile)?;
        if info.is_default() {
            return Err(CoreError::other(
                "default 档案的技能由 `hermes:default` 部署记录表达；请用 `deploy`/`deploy undo` 维护",
            ));
        }
        let library = self.library()?.ok_or_else(|| {
            CoreError::other("尚未接入 SkillManagerVault：档案启用需要库内技能内容")
        })?;
        let disabled = read_disabled(&info.home);
        let default_dir = self.default_skills_dir();

        let mut items = Vec::new();
        for name in names {
            let result = self.set_one(&info, &library, &disabled, &default_dir, name, enabled);
            items.push(match result {
                Ok(item) => item,
                Err(err) => HermesItem {
                    name: name.clone(),
                    ok: false,
                    enabled: !enabled,
                    message: Some(err.to_string()),
                },
            });
        }
        Ok(items)
    }

    fn set_one(
        &self,
        info: &ProfileInfo,
        library: &SkillIndex,
        disabled: &DisabledState,
        default_dir: &Path,
        name: &str,
        enabled: bool,
    ) -> Result<HermesItem> {
        validate_skill_name(name)?;
        let reference = SkillRef {
            domain: Domain::Lobster,
            name: name.to_string(),
        };
        let entry = library.get(&reference).ok_or_else(|| {
            CoreError::other(format!(
                "技能不在龙虾库：{reference}（档案共用库内技能内容）"
            ))
        })?;
        if let Some(reason) = &entry.manager_error {
            return Err(CoreError::other(format!(
                "管理资料读取失败，拒绝变更启用：{reason}"
            )));
        }
        let copy = default_dir.join(name);
        if !copy.is_dir() {
            return Err(CoreError::other(format!(
                "技能尚未部署到 default 档案：{reference}（其他档案共用 default 的库内容）"
            )));
        }

        let admission = policy::support_admission(entry, &Target::HermesDefault, current_os());
        let existing = self
            .state
            .hermes_enabled(&info.name, Domain::Lobster, name)?;
        let link = info.skills_dir.join(name);

        if enabled {
            if let Some(reason) = disabled.error() {
                return Err(CoreError::other(format!(
                    "档案配置读取失败，启用状态不作有效结论，未改动：{reason}"
                )));
            }
            if disabled.contains(name) {
                return Err(CoreError::other(format!(
                    "该档案的 Hermes 配置（skills.disabled）将 {name} 列为禁用；请在 Hermes 中取消禁用后再启用"
                )));
            }
            let mut message = None;
            if let Admission::Unsupported(reason) = &admission {
                if !existing {
                    return Err(CoreError::other(format!("禁止新增启用：{reason}")));
                }
                message = Some(format!("支持范围已收窄，已有启用保留并警告：{reason}"));
            }
            if !copy.is_dir() {
                return Err(CoreError::other(format!(
                    "default 档案的共享副本缺失：{}（请先重新部署）",
                    copy.display()
                )));
            }
            match link_state(&link) {
                LinkState::Real if existing => {
                    // 已是本管理器登记的实体拷贝：再次启用不覆盖档案内改动。
                }
                LinkState::Real => {
                    return Err(CoreError::other(format!(
                        "档案目录已有同名技能（非本管理器安装的副本），未改动：{}",
                        link.display()
                    )));
                }
                LinkState::Symlink(target) if !existing && !same_path(&target, &copy) => {
                    return Err(CoreError::other(format!(
                        "档案内已有同名链接指向其他位置，未改动：{}",
                        link.display()
                    )));
                }
                LinkState::Missing | LinkState::Symlink(_) => {
                    if matches!(link_state(&link), LinkState::Symlink(_)) {
                        std::fs::remove_file(&link)
                            .map_err(|err| CoreError::io(link.clone(), err))?;
                    }
                    replace_dir_from(&copy, &link)?;
                }
            }
            self.state
                .set_hermes_enable(&info.name, Domain::Lobster, name)?;
            return Ok(HermesItem {
                name: name.to_string(),
                ok: true,
                enabled: true,
                message,
            });
        }

        uninstall_profile_skill(&link, &copy, existing)?;
        self.state
            .remove_hermes_enable(&info.name, Domain::Lobster, name)?;
        Ok(HermesItem {
            name: name.to_string(),
            ok: true,
            enabled: false,
            message: None,
        })
    }

    /// 为具名档案按龙虾预设一键启用或禁用。
    ///
    /// - 启用：预设成员若尚未安装到 `hermes:default`，先静默部署到 `hermes:default`，再仅为当前档案启用（不扩散到其他档案）。
    /// - 禁用：仅禁用当前档案中的该预设成员，保留 `hermes:default` 共享副本与其他档案启用状态。
    pub fn set_preset_enabled(
        &self,
        profile: &str,
        preset_id: &str,
        enabled: bool,
    ) -> Result<HermesPresetOutcome> {
        let info = resolve_profile(&self.root, profile)?;
        if info.is_default() {
            return Err(CoreError::other(
                "default 档案不支持预设启停；请使用预设部署/撤销维护 default",
            ));
        }
        let vault_root = self.vault_root.as_deref().ok_or_else(|| {
            CoreError::other("尚未接入 SkillManagerVault：档案预设需要库内技能内容")
        })?;
        let preset = crate::preset::manage::get(vault_root, Domain::Lobster, preset_id)?;
        let names: Vec<String> = preset.members.iter().map(|m| m.name.clone()).collect();
        let default_dir = self.default_skills_dir();

        let mut installed = 0usize;
        let mut items = Vec::new();

        if enabled {
            let pool = installed_hermes_skills(self.state);
            let to_install: Vec<SkillRef> = names
                .iter()
                .filter(|name| {
                    !(default_dir.join(name).is_dir()
                        && pool.iter().any(|r| names_equal(&r.name, name)))
                })
                .map(|name| SkillRef {
                    domain: Domain::Lobster,
                    name: name.clone(),
                })
                .collect();

            let mut deploy_failures: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            if !to_install.is_empty() {
                let deploy = crate::deploy::apply::DeploymentService::new(
                    vault_root.to_path_buf(),
                    self.resolver,
                    self.state,
                );
                let report = deploy.deploy_to(&to_install, &[Target::HermesDefault])?;
                installed = report.items.len();
                for failure in report.failures {
                    if let Some(skill) = failure.skill {
                        let key = skill
                            .strip_prefix("lobster/")
                            .unwrap_or(&skill)
                            .to_lowercase();
                        deploy_failures.insert(key, failure.message);
                    }
                }
            }

            let library = SkillIndex::build(vault_root)?;
            let disabled = read_disabled(&info.home);
            for name in &names {
                if let Some(msg) = deploy_failures.get(&name.to_lowercase()) {
                    if !default_dir.join(name).is_dir() {
                        items.push(HermesItem {
                            name: name.clone(),
                            ok: false,
                            enabled: false,
                            message: Some(msg.clone()),
                        });
                        continue;
                    }
                }
                let item = match self.set_one(&info, &library, &disabled, &default_dir, name, true)
                {
                    Ok(item) => item,
                    Err(err) => HermesItem {
                        name: name.clone(),
                        ok: false,
                        enabled: false,
                        message: Some(err.to_string()),
                    },
                };
                items.push(item);
            }
        } else {
            let library = SkillIndex::build(vault_root)?;
            let disabled = read_disabled(&info.home);
            for name in &names {
                let in_state = self
                    .state
                    .hermes_enabled(&info.name, Domain::Lobster, name)?;
                let link = info.skills_dir.join(name);
                let state = link_state(&link);
                if !in_state && matches!(state, LinkState::Missing) {
                    continue;
                }
                let reference = SkillRef {
                    domain: Domain::Lobster,
                    name: name.clone(),
                };
                let item = if library.get(&reference).is_some() && default_dir.join(name).is_dir() {
                    match self.set_one(&info, &library, &disabled, &default_dir, name, false) {
                        Ok(item) => item,
                        Err(err) => HermesItem {
                            name: name.clone(),
                            ok: false,
                            enabled: true,
                            message: Some(err.to_string()),
                        },
                    }
                } else {
                    match uninstall_profile_skill(&link, &default_dir.join(name), in_state) {
                        Ok(()) => {
                            self.state
                                .remove_hermes_enable(&info.name, Domain::Lobster, name)?;
                            HermesItem {
                                name: name.clone(),
                                ok: true,
                                enabled: false,
                                message: None,
                            }
                        }
                        Err(err) => HermesItem {
                            name: name.clone(),
                            ok: false,
                            enabled: true,
                            message: Some(err.to_string()),
                        },
                    }
                };
                items.push(item);
            }
        }

        let changed = items.iter().filter(|item| item.ok).count();
        Ok(HermesPresetOutcome {
            profile: info.name,
            preset_id: preset.id,
            preset_name: preset.name,
            enabled,
            installed,
            changed,
            items,
        })
    }
}

/// default 技能目录中实际启用（未被 Hermes 禁用）的技能数。
fn count_default_enabled(root: &Path, disabled: &DisabledState) -> usize {
    scan_skill_dirs(&root.join("skills"))
        .into_iter()
        .filter(|(name, _)| !disabled.contains(name))
        .count()
}

/// 部署目标是否表示「已安装进 Hermes」。
/// Agent `hermes` 与 `hermes:default` 是同一次安装的记录名，不是两套列表。
fn is_hermes_install(target: &str) -> bool {
    is_hermes_pool_target(target)
}

/// 移除本管理器为具名档案安装的启用拷贝（实体目录或遗留符号链接）。
///
/// 启用记录存在则实体目录视为我们的拷贝；无记录的实体目录不删除。
fn uninstall_profile_skill(link: &Path, copy: &Path, in_state: bool) -> Result<()> {
    match link_state(link) {
        LinkState::Missing => Ok(()),
        LinkState::Symlink(_) if in_state || is_our_link(link, copy) => {
            std::fs::remove_file(link).map_err(|err| CoreError::io(link, err))
        }
        LinkState::Symlink(_) => Err(CoreError::other(format!(
            "档案内链接指向其他位置，未改动：{}",
            link.display()
        ))),
        LinkState::Real if in_state => {
            std::fs::remove_dir_all(link).map_err(|err| CoreError::io(link, err))
        }
        LinkState::Real => Err(CoreError::other(format!(
            "档案自有技能目录（非本管理器安装的副本），禁用不删除其内容：{}",
            link.display()
        ))),
    }
}

/// 库内容写入 Hermes default 之后，刷新各具名档案中未漂移的启用拷贝。
///
/// 档案内相对库内容已改动的拷贝默认跳过（保留 curator / 会话改写），`--overwrite-local` 才覆盖。
/// `.usage.json` 不参与漂移判定，覆盖时保留，以便 `hermes curator lock` 继续有效。
pub fn refresh_named_profile_copies(
    state: &StateStore,
    hermes_root: &Path,
    name: &str,
    source: &Path,
    previous_content_hash: Option<&str>,
    overwrite: bool,
) -> Result<Vec<String>> {
    let mut notes = Vec::new();
    if !source.is_dir() {
        return Ok(notes);
    }
    for profile in state.hermes_profiles_enabling(Domain::Lobster, name)? {
        let Ok(info) = resolve_profile(hermes_root, &profile) else {
            continue;
        };
        if info.is_default() {
            continue;
        }
        let dest = info.skills_dir.join(name);
        match link_state(&dest) {
            LinkState::Symlink(_) | LinkState::Missing => {
                replace_dir_from(source, &dest)?;
                continue;
            }
            LinkState::Real => {}
        }
        let dest_hash = hash_dir_excluding_sidecars(&dest)?;
        let source_hash = hash_dir_excluding_sidecars(source)?;
        if dest_hash.as_deref() == source_hash.as_deref() {
            continue;
        }
        let in_sync_with_old = match (dest_hash.as_deref(), previous_content_hash) {
            (Some(current), Some(old)) => current == old,
            _ => false,
        };
        if overwrite || in_sync_with_old {
            replace_dir_from(source, &dest)?;
        } else {
            notes.push(format!(
                "档案 {profile} 的 {name} 副本已相对库内容改动，未覆盖（确认按库覆盖请用 deploy sync --overwrite-local）"
            ));
        }
    }
    Ok(notes)
}

/// 从 Hermes default 卸载某技能时，同时去掉各具名档案中本管理器安装的启用拷贝与名单。
pub fn remove_named_profile_copies(
    state: &StateStore,
    hermes_root: &Path,
    name: &str,
) -> Result<()> {
    let copy = hermes_root.join("skills").join(name);
    for profile in state.hermes_profiles_enabling(Domain::Lobster, name)? {
        let Ok(info) = resolve_profile(hermes_root, &profile) else {
            let _ = state.remove_hermes_enable(&profile, Domain::Lobster, name)?;
            continue;
        };
        if info.is_default() {
            continue;
        }
        let dest = info.skills_dir.join(name);
        uninstall_profile_skill(&dest, &copy, true)?;
        state.remove_hermes_enable(&profile, Domain::Lobster, name)?;
    }
    Ok(())
}

/// 各档案共用的候选池：已从龙虾库安装进 Hermes 的技能。
fn installed_hermes_skills(state: &StateStore) -> Vec<SkillRef> {
    let mut references = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let Ok(records) = state.deployments() else {
        return references;
    };
    for record in records {
        if record.domain != Domain::Lobster || !is_hermes_install(&record.target) {
            continue;
        }
        let reference = record.reference();
        if seen
            .iter()
            .any(|existing| names_equal(existing, &reference.name))
        {
            continue;
        }
        seen.push(reference.name.clone());
        references.push(reference);
    }
    references
}

/// 构造一个基础视图（不含支持收窄结论）。
fn base_view(
    name: String,
    domain: Domain,
    entry: Option<&SkillEntry>,
    path: Option<String>,
    in_default_pool: bool,
    enabled: bool,
    needs_adoption: bool,
) -> ProfileSkillView {
    match entry {
        Some(entry) => ProfileSkillView {
            deployment_blocks: entry.deployment_blocks(),
            name,
            domain,
            library_skill: Some(entry.reference().to_string()),
            in_library: true,
            in_default_pool,
            enabled,
            needs_adoption,
            path,
            comment: entry.comment.clone(),
            category: entry.category.clone(),
            tags: entry.tags.clone(),
            description: entry.description.clone(),
            agents: entry.agents.clone(),
            oses: entry.oses.clone(),
            has_translation: entry.has_translation,
            translation_stale: entry.translation_stale,
            warning: None,
            blocked: None,
            manager_error: entry.manager_error.clone(),
        },
        None => ProfileSkillView {
            deployment_blocks: Default::default(),
            name,
            domain,
            library_skill: None,
            in_library: false,
            in_default_pool,
            enabled,
            needs_adoption,
            path: path.clone(),
            comment: None,
            category: None,
            tags: Vec::new(),
            description: path
                .as_deref()
                .and_then(|value| scan::read_frontmatter(Path::new(value)).description),
            agents: None,
            oses: None,
            has_translation: false,
            translation_stale: false,
            warning: None,
            blocked: None,
            manager_error: None,
        },
    }
}

/// 支持声明结论：已有启用保留并警告；未启用时禁止新增。
fn apply_admission(
    view: &mut ProfileSkillView,
    entry: Option<&SkillEntry>,
    enabled: bool,
    os: &str,
) {
    let Some(entry) = entry else {
        return;
    };
    if let Admission::Unsupported(reason) =
        policy::support_admission(entry, &Target::HermesDefault, os)
    {
        if enabled {
            view.warning = Some(format!("支持范围已收窄，已有启用保留并警告：{reason}"));
        } else {
            view.blocked = Some(format!("禁止新增启用：{reason}"));
        }
    }
}

/// 扫描档案技能目录（递归；命中入口文件即不向下递归；跳过点目录与 `.git`）。
pub(crate) fn scan_skill_dirs(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    collect(dir, &mut out);
    out
}

fn collect(dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name == ".git" {
            continue;
        }
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if scan::is_skill_dir(&path) {
            out.push((name, path));
            continue;
        }
        collect(&path, out);
    }
}

// ---------------------------------------------------------------------------
// 组合筛选（沿用库内分类联动标签规则）
// ---------------------------------------------------------------------------

fn matches_query(view: &ProfileSkillView, query: &SearchQuery) -> bool {
    match &query.category {
        CategoryFilter::All => {}
        CategoryFilter::Uncategorized => {
            if view.category.is_some() {
                return false;
            }
        }
        CategoryFilter::Named(name) => {
            if view.category.as_deref() != Some(name.as_str()) {
                return false;
            }
        }
    }

    if let Some(needle) = query
        .query
        .as_ref()
        .map(|text| text.trim().to_lowercase())
        .filter(|text| !text.is_empty())
    {
        let mut haystack = view.name.clone();
        for extra in [
            view.description.as_deref(),
            view.comment.as_deref(),
            view.category.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            haystack.push('\n');
            haystack.push_str(extra);
        }
        for tag in &view.tags {
            haystack.push('\n');
            haystack.push_str(tag);
        }
        if !haystack.to_lowercase().contains(&needle) {
            return false;
        }
    }

    if query.require_untagged {
        return view.tags.is_empty();
    }
    query.tags.is_empty()
        || query
            .tags
            .iter()
            .all(|tag| view.tags.iter().any(|item| item == tag))
}

/// 分类联动的标签候选：只含所选分类内实际出现的标签。
fn tag_candidates(views: &[ProfileSkillView], category: &CategoryFilter) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for view in views {
        let matches = match category {
            CategoryFilter::All => true,
            CategoryFilter::Uncategorized => view.category.is_none(),
            CategoryFilter::Named(name) => view.category.as_deref() == Some(name.as_str()),
        };
        if !matches {
            continue;
        }
        for tag in &view.tags {
            if !tags.iter().any(|existing| existing == tag) {
                tags.push(tag.clone());
            }
        }
    }
    tags.sort();
    tags
}

fn categories(views: &[ProfileSkillView]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for view in views {
        if let Some(category) = &view.category {
            if !out.iter().any(|existing| existing == category) {
                out.push(category.clone());
            }
        }
    }
    out.sort();
    out
}

/// 供 CLI/Tauri 表示：某 Agent 维度键（档案技能沿用 hermes）。
pub fn hermes_agent_key() -> &'static str {
    HERMES_AGENT_KEY
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::StateStore;

    #[test]
    fn link_state_and_our_link_detection() {
        let dir = tempfile::TempDir::new().unwrap();
        let copy = dir.path().join("copy");
        std::fs::create_dir_all(&copy).unwrap();
        let link = dir.path().join("link");

        assert_eq!(link_state(&link), LinkState::Missing);
        create_dir_symlink(&copy, &link).unwrap();
        assert!(is_our_link(&link, &copy));
        assert!(!is_our_link(&link, &dir.path().join("other")));

        let real = dir.path().join("real");
        std::fs::create_dir_all(&real).unwrap();
        assert_eq!(link_state(&real), LinkState::Real);
    }

    #[test]
    fn query_matching_and_tag_linkage() {
        let view = ProfileSkillView {
            deployment_blocks: Default::default(),
            name: "alpha".to_string(),
            domain: Domain::Lobster,
            library_skill: Some("lobster/alpha".to_string()),
            in_library: true,
            in_default_pool: true,
            enabled: true,
            needs_adoption: false,
            path: None,
            comment: Some("服务器管理".to_string()),
            category: Some("运维".to_string()),
            tags: vec!["ops".to_string(), "net".to_string()],
            description: Some("管理服务器".to_string()),
            agents: None,
            oses: None,
            has_translation: false,
            translation_stale: false,
            warning: None,
            blocked: None,
            manager_error: None,
        };
        assert!(matches_query(&view, &SearchQuery::default()));
        assert!(matches_query(
            &view,
            &SearchQuery {
                query: Some("服务器".to_string()),
                ..Default::default()
            }
        ));
        assert!(!matches_query(
            &view,
            &SearchQuery {
                category: CategoryFilter::Named("其他".to_string()),
                ..Default::default()
            }
        ));
        assert!(matches_query(
            &view,
            &SearchQuery {
                tags: vec!["ops".to_string()],
                ..Default::default()
            }
        ));
        assert_eq!(
            tag_candidates(&[view], &CategoryFilter::Named("运维".to_string())),
            vec!["net".to_string(), "ops".to_string()]
        );
        let _ = installed_hermes_skills(&StateStore::open_in_memory().unwrap());
    }

    #[test]
    fn view_serializes_empty_tags_as_array() {
        // 客户端按数组处理 tags；空集合也必须出现，不能因缺字段崩溃（真实 webview 曾因此空白）。
        let view = ProfileSkillView {
            deployment_blocks: Default::default(),
            name: "alpha".to_string(),
            domain: Domain::Lobster,
            library_skill: None,
            in_library: false,
            in_default_pool: false,
            enabled: false,
            needs_adoption: true,
            path: Some("/tmp/alpha".to_string()),
            comment: None,
            category: None,
            tags: Vec::new(),
            description: None,
            agents: None,
            oses: None,
            has_translation: false,
            translation_stale: false,
            warning: None,
            blocked: None,
            manager_error: None,
        };
        let value = serde_json::to_value(&view).unwrap();
        assert_eq!(value["tags"], serde_json::json!([]), "{value}");
    }

    #[test]
    fn set_preset_enabled_rejects_default_and_missing_vault() {
        let home = tempfile::TempDir::new().unwrap();
        let root = home.path().join(".hermes");
        std::fs::create_dir_all(root.join("skills")).unwrap();
        std::fs::create_dir_all(root.join("profiles/blank")).unwrap();
        std::fs::write(root.join("profiles/blank/config.yaml"), "model: test\n").unwrap();

        let state = StateStore::open_in_memory().unwrap();
        let catalog = crate::deploy::targets::AgentCatalog::embedded();
        let resolver = TargetResolver::new(home.path(), catalog);
        let service = HermesService::new(root, None, &resolver, &state);

        let default_err = service
            .set_preset_enabled("default", "ops", true)
            .unwrap_err();
        assert!(default_err.to_string().contains("default"), "{default_err}");

        let vault_err = service
            .set_preset_enabled("blank", "ops", true)
            .unwrap_err();
        assert!(
            vault_err.to_string().contains("SkillManagerVault"),
            "{vault_err}"
        );
    }
}
