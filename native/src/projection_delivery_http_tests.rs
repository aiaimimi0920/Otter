//! 传统投送真实 loopback TLS/HTTP；复用原设备夹具，不访问在线 Loom 或系统证书存储。
use super::*;
use crate::projection_delivery::{self as delivery, Inbox, InboxPolicy};
#[path = "projection_receive_compensation_http_tests.rs"]
mod compensation;
#[path = "projection_receive_offline_http_tests.rs"]
mod offline_receive;
#[path = "projection_receive_reattach_http_tests.rs"]
mod reattach;
#[path = "projection_receive_http_tests.rs"]
mod receive;
#[path = "projection_delivery_runtime_tests.rs"]
mod runtime;
#[path = "projection_sender_http_tests.rs"]
mod sender;
#[path = "projection_targets_http_tests.rs"]
mod targets;

#[test]
fn authenticated_shared_inbox_and_rejection_use_original_routes_distinct_nonces_and_recorded_receipt(
) {
    let (listener, client, config) = fixture();
    let origin = client.origin().to_owned();
    let authorized = session(&origin, u64::MAX);
    let response = delivery::tests::body(&origin);
    let peer = std::thread::spawn(move || {
        let mut nonces = std::collections::BTreeSet::new();
        for index in 0..2 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            let lower = head.to_ascii_lowercase();
            assert!(lower.contains("authorization: device fixture-secret\r\n"));
            let nonce = lower
                .lines()
                .find_map(|line| line.strip_prefix("x-loom-device-nonce: "))
                .unwrap();
            assert_eq!(nonce.len(), 32);
            assert!(nonces.insert(nonce.to_owned()));
            if index == 0 {
                assert!(head.starts_with("POST /v1/projections/inbox HTTP/1.1"));
                assert_eq!(body, serde_json::json!({"policy":"confirm"}));
                reply(&mut stream, 200, &response);
            } else {
                assert!(head.starts_with("POST /v1/projections/receipt HTTP/1.1"));
                assert_eq!(
                    body,
                    serde_json::json!({"projectionId":format!("projection:{}","1".repeat(32)),"status":"rejected"})
                );
                reply(&mut stream, 200, br#"{"recorded":true}"#);
            }
        }
    });
    let cancel = Cancel::new();
    let control = Control::new(&client, &authorized, &cancel);
    let inbox = control.projection_inbox(InboxPolicy::Confirm).unwrap();
    assert_eq!(inbox.entries().len(), 1);
    control.reject_projection(&inbox.entries()[0]).unwrap();
    peer.join().unwrap();
}
#[test]
fn redirect_authentication_denial_and_false_receipt_are_not_success_or_retried() {
    for status in [302, 401, 403, 200] {
        let (listener, client, config) = fixture();
        let authorized = session(client.origin(), u64::MAX);
        let entry = Inbox::parse(&delivery::tests::body(client.origin()), client.origin())
            .unwrap()
            .into_entries()
            .pop()
            .unwrap();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            write!(stream,"HTTP/1.1 {status} Fixture\r\nLocation: https://other.invalid/\r\nContent-Length: 18\r\nConnection: close\r\n\r\n").unwrap();
            let _ = stream.write_all(br#"{"recorded":false}"#);
            let _ = stream.flush();
        });
        let cancel = Cancel::new();
        let error = Control::new(&client, &authorized, &cancel)
            .reject_projection(&entry)
            .unwrap_err();
        assert_eq!(
            error,
            match status {
                401 => LiveError::AuthorizationRequired,
                200 => LiveError::Projection(delivery::Error::Receipt),
                _ => LiveError::Status(status),
            }
        );
        peer.join().unwrap();
    }
}
#[test]
fn wrong_origin_or_expired_authorization_and_accepted_invitation_do_not_connect() {
    let (listener, client, _) = fixture();
    let cancel = Cancel::new();
    for authorized in [
        session("https://other.example", u64::MAX),
        session(client.origin(), 1),
    ] {
        assert_eq!(
            Control::new(&client, &authorized, &cancel)
                .projection_inbox(InboxPolicy::Confirm)
                .unwrap_err(),
            LiveError::Credential
        );
    }
    let body = serde_json::to_vec(
        &serde_json::json!({"invitations":[delivery::tests::invitation(client.origin(),true)]}),
    )
    .unwrap();
    let inbox = Inbox::parse(&body, client.origin()).unwrap();
    let authorized = session(client.origin(), u64::MAX);
    assert_eq!(
        Control::new(&client, &authorized, &cancel).reject_projection(&inbox.entries()[0]),
        Err(LiveError::Projection(delivery::Error::AlreadyAccepted))
    );
    assert_eq!(
        listener.accept().err().unwrap().kind(),
        io::ErrorKind::WouldBlock
    );
}
#[test]
fn inbox_rejects_cross_origin_response_and_oversized_reply() {
    for oversized in [false, true] {
        let (listener, client, config) = fixture();
        let authorized = session(client.origin(), u64::MAX);
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            if oversized {
                write!(
                    stream,
                    "HTTP/1.1 200 Fixture\r\nContent-Length: {}\r\n\r\n",
                    delivery::MAX_JSON + 1
                )
                .unwrap();
                stream.flush().unwrap();
            } else {
                reply(
                    &mut stream,
                    200,
                    &delivery::tests::body("https://other.example"),
                );
            }
        });
        let cancel = Cancel::new();
        let error = Control::new(&client, &authorized, &cancel)
            .projection_inbox(InboxPolicy::Confirm)
            .unwrap_err();
        assert_eq!(
            error,
            if oversized {
                LiveError::Http(Error::Limit)
            } else {
                LiveError::Projection(delivery::Error::Origin)
            }
        );
        peer.join().unwrap();
    }
}
