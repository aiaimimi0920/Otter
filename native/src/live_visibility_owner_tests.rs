use super::*;
use crate::live_recovery_wire::Restoration;
use std::{
    os::windows::io::AsRawHandle,
    time::{Duration, Instant},
};
use windows::{
    core::w,
    Win32::{Foundation::*, System::Threading::WaitForSingleObject, UI::WindowsAndMessaging::*},
};

struct Window(HWND);
impl Window {
    fn new() -> Self {
        Self(unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("visibility owner fixture"),
                WS_POPUP,
                20,
                20,
                32,
                32,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        })
    }
    fn target(&self) -> Target {
        Target::bind(self.0).unwrap()
    }
    fn record(&self) -> crate::live_recovery_wire::Record {
        Snapshot::read(&self.target())
            .unwrap()
            .recovery_record()
            .unwrap()
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
fn owner() -> Owner {
    Owner::fixture()
}
fn advance(owner: &mut Owner, mut done: impl FnMut(&Owner) -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        owner.poll().unwrap();
        if done(owner) {
            return;
        }
        assert!(Instant::now() < end, "Visibility owner timed out");
        let handle = owner
            .completion_handle()
            .or_else(|| owner.child_handle())
            .unwrap();
        let result = unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 1000) };
        assert!(result == WAIT_OBJECT_0 || result == WAIT_TIMEOUT);
    }
}

#[test]
fn shared_source_uses_one_registration_and_last_session_restores_and_reaps() {
    let window = Window::new();
    let before = window.record();
    let mut owner = owner();
    let first = owner.acquire(window.target()).unwrap();
    let second = owner.acquire(window.target()).unwrap();
    assert_eq!(owner.sources.len(), 1);
    first.poll().unwrap();
    assert_eq!(window.record(), before, "No hiding before registration ACK");
    advance(&mut owner, |_| {
        first.poll().unwrap();
        second.poll().unwrap();
        first.source.armed.get()
    });
    let Restoration::Window(hidden) = window.record().restoration else {
        panic!("window fixture")
    };
    assert_eq!(hidden.layered.unwrap().alpha, 1);
    drop(first);
    assert!(second.lease.has_recovery());
    assert_ne!(window.record(), before);
    drop(second);
    assert_eq!(window.record(), before);
    assert!(
        owner.acquire(window.target()).is_err(),
        "Pending retirement blocks new generation"
    );
    advance(&mut owner, Owner::finished);
    assert!(owner.journal.is_none());
}

#[test]
fn cancelled_pending_session_and_owner_close_do_not_hide_source() {
    let window = Window::new();
    let before = window.record();
    let mut owner = owner();
    let protection = owner.acquire(window.target()).unwrap();
    drop(protection);
    owner.close();
    advance(&mut owner, Owner::finished);
    assert_eq!(window.record(), before);
    assert!(owner.acquire(window.target()).is_err());
}

#[test]
fn unarmed_source_cannot_be_reacquired_between_retirement_request_and_ack() {
    let window = Window::new();
    let target = window.target();
    let before = window.record();
    let mut owner = owner();
    let abandoned = owner.acquire(target.clone()).unwrap();
    let ticket = abandoned.source.ticket.borrow().clone();
    advance(&mut owner, |_| ticket.ready()); // Registration completed, but the original consumer never attempted hiding.
    assert!(!abandoned.source.armed.get());
    drop(abandoned);
    // As in Owner::poll's no-consumer branch, record retirement intent first and await ACK later.
    assert_eq!(ticket.retire().unwrap_err().code(), RETIREMENT_PENDING);
    assert!(!ticket.retired());
    assert!(matches!(owner.acquire(target.clone()), Err(error)
        if error.to_string() == "Live recovery source is retiring"));
    assert_eq!(window.record(), before);
    advance(&mut owner, Owner::finished);
    assert!(ticket.retired());

    owner.journal = Some(Journal::fixture());
    let fresh = owner.acquire(target).unwrap();
    advance(&mut owner, |_| {
        fresh.poll().unwrap();
        fresh.source.armed.get()
    });
    assert_ne!(window.record(), before);
    drop(fresh);
    advance(&mut owner, Owner::finished);
    assert_eq!(window.record(), before);
}

#[test]
fn vanished_protected_source_finishes_without_retaining_idle_helper() {
    let window = Window::new();
    let mut owner = owner();
    let protection = owner.acquire(window.target()).unwrap();
    advance(&mut owner, |_| {
        protection.poll().unwrap();
        protection.source.armed.get()
    });
    drop(window);
    drop(protection);
    owner.close();
    advance(&mut owner, Owner::finished);
}

