//! Agent 指令源与分发（T069，data-model §1.1/§2.3、FR-043）。
//!
//! 源内容保存在 SkillManagerVault 的 `instructions/`（`global.md` + `<agent-key>.md`），随库同步；
//! 分发目标为本机已启用且已检测的 Agent 的 `AGENTS.md`（可被 `agents.instructions_target`
//! 覆盖），分发范围、启用名单与路径都是本机状态，不随库同步。
//!
//! 裁剪后的四条拼接规则（content-composition.md）：全局在前、专属在后、一个空行分隔；
//! 单侧为空返回另一侧；双空返回空串。真正的读取失败要与“文件不存在”区分，不能用空内容
//! 覆盖已有目标。

use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::deploy::agent_config::AgentConfigService;
use crate::deploy::targets::AgentCatalog;
use crate::error::{CoreError, Result};
use crate::state::StateStore;

/// 指令源目录（相对 SkillManagerVault 根）。
pub const INSTRUCTIONS_DIR: &str = "instructions";
/// 全局指令源文件名。
pub const GLOBAL_FILE: &str = "global.md";

/// 一次编辑/读取的作用域。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstructionScope {
    Global,
    Agent(String),
}

impl InstructionScope {
    /// 解析 `global` 或 `<agent-key>`。
    pub fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(CoreError::other("指令作用域不能为空（global 或 Agent key）"));
        }
        if trimmed == "global" {
            Ok(InstructionScope::Global)
        } else {
            Ok(InstructionScope::Agent(trimmed.to_string()))
        }
    }

    /// 对外的作用域字符串。
    pub fn as_str(&self) -> &str {
        match self {
            InstructionScope::Global => "global",
            InstructionScope::Agent(key) => key,
        }
    }

    /// 源文件名。
    pub fn file_name(&self) -> String {
        match self {
            InstructionScope::Global => GLOBAL_FILE.to_string(),
            InstructionScope::Agent(key) => format!("{key}.md"),
        }
    }
}

/// 按拼接规则合成全局与专属内容（均已裁剪）。
pub fn compose_instructions(global: &str, agent: &str) -> String {
    let global = global.trim();
    let agent = agent.trim();
    match (global.is_empty(), agent.is_empty()) {
        (true, true) => String::new(),
        (false, true) => global.to_string(),
        (true, false) => agent.to_string(),
        (false, false) => format!("{global}\n\n{agent}"),
    }
}

/// 一个 Agent 目标的分发结果。
#[derive(Clone, Debug, Serialize)]
pub struct InstructionTargetResult {
    pub agent: String,
    pub path: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 一次源读取结果。
#[derive(Clone, Debug, Serialize)]
pub struct InstructionSourceView {
    pub scope: String,
    /// 该作用域的源内容；文件不存在按空内容。
    pub source: String,
    /// 按拼接规则得到的合成预览。
    pub composed: String,
}

/// 指令源与分发服务。
pub struct InstructionsService<'a> {
    vault_root: PathBuf,
    home: PathBuf,
    catalog: &'static AgentCatalog,
    state: &'a StateStore,
}

impl<'a> InstructionsService<'a> {
    pub fn new(
        vault_root: PathBuf,
        home: PathBuf,
        catalog: &'static AgentCatalog,
        state: &'a StateStore,
    ) -> Self {
        Self {
            vault_root,
            home,
            catalog,
            state,
        }
    }

    /// 指令源目录。
    pub fn sources_dir(&self) -> PathBuf {
        self.vault_root.join(INSTRUCTIONS_DIR)
    }

    /// 源文件路径。
    fn source_path(&self, scope: &InstructionScope) -> PathBuf {
        self.sources_dir().join(scope.file_name())
    }

    /// 读取源内容：文件不存在返回空串；其它读取失败如实报错（不与“不存在”混同）。
    pub fn read_source(&self, scope: &InstructionScope) -> Result<String> {
        read_source_file(&self.source_path(scope))
    }

    /// 读取作用域源内容与合成预览。
    pub fn get(&self, scope: &InstructionScope) -> Result<InstructionSourceView> {
        let source = self.read_source(scope)?;
        let composed = match scope {
            InstructionScope::Global => source.trim().to_string(),
            InstructionScope::Agent(_) => {
                let global = self.read_source(&InstructionScope::Global)?;
                compose_instructions(&global, &source)
            }
        };
        Ok(InstructionSourceView {
            scope: scope.as_str().to_string(),
            source,
            composed,
        })
    }

    /// 写入源内容（空内容也写入空文件），再按拼接规则覆盖本机已启用目标。
    ///
    /// 返回每个目标的结果；单目标失败不中断其余。
    pub fn save(&self, scope: &InstructionScope, content: &str) -> Result<Vec<InstructionTargetResult>> {
        let dir = self.sources_dir();
        std::fs::create_dir_all(&dir).map_err(|err| CoreError::io(dir.clone(), err))?;
        let path = self.source_path(scope);
        std::fs::write(&path, content).map_err(|err| CoreError::io(path, err))?;
        self.distribute()
    }

