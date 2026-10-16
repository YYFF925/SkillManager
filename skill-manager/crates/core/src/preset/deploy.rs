//! 预设部署与撤销（T064，FR-024）。
//!
//! 语义与其他部署完全一致：**叠加**部署（不清理目标已有其他技能）、**按成员**撤销
//! （成员即使同时属于其他预设也照撤，不追踪来源预设或引用计数）、逐项复用 US3 的
//! 域边界/支持声明过滤与暂停规则（含 dry-run）。成员编辑不在此触发任何部署。

use crate::deploy::apply::{DeployReport, DeploymentService};
use crate::domain::Target;
use crate::error::Result;
use crate::preset::manage;

impl DeploymentService<'_> {
    /// `deploy preset <domain> <preset-id> --to <target>`：按成员叠加部署到各目标。
    ///
    /// 缺失成员按“技能不存在”逐项报告，其余成员继续。
    pub fn deploy_preset(
        &self,
        domain: crate::domain::Domain,
        preset_id: &str,
        targets: &[Target],
        dry_run: bool,
    ) -> Result<DeployReport> {
        let names = manage::member_names(&self.vault_root, domain, preset_id)?;
        let references: Vec<crate::domain::SkillRef> = names
            .iter()
            .map(|name| crate::domain::SkillRef {
                domain,
                name: name.clone(),
            })
            .collect();
        self.deploy_to(&references, targets, dry_run)
    }

    /// `deploy undo-preset <domain> <preset-id> --from <target>`：按当前成员逐个撤销。
    ///
    /// 与普通撤销同一规则：保留库内技能；外改副本暂停并如实报告（此处不提供丢弃参数）。
    pub fn undo_preset(
        &self,
        domain: crate::domain::Domain,
        preset_id: &str,
        targets: &[Target],
        dry_run: bool,
    ) -> Result<DeployReport> {
        let names = manage::member_names(&self.vault_root, domain, preset_id)?;
        let references: Vec<crate::domain::SkillRef> = names
            .iter()
            .map(|name| crate::domain::SkillRef {
                domain,
                name: name.clone(),
            })
            .collect();
        self.undo(&references, targets, dry_run, false)
    }
}
