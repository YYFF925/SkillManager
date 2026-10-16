//! 文件监听：库内外部改动与目录移动后刷新内存索引并通知调用方。
//!
//! 共享文件的所有写入者（GUI、CLI、外部 Agent）地位平等：监听只刷新索引，
//! **不自动提交、不推送、不做首次部署**（FR-005、宪章 V）。
//! `.git/` 子树的事件被忽略，避免自身仓库操作造成的回环。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use notify::{Event, EventKind, RecursiveMode, Watcher};

use crate::error::{CoreError, Result};
use crate::vault::index::SkillIndex;

/// 正在运行的库监听器；丢弃即停止监听。
pub struct VaultWatcher {
    _watcher: notify::RecommendedWatcher,
    index: Arc<Mutex<SkillIndex>>,
    root: PathBuf,
}

impl VaultWatcher {
    /// 递归监听 SkillManagerVault；每次相关事件后重建索引并调用 `on_change`。
    ///
    /// 回调运行在 notify 的事件线程上，应尽快返回（GUI 侧仅转发事件）。
    pub fn start<F>(root: impl AsRef<Path>, on_change: F) -> Result<Self>
    where
        F: Fn(&SkillIndex) + Send + 'static,
    {
        let root = root.as_ref().to_path_buf();
        let index = Arc::new(Mutex::new(SkillIndex::build(&root)?));

        let shared = Arc::clone(&index);
        let watch_root = root.clone();
        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<Event>| {
                let Ok(event) = result else {
                    return;
                };
                if matches!(event.kind, EventKind::Access(_)) {
                    return;
                }
                if all_ignored(&watch_root, &event.paths) {
                    return;
                }
                let Ok(mut guard) = shared.lock() else {
                    return;
                };
                if guard.refresh().is_err() {
                    // 刷新失败时保留上一份索引；下一次事件会再尝试。
                    return;
                }
                on_change(&guard);
            })
            .map_err(|err| CoreError::other(format!("启动文件监听失败：{err}")))?;

        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|err| CoreError::other(format!("监听 {} 失败：{err}", root.display())))?;

        Ok(Self {
            _watcher: watcher,
            index,
            root,
        })
    }

    /// 共享索引；GUI 读取当前状态。
    pub fn index(&self) -> Arc<Mutex<SkillIndex>> {
        Arc::clone(&self.index)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 手动刷新（例如启动时或事件丢失后的兜底）。
    pub fn refresh(&self) -> Result<()> {
        let mut guard = self
            .index
            .lock()
            .map_err(|_| CoreError::other("索引锁已损坏"))?;
        guard.refresh()
    }
}

/// 事件路径是否全部可忽略（`.git` 子树或库根之外）。
fn all_ignored(root: &Path, paths: &[PathBuf]) -> bool {
    paths.is_empty() || paths.iter().all(|path| is_ignored(root, path))
}

fn is_ignored(root: &Path, path: &Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .any(|component| component.as_os_str() == ".git")
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;
    use crate::domain::SkillRef;

    fn write_skill(root: &Path, rel: &str) {
        let dir = root.join(rel);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), "正文").unwrap();
    }

    #[test]
    fn external_edits_refresh_the_index_and_notify() {
        let root = tempfile::TempDir::new().unwrap();
        write_skill(root.path(), "code/alpha");

        let (tx, rx) = mpsc::channel();
        let watcher = VaultWatcher::start(root.path(), move |_| {
            let _ = tx.send(());
        })
        .unwrap();

        write_skill(root.path(), "code/beta");
        rx.recv_timeout(Duration::from_secs(15))
            .expect("文件监听未在超时内通知");

        let index = watcher.index();
        let guard = index.lock().unwrap();
        assert!(
            guard.get(&SkillRef::parse("code/beta").unwrap()).is_some(),
            "刷新后的索引应包含新技能"
        );
        drop(guard);

        // 手工刷新兜底可用。
        watcher.refresh().unwrap();
    }

    #[test]
    fn git_subtree_events_are_ignored() {
        let root = tempfile::TempDir::new().unwrap();
        write_skill(root.path(), "code/alpha");
        std::fs::create_dir_all(root.path().join(".git/objects")).unwrap();

        let (tx, rx) = mpsc::channel();
        let _watcher = VaultWatcher::start(root.path(), move |_| {
            let _ = tx.send(());
        })
        .unwrap();

        // `.git` 内的事件不触发回调。
        std::fs::write(root.path().join(".git/objects/tmp"), "x").unwrap();
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
    }
}
