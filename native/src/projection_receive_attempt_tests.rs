//! 确定性 worker panic：不依赖网络竞态，沿真实 Job join/Attempt/Slot 验证回收交接。
use super::*;
use crate::projection_delivery::{
    accept::tests as png,
    journal::{tests::Directory, Store},
};

#[test]
fn accept_abandonment_panicked_worker_keeps_original_activity_until_job_join() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let origin = "https://127.0.0.1:46543";
    let mut grant = crate::device_authorization_lease::Grant::new(
        crate::device_session_client::tests::session(origin, u64::MAX),
    );
    let attempt = Arc::new(attempt::Attempt::default());
    let worker = attempt.clone();
    let original_store = store.clone();
    let mut job = Job {
        origin: origin.into(),
        lease: Arc::new(grant.lease(origin, 0).unwrap()),
        thread: Some(std::thread::spawn(move || {
            let _snapshot = worker
                .begin(&original_store, &png::invitation(origin), "device-1")
                .unwrap();
            panic!("synthetic accept failure after original intent activation")
        })),
        slot: Some(Slot::acquire().unwrap()),
        transferred: false,
        attempt,
        sender_attempt: Arc::new(sender_flow::Attempt::default()),
        _held: None,
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !job.thread.as_ref().unwrap().is_finished() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    let mut retry = store.pending().unwrap().pop().unwrap();
    assert_eq!(
        retry.activate(),
        Err(crate::projection_delivery::journal::Error::Busy)
    );
    assert!(job.take_abandoned().is_none());
    assert!(matches!(job.try_join(), Some(Err(Error::Panicked))));
    let abandoned = job.take_abandoned().unwrap();
    assert!(abandoned.same_record(&retry));
    drop(abandoned.queue_cleanup().unwrap());
    assert!(store.pending().unwrap()[0].cleanup_pending());
    assert!(job.take_abandoned().is_none());
}

fn panicked_recovery(drop_job: bool) {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let origin = "https://127.0.0.1:46543";
    let mut grant = crate::device_authorization_lease::Grant::new(
        crate::device_session_client::tests::session(origin, u64::MAX),
    );
    let ticket = store.begin(&png::invitation(origin), "device-1").unwrap();
    let mut retry = ticket.retry();
    let attempt = Arc::new(attempt::Attempt::default());
    let worker = attempt.clone();
    let mut job = Job {
        origin: origin.into(),
        lease: Arc::new(grant.lease(origin, 0).unwrap()),
        thread: Some(std::thread::spawn(move || {
            let _snapshot = worker.recover(ticket).unwrap();
            panic!("synthetic recovery failure after original intent activation")
        })),
        slot: Some(Slot::acquire().unwrap()),
        transferred: false,
        attempt,
        sender_attempt: Arc::new(sender_flow::Attempt::default()),
        _held: None,
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !job.thread.as_ref().unwrap().is_finished() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert_eq!(
        retry.activate(),
        Err(crate::projection_delivery::journal::Error::Busy)
    );
    assert!(job.take_abandoned().is_none());
    if drop_job {
        drop(job);
    } else {
        assert!(matches!(job.try_join(), Some(Err(Error::Panicked))));
        // recovered 的 fresh=false：回收只放许可，不排 cleanup 或自动解绑。
        assert!(job.take_abandoned().is_none());
    }
    retry.activate().unwrap();
    assert!(!store.pending().unwrap()[0].cleanup_pending());
}
#[test]
fn projection_display_recovery_panicked_worker_keeps_activity_until_job_join() {
    panicked_recovery(false);
}
#[test]
fn projection_display_recovery_panicked_worker_drop_reaps_without_cleanup() {
    panicked_recovery(true);
}

#[test]
fn projection_reattach_explicit_retry_reuses_original_intent_and_never_acquires_cleanup() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let origin = "https://loom.example";
    let original = store.begin(&png::accepted(origin), "device-1").unwrap();
    let before = std::fs::read(directory.0.join("projection-receive.json")).unwrap();
    let attempt = attempt::Attempt::default();
    let retried = attempt
        .reattach(&store, &png::accepted(origin), "device-1")
        .unwrap();
    assert!(retried.same_record(&original));
    assert!(attempt.abandon().is_none());
    assert_eq!(
        std::fs::read(directory.0.join("projection-receive.json")).unwrap(),
        before
    );
    assert!(!store.pending().unwrap()[0].cleanup_pending());
}

#[test]
fn completed_plain_metadata_job_still_revokes_untransferred_lease_on_drop() {
    let origin = "https://127.0.0.1:46543";
    let mut grant = crate::device_authorization_lease::Grant::new(
        crate::device_session_client::tests::session(origin, u64::MAX),
    );
    let lease = Arc::new(grant.lease(origin, 0).unwrap());
    let mut job = Job {
        origin: origin.into(),
        lease: lease.clone(),
        thread: Some(std::thread::spawn(|| {
            Ok(Completed::Metadata(Output::Rejected))
        })),
        slot: Some(Slot::acquire().unwrap()),
        transferred: false,
        attempt: Arc::new(attempt::Attempt::default()),
        sender_attempt: Arc::new(sender_flow::Attempt::default()),
        _held: None,
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !job.thread.as_ref().unwrap().is_finished() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(matches!(job.try_join(), Some(Ok(Output::Rejected))));
    assert!(!job.transferred);
    assert!(!lease.is_revoked());
    drop(job);
    assert!(lease.is_revoked());
}
