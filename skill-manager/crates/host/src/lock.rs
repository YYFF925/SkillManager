//! `state-dir/host.lock` 咨询锁：同一 state-dir 只允许一个 Host。
//!
//! 先用 `create_new` 占文件，再尽量加 flock。部分文件系统（overlay/NFS）上
//! flock 会对第二进程也成功，因此以锁文件中存活 PID 为最终占用依据。

use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt;
use serde::{Deserialize, Serialize};

/// 锁文件名（相对 state-dir）。
pub const LOCK_FILE: &str = "host.lock";

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

impl HostLock {
    pub fn path(state_dir: impl AsRef<Path>) -> PathBuf {
        state_dir.as_ref().join(LOCK_FILE)
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
                    let _ = file.try_lock_exclusive();
                    let meta = LockMeta {
                        pid: std::process::id(),
                        listen: None,
                        vault: None,
                    };
                    write_meta(&mut file, &meta).map_err(|source| LockError::Io {
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
            pid: 0,
        })
    }

    pub fn meta(&self) -> &LockMeta {
        &self.meta
    }

    pub fn path_ref(&self) -> &Path {
        &self.path
    }

    /// 更新锁文件中的监听地址 / Vault 路径，供 CLI 发现。
    pub fn update_meta(&mut self, listen: Option<String>, vault: Option<String>) -> std::io::Result<()> {
        self.meta.listen = listen;
        if vault.is_some() {
            self.meta.vault = vault;
        }
        write_meta(&mut self.file, &self.meta)
    }
}

impl Drop for HostLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
        let _ = std::fs::remove_file(&self.path);
    }
}

enum Reclaim {
    Busy(u32),
    Retry,
}

fn try_reclaim_or_busy(state_dir: &Path, path: &Path) -> Result<Reclaim, LockError> {
    let mut file = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Reclaim::Retry),
        Err(source) => {
            return Err(LockError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    if let Err(err) = file.try_lock_exclusive() {
        if is_lock_busy(&err) {
            let existing = read_meta_from(&mut file).unwrap_or_default();
            return Ok(Reclaim::Busy(existing.pid));
        }
    }

    let existing = read_meta_from(&mut file).unwrap_or_default();
    if existing.pid != 0 && process_is_alive(existing.pid) {
        return Ok(Reclaim::Busy(existing.pid));
    }

    drop(file);
    match std::fs::remove_file(path) {
        Ok(()) => Ok(Reclaim::Retry),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(Reclaim::Retry),
        Err(source) => Err(LockError::Io {
            path: state_dir.to_path_buf(),
            source,
        }),
    }
}

/// 在不占用锁的情况下读取当前 Host 元数据。
pub fn peek_meta(state_dir: impl AsRef<Path>) -> Option<LockMeta> {
    let path = HostLock::path(state_dir);
    let mut file = File::open(path).ok()?;
    read_meta_from(&mut file)
}

fn read_meta_from(file: &mut File) -> Option<LockMeta> {
    file.seek(SeekFrom::Start(0)).ok()?;
    let mut buf = String::new();
    file.read_to_string(&mut buf).ok()?;
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

fn write_meta(file: &mut File, meta: &LockMeta) -> std::io::Result<()> {
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(serde_json::to_string(meta).unwrap_or_else(|_| "{}".to_string()).as_bytes())?;
    file.flush()
}

fn is_lock_busy(err: &std::io::Error) -> bool {
    err.kind() == ErrorKind::WouldBlock || err.raw_os_error() == Some(libc_wouldblock())
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
    extern "C" {
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
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        fn GetExitCodeProcess(handle: *mut std::ffi::c_void, code: *mut u32) -> i32;
    }
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut code = 0u32;
        let ok = GetExitCodeProcess(handle, &mut code);
        CloseHandle(handle);
        ok != 0 && code == STILL_ACTIVE
    }
}

#[cfg(windows)]
fn libc_wouldblock() -> i32 {
    33 // ERROR_LOCK_VIOLATION
}

#[cfg(not(windows))]
fn libc_wouldblock() -> i32 {
    11 // EAGAIN / EWOULDBLOCK on Linux
}
