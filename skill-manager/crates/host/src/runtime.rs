//! Host 运行时：从桌面壳抽出的进程内协调（研究决策 D1）。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, RwLock};
use std::time::{Duration, Instant};

use serde_json::json;
use skill_manager_core::vault::VaultWatcher;
use skill_manager_core::{AppContext, ScheduleReport, Trigger};

use crate::events::{
    BroadcastSink, EventSink, FanoutSink, NoopSink, SCHEDULE_FINISHED_EVENT, VAULT_CHANGED_EVENT,
};
use crate::lock::{HostLock, LockError};

const CHANGE_DEBOUNCE: Duration = Duration::from_millis(250);

struct Inner {
    ctx: OnceLock<std::result::Result<Mutex<AppContext>, String>>,
    background_jobs: Mutex<()>,
    watcher: Mutex<Option<VaultWatcher>>,
    fetch_jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
    schedule_report: Mutex<Option<ScheduleReport>>,
    events: RwLock<Arc<dyn EventSink>>,
    lock: Mutex<Option<HostLock>>,
    listen: Mutex<Option<String>>,
    state_dir: PathBuf,
    sse: BroadcastSink,
}

/// 本机唯一 Host 运行时。可在桌面与 `host serve` 之间共用。
#[derive(Clone)]
pub struct HostRuntime {
    inner: Arc<Inner>,
}

impl Default for HostRuntime {
    fn default() -> Self {
        Self::open(None).unwrap_or_else(|err| {
            let runtime = Self::unopened(AppContext::default_state_dir());
            let _ = runtime.inner.ctx.set(Err(err));
            runtime
        })
    }
}

impl HostRuntime {
    fn unopened(state_dir: PathBuf) -> Self {
        let sse = BroadcastSink::new(64);
        Self {
            inner: Arc::new(Inner {
                ctx: OnceLock::new(),
                background_jobs: Mutex::new(()),
                watcher: Mutex::new(None),
                fetch_jobs: Mutex::new(HashMap::new()),
                schedule_report: Mutex::new(None),
                events: RwLock::new(Arc::new(NoopSink)),
                lock: Mutex::new(None),
                listen: Mutex::new(None),
                state_dir,
                sse,
            }),
        }
    }

    /// 打开本机状态目录对应的运行时，尚未占锁。
    pub fn open(state_dir: Option<PathBuf>) -> Result<Self, String> {
        let mut ctx = AppContext::open(state_dir).map_err(|err| err.to_string())?;
        let _ = ctx.attach_remembered_vault();
        Ok(Self::from_context(ctx))
    }

    /// 用已有上下文构造（测试与桌面启动共用）。
    pub fn from_context(ctx: AppContext) -> Self {
        let state_dir = ctx.state().dir().to_path_buf();
        let runtime = Self::unopened(state_dir);
        let _ = runtime.inner.ctx.set(Ok(Mutex::new(ctx)));
        runtime.set_event_sink(Arc::new(runtime.inner.sse.clone()));
        runtime
    }

    pub fn state_dir(&self) -> &std::path::Path {
        &self.inner.state_dir
    }

    pub fn sse(&self) -> &BroadcastSink {
        &self.inner.sse
    }

    pub fn set_event_sink(&self, sink: Arc<dyn EventSink>) {
        if let Ok(mut guard) = self.inner.events.write() {
            *guard = sink;
        }
    }

    /// 在现有 SSE 广播之上再叠加一个入口（例如桌面 Tauri）。
    pub fn add_event_sink(&self, extra: Arc<dyn EventSink>) {
        let current = self
            .inner
            .events
            .read()
            .map(|g| Arc::clone(&*g))
            .unwrap_or_else(|_| Arc::new(NoopSink));
        self.set_event_sink(Arc::new(FanoutSink::new(vec![current, extra])));
    }

    pub fn emit(&self, event: &str, payload: &serde_json::Value) {
        if let Ok(sink) = self.inner.events.read() {
            sink.emit(event, payload);
        }
    }

    /// 占用 `host.lock`。已被占用则失败。
    pub fn acquire_lock(&self) -> Result<(), String> {
        let lock = HostLock::try_acquire(&self.inner.state_dir).map_err(|err| err.to_string())?;
        *self.inner.lock.lock().map_err(|_| "Host 锁已损坏".to_string())? = Some(lock);
        Ok(())
    }

    pub fn lock_error_if_busy(state_dir: &std::path::Path) -> Option<String> {
        match HostLock::try_acquire(state_dir) {
            Ok(lock) => {
                drop(lock);
                None
            }
            Err(LockError::Busy { pid, .. }) => Some(format!(
                "本机状态目录 {} 已有 Host 在运行（pid {pid}）",
                state_dir.display()
            )),
            Err(err) => Some(err.to_string()),
        }
    }

    pub fn set_listen(&self, listen: impl Into<String>) -> Result<(), String> {
        let listen = listen.into();
        *self.inner.listen.lock().map_err(|_| "监听地址锁已损坏".to_string())? = Some(listen.clone());
        if let Ok(mut guard) = self.inner.lock.lock() {
            if let Some(lock) = guard.as_mut() {
                let vault = self
                    .get()
                    .ok()
                    .and_then(|ctx| ctx.vault_opt().map(|v| v.root().display().to_string()));
                let _ = lock.update_meta(Some(listen), vault);
            }
        }
        Ok(())
    }

    pub fn listen_addr(&self) -> Option<String> {
        self.inner.listen.lock().ok().and_then(|g| g.clone())
    }

