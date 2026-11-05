//! 单槽/Stop/重连和后继授权；所有执行资格由同一不可逆gate围住。
use super::*;
use crate::art_live_trigger::tests as f;
fn queue() -> Queue {
    let q = Queue::default();
    q.configure_host(Host {
        binding: f::binding(),
        identity: f::identity(),
    });
    q
}
fn configure(q: &Queue, revision: u64, id: &str) -> Registration {
    let mut c = f::condition();
    c.revision = revision;
    assert!(q.submit(c, id.into()));
    let Command::Configure(r) = q.take_command().unwrap() else {
        panic!("configure expected")
    };
    q.confirm(&r, true);
    r
}
#[test]
fn single_slot_one_delivery_one_receipt_and_explicit_new_authorization() {
    let q = queue();
    let r = configure(&q, 1, "auth:1");
    assert!(q.publish(&r, f::delivery()));
    assert!(!q.publish(&r, f::delivery()));
    assert!(!q.submit(f::condition(), "auth:2".into()));
    let t = q.take_delivery().unwrap();
    assert!(q.take_delivery().is_none());
    assert!(t.complete(Receipt::NoResult));
    assert!(!t.complete(Receipt::Failed));
    assert!(matches!(q.take_receipt().unwrap().2, Receipt::NoResult));
    assert!(q.take_receipt().is_none());
    q.acknowledged();
    assert!(q.poll_registration().is_some());
    q.stop();
    assert!(!t.current(t.binding()));
    let Command::Stop(old) = q.take_command().unwrap() else {
        panic!("stop expected")
    };
    q.confirm(&old, false);
    let r2 = configure(&q, 2, "auth:2");
    assert!(r2.gate.active());
    assert!(!r.gate.active());
    assert!(!q.publish(&r, f::delivery()));
    let mut d = f::delivery();
    d.condition_revision = 2;
    assert!(q.publish(&r2, d));
}
#[test]
fn stop_during_configure_and_late_confirmation_cannot_restore_authorization() {
    let q = queue();
    assert!(q.submit(f::condition(), "auth:1".into()));
    let Command::Configure(r) = q.take_command().unwrap() else {
        panic!("configure expected")
    };
    assert!(q.stop());
    q.confirm(&r, true);
    assert_eq!(q.phase(), Phase::Pending);
    assert!(q.poll_registration().is_none());
    assert!(!q.publish(&r, f::delivery()));
    assert!(matches!(q.take_command(), Some(Command::Stop(_))));
    q.confirm(&r, false);
    let r2 = configure(&q, 2, "auth:2");
    q.confirm(&r, true);
    assert!(r2.gate.active());
    assert_eq!(q.registration().unwrap().0.authorization_id, "auth:2");
}
#[test]
fn recovery_and_close_cancel_old_ticket_and_never_reenable_old_gate() {
    let q = queue();
    let r = configure(&q, 1, "auth:1");
    assert!(q.publish(&r, f::delivery()));
    let t = q.take_delivery().unwrap();
    q.pause(true);
    assert!(!t.current(t.binding()));
    let mut c = f::condition();
    c.revision = 2;
    assert!(!q.submit(c.clone(), "auth:2".into()));
    q.pause(false);
    assert!(q.poll_registration().is_none());
    let r2 = configure(&q, 2, "auth:2");
    q.close();
    assert!(!r2.gate.active());
    c.revision = 3;
    assert!(!q.submit(c, "auth:3".into()));
    assert!(q.host().is_none());
    q.configure_host(Host {
        binding: f::binding(),
        identity: f::identity(),
    });
    assert!(q.host().is_none());
}
#[test]
fn cancelled_or_expired_slot_is_released_but_old_payload_is_not_rebuilt() {
    let q = queue();
    let r = configure(&q, 1, "auth:1");
    assert!(q.publish(&r, f::delivery()));
    let t = q.take_delivery().unwrap();
    r.gate.stop();
    q.expire();
    assert!(q.take_receipt().is_none());
    assert!(!t.complete(Receipt::NoResult));
    assert!(q.state().ticket.is_none());
    configure(&q, 2, "auth:2");
}
#[test]
fn overlapping_media_and_control_recovery_require_both_to_finish() {
    let q = crate::live_relay_control_runtime::interaction::Queue::default();
    q.local_triggers.configure_host(Host {
        binding: f::binding(),
        identity: f::identity(),
    });
    q.media_recovering(true);
    q.control_recovering(true);
    q.media_recovering(false);
    assert!(!q.local_triggers.submit(f::condition(), "auth:1".into()));
    q.control_recovering(false);
    assert!(q.local_triggers.submit(f::condition(), "auth:1".into()));
}
