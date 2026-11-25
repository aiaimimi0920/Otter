//! Trusted plaintext loopback HTTP boundary; QR centers are never transport targets, and Device/management bearer identities stay separate.
use super::*;
use crate::device_session_client::tests::{accept, reply, request, session};
use std::{
    net::TcpListener,
    sync::{mpsc, Arc},
    time::Duration,
};
const AUTHORITY: &str = "https://192.0.2.1:54321";
#[path = "projection_account_image_http_tests.rs"]
mod images;
#[path = "projection_account_tls_tests.rs"]
mod tls;
fn target(listener: &TcpListener) -> crate::loom_media_target::Target {
    crate::loom_media_target::Target::new(
        format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port()),
        Some("admin-never-sent".into()),
    )
}
fn listener() -> TcpListener {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    listener
}
fn respond(listener: &TcpListener, path: &str, expected_device: bool, value: &Value) -> Value {
    let mut stream = accept(listener);
    let (head, body) = request(&mut stream);
    assert!(head.starts_with(&format!("{path} HTTP/1.1\r\n")), "{head}");
    let head = head.to_ascii_lowercase();
    assert!(!head.contains("admin-never-sent"));
    assert!(!head.contains("bearer "));
    assert_eq!(
        head.contains("authorization: device fixture-secret\r\n"),
        expected_device
    );
    if expected_device {
        assert!(head.contains("x-loom-device-nonce:"));
    }
    reply(&mut stream, 200, &serde_json::to_vec(value).unwrap());
    body
}
fn bootstrap(listener: &TcpListener, device: &str) {
    respond(
        listener,
        "GET /v1/otter/identity",
        false,
        &json!({"protocolVersion":"loom.connection.v1", "endpoint":AUTHORITY,"loomId":"loom-fixture"}),
    );
    let body = respond(
        listener,
        "POST /v1/projections/v2/context",
        true,
        &context_json(device),
    );
    assert_eq!(body, json!({"kind":"context"}));
}
#[test]
fn actual_loopback_context_create_update_read_and_unlink_use_device_nonce_not_qr_origin() {
    let first = image(17);
    let second = image(19);
    let envelope = envelope(&first);
    let listener = listener();
    let target = target(&listener);
    let initial = wire(&envelope, None, 1, first.digest(), None);
    let updated = wire(&envelope, None, 2, second.digest(), Some(RECEIVER));
    let peer = std::thread::spawn(move || {
        bootstrap(&listener, SOURCE);
        let body = respond(&listener, "POST /v1/projections/v2/create", true, &initial);
        assert_eq!(body["kind"], "create");
        assert_eq!(body["unitId"], "unit-1");
        let body = respond(&listener, "POST /v1/projections/v2/update", true, &updated);
        assert_eq!(body["kind"], "update");
        assert_eq!(body["sourceSessionId"], "source-epoch");
        let body = respond(&listener, "POST /v1/projections/v2/read", true, &updated);
        assert_eq!(body["knownRevision"], 2);
        respond(
            &listener,
            "POST /v1/projections/v2/unlink",
            true,
            &json!({"unlinked":true}),
        );
    });
    let credential = session(AUTHORITY, now().unwrap() + 120_000);
    let cancel = Cancel::new();
    let account = Account::open(target, &credential, &cancel).unwrap();
    assert_eq!(account.context().device_id, SOURCE);
    assert_ne!(account.context().device_id, credential.device_id());
    let State::Content(source) = account
        .execute(Operation::Create {
            unit: "unit-1",
            kind: "sticker",
            image: &first,
        })
        .unwrap()
    else {
        panic!()
    };
    let State::Content(updated) = account
        .execute(Operation::Update {
            envelope: &source.envelope,
            prior_revision: 1,
            revision: 2,
            image: &second,
        })
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(updated.revision, 2);
    account
        .execute(Operation::Read {
            envelope: &source.envelope,
            known_revision: 2,
            known_digest: second.digest(),
            receiver_unit: None,
        })
        .unwrap();
    assert!(matches!(
        account
            .execute(Operation::Unlink {
                envelope: &source.envelope
            })
            .unwrap(),
        State::Unlinked
    ));
    assert!(!cancel.test_disconnect());
    peer.join().unwrap();
}
#[test]
fn actual_loopback_inspect_accept_and_receiver_read_decode_only_correlated_png() {
    let image = image(17);
    let envelope = envelope(&image);
    let listener = listener();
    let target = target(&listener);
    let preview = wire(&envelope, Some(&image), 1, image.digest(), None);
    let accepted = wire(&envelope, Some(&image), 1, image.digest(), Some(RECEIVER));
    let peer = std::thread::spawn(move || {
        bootstrap(&listener, RECEIVER);
        respond(&listener, "POST /v1/projections/v2/inspect", true, &preview);
        let body = respond(&listener, "POST /v1/projections/v2/accept", true, &accepted);
        assert_eq!(body["confirmed"], true);
        assert_eq!(body["receiverUnitId"], "receive-1");
        respond(&listener, "POST /v1/projections/v2/read", true, &accepted);
    });
    let credential = session(AUTHORITY, now().unwrap() + 120_000);
    let cancel = Cancel::new();
    let account = Account::open(target, &credential, &cancel).unwrap();
    account
        .execute(Operation::Inspect {
            envelope: &envelope,
        })
        .unwrap();
    let State::Content(content) = account
        .execute(Operation::Accept {
            envelope: &envelope,
            unit: "receive-1",
            revision: 1,
            digest: image.digest(),
            confirmed: true,
        })
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(
        &*content.image.unwrap().png(&AtomicBool::new(false)).unwrap(),
        &*png(17)
    );
    account
        .execute(Operation::Read {
            envelope: &envelope,
            known_revision: 1,
            known_digest: image.digest(),
            receiver_unit: Some("receive-1"),
        })
        .unwrap();
    peer.join().unwrap();
}
#[test]
fn different_local_authority_is_rejected_before_device_credentials_are_sent() {
    let listener = listener();
    let target = target(&listener);
    let peer = std::thread::spawn(move || {
        respond(
            &listener,
            "GET /v1/otter/identity",
            false,
            &json!({"protocolVersion":"loom.connection.v1","endpoint":"https://foreign.invalid","loomId":"other"}),
        );
        assert!(listener.accept().is_err());
    });
    let credential = session(AUTHORITY, now().unwrap() + 120_000);
    assert!(matches!(
        Account::open(target, &credential, &Cancel::new()),
        Err(Error::Identity)
    ));
    peer.join().unwrap();
}
#[test]
fn local_endpoint_rejects_remote_manifest_credentials_and_path_expansion() {
    for origin in [
        "http://192.0.2.1:12",
        "https://localhost:12",
        "http://user@127.0.0.1:12",
        "http://127.0.0.1:12/extra",
    ] {
        assert!(
            matches!(
                http::Endpoint::new(crate::loom_media_target::Target::new(
                    origin.into(),
                    Some("secret".into())
                )),
                Err(Error::LocalOnly)
            ),
            "{origin}"
        );
    }
    let listener = listener();
    let endpoint = http::Endpoint::new(target(&listener)).unwrap();
    let credential = session(AUTHORITY, now().unwrap() + 120_000);
    let cancel = Cancel::new();
    for path in [
        "/v1/projections/v2/read?origin=evil",
        "//evil.invalid/",
        "/api/loom/projections",
    ] {
        assert!(matches!(
            endpoint.request("POST", path, b"{}", Some(&credential), &cancel, 1024),
            Err(Error::Input)
        ));
    }
    assert!(listener.accept().is_err());
}
#[test]
fn signed_out_code_and_truncated_unauthorized_response_remain_real_failures() {
    for truncated in [false, true] {
        let listener = listener();
        let endpoint = http::Endpoint::new(target(&listener)).unwrap();
        let peer = std::thread::spawn(move || {
            let mut stream = accept(&listener);
            request(&mut stream);
            if truncated {
                use std::io::Write;
                stream.write_all(b"HTTP/1.1 401 Fixture\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{").unwrap();
            } else {
                reply(
                    &mut stream,
                    401,
                    br#"{"error":{"code":"account_signed_out"}}"#,
                );
            }
        });
        let credential = session(AUTHORITY, now().unwrap() + 120_000);
        let error = endpoint
            .request(
                "POST",
                "/v1/projections/v2/context",
                b"{}",
                Some(&credential),
                &Cancel::new(),
                1024,
            )
            .err()
            .unwrap();
        assert_eq!(
            error,
            Error::Remote {
                status: 401,
                code: (!truncated).then(|| "account_signed_out".into())
            }
        );
        peer.join().unwrap();
    }
}
#[test]
fn revoke_interrupts_pending_local_http_and_retires_socket_without_retry() {
    let listener = listener();
    let endpoint = http::Endpoint::new(target(&listener)).unwrap();
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(Cancel::new());
    let stop = cancel.clone();
    let peer = std::thread::spawn(move || {
        use std::io::Read;
        let mut stream = accept(&listener);
        request(&mut stream);
        tx.send(()).unwrap();
        let mut byte = [0];
        assert_eq!(stream.read(&mut byte).unwrap(), 0);
    });
    let worker = std::thread::spawn(move || {
        let credential = session(AUTHORITY, now().unwrap() + 120_000);
        endpoint
            .request(
                "POST",
                "/v1/projections/v2/context",
                b"{}",
                Some(&credential),
                &stop,
                1024,
            )
            .err()
            .unwrap()
    });
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    cancel.cancel();
    assert_eq!(worker.join().unwrap(), Error::Stopped);
    assert!(!cancel.test_disconnect());
    peer.join().unwrap();
}