#[test]
fn explicit_reclaim_restores_all_peers_without_ending_live_or_rehiding() {
    use crate::live_reclaim::IntentTracker;
    let window = Window::new();
    let before = window.record();
    let hwnd = window.0 .0 as usize;
    let mut owner = owner();
    let first = owner.acquire(window.target()).unwrap();
    let second = owner.acquire(window.target()).unwrap();
    owner.observe_reclaim(2, None, 10, |_| false).unwrap();
    advance(&mut owner, |_| {
        first.poll().unwrap();
        first.source.armed.get()
    });
    owner.observe_reclaim(hwnd, None, 20, |_| false).unwrap();
    assert!(
        !first.source.recalled.get(),
        "Foreground alone cannot authorize reclaim"
    );
    owner.observe_reclaim(2, None, 21, |_| false).unwrap();
    let mut tracker = IntentTracker::default();
    tracker.record(30, 2);
    owner
        .observe_reclaim(hwnd, tracker.snapshot(31), 31, |_| false)
        .unwrap();
    assert!(first.source.recalled.get());
    assert_eq!(first.lease.peer_count(), 2);
    assert_eq!(window.record(), before);
    assert!(
        first.lease.has_recovery(),
        "Retirement ACK is still pending"
    );
    advance(&mut owner, |owner| owner.journal.is_none());
    assert!(!owner.finished(), "Live peers still own the source");
    assert!(!first.lease.has_recovery());
    for _ in 0..3 {
        first.poll().unwrap();
        second.poll().unwrap();
        owner.poll().unwrap();
        assert_eq!(
            window.record(),
            before,
            "Recalled peers must not hide again"
        );
    }
    drop(first);
    assert_eq!(second.lease.peer_count(), 1);
    drop(second);
    owner.poll().unwrap();
    assert!(owner.finished());
}

#[test]
fn preexisting_and_cancelled_intents_do_not_reclaim_hidden_source() {
    use crate::live_reclaim::IntentTracker;
    let window = Window::new();
    let mut owner = owner();
    let protection = owner.acquire(window.target()).unwrap();
    let mut tracker = IntentTracker::default();
    tracker.record(10, 2);
    owner
        .observe_reclaim(2, tracker.snapshot(10), 10, |_| false)
        .unwrap();
    // New gestures while registration is in flight are also pre-hide intent; they must not trigger immediate recall after ACK.
    tracker.record(11, 2);
    owner
        .observe_reclaim(2, tracker.snapshot(11), 11, |_| false)
        .unwrap();
    advance(&mut owner, |_| {
        protection.poll().unwrap();
        protection.source.armed.get()
    });
    owner
        .observe_reclaim(window.0 .0 as usize, tracker.snapshot(12), 12, |_| false)
        .unwrap();
    assert!(!protection.source.recalled.get());
    tracker.record(20, 2);
    tracker.cancel();
    owner.observe_reclaim(2, None, 20, |_| false).unwrap();
    owner
        .observe_reclaim(window.0 .0 as usize, tracker.snapshot(21), 21, |_| false)
        .unwrap();
    assert!(!protection.source.recalled.get());
    drop(protection);
    owner.close();
    advance(&mut owner, Owner::finished);
}

#[test]
fn explicit_b_input_rearms_recalled_minimized_source_for_all_peers() {
    let window = Window::new();
    let mut owner = owner();
    let first = owner.acquire(window.target()).unwrap();
    let second = owner.acquire(window.target()).unwrap();
    advance(&mut owner, |_| {
        first.poll().unwrap();
        first.source.armed.get()
    });
    first.source.recalled.set(true);
    advance(&mut owner, |owner| owner.journal.is_none());
    unsafe {
        let _ = ShowWindow(window.0, SW_SHOWMINNOACTIVE);
    }
    assert!(unsafe { IsIconic(window.0) }.as_bool());
    let minimized = window.record();
    assert!(owner.resume_minimized(&first).unwrap());
    assert!(
        unsafe { IsIconic(window.0) }.as_bool(),
        "no mutation before new registration ACK"
    );
    advance(&mut owner, |_| {
        second.poll().unwrap();
        second.source.armed.get()
    });
    assert!(!unsafe { IsIconic(window.0) }.as_bool());
    assert!(!first.recalled());
    assert!(Rc::ptr_eq(&first.source, &second.source));
    assert_eq!(
        Snapshot::read(&window.target())
            .unwrap()
            .layered
            .unwrap()
            .alpha,
        1
    );
    drop(first);
    second.poll().unwrap();
    drop(second);
    owner.close();
    advance(&mut owner, Owner::finished);
    let restored = window.record();
    let Restoration::Window(restored) = restored.restoration else {
        panic!("window fixture")
    };
    let Restoration::Window(minimized) = minimized.restoration else {
        panic!("window fixture")
    };
    // Expand the minimized source under the existing return contract; validate a fresh snapshot, not its invisible state before first hiding.
    assert!(!restored.iconic);
    assert_eq!(restored.visible, minimized.visible);
    assert_eq!(restored.extended_style, minimized.extended_style);
    assert_eq!(restored.normal_position, minimized.normal_position);
    assert_eq!(restored.layered, minimized.layered);
}
