//! UI projection revocation/explicit retry; the same TLS test endpoint supplies background networking, without user processes/global input.
use super::super::extension_catalog_job::tests::{data, SERIAL};
use super::*;
use crate::loom_extension_session::test_peer as peer;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

fn until(owner: &mut Owner, ready: impl Fn(&View) -> bool) {
    let end = Instant::now() + Duration::from_secs(5);
    while !ready(&owner.view.borrow()) {
        // Source must never be rediscovered/replayed without an explicit request.
        owner.process_with(|| panic!("unrequested extension reconnect"));
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(2));
    }
}
impl Owner {
    pub(in crate::desktop) fn test_set_notify(&mut self, notify: Notify) {
        assert!(!self.job.is_busy());
        self.notify = notify;
    }
    pub(in crate::desktop) fn test_start(&mut self, source: super::super::capability_job::Source) {
        self.view.borrow_mut().requested = true;
        assert!(self.process_with(|| source));
    }
    pub(in crate::desktop) fn test_wait_finished(&self) {
        self.job.test_wait_finished();
    }
}
#[test]
fn failed_ui_wakeup_does_not_leave_a_request_or_start_a_worker() {
    let shared = Shared::default();
    assert!(unsafe { request_for(HWND(-2isize as *mut _), &shared) }.is_err());
    assert!(!shared.borrow().requested && shared.borrow().phase == Phase::Idle);
    let mut owner = Owner::new(shared, Arc::new(|| {}));
    assert!(!owner.process_with(|| panic!("failed wake must not start networking")));
}
#[test]
fn snapshots_reach_the_shared_view_and_disconnect_clears_it_without_automatic_reconnect() {
    let _serial = SERIAL.lock().unwrap();
    let (advance, waiting) = mpsc::channel();
    let (identity, server) = peer::peer(move |peer| {
        peer::handshake(peer, peer::handshake_data());
        waiting.recv_timeout(Duration::from_secs(5)).unwrap();
        peer::send(peer, peer::updated(data(8)));
        waiting.recv_timeout(Duration::from_secs(5)).unwrap();
        peer.close(None).unwrap();
    });
    let shared = Shared::default();
    shared.borrow_mut().requested = true;
    let mut owner = Owner::new(shared.clone(), Arc::new(|| {}));
    assert!(owner.process_with(|| super::super::capability_job::Source::Fixed(identity)));
    until(&mut owner, |v| v.phase == Phase::Ready);
    assert_eq!(shared.borrow().snapshot.as_ref().unwrap().generation(), 7);
    advance.send(()).unwrap();
    until(&mut owner, |v| {
        v.snapshot.as_ref().is_some_and(|s| s.generation() == 8)
    });
    advance.send(()).unwrap();
    until(&mut owner, |v| v.phase == Phase::Failed);
    assert!(shared.borrow().snapshot.is_none());
    // Failed is visible upon revocation; no-reconnect assertions must allow one Stopped settlement after thread-tail exit.
    if owner.job.is_busy() {
        owner.test_wait_finished();
        owner.process_with(|| panic!("no automatic retry"));
    }
    for _ in 0..8 {
        assert!(!owner.process_with(|| panic!("no automatic retry")));
    }
    server.join().unwrap();
}
#[test]
fn shutdown_clears_all_shared_authority_and_rejects_requests_from_remaining_windows() {
    let _serial = SERIAL.lock().unwrap();
    let (identity, server) = peer::peer(|peer| {
        peer::handshake(peer, peer::handshake_data());
        assert!(peer.read().is_err());
    });
    let shared = Shared::default();
    shared.borrow_mut().requested = true;
    let mut owner = Owner::new(shared.clone(), Arc::new(|| {}));
    owner.process_with(|| super::super::capability_job::Source::Fixed(identity));
    until(&mut owner, |v| v.phase == Phase::Ready);
    owner.shutdown();
    assert!(shared.borrow().snapshot.is_none() && shared.borrow().phase == Phase::Closed);
    unsafe {
        request_for(HWND(-2isize as *mut _), &shared).unwrap();
    }
    assert!(!shared.borrow().requested);
    assert!(!owner.process_with(|| panic!("closed owner must not restart")));
    server.join().unwrap();
}

#[test]
fn revoked_tail_preserves_explicit_retry_until_prior_worker_is_reaped() {
    use crate::desktop::loom_execution_tasks::reap_fixture::{Gate, WAIT};
    use std::sync::atomic::{AtomicUsize, Ordering};
    let _serial = SERIAL.lock().unwrap();
    let (advance, waiting) = mpsc::channel();
    let (identity, server) = peer::peer(move |p| {
        peer::handshake(p, peer::handshake_data());
        waiting.recv_timeout(WAIT).unwrap();
        p.close(None).unwrap();
    });
    let gate = Gate::new(|| {});
    let tail = gate.notify.clone();
    let count = AtomicUsize::new(0);
    let notify = Arc::new(move || {
        if count.fetch_add(1, Ordering::AcqRel) == 1 {
            tail();
        }
    });
    let shared = Shared::default();
    let mut owner = Owner::new(shared.clone(), notify);
    owner.test_start(super::super::capability_job::Source::Fixed(identity));
    until(&mut owner, |v| v.phase == Phase::Ready);
    advance.send(()).unwrap();
    gate.noticed.recv_timeout(WAIT).unwrap();
    assert!(owner.process_with(|| panic!("no automatic retry")));
    assert!(shared.borrow().snapshot.is_none() && shared.borrow().phase == Phase::Failed);
    assert!(!owner.process_with(|| panic!("no automatic retry")));
    // Simulate explicit intent successfully enqueued by request_for; notification tails must not consume it.
    shared.borrow_mut().requested = true;
    assert!(!owner.process_with(|| panic!("old worker still owned")));
    assert!(shared.borrow().requested);
    gate.release.send(()).unwrap();
    owner.test_wait_finished();
    server.join().unwrap();
    let (identity, server) = peer::peer(|p| {
        peer::handshake(p, peer::handshake_data());
        assert!(p.read().is_err());
    });
    assert!(owner.process_with(|| super::super::capability_job::Source::Fixed(identity)));
    assert!(!shared.borrow().requested && shared.borrow().phase == Phase::Loading);
    until(&mut owner, |v| v.phase == Phase::Ready);
    assert!(owner.deadline().is_none());
    owner.shutdown();
    server.join().unwrap();
}
