//! Precisely simulate single-slot intermediate states with CLAIMED/FINISHED; successful paths must pass through the real source hook on an owned HWND.
use super::*;
use crate::live_window_target::recovery::popup_layer::{
    fixture::{pump, settle, Fixture},
    wire, Recovery,
};

fn channel(f: &Fixture, lease: Lease) -> Channel {
    Channel::new(&Target::bind(f.popup.0).unwrap(), lease).unwrap()
}
fn pending(result: Result<()>) {
    assert_eq!(result.unwrap_err().code(), p::PENDING);
}

#[test]
fn posted_admission_and_peek_are_not_completion_but_removed_source_request_is() {
    let f = Fixture::new();
    let lease = f.show();
    let mut c = channel(&f, lease);
    pending(c.acquire());
    let request = c.pending.unwrap().request;
    assert!(request.result().is_none());
    assert!(!f.topmost());
    assert!(c.deadline().is_some());
    unsafe {
        let mut message = MSG::default();
        assert!(PeekMessageW(
            &mut message,
            Some(f.popup.0),
            native_layer_dispatch::message_id(),
            native_layer_dispatch::message_id(),
            PM_NOREMOVE
        )
        .as_bool());
    }
    assert_eq!(value(c.window.0, p::CLAIMED), 0);
    pending(c.acquire());
    pump();
    assert_eq!(request.result(), Some(S_OK));
    c.acquire().unwrap();
    assert!(c.pending.is_none() && c.acquired);
    assert!(c.deadline().is_none());
    assert!(f.topmost());
    settle(|| c.restore());
    assert!(!f.topmost());
    assert!(c.deadline().is_none());
}

#[test]
fn cancellation_before_claim_does_not_let_old_posted_packet_acquire() {
    let f = Fixture::new();
    let lease = f.show();
    let mut c = channel(&f, lease);
    pending(c.acquire());
    let old = c.pending.unwrap().request;
    pending(c.restore());
    let restore = c.pending.unwrap().request;
    assert_eq!(restore.operation, p::Operation::Restore);
    assert_ne!(restore.nonce, old.nonce);
    assert!(!old.matches());
    assert!(!old.claim().unwrap());
    old.finish(S_OK).unwrap();
    assert!(
        restore.result().is_none(),
        "late result cannot complete new identity"
    );
    pump();
    c.restore().unwrap();
    assert!(!f.topmost());
    assert_eq!(value(f.popup.0, MANAGED), 0);
}

#[test]
fn claimed_acquire_cancellation_keeps_original_slot_until_source_finishes() {
    let f = Fixture::new();
    let lease = f.show();
    let mut c = channel(&f, lease);
    pending(c.acquire());
    let request = c.pending.unwrap().request;
    assert!(request.claim().unwrap());
    for _ in 0..3 {
        pending(c.restore());
        assert_eq!(c.pending.unwrap().request, request);
        assert_eq!(value(c.window.0, p::COMMIT), 0);
        assert!(request.matches());
        assert!(c.deadline().is_some());
    }
    request.finish(E_ABORT).unwrap();
    pending(c.restore());
    let restore = c.pending.unwrap().request;
    assert_eq!(restore.operation, p::Operation::Restore);
    assert_ne!(restore.nonce, request.nonce);
    settle(|| c.restore());
    assert!(!f.topmost());
}

#[test]
fn claimed_timeout_reports_once_and_retains_identity_and_future_wakeup() {
    let f = Fixture::new();
    let lease = f.show();
    let mut c = channel(&f, lease);
    pending(c.acquire());
    let request = c.pending.unwrap().request;
    assert!(request.claim().unwrap());
    c.pending.as_mut().unwrap().deadline = Instant::now() - POLL;
    assert_eq!(c.acquire().unwrap_err().code(), ERROR_TIMEOUT.to_hresult());
    assert!(c.pending.unwrap().timed_out);
    for _ in 0..3 {
        pending(c.restore());
        assert_eq!(c.pending.unwrap().request, request);
        let now = Instant::now();
        assert!(c.deadline().unwrap() > now);
    }
    request.finish(E_ABORT).unwrap();
    settle(|| c.restore());
    assert!(c.pending.is_none());
}

#[test]
fn unclaimed_timeout_cancels_old_packet_without_late_acquire() {
    let f = Fixture::new();
    let lease = f.show();
    let mut c = channel(&f, lease);
    pending(c.acquire());
    let request = c.pending.unwrap().request;
    c.pending.as_mut().unwrap().deadline = Instant::now() - POLL;
    assert_eq!(c.acquire().unwrap_err().code(), ERROR_TIMEOUT.to_hresult());
    assert!(c.pending.is_none());
    assert!(!request.active());
    pump();
    assert!(!f.topmost());
    settle(|| c.restore());
}

#[test]
fn only_explicit_finished_busy_allows_new_nonce_after_retry_deadline() {
    let f = Fixture::new();
    let lease = f.show();
    let mut c = channel(&f, lease);
    pending(c.acquire());
    let request = c.pending.unwrap().request;
    assert!(request.claim().unwrap());
    request.finish(p::PENDING).unwrap();
    pending(c.acquire());
    assert!(c.pending.is_none());
    assert!(c.retry_at.is_some());
    c.retry_at = Some(Instant::now() + Duration::from_secs(5));
    pending(c.acquire());
    assert!(c.pending.is_none());
    c.retry_at = Some(Instant::now() - POLL);
    pending(c.acquire());
    assert_ne!(c.pending.unwrap().request.nonce, request.nonce);
    settle(|| c.acquire());
    assert!(f.topmost());
    settle(|| c.restore());
}

#[test]
fn old_instance_restore_never_downgrades_new_display_instance() {
    let f = Fixture::new();
    let first = f.show();
    let mut c = channel(&f, first);
    settle(|| c.acquire());
    let second = f.show();
    assert_ne!(first.instance, second.instance);
    f.borrow(second);
    c.restore().unwrap();
    assert!(f.topmost());
    assert_eq!(value(f.popup.0, MANAGED), second.instance);
    assert!(c.pending.is_none());
    assert!(c.deadline().is_none());
}

#[test]
fn independent_recovery_keeps_source_hooks_after_input_owner_and_control_are_gone() {
    let mut f = Fixture::new();
    let lease = f.show();
    let mut c = channel(&f, lease);
    settle(|| c.acquire());
    let recovery = Recovery::bind(Target::bind(f.popup.0).unwrap(), &wire(lease)).unwrap();
    drop(c);
    f.detach_input();
    f.owner.control(false).unwrap();
    assert!(!lease.valid() && lease.same_instance());
    pending(recovery.restore());
    assert!(recovery.deadline().is_some());
    settle(|| recovery.restore());
    assert!(!f.topmost());
    assert_eq!(value(f.popup.0, MANAGED), 0);
    assert!(recovery.deadline().is_none());
}

#[test]
fn dropping_request_window_cancels_queued_acquire_and_does_not_touch_source() {
    let f = Fixture::new();
    let lease = f.show();
    let mut c = channel(&f, lease);
    let window = c.window.0;
    pending(c.acquire());
    let request = c.pending.unwrap().request;
    drop(c);
    assert!(!unsafe { IsWindow(Some(window)) }.as_bool());
    assert!(!request.matches());
    pump();
    assert!(!f.topmost());
    assert!(lease.valid());
}
