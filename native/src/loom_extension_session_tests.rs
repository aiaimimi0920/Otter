//! Extension-admission chain over real loopback TLS/WS; synthetic catalogs only, without user discovery files or model calls.
use super::test_peer::*;
use super::*;
use serde_json::json;
use std::{
    sync::{mpsc, Arc},
    thread,
};
use tungstenite::Message;

#[test]
fn same_authenticated_socket_negotiates_and_checks_authority_without_reading_resources() {
    let (identity, worker) = peer(|peer| {
        handshake(peer, handshake_data());
        let id = authority_request(peer);
        // Unrelated requests/pings cannot impersonate this authorization result.
        send(peer, response(&json!("other"), json!({"authorized":false})));
        peer.send(Message::Ping(vec![1, 2])).unwrap();
        assert_eq!(peer.read().unwrap(), Message::Pong(vec![1, 2]));
        send(peer, response(&id, json!({"authorized":true})));
    });
    let cancel = AtomicBool::new(false);
    let mut session = Session::open(&identity, &cancel).unwrap();
    assert_eq!(session.snapshot().unwrap().generation(), 7);
    assert!(matches!(
        session.authorize(
            COMMAND,
            &Context {
                has_image: false,
                ..context()
            },
            &target(),
            &cancel
        ),
        Err(Error::Unavailable)
    ));
    session
        .authorize(COMMAND, &context(), &target(), &cancel)
        .unwrap();
    assert!(session.snapshot().is_some());
    drop(session);
    worker.join().unwrap();
}

#[test]
fn malformed_or_unsupported_handshakes_are_rejected_without_authorize_or_invoke() {
    for case in 0..6 {
        let (identity, worker) = peer(move |peer| {
            let mut data = handshake_data();
            match case {
                0 => data["features"] = json!(["contribution.snapshot", "command.invoke"]),
                1 => data["sessionId"] = json!(""),
                2 => data["snapshot"]["plugins"][0]["scopeId"] = json!("wrong"),
                3 => data["features"] = json!([true]),
                4 => data["snapshot"]["generation"] = json!(-1),
                _ => data["sessionId"] = json!("x".repeat(385)),
            }
            handshake(peer, data);
            assert!(peer.read().is_err());
        });
        assert!(
            Session::open(&identity, &AtomicBool::new(false)).is_err(),
            "case {case}"
        );
        worker.join().unwrap();
    }
}

#[test]
fn high_generation_revocation_rejects_even_a_successful_inflight_authorization() {
    for revoked in [false, true] {
        let (identity, worker) = peer(move |peer| {
            handshake(peer, handshake_data());
            let id = authority_request(peer);
            let mut snapshot = fixture();
            snapshot["generation"] = json!(8);
            if revoked {
                snapshot["plugins"][0]["trustStatus"] = json!("revoked");
            }
            send(peer, updated(snapshot));
            send(peer, response(&id, json!({"authorized":true})));
            assert!(
                peer.read().is_err(),
                "failed authority must close the transport"
            );
        });
        let cancel = AtomicBool::new(false);
        let mut session = Session::open(&identity, &cancel).unwrap();
        assert!(matches!(
            session.authorize(COMMAND, &context(), &target(), &cancel),
            Err(Error::AuthorityChanged)
        ));
        assert!(session.snapshot().is_none());
        assert!(matches!(
            session.authorize(COMMAND, &context(), &target(), &cancel),
            Err(Error::Closed)
        ));
        worker.join().unwrap();
    }
}

#[test]
fn stale_and_same_generation_snapshots_do_not_roll_back_the_catalog() {
    let (identity, worker) = peer(|peer| {
        handshake(peer, handshake_data());
        let id = authority_request(peer);
        for generation in [6, 7] {
            let mut snapshot = fixture();
            snapshot["generation"] = json!(generation);
            snapshot["plugins"][0]["trustStatus"] = json!("revoked");
            send(peer, updated(snapshot));
        }
        send(peer, response(&id, json!({"authorized":true})));
    });
    let cancel = AtomicBool::new(false);
    let mut session = Session::open(&identity, &cancel).unwrap();
    session
        .authorize(COMMAND, &context(), &target(), &cancel)
        .unwrap();
    assert!(session
        .snapshot()
        .unwrap()
        .command(COMMAND, &context())
        .is_some());
    drop(session);
    worker.join().unwrap();
}

