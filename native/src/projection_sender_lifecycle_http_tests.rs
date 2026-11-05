//! 同意图及四槽随未领取/Ready 保留；显式解绑才退休，晚撤权不得发布或静默抹账。
use super::*;
use crate::live_relay_control_runtime::Slot;
#[path = "projection_sender_cancel_http_tests.rs"]
mod cancellation;

fn unlink(client: &Client, grant: &mut Grant, request: SendRequest) -> Job {
    Job::start_prepared(
        client.clone(),
        grant.lease(client.origin(), 0).unwrap(),
        Operation::CancelSend(Box::new(request)),
    )
    .unwrap()
}
fn successful_read(
    listener: TcpListener,
    tls: Arc<ServerConfig>,
    value: Value,
) -> std::thread::JoinHandle<TcpListener> {
    std::thread::spawn(move || {
        let mut stream = receive(&listener, tls);
        assert!(http_request(&mut stream)
            .0
            .starts_with("POST /v1/projections/read "));
        reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
        listener
    })
}
#[test]
fn sender_completed_and_ready_keep_same_intent_and_four_slot_capacity_until_drop() {
    let (listener, client, tls) = fixture();
    let (_directory, _store, request) = prepared(&client);
    let peer = successful_read(listener, tls, response(&request));
    let mut grant = grant(&client);
    let mut job = start(&client, &mut grant, request.clone());
    wait(&job);
    let mut duplicate = start(&client, &mut grant, request.clone());
    wait(&duplicate);
    assert!(matches!(
        duplicate.try_join(),
        Some(Err(JobError::Sender(sender::Error::Busy)))
    ));
    let slots: Vec<_> = (0..3).map(|_| Slot::acquire().unwrap()).collect();
    assert!(Slot::acquire().is_err());
    let Output::Sent(ready) = job.try_join().unwrap().unwrap() else {
        panic!("missing sender")
    };
    assert!(Slot::acquire().is_err());
    drop(slots);
    let mut duplicate = start(&client, &mut grant, request);
    wait(&duplicate);
    assert!(matches!(
        duplicate.try_join(),
        Some(Err(JobError::Sender(sender::Error::Busy)))
    ));
    let slots: Vec<_> = (0..3).map(|_| Slot::acquire().unwrap()).collect();
    assert!(Slot::acquire().is_err());
    drop(ready);
    let _fourth = Slot::acquire().unwrap();
    drop(slots);
    no_request(&peer.join().unwrap());
}
#[test]
fn sender_ready_requires_explicit_cancel_and_drop_before_unlink_retry_can_retire() {
    let (listener, client, tls) = fixture();
    let (directory, store, request) = prepared(&client);
    let peer = successful_read(listener, tls.clone(), response(&request));
    let mut grant = grant(&client);
    let mut job = start(&client, &mut grant, request.clone());
    wait(&job);
    let Output::Sent(ready) = job.try_join().unwrap().unwrap() else {
        panic!("missing sender")
    };
    let mut cleanup = unlink(&client, &mut grant, ready.request());
    wait(&cleanup);
    assert!(matches!(
        cleanup.try_join(),
        Some(Err(JobError::Sender(sender::Error::Busy)))
    ));
    assert!(store.pending(&AtomicBool::new(false)).unwrap()[0].cancelled());
    assert!(ready
        .validate(&grant.lease(client.origin(), 0).unwrap(), 0)
        .is_err());
    let listener = peer.join().unwrap();
    no_request(&listener);
    drop(ready);
    let path = directory.path();
    let id = request.intent().envelope().projection_id().to_owned();
    let peer = std::thread::spawn(move || {
        let mut stream = receive(&listener, tls);
        let (head, body) = http_request(&mut stream);
        assert!(head.starts_with("POST /v1/projections/unlink "));
        assert_eq!(body, json!({"projectionId":id}));
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(path).unwrap()).unwrap()["entries"][0]
                ["cancelled"],
            true
        );
        reply(&mut stream, 200, br#"{"unlinked":true}"#);
        listener
    });
    let mut cleanup = unlink(&client, &mut grant, request.clone());
    wait(&cleanup);
    assert!(matches!(
        cleanup.try_join(),
        Some(Ok(Output::SenderUnlinked))
    ));
    assert!(store.pending(&AtomicBool::new(false)).unwrap().is_empty());
    let next = store
        .prepare(
            sender::Preparation {
                selection: &sender::Selection::new(
                    &"a".repeat(64),
                    client.origin(),
                    "unit-1",
                    "target-1",
                )
                .unwrap(),
                frame: frame(),
                identity_root: &directory.identity(),
                device: "device-1",
                now_ms: 2_000,
            },
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_ne!(next.envelope(), request.intent().envelope());
    let saved = std::fs::read(directory.path()).unwrap();
    let mut stale = unlink(&client, &mut grant, request);
    wait(&stale);
    assert!(matches!(
        stale.try_join(),
        Some(Err(JobError::Sender(sender::Error::Changed)))
    ));
    assert_eq!(std::fs::read(directory.path()).unwrap(), saved);
    no_request(&peer.join().unwrap());
}
#[test]
fn sender_false_or_failed_unlink_keeps_cancelled_exact_original_record() {
    for (status, body) in [
        (200, br#"{"unlinked":false}"#.as_slice()),
        (401, b"{}"),
        (500, b"{}"),
        (200, br#"{"unlinked":true,"unlinked":true}"#),
    ] {
        let (listener, client, tls) = fixture();
        let (directory, store, request) = prepared(&client);
        let original: Value =
            serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap();
        let peer = std::thread::spawn(move || {
            let mut stream = receive(&listener, tls);
            assert!(http_request(&mut stream)
                .0
                .starts_with("POST /v1/projections/unlink "));
            reply(&mut stream, status, body);
            listener
        });
        let mut grant = grant(&client);
        let mut job = unlink(&client, &mut grant, request);
        wait(&job);
        assert!(job.try_join().unwrap().is_err());
        let pending = store.pending(&AtomicBool::new(false)).unwrap();
        assert_eq!(pending.len(), 1);
        assert!(pending[0].cancelled());
        let mut expected = original;
        expected["entries"][0]["cancelled"] = true.into();
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(directory.path()).unwrap()).unwrap(),
            expected
        );
        no_request(&peer.join().unwrap());
    }
}
#[test]
fn sender_completed_then_revoked_cannot_publish_and_requires_explicit_background_cleanup() {
    let (listener, client, tls) = fixture();
    let (directory, store, request) = prepared(&client);
    let original = std::fs::read(directory.path()).unwrap();
    let peer = successful_read(listener, tls.clone(), response(&request));
    let mut grant = grant(&client);
    let mut job = start(&client, &mut grant, request.clone());
    wait(&job);
    drop(grant);
    assert!(matches!(job.try_join(), Some(Err(JobError::Stopped))));
    // 晚撤销不在 UI join/Drop 内写大日志或联网；原记录完整保留，不自动重试 Create。
    assert_eq!(std::fs::read(directory.path()).unwrap(), original);
    let listener = peer.join().unwrap();
    no_request(&listener);
    let peer = std::thread::spawn(move || {
        let mut stream = receive(&listener, tls);
        assert!(http_request(&mut stream)
            .0
            .starts_with("POST /v1/projections/unlink "));
        reply(&mut stream, 200, br#"{"unlinked":true}"#);
        listener
    });
    let mut grant = super::grant(&client);
    let mut cleanup = unlink(&client, &mut grant, request);
    wait(&cleanup);
    assert!(matches!(
        cleanup.try_join(),
        Some(Ok(Output::SenderUnlinked))
    ));
    assert!(store.pending(&AtomicBool::new(false)).unwrap().is_empty());
    no_request(&peer.join().unwrap());
}
