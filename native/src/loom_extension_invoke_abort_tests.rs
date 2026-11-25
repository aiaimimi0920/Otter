//! Invalidation during authorization/preparation/execution must never deliver effects or replay requests after failure.
use super::*;
use std::{sync::mpsc, thread};
#[test]
fn denied_or_invalid_tickets_never_read_host_resources_or_send_invoke() {
    for denied in [false, true] {
        let (identity, server) = peer(move |peer| {
            handshake(peer, data(true));
            if denied {
                let request = read(peer);
                assert_eq!(request["method"], "loom.extension.command.authorize");
                let mut reply = response(&request["params"]["requestId"], json!({}));
                reply["status"] = json!("failed");
                send(peer, reply);
            } else {
                authorize(peer, "invalid-ticket");
            }
            assert!(peer.read().is_err());
        });
        let cancel = AtomicBool::new(false);
        let mut session = Session::open(&identity, &cancel).unwrap();
        let result = session.invoke(
            RUN,
            &context(),
            &target(),
            |_| panic!("resource read before valid lease"),
            &cancel,
        );
        assert!(result.is_err());
        assert!(session.snapshot().is_none());
        server.join().unwrap();
    }
}
#[test]
fn authority_change_before_preparation_and_after_execution_rejects_the_whole_result() {
    for before in [false, true] {
        let (identity, server) = peer(move |peer| {
            let initial = data(true);
            let mut snapshot = initial["snapshot"].clone();
            snapshot["generation"] = json!(8);
            snapshot["plugins"][0]["trustStatus"] = json!("revoked");
            handshake(peer, initial);
            if before {
                let request = read(peer);
                assert_eq!(request["method"], "loom.extension.command.authorize");
                send(peer, updated(snapshot));
                send(
                    peer,
                    response(
                        &request["params"]["requestId"],
                        json!({"authorizationId":LEASE}),
                    ),
                );
            } else {
                authorize(peer, LEASE);
                let request = invocation(peer, true);
                send(peer, updated(snapshot));
                complete(peer, &request["params"]["invocation"]["requestId"]);
            }
            assert!(peer.read().is_err());
        });
        let cancel = AtomicBool::new(false);
        let mut session = Session::open(&identity, &cancel).unwrap();
        let result = session.invoke(
            RUN,
            &context(),
            &target(),
            |_| {
                assert!(!before);
                prepared()
            },
            &cancel,
        );
        assert!(matches!(result, Err(Error::AuthorityChanged)));
        assert!(session.snapshot().is_none());
        server.join().unwrap();
    }
}
#[test]
fn preparation_failure_cancel_or_foreign_opaque_state_closes_unused_lease_without_invoking() {
    for case in 0..3 {
        let (identity, server) = peer(|peer| {
            handshake(peer, data(true));
            authorize(peer, LEASE);
            assert!(peer.read().is_err());
        });
        let cancel = AtomicBool::new(false);
        let mut session = Session::open(&identity, &cancel).unwrap();
        let result = session.invoke(RUN, &context(), &target(), |_| match case {
            0 => Err(Error::Cancelled),
            1 => { cancel.store(true, Ordering::Release); prepared() },
            _ => Prepared::new(json!({}))?.attachment(Attachment::parse(json!({"attachmentId":"foreign.plugin.data", "typeId":"data",
                "schemaVersion":"1", "revision":1, "pluginId":"foreign.plugin", "pluginVersion":"1", "payload":{"private":"opaque"}}))?),
        }, &cancel);
        assert!(matches!(result, Err(Error::Cancelled | Error::Denied)));
        assert!(session.snapshot().is_none());
        server.join().unwrap();
    }
}
#[test]
fn invalid_grants_are_rejected_before_authorization_and_callbacks() {
    let (identity, server) = peer(|peer| {
        let mut initial = data(true);
        initial["snapshot"]["plugins"][0]["effectivePermissions"] = json!([]);
        handshake(peer, initial);
        assert!(peer.read().is_err());
    });
    let cancel = AtomicBool::new(false);
    let mut session = Session::open(&identity, &cancel).unwrap();
    assert!(matches!(
        session.invoke(
            RUN,
            &context(),
            &target(),
            |_| panic!("ungranted read"),
            &cancel
        ),
        Err(Error::Denied)
    ));
    drop(session);
    server.join().unwrap();
}
#[test]
fn an_inner_result_for_another_request_is_not_delivered_to_the_document_owner() {
    let (identity, server) = peer(|peer| {
        handshake(peer, data(true));
        authorize(peer, LEASE);
        let request = invocation(peer, true);
        send(
            peer,
            response(
                &request["params"]["invocation"]["requestId"],
                json!({"protocol":catalog::PROTOCOL, "apiVersion":catalog::API,
            "requestId":"other", "status":"succeeded", "output":{}, "effects":[]}),
            ),
        );
        assert!(peer.read().is_err());
    });
    let cancel = AtomicBool::new(false);
    let mut session = Session::open(&identity, &cancel).unwrap();
    assert!(matches!(
        session.invoke(RUN, &context(), &target(), |_| prepared(), &cancel),
        Err(Error::Wire)
    ));
    assert!(session.snapshot().is_none());
    server.join().unwrap();
}
#[test]
fn cancel_interrupts_an_inflight_command_and_does_not_publish_a_late_result() {
    let (ready, waiting) = mpsc::channel();
    let (identity, server) = peer(move |peer| {
        handshake(peer, data(true));
        authorize(peer, LEASE);
        invocation(peer, true);
        ready.send(()).unwrap();
        assert!(peer.read().is_err());
    });
    let cancel = Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    let mut session = Session::open(&identity, &cancel).unwrap();
    let interrupt = session.interrupt().unwrap();
    let client = thread::spawn(move || {
        assert!(matches!(
            session.invoke(RUN, &context(), &target(), |_| prepared(), &signal),
            Err(Error::Cancelled)
        ));
        assert!(session.snapshot().is_none());
    });
    waiting.recv_timeout(Duration::from_secs(3)).unwrap();
    let start = Instant::now();
    cancel.store(true, Ordering::Release);
    interrupt.cancel().unwrap();
    client.join().unwrap();
    assert!(start.elapsed() < Duration::from_secs(2));
    server.join().unwrap();
}
#[test]
fn combined_upload_and_input_cannot_exceed_the_real_bridge_request_budget() {
    let (identity, server) = peer(|peer| {
        handshake(peer, data(true));
        authorize(peer, LEASE);
        assert!(peer.read().is_err());
    });
    let cancel = AtomicBool::new(false);
    let mut session = Session::open(&identity, &cancel).unwrap();
    let result = session.invoke(
        RUN,
        &context(),
        &target(),
        |_| {
            Ok(Prepared::new(json!("x".repeat(16 * 1024 * 1024)))?
                .image(Image::new("image/png", &vec![1; 8 * 1024 * 1024])?))
        },
        &cancel,
    );
    assert!(matches!(result, Err(Error::Limit)));
    assert!(session.snapshot().is_none());
    server.join().unwrap();
}
