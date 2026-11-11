//! Hermes 档案命令：档案发现、档案技能列表、明确启停与收编。
//!
//! 薄封装 core：协议与业务规则都在 `skill_manager_core::hermes`；前端只调用这里。

use serde::Serialize;
use skill_manager_core::hermes::{AdoptItem, HermesItem, HermesService, ProfileSkillsView, ProfileView};
use skill_manager_core::vault::index::{CategoryFilter, SearchQuery};

use crate::HostRuntime;

/// 本机 Hermes 根与档案列表。
#[derive(Clone, Debug, Serialize)]
pub struct HermesProfilesView {
    pub root: String,
    pub profiles: Vec<ProfileView>,
}

/// 启用/禁用结果。
#[derive(Clone, Debug, Serialize)]
pub struct HermesToggleView {
    pub profile: String,
    pub enabled: bool,
    pub items: Vec<HermesItem>,
    pub changed: usize,
}

/// 收编结果。
#[derive(Clone, Debug, Serialize)]
pub struct HermesAdoptView {
    pub profile: String,
    pub items: Vec<AdoptItem>,
    pub adopted: usize,
}

/// 组合筛选条件（分类联动规则由 core 决定）。
pub struct HermesQuery {
    pub query: Option<String>,
    pub category: Option<String>,
    pub uncategorized: bool,
    pub tags: Vec<String>,
    pub no_tag: bool,
}

impl HermesQuery {
    fn into_search(self) -> SearchQuery {
        let category = if self.uncategorized {
            CategoryFilter::Uncategorized
        } else {
            match self.category {
                Some(name) => CategoryFilter::Named(name),
                None => CategoryFilter::All,
            }
        };
        SearchQuery {
            domain: None,
            query: self.query,
            category,
            tags: self.tags,
            require_untagged: self.no_tag,
            agent: None,
            preset: None,
        }
    }
}

fn run_with<T>(
    runtime: &HostRuntime,
    run: impl FnOnce(&HermesService<'_>) -> Result<T, String>,
) -> Result<T, String> {
    let ctx = runtime.get()?;
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = HermesService::from_context(&ctx, &resolver).map_err(|err| err.to_string())?;
    run(&service)
}

pub fn hermes_read(runtime: &HostRuntime, profile: String, name: String, file: Option<String>) -> Result<skill_manager_core::deploy::local_skills::LocalSkillRead, String> {
    run_with(runtime, |service| service.read(&profile, &name, file.as_deref()).map_err(|err| err.to_string()))
}

/// 本机现有档案（default 与具名档案）。
pub fn hermes_profiles(runtime: &HostRuntime) -> Result<HermesProfilesView, String> {
    run_with(runtime, |service| {
        let profiles = service.profiles().map_err(|err| err.to_string())?;
        Ok(HermesProfilesView {
            root: service.root().display().to_string(),
            profiles,
        })
    })
}

/// 列出某档案的技能（组合筛选）。
pub fn hermes_list(
    runtime: &HostRuntime,
    profile: String,
    query: Option<String>,
    category: Option<String>,
    uncategorized: Option<bool>,
    tags: Option<Vec<String>>,
    no_tag: Option<bool>,
) -> Result<ProfileSkillsView, String> {
    let query = HermesQuery {
        query,
        category,
        uncategorized: uncategorized.unwrap_or(false),
        tags: tags.unwrap_or_default(),
        no_tag: no_tag.unwrap_or(false),
    };
    run_with(runtime, |service| {
        service
            .list(&profile, &query.into_search())
            .map_err(|err| err.to_string())
    })
}

/// 为某具名档案明确启用技能。
pub fn hermes_enable(
    runtime: &HostRuntime,
    profile: String,
    names: Vec<String>,
) -> Result<HermesToggleView, String> {
    toggle(runtime, profile, names, true)
}

/// 为某具名档案禁用技能。
pub fn hermes_disable(
    runtime: &HostRuntime,
    profile: String,
    names: Vec<String>,
) -> Result<HermesToggleView, String> {
    toggle(runtime, profile, names, false)
}

fn toggle(
    runtime: &HostRuntime,
    profile: String,
    names: Vec<String>,
    enabled: bool,
) -> Result<HermesToggleView, String> {
    run_with(runtime, |service| {
        let items = service
            .set_enabled(&profile, &names, enabled)
            .map_err(|err| err.to_string())?;
        let changed = items.iter().filter(|item| item.ok).count();
        Ok(HermesToggleView {
            profile,
            enabled,
            items,
            changed,
        })
    })
}

/// 收编档案独有技能到龙虾库并部署 default 档案。
pub fn hermes_adopt(
    runtime: &HostRuntime,
    profile: String,
    names: Vec<String>,
) -> Result<HermesAdoptView, String> {
    run_with(runtime, |service| {
        let items = service
            .adopt(&profile, &names)
            .map_err(|err| err.to_string())?;
        let adopted = items.iter().filter(|item| item.ok).count();
        Ok(HermesAdoptView {
            profile,
            items,
            adopted,
        })
    })
}
