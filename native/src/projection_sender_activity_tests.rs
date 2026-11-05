//! 控制 cancel 首次撤销到取得 state 的交错；新登记租约也必须在持久取消前被撤销。
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
    // 在持有 state 的受控夹具里重现 activate 在首次 stop 之后登记的精确中间态。
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
