//! 桌面壳诊断：安装 panic hook，把崩溃线索写入本机日志目录的真实文件（T085）。
//!
//! 诊断内容与脱敏规则全部在 core（`skill_manager_core::diagnostics`）；这里只负责在
//! 桌面进程崩溃时留下可获取的记录，并报告它实际写到了哪里。不自动上传任何诊断资料。

use std::path::PathBuf;
use std::sync::OnceLock;

/// 启动时确定的日志目录；panic 时不能再安全地获取应用状态锁，因此在此缓存。
static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 记录日志目录并安装 panic hook；重复调用只有首次生效。
pub fn init(log_dir: PathBuf) {
    let _ = LOG_DIR.set(log_dir);

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(dir) = LOG_DIR.get() {
            let message = format!("panic：{info}");
            if let Err(err) = skill_manager_core::diagnostics::record_crash_at(dir, &message) {
                eprintln!("写入崩溃记录失败：{err}");
            }
        }
        previous(info);
    }));
}
