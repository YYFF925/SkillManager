//! 同一 state-dir 只允许一个 Host 持有 host.lock。

use skill_manager_host::{HostLock, LockError};

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