#[test]
fn denial_remote_error_wrong_protocol_and_invalid_json_stop_without_replay() {
    for case in 0..6 {
        let (identity, worker) = peer(move |peer| {
            handshake(peer, handshake_data());
            let id = authority_request(peer);
            let mut reply = response(&id, json!({"authorized":true}));
            match case {
                0 => reply["data"]["authorized"] = json!(false),
                1 => {
                    reply["status"] = json!("failed");
                    reply["error"] = json!({"code":"denied", "message":"scope changed"});
                }
                2 => reply["apiVersion"] = json!("2.0"),
                3 => reply["protocol"] = json!("other"),
                4 => reply["data"]["authorized"] = json!("true"),
                _ => (),
            }
            if case == 5 {
                peer.send(Message::Text("{invalid".into())).unwrap();
            } else {
                send(peer, reply);
            }
            assert!(peer.read().is_err());
        });
        let cancel = AtomicBool::new(false);
        let mut session = Session::open(&identity, &cancel).unwrap();
        let error = session
            .authorize(COMMAND, &context(), &target(), &cancel)
            .unwrap_err();
        match case {
            0 | 4 => assert!(matches!(error, Error::Denied)),
            1 => assert!(
                matches!(error, Error::Remote { code, message } if code == "denied" && message == "scope changed")
            ),
            _ => assert!(matches!(error, Error::Wire)),
        }
        assert!(session.snapshot().is_none());
        assert!(matches!(
            session.authorize(COMMAND, &context(), &target(), &cancel),
            Err(Error::Closed)
        ));
        worker.join().unwrap();
    }
}

#[test]
fn precancel_never_connects_and_target_is_validated_at_the_boundary() {
    let (listener, identity, _) = crate::loom_bridge_client::tests::fixture();
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        Session::open(&identity, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    for (unit, revision) in [("", 0), ("test", catalog::MAX_INTEGER + 1)] {
        assert!(Target::new(unit, revision).is_err());
    }
    assert!(Target::new(&"😀".repeat(193), 0).is_err());
    assert!(Target::new("unit:test", catalog::MAX_INTEGER).is_ok());
}

#[test]
fn cancellation_interrupts_response_wait_and_discards_all_session_authority() {
    // Cover cancellation races between send completion, read-deadline setup, and actual blocking read; still count one regression only.
    for _ in 0..32 {
        let (ready, waiting) = mpsc::channel();
        let (identity, server) = peer(move |peer| {
            handshake(peer, handshake_data());
            authority_request(peer);
            ready.send(()).unwrap();
            assert!(peer.read().is_err());
        });
        let cancel = Arc::new(AtomicBool::new(false));
        let mut session = Session::open(&identity, &cancel).unwrap();
        let interrupt = session.interrupt().unwrap();
        let signal = cancel.clone();
        let client = thread::spawn(move || {
            let result = session.authorize(COMMAND, &context(), &target(), &signal);
            assert!(matches!(result, Err(Error::Cancelled)), "{result:?}");
            assert!(session.snapshot().is_none());
            assert!(matches!(session.interrupt(), Err(Error::Closed)));
        });
        waiting.recv_timeout(Duration::from_secs(3)).unwrap();
        let start = Instant::now();
        cancel.store(true, Ordering::Release);
        interrupt.cancel().unwrap();
        client.join().unwrap();
        assert!(start.elapsed() < Duration::from_secs(2));
        server.join().unwrap();
    }
}

#[test]
fn unrelated_responses_and_events_cannot_extend_the_fixed_total_deadline() {
    let (identity, server) = peer(|peer| {
        handshake(peer, handshake_data());
        assert_eq!(read(peer)["method"], "fixture:deadline");
        loop {
            if peer
                .send(Message::Text(
                    response(&json!("not-matching"), json!({"authorized":true})).to_string(),
                ))
                .is_err()
            {
                break;
            }
            if peer
                .send(Message::Text(updated(fixture()).to_string()))
                .is_err()
            {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
    });
    let cancel = AtomicBool::new(false);
    let mut session = Session::open(&identity, &cancel).unwrap();
    let start = Instant::now();
    let result = wire::exchange(
        session.socket.as_mut().unwrap(),
        &mut session.snapshot,
        "deadline",
        json!({"method":"fixture:deadline"}),
        Duration::from_millis(100),
        &cancel,
    );
    assert!(matches!(result, Err(Error::Timeout)));
    assert!(start.elapsed() < Duration::from_secs(2));
    drop(session);
    server.join().unwrap();
}
