//! 仓库操作：调用本机 `git` 命令，复用本机 Git 与既有认证配置（research.md D8）。
//!
//! 不使用 libgit2/git2。子进程继承当前环境，因此沿用用户的 SSH/凭据配置。

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{CoreError, Result};

/// 一步 git 进度：阶段、可得的进展量与原始文本。
///
/// 只解释 git 自己输出的进度（`<阶段>: NN% (a/b)` 或 `remote: ...`），
/// 未知总量时 `percent`/`counts` 为 `None`，如实保留 `raw`；不虚构百分比。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GitProgress {
    pub raw: String,
    pub stage: Option<String>,
    pub percent: Option<u8>,
    pub counts: Option<(u64, u64)>,
}

impl GitProgress {
    /// 解析一行 git 进度文本。
    pub fn parse(line: &str) -> Self {
        let raw = line.trim().to_string();
        let stage = raw
            .split_once(':')
            .map(|(head, _)| head.trim().trim_start_matches("remote:").trim().to_string())
            .filter(|text| !text.is_empty());

        let mut percent = None;
        let mut counts = None;
        if let Some(open) = raw.find('(') {
            if let Some(close_rel) = raw[open + 1..].find(')') {
                let inner = &raw[open + 1..open + 1 + close_rel];
                if let Some((left, right)) = inner.split_once('/') {
                    if let (Ok(a), Ok(b)) = (left.trim().parse::<u64>(), right.trim().parse::<u64>()) {
                        counts = Some((a, b));
                    }
                }
            }
        }
        // 取第一个以 `%` 结尾的数值作为该阶段完成度。
        for (index, _) in raw.match_indices('%') {
            let head = &raw[..index];
            let digits: String = head
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_digit())
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            if let Ok(value) = digits.parse::<u8>() {
                percent = Some(value);
                break;
            }
        }

        Self {
            raw,
            stage,
            percent,
            counts,
        }
    }
}

/// 一次 git 命令的结果。
#[derive(Clone, Debug)]
pub struct GitOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl GitOutput {
    pub fn success(&self) -> bool {
        self.status == 0
    }

    /// 去掉首尾空白的标准输出。
    pub fn trimmed(&self) -> &str {
        self.stdout.trim()
    }
}

/// 在某个工作目录内执行 git 命令。
#[derive(Clone, Debug)]
pub struct Git {
    cwd: PathBuf,
}

