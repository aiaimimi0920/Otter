//! Real local TLS: public invitations have no fake target, explicit import inspects before accepting latest versions, and cancellation forbids receipt.
use super::*;
use crate::projection_delivery::share::Import;
fn share_loom() -> String {
    format!("loom-{}", "a".repeat(64))
}
fn prepared_share(client: &Client) -> (Directory, sender::Store, SendRequest) {
    prepared_as(client, "device-1")
}
fn prepared_as(client: &Client, device: &str) -> (Directory, sender::Store, SendRequest) {
    let directory = Directory::new();
    let identity = crate::device_identity_store::Store::new(&directory.identity()).unwrap();
    let mut loaded = identity.load_or_create().unwrap();
    identity.bind_and_save(&mut loaded, device).unwrap();
    let store = sender::Store::open(&directory.0, &AtomicBool::new(false)).unwrap();
    let selection =
        sender::Selection::invitation(&"a".repeat(64), client.origin(), "unit-1").unwrap();
    let intent = store
        .prepare(
            sender::Preparation {
                selection: &selection,
                frame: frame(),
                identity_root: &directory.identity(),
                device,
                now_ms: 1000,
            },
            &AtomicBool::new(false),
        )
        .unwrap();
    let request = SendRequest::new(intent, directory.identity());
    (directory, store, request)
}
#[test]
fn public_sender_uses_original_signed_create_without_target_and_accepts_unconsumed_loom_response() {
    let (listener, client, tls) = fixture();
    let (dir, _store, request) = prepared_share(&client);
    let mut expected = response(&request);
    expected["delivery"] = Value::Null;
    let path = dir.path();
    let before = std::fs::read(&path).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream = receive(&listener, tls.clone());
        assert!(http_request(&mut stream)
            .0
            .starts_with("POST /v1/projections/read "));
        reply(&mut stream, 404, MISSING);
        drop(stream);
        let mut stream = receive(&listener, tls);
        let (head, body) = http_request(&mut stream);
        assert!(head.starts_with("POST /v1/projections/create "));
        assert!(body.get("targetDeviceId").is_none());
        assert!(body.get("peerId").is_none());
        assert_eq!(body["envelope"], expected["envelope"]);
        assert_eq!(std::fs::read(path).unwrap(), before);
        reply(&mut stream, 200, &serde_json::to_vec(&expected).unwrap());
        listener
    });
    let mut grant = grant(&client);
    let mut job = start(&client, &mut grant, request);
    wait(&job);
    let Output::Sent(ready) = job.try_join().unwrap().unwrap() else {
        panic!("expected sender")
    };
    assert!(ready.response().receiver_device().is_none());
    no_request(&peer.join().unwrap());
}
#[test]
fn public_import_inspects_then_durably_accepts_fresh_revision_and_retains_receipt_kind_after_restart(
) {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let (listener, client, tls) = fixture();
    let (dir, _store, request) = prepared_as(&client, "source-1");
    let envelope = request.intent().envelope().clone();
    let receiver = format!("delivery-{}", &envelope.projection_id()[11..]);
    let snapshot =
        json!({"width":2,"height":1,"imageBase64":STANDARD.encode(delivery::accept::tests::png())});
    let expected = json!({"sourceLoomId":share_loom(),"envelope":envelope,"revision":3,"digest":request.intent().digest(),"linked":true,
        "width":2,"height":1,"receiverDeviceId":null,"receiverUnitId":null,"delivery":null,"snapshot":snapshot});
    let journal = delivery::journal::Store::open(&dir.0).unwrap();
    let disk = journal.clone();
    let peer = std::thread::spawn(move || {
        let mut stream = receive(&listener, tls.clone());
        let (head, body) = http_request(&mut stream);
        assert!(head.starts_with("POST /v1/offline-projections/import "));
        assert_eq!(body["envelope"], expected["envelope"]);
        reply(&mut stream, 200, &serde_json::to_vec(&expected).unwrap());
        drop(stream);
        let mut stream = receive(&listener, tls);
        let (head, body) = http_request(&mut stream);
        assert!(head.starts_with("POST /v1/projections/accept "));
        assert_eq!(body["expectedRevision"], 3);
        assert_eq!(body["expectedDigest"], expected["digest"]);
        assert_eq!(body["receiverUnitId"], receiver);
        assert_eq!(body["confirmed"], true);
        assert_eq!(disk.pending().unwrap().len(), 1);
        let mut accepted = expected;
        accepted["receiverDeviceId"] = "device-1".into();
        accepted["receiverUnitId"] = receiver.into();
        reply(&mut stream, 200, &serde_json::to_vec(&accepted).unwrap());
        listener
    });
    let import = Arc::new(Import {
        envelope,
        source_loom_id: share_loom(),
        cancel: Arc::new(AtomicBool::new(false)),
    });
    let mut grant = grant(&client);
    let mut job = Job::start_prepared(
        client.clone(),
        grant.lease(client.origin(), 0).unwrap(),
        Operation::Import {
            request: import,
            journal: journal.clone(),
        },
    )
    .unwrap();
    wait(&job);
    let Output::Received(ready) = job.try_join().unwrap().unwrap() else {
        panic!("expected receiver")
    };
    assert!(!ready.ticket().invitation().requires_receipt());
    drop(ready);
    drop(job);
    peer.join().unwrap();
    drop(journal);
    let reopened = delivery::journal::Store::open(&dir.0).unwrap();
    assert!(!reopened.pending().unwrap()[0]
        .invitation()
        .requires_receipt());
}

