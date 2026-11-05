use super::*;
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::WaitForSingleObject,
};
fn ready(notice: &Notification) -> bool {
    let result = unsafe { WaitForSingleObject(HANDLE(notice.handle().as_raw_handle()), 0) };
    assert!(result == WAIT_OBJECT_0 || result == WAIT_TIMEOUT);
    result == WAIT_OBJECT_0
}
#[test]
fn execution_stop_is_terminal_but_only_authority_revocation_stops_cleanup() {
    for stop_before_notice in [false, true] {
        let mut grant = grant();
        let lease = grant.lease("https://localhost:1234", 0).unwrap();
        let sibling = grant.lease("https://localhost:1234", 0).unwrap();
        let authority = lease.revocation_notification().unwrap();
        if stop_before_notice {
            lease.stop_execution();
        }
        let execution = lease.execution_notification().unwrap();
        lease.stop_execution();
        assert!(ready(&execution));
        assert!(!ready(&authority));
        assert!(lease.execution_stopped());
        assert!(!lease.is_revoked());
        assert!(!sibling.execution_stopped());
        assert_eq!(lease.validate("https://localhost:1234", 99), Ok(()));
        drop(grant);
        assert!(ready(&authority));
        assert!(ready(&sibling.execution_notification().unwrap()));
        assert!(lease.is_revoked() && sibling.is_revoked());
        assert_eq!(
            lease.validate("https://localhost:1234", 99),
            Err(Error::Revoked)
        );
    }
}
#[test]
fn revocation_notice_is_shared_terminal_and_survives_lease_drop() {
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    assert_eq!(lease.expires_at_ms(), 100);
    let first = lease.revocation_notification().unwrap();
    let second = lease.revocation_notification().unwrap();
    assert_eq!(
        first.handle().as_raw_handle(),
        second.handle().as_raw_handle()
    );
    assert!(!ready(&first));
    drop(lease);
    assert!(ready(&first));
    assert!(ready(&second));
    assert!(ready(&first));
}
#[test]
fn revocation_before_registration_and_racing_registration_never_lose_wake() {
    for _ in 0..32 {
        let mut grant = grant();
        let lease = grant.lease("https://localhost:1234", 0).unwrap();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let worker_barrier = barrier.clone();
        let worker = std::thread::spawn(move || {
            worker_barrier.wait();
            let notice = lease.revocation_notification().unwrap();
            (lease, notice)
        });
        barrier.wait();
        drop(grant);
        let (lease, notice) = worker.join().unwrap();
        assert!(lease.is_revoked());
        assert!(ready(&notice));
    }
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    drop(grant);
    assert!(ready(&lease.revocation_notification().unwrap()));
}
