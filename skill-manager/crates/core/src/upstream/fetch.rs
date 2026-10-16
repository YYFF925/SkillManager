//! 来源仓库获取与缓存（T030/T032）。
//!
//! 设计结论见 `specs/002-skill-management/research.md` D19：
//! - 缓存目录 `<state-dir>/upstream-cache/<repo-key 哈希>/`，只做浅克隆（`--depth 1`）；
//!   已存在且可用时复用（fetch + reset），失败或取消留下的半成品下次会先清理再重克隆。
//! - 通过本机 Git 的 `--progress` 流式读取**真实进度**（阶段 + 可得的进展量）；
//!   进度回调与取消共用同一子进程，取消即终止本次获取并如实返回 `Cancelled`。
//! - 只承诺“可复用缓存”，不承诺清除所有临时字节（取消后残留按边界说明处理）。
//!
//! 该模块不把获取结果登记为已入库：登记只发生在 `vault/acquire.rs`。

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use sha2::{Digest, Sha256};

use crate::error::{CoreError, Result};
use crate::manager::SourceRecord;
use crate::sync::{Git, GitProgress};

/// 缓存根目录名（位于本机状态目录下，随 `--state-dir` 隔离）。
pub const CACHE_DIR: &str = "upstream-cache";

/// 仓库地址的稳定缓存键：按去重后的地址做哈希。
pub fn cache_key(repo: &str) -> String {
    let normalized = SourceRecord::repo_key(repo);
    let digest = format!("{:x}", Sha256::digest(normalized.as_bytes()));
    digest[..16].to_string()
}

/// 某仓库在本机状态目录下的缓存目录。
pub fn cache_dir(state_dir: &Path, repo: &str) -> PathBuf {
    state_dir.join(CACHE_DIR).join(cache_key(repo))
}

/// 一次获取的结果。
#[derive(Clone, Debug)]
pub struct FetchedRepo {
    /// 检出目录（缓存目录本身）。
    pub path: PathBuf,
    /// 本次解析到的完整提交 OID。
    pub oid: String,
    /// 描述本次是否复用了已有缓存。
    pub reused: bool,
}

/// 获取并检出指定提交；缓存 HEAD 不是该提交时单独获取。
///
/// 远端若不支持按 OID 浅获取（`allowReachableSHA1InWant` 关闭等），如实报错，
/// 不静默改用另一个版本。
pub fn fetch_oid(
    state_dir: &Path,
    repo: &str,
    reference: Option<&str>,
    oid: &str,
    on_progress: &mut dyn FnMut(&GitProgress),
    cancel: &AtomicBool,
) -> Result<FetchedRepo> {
    let fetched = fetch_repo(state_dir, repo, reference, on_progress, cancel)?;
    if fetched.oid == oid {
        return Ok(fetched);
    }
    let dir = fetched.path;
    let git = Git::new(&dir);
    let mut line_cb = |line: &str| on_progress(&GitProgress::parse(line));
    let output = git.run_streaming(
        &["fetch", "--progress", "--depth", "1", "origin", oid],
        &mut line_cb,
        cancel,
    )?;
    if !output.success() {
        return Err(CoreError::Git {
            args: format!("fetch --depth 1 origin {oid}"),
            cwd: dir.clone(),
            stderr: output.stderr.trim().to_string(),
        });
    }
    git.ok(&["reset", "--hard", "FETCH_HEAD"])?;
    let head = git.rev_parse("HEAD")?;
    if head != oid {
        return Err(CoreError::other(format!(
            "获取提交 {oid} 后 HEAD 为 {head}，未采用静默替换"
        )));
    }
    Ok(FetchedRepo {
        path: dir,
        oid: oid.to_string(),
        reused: true,
    })
}

/// 解析引用为完整提交 OID（`git ls-remote`，不克隆）。
///
/// `reference` 为 `None` 时解析远端默认分支（`HEAD`）。标签的 peeled 提交优先。
pub fn resolve_remote_oid(cwd: &Path, repo: &str, reference: Option<&str>) -> Result<String> {
    let git = Git::new(cwd);
    let pattern = reference.map(str::to_string).unwrap_or_else(|| "HEAD".to_string());
    let output = git.run(&["ls-remote", repo, &pattern])?;
    if !output.success() {
        return Err(CoreError::Git {
            args: format!("ls-remote {repo} {pattern}"),
            cwd: cwd.to_path_buf(),
            stderr: output.stderr.trim().to_string(),
        });
    }
    let mut first: Option<String> = None;
    let mut peeled: Option<String> = None;
    for line in output.stdout.lines() {
        let mut fields = line.split_whitespace();
        let Some(oid) = fields.next() else { continue };
        let Some(name) = fields.next() else { continue };
        if oid.len() != 40 {
            continue;
        }
        if name.ends_with("^{}") {
            peeled = Some(oid.to_string());
        } else if first.is_none() {
            first = Some(oid.to_string());
        }
    }
    peeled
        .or(first)
        .ok_or_else(|| CoreError::Other(format!("远端没有引用 {pattern}（{repo}）")))
}

