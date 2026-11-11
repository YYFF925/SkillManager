//! `state-dir/host.lock` 咨询锁：同一 state-dir 只允许一个 Host。
//!
//! 先用 `create_new` 占文件，再尽量加 flock。`fs4::try_lock_exclusive` 在争用时
//! 返回 `Ok(false)`（不是 Err）。Windows 上 `LockFileEx` 锁整文件后，其它进程
//! `ReadFile` 会 ERROR_LOCK_VIOLATION，所以元数据另写 `host.lock.meta` 供 peek。
//! 只有拿到独占 flock 且 PID 已死才回收；争用中途不得删锁文件。

use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt;
use serde::{Deserialize, Serialize};

/// 锁文件名（相对 state-dir）。
pub const LOCK_FILE: &str = "host.lock";

/// 不被 flock 挡住的元数据旁路，Windows peek / 第二实例读 PID 用。
pub const LOCK_META_FILE: &str = "host.lock.meta";

/// 锁文件中的运行时元数据，供 CLI 发现本机 Host HTTP。
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockMeta {
    pub pid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault: Option<String>,
}

/// 已持有的 Host 咨询锁。丢弃时释放文件锁并删除锁文件。
#[derive(Debug)]
pub struct HostLock {
    file: File,
    path: PathBuf,
    meta: LockMeta,
}

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("本机状态目录 {state_dir} 已有 Host 在运行（pid {pid}）")]
    Busy { state_dir: PathBuf, pid: u32 },
    #[error("无法占用 Host 锁（{path}）：{source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

enum TryLock {
    Acquired,
    Contended,
}

impl HostLock {
    pub fn path(state_dir: impl AsRef<Path>) -> PathBuf {
        state_dir.as_ref().join(LOCK_FILE)
    }

    pub fn meta_path(state_dir: impl AsRef<Path>) -> PathBuf {
        state_dir.as_ref().join(LOCK_META_FILE)
    }

    /// 尝试占用锁。已被其它存活进程占用则失败并说明。
    pub fn try_acquire(state_dir: impl AsRef<Path>) -> Result<Self, LockError> {
        let state_dir = state_dir.as_ref();
        std::fs::create_dir_all(state_dir).map_err(|source| LockError::Io {
            path: state_dir.to_path_buf(),
            source,
        })?;
        let path = Self::path(state_dir);

        for _ in 0..4 {
            match OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut file) => {
                    // overlay 上 flock 可能对第二进程也成功；PID 仍是占用依据。
                    let _ = try_exclusive(&file);
                    let meta = LockMeta {
                        pid: std::process::id(),
                        listen: None,
                        vault: None,
                    };
                    write_meta(&mut file, &path, &meta).map_err(|source| LockError::Io {
                        path: path.clone(),
                        source,
                    })?;
                    return Ok(Self { file, path, meta });
                }
                Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                    match try_reclaim_or_busy(state_dir, &path)? {
                        Reclaim::Busy(pid) => {
                            return Err(LockError::Busy {
                                state_dir: state_dir.to_path_buf(),
                                pid,
                            });
                        }
                        Reclaim::Retry => continue,
                    }
                }
                Err(source) => {
                    return Err(LockError::Io {
                        path: path.clone(),
                        source,
                    });
                }
            }
        }

        Err(LockError::Busy {
            state_dir: state_dir.to_path_buf(),
            pid: busy_pid(state_dir),
        })
    }

    pub fn meta(&self) -> &LockMeta {
        &self.meta
    }

    pub fn path_ref(&self) -> &Path {
        &self.path
    }

    /// 更新锁文件中的监听地址 / Vault 路径，供 CLI 发现。
    pub fn update_meta(
        &mut self,
        listen: Option<String>,
        vault: Option<String>,
    ) -> std::io::Result<()> {
        self.meta.listen = listen;
        if vault.is_some() {
            self.meta.vault = vault;
        }
        write_meta(&mut self.file, &self.path, &self.meta)
    }
}

impl Drop for HostLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
        let _ = std::fs::remove_file(meta_path(&self.path));
        let _ = std::fs::remove_file(&self.path);
    }
}

enum Reclaim {
    Busy(u32),
    Retry,
}

fn try_exclusive(file: &File) -> Result<TryLock, std::io::Error> {
    match file.try_lock_exclusive() {
        Ok(true) => Ok(TryLock::Acquired),
        Ok(false) => Ok(TryLock::Contended),
        Err(err) if is_lock_busy(&err) => Ok(TryLock::Contended),
        Err(err) => Err(err),
    }
}

fn busy_pid(state_dir: &Path) -> u32 {
    recorded_meta(state_dir).map(|m| m.pid).unwrap_or(0)
}

