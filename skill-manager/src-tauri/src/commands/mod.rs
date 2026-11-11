//! Tauri command 层：薄封装 core，不承载业务规则。

pub mod acquire;
pub mod agent;
pub mod builtin_agent;
pub mod deploy;
pub mod environment;
pub mod hermes;
pub mod preset;
pub mod settings;
pub mod skill;
pub mod sync;
pub mod tool;
pub mod upstream;
pub mod vault;

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, MutexGuard};

use skill_manager_core::AppContext;
use skill_manager_host::HostRuntime;
use tauri::{Emitter, Manager};

/// 库内变化事件名。
pub const VAULT_CHANGED_EVENT: &str = "vault-changed";
pub const SCHEDULE_FINISHED_EVENT: &str = "schedule-finished";

/// 桌面应用持有的 Host 运行时。
pub struct AppState {
    pub runtime: HostRuntime,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            runtime: HostRuntime::default(),
        }
    }

    pub fn boot() -> Result<Self, String> {
        let runtime = HostRuntime::open(None)?;
        runtime.acquire_lock()?;
        Ok(Self { runtime })
    }

    pub fn from_runtime(runtime: HostRuntime) -> Self {
        Self { runtime }
    }

    pub fn register_fetch(&self, job_id: &str) -> Arc<AtomicBool> {
        self.runtime.register_fetch(job_id)
    }

    pub fn cancel_fetch(&self, job_id: &str) -> bool {
        self.runtime.cancel_fetch(job_id)
    }

    pub fn finish_fetch(&self, job_id: &str) {
        self.runtime.finish_fetch(job_id);
    }

    pub fn get(&self) -> Result<MutexGuard<'_, AppContext>, String> {
        self.runtime.get()
    }

    pub fn with_background_context<T>(
        &self,
        operation: impl FnOnce(&AppContext) -> Result<T, String>,
    ) -> Result<T, String> {
        self.runtime.with_background_context(operation)
    }

    pub fn watch_vault(&self, _app: &tauri::AppHandle) -> Result<(), String> {
        self.runtime.watch_vault()
    }

    pub fn current_schedule(&self) -> Result<Option<skill_manager_core::ScheduleReport>, String> {
        self.runtime.current_schedule()
    }
}

/// 把阻塞的 Git/文件操作与任务等待移到后台；不改变调用方的授权与结果语义。
pub async fn background<T, F>(app: tauri::AppHandle, operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&AppContext) -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<AppState>().with_background_context(operation)
    })
    .await
    .map_err(|err| err.to_string())?
}

/// 各桌面触发入口共用；仅在内存保留最近一次实际执行报告，避免启动事件早于界面订阅而丢失。
pub fn run_schedule<R: tauri::Runtime>(app: &tauri::AppHandle<R>, trigger: skill_manager_core::Trigger) -> Result<skill_manager_core::ScheduleReport, String> {
    let state: tauri::State<'_, AppState> = app.state();
    state.runtime.run_schedule(trigger)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};
    use skill_manager_core::sync::{Git, SyncService};

    #[test]
    fn slow_git_upload_does_not_block_local_context_reads_or_edits() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo = temp.path().join("vault");
        let remote = temp.path().join("remote.git");
        std::fs::create_dir(&repo).unwrap();
        let git = Git::new(&repo);
        git.ok(&["init", "--quiet", "--initial-branch=main"]).unwrap();
        git.ok(&["init", "--quiet", "--bare", remote.to_str().unwrap()]).unwrap();
        git.ok(&["config", "user.name", "Test"]).unwrap();
        git.ok(&["config", "user.email", "test@example.invalid"]).unwrap();
        git.ok(&["config", "commit.gpgsign", "false"]).unwrap();
        git.ok(&["config", "core.hooksPath", repo.join(".git/hooks").to_str().unwrap()]).unwrap();
        git.ok(&["remote", "add", "origin", remote.to_str().unwrap()]).unwrap();
        std::fs::write(repo.join("README.md"), "test vault\n").unwrap();
        git.ok(&["add", "-A"]).unwrap();
        git.ok(&["commit", "--quiet", "-m", "initial"]).unwrap();
        git.ok(&["push", "-u", "origin", "main"]).unwrap();
        std::fs::write(repo.join("README.md"), "updated vault\n").unwrap();

        let started = temp.path().join("started");
        let release = temp.path().join("release");
        let hook = repo.join(".git/hooks/pre-push");
        std::fs::write(&hook, format!(
            "#!/bin/sh\ntouch '{0}'\nwhile [ ! -f '{1}' ]; do sleep 0.01; done\n",
            started.display(), release.display(),
        )).unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();

        let ctx = AppContext::open_with_vault(&repo, Some(temp.path().join("state"))).unwrap();
        let state = Arc::new(AppState::from_runtime(HostRuntime::from_context(ctx)));
        let worker_state = Arc::clone(&state);
        let worker = std::thread::spawn(move || {
            worker_state.with_background_context(|ctx| {
                let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
                SyncService::new(ctx.vault().unwrap(), ctx.home().unwrap(), ctx.catalog(), ctx.state(), &resolver)
                    .push(Some("test upload"))
                    .map_err(|err| err.to_string())
            })
        });

        let deadline = Instant::now() + Duration::from_secs(5);
        while !started.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let upload_waiting = started.exists();
        let local_access = state.get().ok().and_then(|ctx| {
            ctx.state().set_setting("theme", "dark").ok()?;
            ctx.state().theme().ok()
        });
        std::fs::write(&release, "continue").unwrap();
        let result = worker.join().unwrap();

        assert!(upload_waiting, "upload did not reach the hook: {result:?}");
        assert_eq!(local_access.as_deref(), Some("dark"));
        assert!(result.unwrap().pushed);
        assert_eq!(state.get().unwrap().state().theme().unwrap(), "dark");
    }
}