fn is_usable_cache(dir: &Path) -> bool {
    if !dir.join(".git").exists() {
        return false;
    }
    Git::new(dir)
        .run(&["rev-parse", "HEAD"])
        .map(|out| out.success())
        .unwrap_or(false)
}

/// 获取来源仓库并返回检出目录与解析出的完整提交 OID。
///
/// 复用已有缓存；半成品缓存先清理再重克隆。取消时终止子进程并返回 `Cancelled`。
pub fn fetch_repo(
    state_dir: &Path,
    repo: &str,
    reference: Option<&str>,
    on_progress: &mut dyn FnMut(&GitProgress),
    cancel: &AtomicBool,
) -> Result<FetchedRepo> {
    std::fs::create_dir_all(state_dir).map_err(|err| CoreError::io(state_dir.to_path_buf(), err))?;
    let dir = cache_dir(state_dir, repo);

    let mut line_cb = |line: &str| on_progress(&GitProgress::parse(line));

    let reused = if is_usable_cache(&dir) {
        let git = Git::new(&dir);
        let mut args = vec!["fetch", "--progress", "--depth", "1", "origin"];
        let pattern = reference.unwrap_or("HEAD");
        args.push(pattern);
        let output = git.run_streaming(&args, &mut line_cb, cancel)?;
        if !output.success() {
            return Err(CoreError::Git {
                args: format!("fetch --depth 1 origin {pattern}"),
                cwd: dir.clone(),
                stderr: output.stderr.trim().to_string(),
            });
        }
        git.ok(&["reset", "--hard", "FETCH_HEAD"])?;
        true
    } else {
        if dir.exists() {
            // 上次获取被取消或失败留下的半成品：先清理再重克隆。
            crate::deploy::compose::remove_path(&dir)?;
        }
        let parent = dir
            .parent()
            .ok_or_else(|| CoreError::other("缓存目录没有父目录"))?;
        std::fs::create_dir_all(parent).map_err(|err| CoreError::io(parent.to_path_buf(), err))?;
        let dir_str = dir.to_string_lossy().into_owned();
        let git = Git::new(parent);
        let mut args = vec!["clone", "--progress", "--depth", "1"];
        if let Some(reference) = reference {
            args.push("--branch");
            args.push(reference);
        }
        args.push(repo);
        args.push(&dir_str);
        let output = git.run_streaming(&args, &mut line_cb, cancel)?;
        if !output.success() {
            // 克隆失败同样不留半成品复用；清理后如实报错。
            let _ = crate::deploy::compose::remove_path(&dir);
            return Err(CoreError::Git {
                args: format!("clone --depth 1 {repo}"),
                cwd: parent.to_path_buf(),
                stderr: output.stderr.trim().to_string(),
            });
        }
        false
    };

    let oid = Git::new(&dir).rev_parse("HEAD")?;
    Ok(FetchedRepo {
        path: dir,
        oid,
        reused,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_is_stable_and_normalized() {
        assert_eq!(cache_key("https://x/y.git"), cache_key(" https://x/y.git/ "));
        assert_ne!(cache_key("https://x/y.git"), cache_key("https://x/z.git"));
    }

    #[test]
    fn cancel_terminates_fetch_and_reports_cancelled() {
        let dir = tempfile::TempDir::new().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(&repo)
                .env("HOME", dir.path())
                .env("GIT_AUTHOR_NAME", "t")
                .env("GIT_AUTHOR_EMAIL", "t@x")
                .env("GIT_COMMITTER_NAME", "t")
                .env("GIT_COMMITTER_EMAIL", "t@x")
                .output()
                .unwrap()
        };
        git(&["init", "--quiet", "--initial-branch=main"]);
        std::fs::write(repo.join("SKILL.md"), "x").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "--quiet", "-m", "init"]);

        let state = dir.path().join("state");
        let cancel = std::sync::atomic::AtomicBool::new(true);
        let err = fetch_repo(
            &state,
            repo.to_str().unwrap(),
            None,
            &mut |_p| {},
            &cancel,
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::Cancelled(_)), "{err}");
        // 取消如实返回 Cancelled；登记入库只发生在 vault/acquire.rs，这里不产生任何入库事实。
        // 允许残留不可复用的半成品缓存（设计边界：不承诺清除所有临时字节）。
    }

    #[test]
    fn progress_parses_stage_percent_and_counts() {
        let p = GitProgress::parse("Receiving objects:  45% (450/1000), 1.20 MiB | 2.00 MiB/s");
        assert_eq!(p.stage.as_deref(), Some("Receiving objects"));
        assert_eq!(p.percent, Some(45));
        assert_eq!(p.counts, Some((450, 1000)));

        let unknown = GitProgress::parse("Cloning into 'x'...");
        assert_eq!(unknown.percent, None);
        assert_eq!(unknown.counts, None);
        assert!(unknown.raw.contains("Cloning"));
    }
}
