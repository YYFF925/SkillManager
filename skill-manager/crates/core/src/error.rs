//! 统一错误类型。错误消息面向用户/Agent，需能定位到具体对象与路径。

use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("路径不存在：{0}")]
    PathMissing(PathBuf),

    #[error("不是目录：{0}")]
    NotADirectory(PathBuf),

    #[error("不是 Git 仓库：{0}")]
    NotAGitRepo(PathBuf),

    #[error("SkillVault 未配置远端：{0}")]
    NoRemote(PathBuf),

    #[error("技能名不合法（{name}）：{reason}")]
    InvalidSkillName { name: String, reason: String },

    #[error("分类名不合法（{name}）：{reason}")]
    InvalidCategory { name: String, reason: String },

    #[error("同一管理域已存在同名技能：{domain}/{name}")]
    DuplicateSkill { domain: String, name: String },

    #[error("技能不存在：{domain}/{name}")]
    SkillNotFound { domain: String, name: String },

    #[error("引用无法解析：{0}")]
    InvalidRef(String),

    #[error("本次获取已取消：{0}")]
    Cancelled(String),

    #[error("目标无法解析：{0}")]
    InvalidTarget(String),

    #[error("git 命令失败：git {args}（{cwd}）：{stderr}")]
    Git {
        args: String,
        cwd: PathBuf,
        stderr: String,
    },

    #[error("本机状态数据库错误：{0}")]
    State(String),

    #[error("读写失败（{path}）：{source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("YAML 解析失败（{path}）：{message}")]
    Yaml { path: PathBuf, message: String },

    #[error("{0}")]
    Other(String),
}

impl CoreError {
    /// 读取文件失败，带路径。
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        CoreError::Io {
            path: path.into(),
            source,
        }
    }

    pub fn other(message: impl Into<String>) -> Self {
        CoreError::Other(message.into())
    }

    /// 供结构化输出使用的技能/目标/路径定位信息。
    pub fn path_hint(&self) -> Option<String> {
        match self {
            CoreError::PathMissing(p)
            | CoreError::NotADirectory(p)
            | CoreError::NotAGitRepo(p)
            | CoreError::NoRemote(p)
            | CoreError::Io { path: p, .. }
            | CoreError::Yaml { path: p, .. }
            | CoreError::Git { cwd: p, .. } => Some(p.display().to_string()),
            _ => None,
        }
    }

    pub fn skill_hint(&self) -> Option<String> {
        match self {
            CoreError::DuplicateSkill { domain, name }
            | CoreError::SkillNotFound { domain, name } => Some(format!("{domain}/{name}")),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for CoreError {
    fn from(value: rusqlite::Error) -> Self {
        CoreError::State(value.to_string())
    }
}
