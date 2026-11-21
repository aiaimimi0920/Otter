//! 实际 TLS/WS 订阅的空闲推送、单槽合并、失效与中断；不访问用户配置或真实 Loom。
use super::*;
use crate::loom_extension_session::test_peer as peer;
use serde_json::json;
use std::{
    sync::{atomic::AtomicUsize, mpsc},
    time::{Duration, Instant},
};

pub(in crate::desktop) static SERIAL: Mutex<()> = Mutex::new(());
fn next(job: &mut Job) -> Event {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(event) = job.poll() {
            return event;
        }
        assert!(Instant::now() < end, "extension owner did not publish");
        thread::sleep(Duration::from_millis(2));
    }
}
fn snapshot(event: Event) -> Arc<Snapshot> {
    match event {
        Event::Snapshot(snapshot) => snapshot,
        Event::Stopped(error) => panic!("unexpected terminal: {error:?}"),
    }
}
pub(in crate::desktop) fn data(generation: u64) -> serde_json::Value {
    let mut value = crate::loom_extension_snapshot::tests::fixture();
    value["generation"] = json!(generation);
    value
}
fn context() -> crate::extension_when::Context<'static> {
    crate::extension_when::Context {
        kind: Some(crate::extension_when::UnitKind::Sticker),
        has_image: true,
        attachment_types: &[],
    }
}
#[test]
fn idle_catalog_updates_arrive_without_an_rpc_and_revoke_the_old_command() {
    let _serial = SERIAL.lock().unwrap();
    let (advance, waiting) = mpsc::channel();
    let (identity, server) = peer::peer(move |peer| {
        peer::handshake(peer, peer::handshake_data());
        waiting.recv_timeout(Duration::from_secs(5)).unwrap();
        let mut revoked = data(8);
        revoked["plugins"][0]["trustStatus"] = json!("revoked");
        peer::send(peer, peer::updated(revoked));
        // No command/poll/keepalive request is emitted by the subscriber.
        assert!(peer.read().is_err());
    });
    let mut job = Job::default();
    job.start(Source::Fixed(identity), Arc::new(|| {})).unwrap();
    let original = snapshot(next(&mut job));
    assert_eq!(original.generation(), 7);
    assert!(original
        .command("neuro.official/ocr.toggle-overlay", &context())
        .is_some());
    advance.send(()).unwrap();
    let next = snapshot(next(&mut job));
    assert_eq!(next.generation(), 8);
    assert!(next
        .command("neuro.official/ocr.toggle-overlay", &context())
        .is_none());
    assert!(!Arc::ptr_eq(&original, &next));
    job.shutdown();
    server.join().unwrap();
}
#[test]
fn push_flood_replaces_one_slot_instead_of_queueing_old_snapshots() {
    let _serial = SERIAL.lock().unwrap();
    let (advance, waiting) = mpsc::channel();
    let (identity, server) = peer::peer(move |peer| {
        peer::handshake(peer, peer::handshake_data());
        waiting.recv_timeout(Duration::from_secs(5)).unwrap();
        for generation in 8..=63 {
            peer::send(peer, peer::updated(data(generation)));
        }
        assert!(peer.read().is_err());
    });
    let notices = Arc::new(AtomicUsize::new(0));
    let count = notices.clone();
    let mut job = Job::default();
    job.start(
        Source::Fixed(identity),
        Arc::new(move || {
            count.fetch_add(1, Ordering::AcqRel);
        }),
    )
    .unwrap();
    assert_eq!(snapshot(next(&mut job)).generation(), 7);
    advance.send(()).unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    while notices.load(Ordering::Acquire) < 57 {
        assert!(Instant::now() < end);
        thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(snapshot(next(&mut job)).generation(), 63);
    assert!(job.poll().is_none());
    job.shutdown();
    server.join().unwrap();
}
#[test]
fn terminal_connection_discards_an_unread_snapshot_before_publishing_completion() {
    let _serial = SERIAL.lock().unwrap();
    let (identity, server) = peer::peer(|peer| {
        peer::handshake(peer, peer::handshake_data());
        peer.close(None).unwrap();
    });
    let mut job = Job::default();
    job.start(Source::Fixed(identity), Arc::new(|| {})).unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    while !job.running.as_ref().unwrap().ready.load(Ordering::Acquire) {
        assert!(Instant::now() < end);
        thread::sleep(Duration::from_millis(2));
    }
    assert!(job
        .running
        .as_ref()
        .unwrap()
        .mailbox
        .lock()
        .unwrap()
        .snapshot
        .is_none());
    assert!(matches!(
        next(&mut job),
        Event::Stopped(Err(Error::Exchange(ExchangeError::Closed)))
    ));
    assert!(!job.is_busy() && job.poll().is_none());
    server.join().unwrap();
}
#[test]
fn malformed_push_fails_closed_and_never_publishes_an_unvalidated_generation() {
    let _serial = SERIAL.lock().unwrap();
    let (advance, waiting) = mpsc::channel();
    let (identity, server) = peer::peer(move |peer| {
        peer::handshake(peer, peer::handshake_data());
        waiting.recv_timeout(Duration::from_secs(5)).unwrap();
        let mut invalid = data(8);
        invalid["plugins"][0]["scopeId"] = json!("wrong");
        peer::send(peer, peer::updated(invalid));
        assert!(peer.read().is_err());
    });
    let mut job = Job::default();
    job.start(Source::Fixed(identity), Arc::new(|| {})).unwrap();
    assert_eq!(snapshot(next(&mut job)).generation(), 7);
    advance.send(()).unwrap();
    assert!(matches!(
        next(&mut job),
        Event::Stopped(Err(Error::Exchange(ExchangeError::Snapshot(_))))
    ));
    assert!(job.poll().is_none());
    server.join().unwrap();
}
#[test]
fn idle_shutdown_interrupts_blocking_receive_and_releases_the_only_worker_permit() {
    let _serial = SERIAL.lock().unwrap();
    for _ in 0..2 {
        let (identity, server) = peer::peer(|peer| {
            peer::handshake(peer, peer::handshake_data());
            assert!(peer.read().is_err());
        });
        let mut job = Job::default();
        job.start(Source::Fixed(identity.clone()), Arc::new(|| {}))
            .unwrap();
        assert_eq!(snapshot(next(&mut job)).generation(), 7);
        assert!(matches!(
            job.start(Source::Fixed(identity.clone()), Arc::new(|| {})),
            Err(Error::Busy)
        ));
        assert!(matches!(
            Job::default().start(Source::Fixed(identity), Arc::new(|| {})),
            Err(Error::Busy)
        ));
        let start = Instant::now();
        job.shutdown();
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(!job.is_busy());
        server.join().unwrap();
    }
}