#[test]
fn public_import_rejects_own_invitation_and_cancellation_before_any_accept_or_journal() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    for own in [true, false] {
        let (listener, client, tls) = fixture();
        let (dir, store, request) = prepared_as(&client, if own { "device-1" } else { "source-1" });
        let envelope = request.intent().envelope().clone();
        let wire = json!({"sourceLoomId":share_loom(),"envelope":envelope,"revision":1,"digest":request.intent().digest(),"linked":true,
            "width":2,"height":1,"delivery":null,"receiverDeviceId":null,"receiverUnitId":null,
            "snapshot":{"width":2,"height":1,"imageBase64":STANDARD.encode(delivery::accept::tests::png())}});
        let import = Arc::new(Import {
            envelope,
            source_loom_id: share_loom(),
            cancel: Arc::new(AtomicBool::new(false)),
        });
        let signal = import.cancel.clone();
        let peer = std::thread::spawn(move || {
            let mut stream = receive(&listener, tls);
            assert!(http_request(&mut stream)
                .0
                .starts_with("POST /v1/offline-projections/import "));
            if !own {
                signal.store(true, std::sync::atomic::Ordering::Release);
            }
            reply(&mut stream, 200, &serde_json::to_vec(&wire).unwrap());
            listener
        });
        let journal = delivery::journal::Store::open(&dir.0).unwrap();
        let mut grant = grant(&client);
        let mut job = Job::start_prepared(
            client.clone(),
            grant.lease(client.origin(), 0).unwrap(),
            Operation::Import {
                request: import,
                journal: journal.clone(),
            },
        )
        .unwrap();
        wait(&job);
        assert!(job.try_join().unwrap().is_err());
        assert!(job.take_abandoned().is_none());
        assert!(journal.pending().unwrap().is_empty());
        assert_eq!(store.pending(&AtomicBool::new(false)).unwrap().len(), 1);
        no_request(&peer.join().unwrap());
    }
}

#[test]
fn public_existing_receiver_uses_read_and_never_gains_fresh_cleanup_on_failure() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let (listener, client, tls) = fixture();
    let (dir, _store, request) = prepared_as(&client, "source-1");
    let envelope = request.intent().envelope().clone();
    let receiver = format!("delivery-{}", &envelope.projection_id()[11..]);
    let wire = json!({"sourceLoomId":share_loom(),"envelope":envelope,"revision":1,"digest":request.intent().digest(),"linked":true,
        "width":2,"height":1,"delivery":null,"receiverDeviceId":"device-1","receiverUnitId":receiver,
        "snapshot":{"width":2,"height":1,"imageBase64":STANDARD.encode(delivery::accept::tests::png())}});
    let peer = std::thread::spawn(move || {
        let mut stream = receive(&listener, tls.clone());
        assert!(http_request(&mut stream)
            .0
            .starts_with("POST /v1/offline-projections/import "));
        reply(&mut stream, 200, &serde_json::to_vec(&wire).unwrap());
        drop(stream);
        let mut stream = receive(&listener, tls);
        assert!(http_request(&mut stream)
            .0
            .starts_with("POST /v1/projections/read "));
        reply(&mut stream, 404, MISSING);
        listener
    });
    let journal = delivery::journal::Store::open(&dir.0).unwrap();
    let mut grant = grant(&client);
    let mut job = Job::start_prepared(
        client.clone(),
        grant.lease(client.origin(), 0).unwrap(),
        Operation::Import {
            request: Arc::new(Import {
                envelope,
                source_loom_id: share_loom(),
                cancel: Arc::new(AtomicBool::new(false)),
            }),
            journal: journal.clone(),
        },
    )
    .unwrap();
    wait(&job);
    assert!(job.try_join().unwrap().is_err());
    assert!(job.take_abandoned().is_none());
    assert_eq!(journal.pending().unwrap().len(), 1);
    assert!(!journal.pending().unwrap()[0]
        .invitation()
        .requires_receipt());
    no_request(&peer.join().unwrap());
}
