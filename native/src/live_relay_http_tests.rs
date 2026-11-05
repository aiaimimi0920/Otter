//! 真实回环 HTTPS 合同：Device/nonce、路径转义、加入与控制历史；不操作在线 Loom。
use super::*;
use crate::{
    device_request_cancel::Cancel,
    live_relay_http::{Attach, Control, Error as LiveError},
    live_relay_viewer_identity::ControlCursor,
};
#[path = "art_prepare_http_tests.rs"]
mod art_prepare;
#[path = "live_relay_catalog_http_tests.rs"]
mod catalog_http;
#[path = "live_relay_catalog_runtime_tests.rs"]
mod catalog_runtime;
#[path = "live_relay_control_http_tests.rs"]
mod control_http;
#[path = "live_relay_control_runtime_tests.rs"]
mod control_runtime;
#[path = "live_relay_join_http_tests.rs"]
mod join_http;
#[path = "device_authorization_lease_http_tests.rs"]
mod lease_http;
#[path = "projection_delivery_http_tests.rs"]
mod projection_delivery;
#[path = "projection_edit_http_tests.rs"]
mod projection_edit;
#[path = "projection_edit_view_http_tests.rs"]
mod projection_edit_view;
#[path = "live_relay_source_http_tests.rs"]
mod source_http;
#[path = "live_source_observation_http_tests.rs"]
mod source_observation;
#[path = "live_relay_source_runtime_tests.rs"]
mod source_runtime;
#[path = "surface_binding_http_tests.rs"]
mod surface_binding;
#[path = "live_relay_viewer_input_http_tests.rs"]
mod viewer_input;
#[path = "live_relay_viewer_trigger_http_tests.rs"]
mod viewer_trigger;

