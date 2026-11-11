//! 同一 state-dir 只允许一个 Host 持有 host.lock。

use std::fs::OpenOptions;
use std::io::Write;

use fs4::fs_std::FileExt;
use skill_manager_host::lock::peek_meta;
use skill_manager_host::{HostLock, LockError, LockMeta};

#[test]
fn second_instance_fails_with_busy() {
    let dir = tempfile::tempdir().unwrap();
    let first = HostLock::try_acquire(dir.path()).expect("first lock");
    let second = HostLock::try_acquire(dir.path());
    match second {
        Err(LockError::Busy { pid, .. }) => {
            assert_eq!(pid, std::process::id(), "busy pid should be the holder");
        }
        other => panic!("expected Busy, got {other:?}"),
    }
    drop(first);
    let third = HostLock::try_acquire(dir.path());
    assert!(third.is_ok(), "lock should be reusable after drop: {third:?}");
}

#[test]
fn peek_meta_while_lock_held_returns_pid_and_listen() {
    let dir = tempfile::tempdir().unwrap();
    let mut first = HostLock::try_acquire(dir.path()).expect("first lock");
    first
        .update_meta(Some("127.0.0.1:55754".into()), Some("/vault".into()))
        .unwrap();
    let meta = peek_meta(dir.path()).expect("peek must work while flock is held");
    assert_eq!(meta.pid, std::process::id());
    assert_eq!(meta.listen.as_deref(), Some("127.0.0.1:55754"));
    assert_eq!(meta.vault.as_deref(), Some("/vault"));
    let second = HostLock::try_acquire(dir.path());
    match second {
        Err(LockError::Busy { pid, .. }) => assert_eq!(pid, std::process::id()),
        other => panic!("second serve must fail while first holds lock: {other:?}"),
    }
}

#[test]
fn peek_meta_uses_sidecar_when_lock_file_is_exclusively_locked() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = dir.path().join("host.lock");
    let mut lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&lock_path)
        .unwrap();
    lock.write_all(b"not-json").unwrap();
    lock.flush().unwrap();
    assert!(
        lock.try_lock_exclusive().expect("lock host.lock"),
        "test process should hold exclusive flock"
    );
    let expected = LockMeta {
        pid: std::process::id(),
        listen: Some("127.0.0.1:9".into()),
        vault: None,
    };
    std::fs::write(
        dir.path().join("host.lock.meta"),
        serde_json::to_string(&expected).unwrap(),
    )
    .unwrap();
    let peeked = peek_meta(dir.path()).expect("sidecar must be readable without unlocking host.lock");
    assert_eq!(peeked, expected);

    let second = HostLock::try_acquire(dir.path());
    match second {
        Err(LockError::Busy { pid, .. }) => {
            assert_eq!(pid, std::process::id());
        }
        other => panic!("contended flock must not reclaim; got {other:?}"),
    }
    drop(lock);
}