impl Git {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self { cwd: cwd.into() }
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = crate::process::command("git");
        command
            .arg("-C")
            .arg(&self.cwd)
            .arg("--no-pager")
            .args(args)
            .stdin(Stdio::null());
        command
    }

    /// 执行命令并收集输出；非零退出不视为错误，由调用者判断。
    pub fn run(&self, args: &[&str]) -> Result<GitOutput> {
        let output = self
            .command(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|err| CoreError::io(self.cwd.clone(), err))?;

        Ok(GitOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    /// 执行命令并要求成功；失败时给出命令与标准错误。
    pub fn ok(&self, args: &[&str]) -> Result<String> {
        let output = self.run(args)?;
        if !output.success() {
            return Err(CoreError::Git {
                args: args.join(" "),
                cwd: self.cwd.clone(),
                stderr: output.stderr.trim().to_string(),
            });
        }
        Ok(output.stdout)
    }

    /// 该目录是否位于 Git 工作树内。
    pub fn is_repo(&self) -> bool {
        self.run(&["rev-parse", "--is-inside-work-tree"])
            .map(|out| out.success() && out.trimmed() == "true")
            .unwrap_or(false)
    }

    /// 仓库根目录（解析符号链接后的实际位置）。
    pub fn top_level(&self) -> Result<PathBuf> {
        let out = self.ok(&["rev-parse", "--show-toplevel"])?;
        Ok(PathBuf::from(out.trim()))
    }

    /// 已配置的远端名。
    pub fn remotes(&self) -> Result<Vec<String>> {
        let out = self.ok(&["remote"])?;
        Ok(out
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect())
    }

    /// 指定远端的地址。
    pub fn remote_url(&self, name: &str) -> Result<Option<String>> {
        let out = self.run(&["remote", "get-url", name])?;
        if !out.success() {
            return Ok(None);
        }
        let url = out.trimmed().to_string();
        Ok(if url.is_empty() { None } else { Some(url) })
    }

    /// 解析引用为完整提交 OID。
    pub fn rev_parse(&self, rev: &str) -> Result<String> {
        Ok(self.ok(&["rev-parse", rev])?.trim().to_string())
    }

    /// `git status --porcelain` 原始输出（空表示工作区干净）。
    pub fn status_porcelain(&self) -> Result<String> {
        Ok(self.ok(&["status", "--porcelain"])?)
    }

    /// 当前分支名；游离 HEAD 返回 None。
    pub fn current_branch(&self) -> Result<Option<String>> {
        let out = self.run(&["symbolic-ref", "--short", "-q", "HEAD"])?;
        if !out.success() {
            return Ok(None);
        }
        let name = out.trimmed().to_string();
        Ok(if name.is_empty() { None } else { Some(name) })
    }

    /// 本地分支配置的上游引用（`origin/main`）；未配置返回 None。
    pub fn upstream_ref(&self) -> Result<Option<String>> {
        let out = self.run(&[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ])?;
        if !out.success() {
            return Ok(None);
        }
        let name = out.trimmed().to_string();
        Ok(if name.is_empty() { None } else { Some(name) })
    }

    /// 指定引用是否存在（可解析为提交）。
    pub fn has_rev(&self, rev: &str) -> bool {
        self.run(&["rev-parse", "--verify", "--quiet", rev])
            .map(|out| out.success())
            .unwrap_or(false)
    }

    /// `left` 相对 `right` 的 (ahead, behind) 提交数（`rev-list --left-right --count left...right`）。
    pub fn ahead_behind(&self, left: &str, right: &str) -> Result<(u64, u64)> {
        let spec = format!("{left}...{right}");
        let out = self.ok(&["rev-list", "--left-right", "--count", &spec])?;
        let mut parts = out.split_whitespace();
        let ahead = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        let behind = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        Ok((ahead, behind))
    }

    /// 进行中的合并冲突路径（`git diff --name-only --diff-filter=U`）。
    pub fn unmerged_paths(&self) -> Result<Vec<String>> {
        self.name_only(&["diff", "--name-only", "--diff-filter=U"])
    }

    fn name_only(&self, args: &[&str]) -> Result<Vec<String>> {
        let out = self.run(args)?;
        if !out.success() {
            return Ok(Vec::new());
        }
        Ok(out
            .stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect())
    }

    /// 工作区未提交路径（已暂存、未暂存、未跟踪，不含忽略文件）。
    pub fn uncommitted_paths(&self) -> Result<Vec<String>> {
        let mut paths = self.name_only(&["diff", "--name-only"])?;
        for path in self.name_only(&["diff", "--name-only", "--cached"])? {
            if !paths.iter().any(|existing| existing == &path) {
                paths.push(path);
            }
        }
        let untracked = self.ok(&["ls-files", "--others", "--exclude-standard"])?;
        for path in untracked.lines().map(str::trim).filter(|line| !line.is_empty()) {
            if !paths.iter().any(|existing| existing == path) {
                paths.push(path.to_string());
            }
        }
        Ok(paths)
    }

    /// `from...to` 之间有内容变化的路径（三方点范围，与 merge 将触及的文件一致）。
    pub fn incoming_paths(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let spec = format!("{from}...{to}");
        self.name_only(&["diff", "--name-only", &spec])
    }

    /// 是否处于未完成的合并（`MERGE_HEAD` 存在）。
    pub fn merge_in_progress(&self) -> bool {
        self.has_rev("MERGE_HEAD")
    }

    /// 拉取前暂存未提交改动时使用的 stash 说明，便于识别并在合并后恢复。
    pub const PULL_AUTOSTASH_MESSAGE: &str = "skill-manager: autostash before pull";

    /// 工作区是否有未提交改动（含未跟踪文件）。
    pub fn is_dirty(&self) -> Result<bool> {
        Ok(!self.status_porcelain()?.trim().is_empty())
    }

    /// 暂存当前工作区（含未跟踪文件），让随后的 merge 能在干净树上执行。
    ///
    /// 没有可暂存改动时返回 `false`，不视为错误。
    pub fn stash_push_pull_guard(&self) -> Result<bool> {
        let out = self.run(&[
            "stash",
            "push",
            "--include-untracked",
            "--quiet",
            "-m",
            Self::PULL_AUTOSTASH_MESSAGE,
        ])?;
        if out.success() {
            return Ok(true);
        }
        let combined = format!("{}{}", out.stdout, out.stderr);
        if combined.contains("No local changes to save") {
            return Ok(false);
        }
        Err(CoreError::Git {
            args: "stash push --include-untracked".to_string(),
            cwd: self.cwd.clone(),
            stderr: out.stderr.trim().to_string(),
        })
    }

    /// 查找本程序留下的拉取暂存；`stash list` 新条目在前。
    pub fn pull_autostash_ref(&self) -> Result<Option<String>> {
        let out = self.ok(&["stash", "list"])?;
        for line in out.lines() {
            if !line.contains(Self::PULL_AUTOSTASH_MESSAGE) {
                continue;
            }
            let spec = line.split(':').next().unwrap_or("").trim();
            if !spec.is_empty() {
                return Ok(Some(spec.to_string()));
            }
        }
        Ok(None)
    }

    /// 恢复拉取前的暂存。没有对应 stash 视为已恢复。
    ///
    /// 成功返回 `Ok(true)`；`git stash pop` 非零（冲突或无法覆盖）返回 `Ok(false)`，
    /// 由调用方读取未合并路径或标准错误后决定。
    pub fn stash_pop_pull_guard(&self) -> Result<bool> {
        let Some(spec) = self.pull_autostash_ref()? else {
            return Ok(true);
        };
        let out = self.run(&["stash", "pop", "--quiet", &spec])?;
        Ok(out.success())
    }

    /// 丢弃本程序留下的拉取暂存（恢复成功或已手工落实内容后）。
    pub fn drop_pull_autostash(&self) -> Result<()> {
        let Some(spec) = self.pull_autostash_ref()? else {
            return Ok(());
        };
        let _ = self.run(&["stash", "drop", "--quiet", &spec])?;
        Ok(())
    }

    /// 远端是否可访问（`git ls-remote --exit-code <remote>`）；失败返回原因文本。
    pub fn remote_reachable(&self, remote: &str) -> std::result::Result<(), String> {
        match self.run(&["ls-remote", "--exit-code", remote]) {
            Ok(out) if out.success() => Ok(()),
            Ok(out) => {
                let detail = {
                    let trimmed = out.stderr.trim();
                    if trimmed.is_empty() {
                        out.stdout.trim().to_string()
                    } else {
                        trimmed.to_string()
                    }
                };
                Err(if detail.is_empty() {
                    format!("远端 {remote} 不可访问")
                } else {
                    detail
                })
            }
            Err(err) => Err(err.to_string()),
        }
    }

    /// 执行命令并把标准错误逐行回调（用于获取/同步的真实进度）。
    pub fn run_with_progress(
        &self,
        args: &[&str],
        on_stderr_line: &mut dyn FnMut(&str),
    ) -> Result<GitOutput> {
        let mut child = self
            .command(args)
            .arg("--progress")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| CoreError::io(self.cwd.clone(), err))?;

        // stdout 在后台线程收集；stderr 在本线程逐行回调，保证回调无需跨线程。
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CoreError::other("无法读取 git 标准输出"))?;
        let reader = std::thread::spawn(move || {
            let mut buffer = String::new();
            for line in BufReader::new(stdout).lines().map_while(std::result::Result::ok) {
                buffer.push_str(&line);
                buffer.push('\n');
            }
            buffer
        });

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| CoreError::other("无法读取 git 标准错误"))?;
        let mut stderr_text = String::new();
        for line in BufReader::new(stderr).lines().map_while(std::result::Result::ok) {
            on_stderr_line(&line);
            stderr_text.push_str(&line);
            stderr_text.push('\n');
        }

        let status = child
            .wait()
            .map_err(|err| CoreError::io(self.cwd.clone(), err))?;
        let stdout_text = reader.join().unwrap_or_default();

        Ok(GitOutput {
            status: status.code().unwrap_or(-1),
            stdout: stdout_text,
            stderr: stderr_text,
        })
    }

    /// 执行命令并逐段回调标准错误，同时支持取消。
    ///
    /// 与 `run_with_progress` 的区别：进度按 `\r` 与 `\n` 双分隔切分（终端进度常在同一行刷新），
    /// 并且每次读取后检查 `cancel`；取消时终止子进程并如实返回 `Cancelled`，
    /// 由调用者决定不把本次获取当作成功。调用者需在 `args` 中自行加入 `--progress`。
    pub fn run_streaming(
        &self,
        args: &[&str],
        on_line: &mut dyn FnMut(&str),
        cancel: &AtomicBool,
    ) -> Result<GitOutput> {
        let mut child = self
            .command(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| CoreError::io(self.cwd.clone(), err))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CoreError::other("无法读取 git 标准输出"))?;
        let reader = std::thread::spawn(move || {
            let mut buffer = String::new();
            for line in BufReader::new(stdout).lines().map_while(std::result::Result::ok) {
                buffer.push_str(&line);
                buffer.push('\n');
            }
            buffer
        });

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| CoreError::other("无法读取 git 标准错误"))?;
        let mut stderr_reader = BufReader::new(stderr);
        let mut pending: Vec<u8> = Vec::new();
        let mut stderr_text = String::new();
        let mut buffer = [0u8; 1024];
        let cancelled;
        loop {
            if cancel.load(Ordering::SeqCst) {
                let _ = child.kill();
                cancelled = true;
                break;
            }
            let read = stderr_reader
                .read(&mut buffer)
                .map_err(|err| CoreError::io(self.cwd.clone(), err))?;
            if read == 0 {
                cancelled = false;
                break;
            }
            pending.extend_from_slice(&buffer[..read]);
            let mut start = 0;
            for index in 0..pending.len() {
                if pending[index] == b'\r' || pending[index] == b'\n' {
                    let line = String::from_utf8_lossy(&pending[start..index]).into_owned();
                    if !line.is_empty() {
                        on_line(&line);
                        stderr_text.push_str(&line);
                        stderr_text.push('\n');
                    }
                    start = index + 1;
                }
            }
            pending.drain(0..start);
        }
        if !pending.is_empty() {
            let line = String::from_utf8_lossy(&pending).into_owned();
            if !line.is_empty() {
                on_line(&line);
                stderr_text.push_str(&line);
                stderr_text.push('\n');
            }
        }

        let status = child
            .wait()
            .map_err(|err| CoreError::io(self.cwd.clone(), err))?;
        let _stdout_text = reader.join().unwrap_or_default();
        if cancelled {
            return Err(CoreError::Cancelled(format!(
                "已终止本次 git {}（后续不继续该获取）",
                args.join(" ")
            )));
        }
        Ok(GitOutput {
            status: status.code().unwrap_or(-1),
            stdout: _stdout_text,
            stderr: stderr_text,
        })
    }

    /// 启动一个无需等待的命令（获取/同步的取消入口）。
    pub fn spawn(&self, args: &[&str]) -> Result<GitProcess> {
        let child = self
            .command(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| CoreError::io(self.cwd.clone(), err))?;
        Ok(GitProcess { child })
    }
}