fn try_reclaim_or_busy(state_dir: &Path, path: &Path) -> Result<Reclaim, LockError> {
    let mut file = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Reclaim::Retry),
        Err(err) if is_lock_busy(&err) => return Ok(Reclaim::Busy(busy_pid(state_dir))),
        Err(source) => {
            return Err(LockError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    match try_exclusive(&file) {
        Ok(TryLock::Contended) => return Ok(Reclaim::Busy(busy_pid(state_dir))),
        Err(err) if is_lock_busy(&err) => return Ok(Reclaim::Busy(busy_pid(state_dir))),
        Err(source) => {
            return Err(LockError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
        Ok(TryLock::Acquired) => {}
    }

    // 只有拿到独占 flock 才允许回收。同进程二次占用会 flock 成功，
    // 但 PID 仍存活，必须 Busy，不能删锁（否则会变成两把锁）。
    let existing = read_sidecar(path)
        .or_else(|| read_meta_from(&mut file))
        .unwrap_or_default();
    if existing.pid != 0 && process_is_alive(existing.pid) {
        let _ = FileExt::unlock(&file);
        return Ok(Reclaim::Busy(existing.pid));
    }

    drop(file);
    let _ = std::fs::remove_file(meta_path(path));
    match std::fs::remove_file(path) {
        Ok(()) => Ok(Reclaim::Retry),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(Reclaim::Retry),
        Err(err) if is_lock_busy(&err) => Ok(Reclaim::Busy(busy_pid(state_dir))),
        Err(source) => Err(LockError::Io {
            path: state_dir.to_path_buf(),
            source,
        }),
    }
}

/// 在不占用锁的情况下读取当前 Host 元数据。
/// Windows 整文件独占锁会挡住 `host.lock` 的 ReadFile，故优先读 sidecar。
pub fn peek_meta(state_dir: impl AsRef<Path>) -> Option<LockMeta> {
    let meta = recorded_meta(state_dir.as_ref())?;
    if meta.pid != 0 && process_is_alive(meta.pid) {
        Some(meta)
    } else {
        None
    }
}

fn recorded_meta(state_dir: &Path) -> Option<LockMeta> {
    let lock_path = HostLock::path(state_dir);
    read_sidecar(&lock_path).or_else(|| {
        let mut file = File::open(&lock_path).ok()?;
        read_meta_from(&mut file)
    })
}

fn meta_path(lock_path: &Path) -> PathBuf {
    lock_path.with_file_name(LOCK_META_FILE)
}

fn read_sidecar(lock_path: &Path) -> Option<LockMeta> {
    let raw = std::fs::read_to_string(meta_path(lock_path)).ok()?;
    parse_meta(&raw)
}

fn read_meta_from(file: &mut File) -> Option<LockMeta> {
    file.seek(SeekFrom::Start(0)).ok()?;
    let mut buf = String::new();
    file.read_to_string(&mut buf).ok()?;
    parse_meta(&buf)
}

fn parse_meta(raw: &str) -> Option<LockMeta> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

fn write_meta(file: &mut File, lock_path: &Path, meta: &LockMeta) -> std::io::Result<()> {
    let payload = serde_json::to_string(meta).unwrap_or_else(|_| "{}".to_string());
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(payload.as_bytes())?;
    file.flush()?;
    std::fs::write(meta_path(lock_path), payload)
}

fn is_lock_busy(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        ErrorKind::WouldBlock | ErrorKind::ResourceBusy | ErrorKind::PermissionDenied
    ) || matches!(err.raw_os_error(), Some(11 | 32 | 33 | 35))
}

fn process_is_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(target_os = "linux")]
    {
        return Path::new(&format!("/proc/{pid}")).is_dir();
    }
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        return unix_pid_alive(pid);
    }
    #[cfg(windows)]
    {
        return windows_pid_alive(pid);
    }
    #[allow(unreachable_code)]
    true
}

#[cfg(all(unix, not(target_os = "linux")))]
fn unix_pid_alive(pid: u32) -> bool {
    unsafe extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    const ESRCH: i32 = 3;
    let rc = unsafe { kill(pid as i32, 0) };
    if rc == 0 {
        true
    } else {
        std::io::Error::last_os_error().raw_os_error() != Some(ESRCH)
    }
}

#[cfg(windows)]
fn windows_pid_alive(pid: u32) -> bool {
    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        fn GetExitCodeProcess(handle: *mut std::ffi::c_void, code: *mut u32) -> i32;
    }
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
    const STILL_ACTIVE: u32 = 259;
    unsafe {
        let mut handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
        }
        if handle.is_null() {
            // 5 = ERROR_ACCESS_DENIED：进程在，只是无权查询。当作存活，避免误回收。
            return std::io::Error::last_os_error().raw_os_error() == Some(5);
        }
        let mut code = 0u32;
        let ok = GetExitCodeProcess(handle, &mut code);
        CloseHandle(handle);
        ok != 0 && code == STILL_ACTIVE
    }
}
