//! SkillManagerVault 接入命令。

use skill_manager_core::vault::VaultInfo;

use crate::HostRuntime;

/// 接入用户选择的本地 SkillManagerVault。
///
/// 首次接入要求所选目录已存在、是 Git 仓库且已配置远端（FR-002）；
/// 不提供克隆或创建向导。接入成功后开始监听库内外部改动。
pub fn vault_open(
    runtime: &HostRuntime,
    path: String,
) -> Result<VaultInfo, String> {
    let info = {
        let mut ctx = runtime.get()?;
        ctx.attach_vault(&path).map_err(|err| err.to_string())?;
        let vault = ctx.vault().map_err(|err| err.to_string())?;
        vault.validate_intake().map_err(|err| err.to_string())?
    };
    runtime.watch_vault()?;
    Ok(info)
}

/// 当前的 SkillManagerVault 信息；尚未接入时为 null。
pub fn vault_current(runtime: &HostRuntime) -> Result<Option<VaultInfo>, String> {
    let ctx = runtime.get()?;
    Ok(ctx.vault_opt().map(|vault| vault.info()))
}