/// 正在运行的 git 子进程，支持终止（取消本次获取）。
#[derive(Debug)]
pub struct GitProcess {
    child: Child,
}

impl GitProcess {
    /// 终止该进程；取消意味着停止本次获取。
    pub fn cancel(&mut self) -> Result<()> {
        self.child
            .kill()
            .map_err(|err| CoreError::io(PathBuf::from("git"), err))
    }

    pub fn wait(mut self) -> Result<i32> {
        let status = self
            .child
            .wait()
            .map_err(|err| CoreError::io(PathBuf::from("git"), err))?;
        Ok(status.code().unwrap_or(-1))
    }

    pub fn id(&self) -> Option<u32> {
        Some(self.child.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_repo() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        let git = Git::new(dir.path());
        git.ok(&["init", "--quiet", "--initial-branch=main"]).unwrap();
        dir
    }

    #[test]
    fn is_repo_distinguishes_plain_directory() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(!Git::new(dir.path()).is_repo());
        let repo = temp_repo();
        assert!(Git::new(repo.path()).is_repo());
    }

    #[test]
    fn missing_remote_reports_none_instead_of_error() {
        let repo = temp_repo();
        let git = Git::new(repo.path());
        assert!(git.remotes().unwrap().is_empty());
        assert_eq!(git.remote_url("origin").unwrap(), None);
    }

