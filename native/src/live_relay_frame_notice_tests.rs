//! Win32 帧合并通知：按状态锁复位，无定时轮询，无通知队列增长。
use super::*;
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::WaitForSingleObject,
};

fn signaled(session: &ViewerSession) -> bool {
    let status = unsafe { WaitForSingleObject(HANDLE(session.change_handle().as_raw_handle()), 0) };
    assert!(status == WAIT_OBJECT_0 || status == WAIT_TIMEOUT);
    status == WAIT_OBJECT_0
}
fn beginning() -> Cursor {
    Cursor {
        epoch: 1,
        frame_id: 0,
    }
}

#[test]
fn changes_coalesce_and_poll_takes_latest_frame_without_losing_next_wake() {
    let (owner, snapshot) = prepared();
    assert!(!signaled(&owner));
    owner.activate(&snapshot).unwrap();
    assert!(signaled(&owner));
    assert!(owner.poll_frame(beginning()).unwrap().is_none());
    assert!(!signaled(&owner));
    let permit = owner.admit(1).unwrap();
    for id in [1, 2] {
        owner
            .commit(&permit, frame(id, vec![255; 4].into()), false)
            .unwrap();
    }
    assert!(signaled(&owner));
    let latest = owner.poll_frame(beginning()).unwrap().unwrap();
    assert_eq!(latest.metadata().frame_id, 2);
    assert!(!signaled(&owner));
    assert!(owner
        .take(Cursor {
            epoch: 1,
            frame_id: 1
        })
        .unwrap()
        .is_none());
    owner
        .commit(&permit, frame(3, vec![255; 4].into()), false)
        .unwrap();
    assert!(signaled(&owner));
    assert_eq!(
        owner
            .poll_frame(latest.metadata().into())
            .unwrap()
            .unwrap()
            .metadata()
            .frame_id,
        3
    );
    assert!(!signaled(&owner));
}
#[test]
fn suspend_and_close_notify_even_without_new_pixels_and_stale_work_does_not() {
    let (owner, snapshot) = prepared();
    owner.activate(&snapshot).unwrap();
    let old = owner.admit(1).unwrap();
    owner.poll_frame(beginning()).unwrap();
    owner.suspend().unwrap();
    assert!(signaled(&owner));
    assert!(matches!(
        owner.poll_frame(beginning()),
        Err(Error::Suspended)
    ));
    assert!(!signaled(&owner));
    owner.activate(&snapshot).unwrap();
    owner.poll_frame(beginning()).unwrap();
    assert!(owner
        .commit(&old, frame(1, vec![255; 4].into()), false)
        .is_err());
    assert!(!signaled(&owner));
    owner.close();
    assert!(signaled(&owner));
    assert!(matches!(owner.poll_frame(beginning()), Err(Error::Closed)));
    assert!(!signaled(&owner));
}
#[test]
fn terminal_snapshot_and_current_connection_revoke_signal_consumers() {
    for terminal_snapshot in [true, false] {
        let (owner, _) = prepared();
        let snapshot = Snapshot::parse(&snapshot(true, false), "live:test").unwrap();
        owner.activate(&snapshot).unwrap();
        let permit = owner.admit(1).unwrap();
        owner.poll_frame(beginning()).unwrap();
        if terminal_snapshot {
            owner
                .observe_terminal(&super::snapshot(true, true))
                .unwrap();
        } else {
            owner.close_if_current(&permit).unwrap();
        }
        assert!(signaled(&owner));
        assert!(matches!(owner.poll_frame(beginning()), Err(Error::Closed)));
        assert!(!signaled(&owner));
    }
}
#[test]
fn publication_racing_reset_is_either_consumed_or_still_signaled() {
    for _ in 0..64 {
        let (owner, snapshot) = prepared();
        owner.activate(&snapshot).unwrap();
        owner.poll_frame(beginning()).unwrap();
        let permit = owner.admit(1).unwrap();
        let owner = Arc::new(owner);
        let writer = owner.clone();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let other = barrier.clone();
        let worker = std::thread::spawn(move || {
            other.wait();
            writer
                .commit(&permit, frame(1, vec![255; 4].into()), false)
                .unwrap();
        });
        barrier.wait();
        let consumed = owner.poll_frame(beginning()).unwrap().is_some();
        worker.join().unwrap();
        if consumed {
            assert!(!signaled(&owner));
        } else {
            assert!(signaled(&owner));
            assert!(owner.poll_frame(beginning()).unwrap().is_some());
        }
        assert!(!signaled(&owner));
    }
}
