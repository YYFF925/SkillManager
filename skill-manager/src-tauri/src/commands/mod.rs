//! Tauri command 层：薄封装 core，不承载业务规则。

pub mod acquire;
pub mod agent;
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

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use skill_manager_core::AppContext;
use skill_manager_core::vault::VaultWatcher;
use tauri::{Emitter, Manager};

/// 库内变化的去抖窗口：GUI 自身一次写入会连续触发多个事件，
/// 期间只通知前端一次；前端收到通知后重新读取库文件。
const CHANGE_DEBOUNCE: Duration = Duration::from_millis(250);

/// 库内变化事件名。
pub const VAULT_CHANGED_EVENT: &str = "vault-changed";
pub const SCHEDULE_FINISHED_EVENT: &str = "schedule-finished";

/// 桌面应用的共享上下文。
pub struct AppState {
    ctx: OnceLock<std::result::Result<Mutex<AppContext>, String>>,
    /// 耗时库任务依次执行；等待只发生在后台线程，不占用界面读取的上下文锁。
    background_jobs: Mutex<()>,
    /// 当前库的文件监听；丢弃即停止监听，因此必须留在这里保持存活。
    watcher: Mutex<Option<VaultWatcher>>,
    /// 进行中的仓库获取任务：jobId → 取消标记（GUI 取消本次获取）。
    fetch_jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub schedule_report: Mutex<Option<skill_manager_core::ScheduleReport>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            ctx: OnceLock::new(),
            background_jobs: Mutex::new(()),
            watcher: Mutex::new(None),
            fetch_jobs: Mutex::new(HashMap::new()),
            schedule_report: Mutex::new(None),
        }
    }

    /// 登记一次仓库获取的取消标记。
    pub fn register_fetch(&self, job_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        if let Ok(mut jobs) = self.fetch_jobs.lock() {
            jobs.insert(job_id.to_string(), flag.clone());
        }
        flag
    }

    /// 取消一次获取；返回是否找到该任务。
    pub fn cancel_fetch(&self, job_id: &str) -> bool {
        match self.fetch_jobs.lock() {
            Ok(jobs) => match jobs.get(job_id) {
                Some(flag) => {
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    true
                }
                None => false,
            },
            Err(_) => false,
        }
    }

    /// 获取结束后移除取消标记。
    pub fn finish_fetch(&self, job_id: &str) {
        if let Ok(mut jobs) = self.fetch_jobs.lock() {
            jobs.remove(job_id);
        }
    }

    /// 取得上下文；首次调用时打开本机状态目录并尝试接入记住的 SkillManagerVault。
    pub fn get(&self) -> Result<MutexGuard<'_, AppContext>, String> {
        let cell = self.ctx.get_or_init(|| {
            let mut ctx = AppContext::open(None).map_err(|err| err.to_string())?;
            // 记住的库缺失或不再是仓库时不阻止启动；界面据此显示未接入。
            let _ = ctx.attach_remembered_vault();
            Ok(Mutex::new(ctx))
        });

        match cell {
            Ok(ctx) => ctx.lock().map_err(|_| "上下文锁已损坏".to_string()),
            Err(message) => Err(message.clone()),
        }
    }

    fn with_background_context<T>(
        &self,
        operation: impl FnOnce(&AppContext) -> Result<T, String>,
    ) -> Result<T, String> {
        let _job = self.background_jobs.lock().map_err(|_| "后台任务锁已损坏".to_string())?;
        let ctx = { self.get()?.fork().map_err(|err| err.to_string())? };
        operation(&ctx)
    }

    /// 让文件监听跟随当前已接入的库；已在监听同一库时不做变化。
    ///
    /// 监听只刷新索引并通知界面，不自动提交、推送或部署。
    pub fn watch_vault(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let root = {
            let ctx = self.get()?;
            let vault = ctx
                .vault_opt()
                .ok_or_else(|| "尚未接入 SkillManagerVault".to_string())?;
            vault.root().to_path_buf()
        };

        let mut guard = self.watcher.lock().map_err(|_| "监听锁已损坏".to_string())?;
        if guard.as_ref().is_some_and(|watcher| watcher.root() == root) {
            return Ok(());
        }

        let handle = app.clone();
        let last_emit = Mutex::new(None::<Instant>);
        let watcher = VaultWatcher::start(&root, move |_index| {
            let Ok(mut last) = last_emit.lock() else {
                return;
            };
            if last.is_some_and(|previous| previous.elapsed() < CHANGE_DEBOUNCE) {
                return;
            }
            *last = Some(Instant::now());
            let _ = handle.emit(VAULT_CHANGED_EVENT, ());
        })
        .map_err(|err| err.to_string())?;

        *guard = Some(watcher);
        Ok(())
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
    let report = state.with_background_context(|ctx| {
        skill_manager_core::scheduler::run(ctx, trigger).map_err(|err| err.to_string())
    })?;
    if !report.skipped_by_interval && !report.not_due {
        *state.schedule_report.lock().map_err(|_| "检查报告锁已损坏".to_string())? = Some(report.clone());
        app.emit(SCHEDULE_FINISHED_EVENT, &report).map_err(|err| err.to_string())?;
    }
    Ok(report)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
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

        // 使用本地裸仓库与受控 pre-push hook，真实 Git 上传等待期间验证界面数据访问。
        let started = temp.path().join("started");
        let release = temp.path().join("release");
        let hook = repo.join(".git/hooks/pre-push");
        std::fs::write(&hook, format!(
            "#!/bin/sh\ntouch '{0}'\nwhile [ ! -f '{1}' ]; do sleep 0.01; done\n",
            started.display(), release.display(),
        )).unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();

        let ctx = AppContext::open_with_vault(&repo, Some(temp.path().join("state"))).unwrap();
        let state = Arc::new(AppState::new());
        assert!(state.ctx.set(Ok(Mutex::new(ctx))).is_ok());
        let worker_state = state.clone();
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
        let local_access = state.ctx.get().unwrap().as_ref().unwrap().try_lock().map(|ctx| {
            ctx.state().set_setting("theme", "dark").unwrap();
            ctx.state().theme().unwrap()
        }).map_err(|_| "后台上传占用了界面上下文锁");
        std::fs::write(&release, "continue").unwrap();
        let result = worker.join().unwrap();

        assert!(upload_waiting, "upload did not reach the hook: {result:?}");
        assert_eq!(local_access.unwrap(), "dark");
        assert!(result.unwrap().pushed);
        assert_eq!(state.get().unwrap().state().theme().unwrap(), "dark");
    }
}
