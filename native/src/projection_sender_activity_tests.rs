//! Control interleaving between cancel's first revocation and state acquisition; newly registered leases must also revoke before persistent cancellation.
use super::*;
use crate::device_authorization_lease::Grant;
#[test]
fn sender_cancel_rechecks_registration_after_state_lock_before_durable_marker() {
    let (directory, store) = super::super::tests::setup();
    let intent =
        super::super::tests::prepare(&store, &directory, &super::super::tests::selection(1))
            .unwrap();
    let mut grant = Grant::new(crate::device_session_client::tests::session(
        intent.envelope().origin(),
        u64::MAX,
    ));
    let old = Arc::new(grant.lease(intent.envelope().origin(), 0).unwrap());
    let active = Request::new(intent.clone(), directory.identity())
        .activate(old.clone(), false)
        .unwrap();
    let state = store.0.state.lock().unwrap();
    let thread_store = store.clone();
    let thread_intent = intent.clone();
    let task = std::thread::spawn(move || thread_store.cancel(&thread_intent));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !old.is_revoked() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    drop(active);
    let replacement = Arc::new(grant.lease(intent.envelope().origin(), 0).unwrap());
    // Reproduce the exact intermediate state where activate registers after first stop in a controlled state-lock-holding fixture.
    store.0.activity.0.lock().unwrap().insert(
        intent.record.selection.clone(),
        Entry {
            envelope: intent.envelope().clone(),
            lease: Arc::downgrade(&replacement),
        },
    );
    drop(state);
    task.join().unwrap().unwrap();
    assert!(replacement.is_revoked());
    assert!(store.pending(&AtomicBool::new(false)).unwrap()[0].cancelled());
}
