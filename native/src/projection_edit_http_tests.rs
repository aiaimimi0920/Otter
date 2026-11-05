//! 隔离真实TLS覆盖写前/丢回复/冲突/取消，复用Device与nonce框架，不访问真实设备。
use super::*;
use crate::projection_edit::{fixture::*, journal::Store, Code, Failure, Route};
fn begin(directory: &Directory, origin: &str) -> (Store, crate::projection_edit::journal::Ticket) {
    let store = directory.store();
    let ticket = store
        .begin(
            context(origin),
            Route::Shared,
            initialize(),
            None,
            view(),
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
    (store, ticket)
}
#[test]
fn shared_and_offline_reads_use_fixed_authenticated_routes_with_distinct_nonces() {
    let (listener, client, config) = fixture();
    let authorized = session(client.origin(), u64::MAX);
    let peer = std::thread::spawn(move || {
        let mut nonces = std::collections::HashSet::new();
        for path in ["/v1/projections/edit", "/v1/offline-projections/edit"] {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with(&format!("POST {path} HTTP/1.1\r\n")));
            let lower = head.to_ascii_lowercase();
            assert!(lower.contains("authorization: device fixture-secret\r\n"));
            let nonce = lower
                .lines()
                .find_map(|l| l.strip_prefix("x-loom-device-nonce: "))
                .unwrap();
            assert_eq!(nonce.len(), 32);
            assert!(nonces.insert(nonce.to_owned()));
            assert_eq!(
                body,
                serde_json::json!({"operation":"read","projectionId":PROJECTION})
            );
            reply(
                &mut stream,
                200,
                &serde_json::to_vec(&document_value()).unwrap(),
            );
        }
    });
    let cancel = Cancel::new();
    let control = Control::new(&client, &authorized, &cancel);
    for route in [Route::Shared, Route::Offline] {
        assert_eq!(
            control
                .read_projection_edit(&context(client.origin()), PROJECTION, route, None)
                .unwrap(),
            document()
        );
    }
    peer.join().unwrap();
}
#[test]
fn write_is_durable_and_http_success_does_not_retire_pending_checkpoint() {
    let (listener, client, config) = fixture();
    let authorized = session(client.origin(), u64::MAX);
    let directory = Directory::new();
    let (store, ticket) = begin(&directory, client.origin());
    let saved = std::fs::read(directory.path()).unwrap();
    let path = directory.path();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (_, body) = request(&mut stream);
        assert_eq!(std::fs::read(path).unwrap(), saved);
        assert_eq!(
            body,
            serde_json::from_slice::<serde_json::Value>(initialize().bytes()).unwrap()
        );
        reply(
            &mut stream,
            200,
            &serde_json::to_vec(&document_value()).unwrap(),
        );
    });
    let result = Control::new(&client, &authorized, &Cancel::new())
        .projection_edit(&ticket)
        .unwrap();
    assert_eq!(result, document());
    assert_eq!(
        store
            .pending(&std::sync::atomic::AtomicBool::new(false))
            .unwrap()
            .len(),
        1
    );
    ticket
        .commit_after_checkpoint(result, &std::sync::atomic::AtomicBool::new(false))
        .unwrap();
    assert!(store
        .pending(&std::sync::atomic::AtomicBool::new(false))
        .unwrap()
        .is_empty());
    peer.join().unwrap();
}
#[test]
fn dropped_reply_reopen_retries_identical_original_request_not_new_operation() {
    let (listener, client, config) = fixture();
    let authorized = session(client.origin(), u64::MAX);
    let directory = Directory::new();
    let (store, ticket) = begin(&directory, client.origin());
    let original = ticket.request().bytes().to_vec();
    let peer = std::thread::spawn(move || {
        let mut first = StreamOwned::new(
            ServerConnection::new(config.clone()).unwrap(),
            accept(&listener),
        );
        let (_, before) = request(&mut first);
        drop(first);
        let mut second =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (_, after) = request(&mut second);
        assert_eq!(before, after);
        reply(
            &mut second,
            200,
            &serde_json::to_vec(&document_value()).unwrap(),
        );
    });
    assert!(Control::new(&client, &authorized, &Cancel::new())
        .projection_edit(&ticket)
        .is_err());
    drop((ticket, store));
    let reopened = directory.store();
    let pending = reopened
        .pending(&std::sync::atomic::AtomicBool::new(false))
        .unwrap();
    assert_eq!(pending[0].request().bytes(), original);
    assert_eq!(
        Control::new(&client, &authorized, &Cancel::new())
            .projection_edit(&pending[0])
            .unwrap(),
        document()
    );
    assert_eq!(
        reopened
            .pending(&std::sync::atomic::AtomicBool::new(false))
            .unwrap()
            .len(),
        1
    );
    peer.join().unwrap();
}
#[test]
fn rejection_redirect_and_conflict_preserve_view_and_request_without_automatic_retry() {
    for (status, code, expected) in [
        (409, "projection_edit_object_conflict", Code::ObjectConflict),
        (409, "projection_edit_mode_conflict", Code::ModeConflict),
        (403, "projection_edit_read_only", Code::ReadOnly),
        (413, "projection_edit_log_full", Code::LogFull),
        (302, "secret", Code::Other),
        (429, "projection_busy", Code::Busy),
    ] {
        let (listener, client, config) = fixture();
        let authorized = session(client.origin(), u64::MAX);
        let directory = Directory::new();
        let (store, ticket) = begin(&directory, client.origin());
        let saved = std::fs::read(directory.path()).unwrap();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            reply(
                &mut stream,
                status,
                &serde_json::to_vec(
                    &serde_json::json!({"error":{"code":code,"detail":"fixture-secret"}}),
                )
                .unwrap(),
            );
            assert!(matches!(listener.accept(),Err(e) if e.kind()==io::ErrorKind::WouldBlock));
        });
        assert_eq!(
            Control::new(&client, &authorized, &Cancel::new()).projection_edit(&ticket),
            Err(Failure::Rejected {
                status,
                code: expected
            })
        );
        assert_eq!(std::fs::read(directory.path()).unwrap(), saved);
        assert_eq!(
            store
                .pending(&std::sync::atomic::AtomicBool::new(false))
                .unwrap()[0]
                .view(),
            &view()
        );
        peer.join().unwrap();
    }
}
#[test]
fn invalid_identity_expired_session_precancel_and_stale_ticket_do_not_connect() {
    let (listener, client, _) = fixture();
    let directory = Directory::new();
    let (_store, ticket) = begin(&directory, client.origin());
    for credential in [
        session("https://other.example", u64::MAX),
        session(client.origin(), 1),
    ] {
        assert!(Control::new(&client, &credential, &Cancel::new())
            .projection_edit(&ticket)
            .is_err());
    }
    let authorized = session(client.origin(), u64::MAX);
    let cancel = Cancel::new();
    cancel.cancel();
    assert!(Control::new(&client, &authorized, &cancel)
        .projection_edit(&ticket)
        .is_err());
    ticket
        .commit_after_checkpoint(document(), &std::sync::atomic::AtomicBool::new(false))
        .unwrap();
    assert!(Control::new(&client, &authorized, &Cancel::new())
        .projection_edit(&ticket)
        .is_err());
    assert!(matches!(listener.accept(),Err(e) if e.kind()==io::ErrorKind::WouldBlock));
}
#[test]
fn late_response_after_cancellation_or_external_disk_change_is_not_delivered() {
    for cancel_late in [false, true] {
        let (listener, client, config) = fixture();
        let authorized = session(client.origin(), u64::MAX);
        let directory = Directory::new();
        let (_store, ticket) = begin(&directory, client.origin());
        let path = directory.path();
        let cancel = std::sync::Arc::new(Cancel::new());
        let cancel_peer = cancel.clone();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            if cancel_late {
                cancel_peer.cancel();
            } else {
                std::fs::write(path, b"{}").unwrap();
            }
            let _ = write!(
                stream,
                "HTTP/1.1 200 Fixture\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.flush();
        });
        assert!(matches!(
            Control::new(&client, &authorized, &cancel).projection_edit(&ticket),
            Err(Failure::Journal(_))
        ));
        peer.join().unwrap();
    }
}
#[test]
fn oversized_unauthorized_or_mismatched_success_does_not_acknowledge_write() {
    for scenario in 0..3 {
        let (listener, client, config) = fixture();
        let authorized = session(client.origin(), u64::MAX);
        let directory = Directory::new();
        let (store, ticket) = begin(&directory, client.origin());
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            match scenario {
                0 => {
                    write!(
                        stream,
                        "HTTP/1.1 200 Fixture\r\nContent-Length: 262145\r\n\r\n"
                    )
                    .unwrap();
                    stream.flush().unwrap();
                }
                1 => reply(&mut stream, 401, b"invalid-json"),
                _ => {
                    let mut v = document_value();
                    v["basis"]["digest"] = "b".repeat(64).into();
                    reply(&mut stream, 200, &serde_json::to_vec(&v).unwrap());
                }
            }
        });
        let error = Control::new(&client, &authorized, &Cancel::new())
            .projection_edit(&ticket)
            .unwrap_err();
        if scenario == 1 {
            assert_eq!(error, Failure::Transport(LiveError::AuthorizationRequired));
        }
        assert_eq!(
            store
                .pending(&std::sync::atomic::AtomicBool::new(false))
                .unwrap()
                .len(),
            1
        );
        peer.join().unwrap();
    }
}