    #[test]
    fn failed_command_reports_args_and_stderr() {
        let repo = temp_repo();
        let git = Git::new(repo.path());
        let err = git.ok(&["rev-parse", "no-such-rev"]).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("rev-parse no-such-rev"), "{message}");
    }

    #[test]
    fn rev_parse_returns_full_oid() {
        let repo = temp_repo();
        let git = Git::new(repo.path());
        std::fs::write(repo.path().join("a.txt"), "a").unwrap();
        git.ok(&["add", "-A"]).unwrap();
        git.ok(&["-c", "user.email=t@example.invalid", "-c", "user.name=t", "commit", "--quiet", "-m", "init"])
            .unwrap();
        let oid = git.rev_parse("HEAD").unwrap();
        assert_eq!(oid.len(), 40, "应采用完整提交 OID：{oid}");
    }

    #[test]
    fn pull_autostash_roundtrip_restores_uncommitted_and_untracked() {
        let repo = temp_repo();
        let git = Git::new(repo.path());
        git.ok(&["config", "user.email", "t@example.invalid"]).unwrap();
        git.ok(&["config", "user.name", "t"]).unwrap();
        std::fs::write(repo.path().join("tracked.txt"), "base\n").unwrap();
        git.ok(&["add", "-A"]).unwrap();
        git.ok(&["commit", "--quiet", "-m", "init"]).unwrap();
        std::fs::write(repo.path().join("tracked.txt"), "edited\n").unwrap();
        std::fs::write(repo.path().join("new.txt"), "untracked\n").unwrap();
        assert!(git.is_dirty().unwrap());
        assert!(git.stash_push_pull_guard().unwrap());
        assert!(!git.is_dirty().unwrap());
        assert!(git.pull_autostash_ref().unwrap().is_some());
        assert!(git.stash_pop_pull_guard().unwrap());
        assert_eq!(std::fs::read_to_string(repo.path().join("tracked.txt")).unwrap(), "edited\n");
        assert_eq!(std::fs::read_to_string(repo.path().join("new.txt")).unwrap(), "untracked\n");
        assert!(git.pull_autostash_ref().unwrap().is_none());
    }
}