pub(crate) fn session(origin: &str, expiry: u64) -> Session {
    let mut identity = Identity::generate().unwrap();
    identity.bind("device-1").unwrap();
    Attempt::new(&identity, origin)
        .unwrap()
        .challenge(
            200,
            br#"{"deviceId":"device-1","challengeId":"id","challenge":"value"}"#,
        )
        .unwrap()
        .finish(
            200,
            &serde_json::to_vec(&serde_json::json!({"deviceId":"device-1",
            "token":"fixture-secret","expiresAtMs":expiry}))
            .unwrap(),
            0,
        )
        .unwrap()
}
fn snapshot() -> Vec<u8> {
    serde_json::to_vec(
        &serde_json::json!({"session":{"protocolVersion":"loom.live.v1",
        "sessionId":"live:test/part","sourceDeviceId":"source-1","sourceHookId":"hook-1",
        "revision":2,"viewerDevices":["device-1"],"observationCapabilities":[]},"epoch":1,"closed":false,"observations":[]}),
    )
    .unwrap()
}
fn history(reset: bool) -> Vec<u8> {
    serde_json::to_vec(
        &serde_json::json!({"protocolVersion":"loom.live.v1","next":7,"reset":reset,
        "events":[{"protocolVersion":"loom.live.v1","sessionId":"live:test/part","epoch":1,
        "sequence":7,"messageType":"session_state","payload":{"revision":2,
        "reason":"viewer_joined","visibility":"visible","viewers":["device-1"]}}]}),
    )
    .unwrap()
}
#[test]
fn actual_https_snapshot_attach_bootstrap_and_poll_keep_contract_and_distinct_nonces() {
    let (listener, client, config) = fixture();
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        let mut nonces = std::collections::HashSet::new();
        for index in 0..4 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            let headers = head.to_ascii_lowercase();
            assert!(headers.contains("authorization: device fixture-secret\r\n"));
            let nonce = headers
                .lines()
                .find_map(|line| line.strip_prefix("x-loom-device-nonce: "))
                .unwrap();
            assert_eq!(nonce.len(), 32);
            assert!(nonces.insert(nonce.to_owned()));
            let route = match index {
                0 => "GET /v1/live/sessions/live:test%2Fpart HTTP/1.1",
                1 => "POST /v1/live/sessions/live:test%2Fpart/viewers HTTP/1.1",
                2 => "GET /v1/live/sessions/live:test%2Fpart/events?after=0&timeoutMs=0 HTTP/1.1",
                _ => "GET /v1/live/sessions/live:test%2Fpart/events?after=6&timeoutMs=750 HTTP/1.1",
            };
            assert!(
                head.starts_with(route),
                "unexpected route: {}",
                head.lines().next().unwrap()
            );
            if index == 1 {
                assert_eq!(body["surfaceInstanceId"], "surface-1");
                assert_eq!(body["attachmentId"], "attachment-1");
                assert_eq!(body["requireExistingMembership"], true);
                assert_eq!(body["envelope"]["sequence"], 4);
                assert_eq!(body["envelope"]["messageType"], "session_ack");
                assert_eq!(body["envelope"]["payload"]["responderDeviceId"], "device-1");
            } else {
                assert!(body.is_null());
            }
            reply(
                &mut stream,
                200,
                &if index < 2 {
                    snapshot()
                } else {
                    history(index == 2)
                },
            );
        }
    });
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    let preview = control.snapshot("live:test/part").unwrap();
    let attached = control
        .attach(
            preview.identity(),
            Attach {
                surface: "surface-1",
                attachment: "attachment-1",
                sequence: 4,
                require_existing_membership: true,
            },
        )
        .unwrap();
    let after = control.bootstrap_cursor(&attached).unwrap();
    assert_eq!(after, 6);
    let cursor = ControlCursor::new(attached.identity().clone(), after);
    let batch = control.events("live:test/part", &cursor).unwrap();
    assert_eq!(batch.next(), 7);
    assert_eq!(
        cursor.after(),
        6,
        "HTTP fetch must not commit unapplied control events"
    );
    peer.join().unwrap();
}
#[test]
fn authentication_status_is_terminal_even_with_invalid_or_oversized_error_body() {
    let (listener, client, config) = fixture();
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        stream
            .write_all(
                b"HTTP/1.1 401 Fixture\r\nContent-Length: 999999999\r\nConnection: close\r\n\r\n",
            )
            .unwrap();
        stream.flush().unwrap();
    });
    assert!(matches!(
        Control::new(&client, &session, &Cancel::new()).snapshot("live:test"),
        Err(LiveError::AuthorizationRequired)
    ));
    peer.join().unwrap();
}
#[test]
fn credential_scope_expiry_input_and_pre_cancel_fail_without_connecting() {
    let (listener, client, _) = fixture();
    let wrong = session("https://other.example", u64::MAX);
    let expired = session(&client.origin, 1);
    for credential in [&wrong, &expired] {
        assert!(matches!(
            Control::new(&client, credential, &Cancel::new()).snapshot("live:test"),
            Err(LiveError::Credential)
        ));
    }
    let valid = session(&client.origin, u64::MAX);
    let cancel = Cancel::new();
    let control = Control::new(&client, &valid, &cancel);
    for id in ["", ".", "..", "id?other", "id#fragment", "id%2fother"] {
        assert!(matches!(control.snapshot(id), Err(LiveError::Input)));
    }
    cancel.cancel();
    assert!(matches!(
        control.snapshot("live:test"),
        Err(LiveError::Http(Error::Stopped))
    ));
    assert!(matches!(listener.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
}
#[test]
fn live_response_limit_and_redirect_fail_without_accepting_snapshot() {
    for status in [200, 302] {
        let (listener, client, config) = fixture();
        let session = session(&client.origin, u64::MAX);
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            if status == 200 {
                stream
                    .write_all(b"HTTP/1.1 200 Fixture\r\nContent-Length: 1048577\r\n\r\n")
                    .unwrap();
                stream.flush().unwrap();
            } else {
                reply(&mut stream, status, b"{}");
            }
        });
        let result = Control::new(&client, &session, &Cancel::new()).snapshot("live:test");
        if status == 200 {
            assert!(matches!(result, Err(LiveError::Http(Error::Limit))));
        } else {
            assert!(matches!(result, Err(LiveError::Status(302))));
        }
        peer.join().unwrap();
    }
}

#[test]
fn terminal_requires_authenticated_member_snapshot_not_404_or_other_member() {
    use crate::live_relay_viewer_identity::Snapshot;
    for scenario in 0..3 {
        let (listener, client, config) = fixture();
        let session = session(&client.origin, u64::MAX);
        let expected = Snapshot::parse(&snapshot(), "live:test/part").unwrap();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            let mut body: serde_json::Value = serde_json::from_slice(&snapshot()).unwrap();
            body["closed"] = true.into();
            if scenario == 2 {
                body["session"]["viewerDevices"] = serde_json::json!(["other-device"]);
            }
            reply(
                &mut stream,
                if scenario == 1 { 404 } else { 200 },
                &serde_json::to_vec(&body).unwrap(),
            );
        });
        let result = Control::new(&client, &session, &Cancel::new()).terminal(expected.identity());
        match scenario {
            0 => assert_eq!(result, Ok(true)),
            1 => assert_eq!(result, Err(LiveError::Status(404))),
            _ => assert!(matches!(result, Err(LiveError::Protocol(_)))),
        }
        peer.join().unwrap();
    }
}