    /// 按当前源内容刷新本机已启用目标（库拉取到指令源变化后调用）。
    ///
    /// 不新增启用目标、不改变本机路径。
    pub fn distribute(&self) -> Result<Vec<InstructionTargetResult>> {
        let config = AgentConfigService::new(self.home.clone(), self.catalog, self.state);
        let views = config.list()?;
        let mut results = Vec::new();
        for view in views {
            let has_override = self
                .state
                .agent(&view.key)?
                .and_then(|agent| agent.instructions_target)
                .is_some();
            // 分发范围：已启用且（已检测 或 显式设置了指令路径）。
            // 无覆盖要求的“已检测”条件避免在未安装 Agent 的默认目录下创建 53 个 AGENTS.md；
            // 显式设置路径说明用户明确要求写该目标，即使检测目录不存在也应生效。
            if !view.enabled || !(view.detected || has_override) {
                continue;
            }
            let path = PathBuf::from(&view.instructions_target);
            let result = match self.compose_for(&view.key) {
                Ok(text) => match write_target(&path, &text) {
                    Ok(()) => InstructionTargetResult {
                        agent: view.key.clone(),
                        path: path.display().to_string(),
                        ok: true,
                        error: None,
                    },
                    Err(err) => InstructionTargetResult {
                        agent: view.key.clone(),
                        path: path.display().to_string(),
                        ok: false,
                        error: Some(err.to_string()),
                    },
                },
                Err(err) => InstructionTargetResult {
                    agent: view.key.clone(),
                    path: path.display().to_string(),
                    ok: false,
                    error: Some(err.to_string()),
                },
            };
            results.push(result);
        }
        Ok(results)
    }

    /// 某 Agent 目标的合成内容（全局 + 专属）。
    pub fn compose_for(&self, agent_key: &str) -> Result<String> {
        let global = self.read_source(&InstructionScope::Global)?;
        let agent = self.read_source(&InstructionScope::Agent(agent_key.to_string()))?;
        Ok(compose_instructions(&global, &agent))
    }

    /// 指令源整体签名（用于判断库拉取后源是否变化）。
    pub fn signature(&self) -> Result<String> {
        source_signature(&self.sources_dir())
    }
}

/// 读取源文件；不存在返回空串，其它失败如实报错。
fn read_source_file(path: &Path) -> Result<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(CoreError::io(path.to_path_buf(), err)),
    }
}

/// 覆盖写入目标 `AGENTS.md`（父目录不存在则创建）。
fn write_target(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| CoreError::io(parent.to_path_buf(), err))?;
    }
    std::fs::write(path, content).map_err(|err| CoreError::io(path.to_path_buf(), err))
}

/// 指令源目录整体签名：按文件名词典序连接“文件名 + 内容”后取 SHA-256。
///
/// 目录不存在视为空目录签名；单个文件读取失败如实报错，不折叠为空。
pub fn source_signature(dir: &Path) -> Result<String> {
    let mut entries: Vec<(String, String)> = Vec::new();
    match std::fs::read_dir(dir) {
        Ok(iter) => {
            for entry in iter {
                let entry = entry.map_err(|err| CoreError::io(dir.to_path_buf(), err))?;
                let file_name = entry.file_name().to_string_lossy().into_owned();
                if !file_name.ends_with(".md") {
                    continue;
                }
                let content = read_source_file(&entry.path())?;
                entries.push((file_name, content));
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(CoreError::io(dir.to_path_buf(), err)),
    }
    entries.sort();
    let mut hasher = Sha256::new();
    for (name, content) in entries {
        hasher.update(name.as_bytes());
        hasher.update([0u8]);
        hasher.update(content.as_bytes());
        hasher.update([0u8]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_covers_four_states() {
        assert_eq!(compose_instructions("", ""), "");
        assert_eq!(compose_instructions("  全局  ", ""), "全局");
        assert_eq!(compose_instructions("", "专属"), "专属");
        assert_eq!(compose_instructions("全局", "专属"), "全局\n\n专属");
    }

    #[test]
    fn source_scope_file_names() {
        assert_eq!(InstructionScope::parse("global").unwrap().file_name(), "global.md");
        assert_eq!(
            InstructionScope::parse("codex").unwrap().file_name(),
            "codex.md"
        );
        assert!(InstructionScope::parse("  ").is_err());
    }

    #[test]
    fn missing_source_is_empty_but_read_failure_is_reported() {
        let root = tempfile::TempDir::new().unwrap();
        let dir = root.path().join(INSTRUCTIONS_DIR);
        std::fs::create_dir_all(&dir).unwrap();

        // 文件不存在 = 空内容。
        assert_eq!(read_source_file(&dir.join(GLOBAL_FILE)).unwrap(), "");

        // 同名但为目录 = 真正的读取失败，不能被折叠为空内容。
        std::fs::create_dir_all(dir.join(GLOBAL_FILE)).unwrap();
        assert!(read_source_file(&dir.join(GLOBAL_FILE)).is_err());
    }
}
