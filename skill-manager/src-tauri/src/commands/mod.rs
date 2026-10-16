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
use tauri::Emitter;

/// 库内变化的去抖窗口：GUI 自身一次写入会连续触发多个事件，
/// 期间只通知前端一次；前端收到通知后重新读取库文件。
const CHANGE_DEBOUNCE: Duration = Duration::from_millis(250);

/// 库内变化事件名。
pub const VAULT_CHANGED_EVENT: &str = "vault-changed";

/// 桌面应用的共享上下文。
pub struct AppState {
    ctx: OnceLock<std::result::Result<Mutex<AppContext>, String>>,
    /// 当前库的文件监听；丢弃即停止监听，因此必须留在这里保持存活。
    watcher: Mutex<Option<VaultWatcher>>,
    /// 进行中的仓库获取任务：jobId → 取消标记（GUI 取消本次获取）。
    fetch_jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
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
            watcher: Mutex::new(None),
            fetch_jobs: Mutex::new(HashMap::new()),
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