    pub fn register_fetch(&self, job_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        if let Ok(mut jobs) = self.inner.fetch_jobs.lock() {
            jobs.insert(job_id.to_string(), flag.clone());
        }
        flag
    }

    pub fn cancel_fetch(&self, job_id: &str) -> bool {
        match self.inner.fetch_jobs.lock() {
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

    pub fn finish_fetch(&self, job_id: &str) {
        if let Ok(mut jobs) = self.inner.fetch_jobs.lock() {
            jobs.remove(job_id);
        }
    }

    pub fn get(&self) -> Result<MutexGuard<'_, AppContext>, String> {
        let cell = self.inner.ctx.get_or_init(|| {
            let mut ctx = AppContext::open(Some(self.inner.state_dir.clone()))
                .map_err(|err| err.to_string())?;
            let _ = ctx.attach_remembered_vault();
            Ok(Mutex::new(ctx))
        });
        match cell {
            Ok(ctx) => ctx.lock().map_err(|_| "上下文锁已损坏".to_string()),
            Err(message) => Err(message.clone()),
        }
    }

    pub fn with_background_context<T>(
        &self,
        operation: impl FnOnce(&AppContext) -> Result<T, String>,
    ) -> Result<T, String> {
        let _job = self
            .inner
            .background_jobs
            .lock()
            .map_err(|_| "后台任务锁已损坏".to_string())?;
        let ctx = { self.get()?.fork().map_err(|err| err.to_string())? };
        operation(&ctx)
    }

    pub fn watch_vault(&self) -> Result<(), String> {
        let root = {
            let ctx = self.get()?;
            let vault = ctx
                .vault_opt()
                .ok_or_else(|| "尚未接入 SkillManagerVault".to_string())?;
            vault.root().to_path_buf()
        };

        let mut guard = self
            .inner
            .watcher
            .lock()
            .map_err(|_| "监听锁已损坏".to_string())?;
        if guard.as_ref().is_some_and(|watcher| watcher.root() == root) {
            return Ok(());
        }

        let runtime = self.clone();
        let last_emit = Mutex::new(None::<Instant>);
        let watcher = VaultWatcher::start(&root, move |_index| {
            let Ok(mut last) = last_emit.lock() else {
                return;
            };
            if last.is_some_and(|previous| previous.elapsed() < CHANGE_DEBOUNCE) {
                return;
            }
            *last = Some(Instant::now());
            runtime.emit(VAULT_CHANGED_EVENT, &json!(null));
        })
        .map_err(|err| err.to_string())?;

        *guard = Some(watcher);
        Ok(())
    }

    pub fn run_schedule(&self, trigger: Trigger) -> Result<ScheduleReport, String> {
        let report = self.with_background_context(|ctx| {
            skill_manager_core::scheduler::run(ctx, trigger).map_err(|err| err.to_string())
        })?;
        if !report.skipped_by_interval && !report.not_due {
            *self
                .inner
                .schedule_report
                .lock()
                .map_err(|_| "检查报告锁已损坏".to_string())? = Some(report.clone());
            let payload = serde_json::to_value(&report).unwrap_or(json!(null));
            self.emit(SCHEDULE_FINISHED_EVENT, &payload);
        }
        Ok(report)
    }

    pub fn current_schedule(&self) -> Result<Option<ScheduleReport>, String> {
        self.inner
            .schedule_report
            .lock()
            .map(|report| report.clone())
            .map_err(|_| "检查报告锁已损坏".to_string())
    }

    pub fn spawn_scheduler(&self) {
        let runtime = self.clone();
        std::thread::spawn(move || {
            let _ = runtime.run_schedule(Trigger::Startup);
            loop {
                std::thread::sleep(Duration::from_secs(60));
                let _ = runtime.run_schedule(Trigger::Timer);
            }
        });
    }

    /// 本机状态摘要，与 `host status` / `GET /host/status` 同一事实。
    pub fn status_view(&self) -> HostStatus {
        let ctx = self.get().ok();
        let vault = ctx.as_ref().and_then(|c| c.vault_opt().map(|v| v.root().display().to_string()));
        let identity = ctx.as_ref().and_then(|c| crate::identity::read(c.state()).ok()).flatten();
        HostStatus {
            running: true,
            pid: std::process::id(),
            state_dir: self.inner.state_dir.display().to_string(),
            listen: self.listen_addr(),
            vault,
            paired: identity
                .as_ref()
                .map(|row| !row.station_url.is_empty())
                .unwrap_or(false),
            station_url: identity.and_then(|row| {
                if row.station_url.is_empty() {
                    None
                } else {
                    Some(row.station_url)
                }
            }),
            lock_held: self.inner.lock.lock().ok().is_some_and(|g| g.is_some()),
        }
    }
}

/// `host status` 与 HTTP 共用的本机事实。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostStatus {
    pub running: bool,
    pub pid: u32,
    pub state_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listen: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<String>,
    pub paired: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub station_url: Option<String>,
    pub lock_held: bool,
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

        let started = temp.path().join("started");
        let release = temp.path().join("release");
        let hook = repo.join(".git/hooks/pre-push");
        std::fs::write(
            &hook,
            format!(
                "#!/bin/sh\ntouch '{0}'\nwhile [ ! -f '{1}' ]; do sleep 0.01; done\n",
                started.display(),
                release.display(),
            ),
        )
        .unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();

        let ctx = AppContext::open_with_vault(&repo, Some(temp.path().join("state"))).unwrap();
        let state = HostRuntime::from_context(ctx);
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
