//! Owned TLS/raw-source recovery: independent wire pixels/watermarks/nonces/create counts; no desktop capture or real-configuration operations.
use super::*;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};
fn spin(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(12);
    while !predicate() {
        assert!(Instant::now() < deadline, "source recovery deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn created(listener: &std::net::TcpListener, tls: &std::sync::Arc<rustls::ServerConfig>) -> Value {
    let mut stream = StreamOwned::new(
        ServerConnection::new(tls.clone()).unwrap(),
        accept(listener),
    );
    let (head, body) = request(&mut stream);
    assert!(head.starts_with("POST /v1/live/sessions "));
    let value = json!({"session":body["envelope"]["payload"]["session"],"epoch":1,"closed":false});
    reply(&mut stream, 201, &serde_json::to_vec(&value).unwrap());
    value
}
#[allow(clippy::result_large_err)]
fn socket(
    listener: &std::net::TcpListener,
    tls: &std::sync::Arc<rustls::ServerConfig>,
    nonces: &mut std::collections::HashSet<String>,
) -> tungstenite::WebSocket<StreamOwned<ServerConnection, crate::loom_bridge_stream::BridgeStream>>
{
    tungstenite::accept_hdr(
        StreamOwned::new(
            ServerConnection::new(tls.clone()).unwrap(),
            accept(listener),
        ),
        |request: &tungstenite::handshake::server::Request,
         mut response: tungstenite::handshake::server::Response| {
            assert!(request.uri().query().unwrap().contains("role=source"));
            assert!(nonces.insert(
                request.headers()["x-loom-device-nonce"]
                    .to_str()
                    .unwrap()
                    .into()
            ));
            assert_eq!(request.headers()["sec-websocket-protocol"], "loom.live.v1");
            response
                .headers_mut()
                .insert("sec-websocket-protocol", "loom.live.v1".parse().unwrap());
            Ok(response)
        },
    )
    .unwrap()
}
fn frame(
    socket: &mut tungstenite::WebSocket<
        StreamOwned<ServerConnection, crate::loom_bridge_stream::BridgeStream>,
    >,
    expected: u64,
) {
    let bytes = socket.read().unwrap().into_data();
    assert_eq!(bytes.len(), 68);
    assert_eq!(&bytes[..8], b"NLLV\x01\x01\x00\x40");
    assert_eq!(&bytes[8..16], &1u64.to_be_bytes());
    assert_eq!(&bytes[16..24], &expected.to_be_bytes());
    assert_eq!(&bytes[40..48], &[0, 0, 0, 1, 0, 0, 0, 1]);
    assert_eq!(&bytes[64..], &[17, 83, 211, 255]);
}
fn closed(
    listener: &std::net::TcpListener,
    tls: &std::sync::Arc<rustls::ServerConfig>,
    value: &mut Value,
) {
    let mut stream = StreamOwned::new(
        ServerConnection::new(tls.clone()).unwrap(),
        accept(listener),
    );
    let (head, body) = request(&mut stream);
    assert!(head.starts_with("POST ") && head.contains("/close HTTP/1.1"));
    assert_eq!(body["envelope"]["sequence"], 2);
    value["closed"] = json!(true);
    reply(&mut stream, 200, &serde_json::to_vec(value).unwrap());
}
fn start(
    origin: &str,
    root: &std::path::Path,
    grant: &mut Grant,
) -> (
    Source<Texture>,
    std::sync::Arc<crate::byte_budget::Budget>,
    Runtime,
) {
    let frames = Source::new();
    let consumer = frames.subscribe().unwrap();
    frames
        .publish(crate::live_relay_source_runtime::test_texture::texture(
            [1, 1],
            [17, 83, 211, 255],
        ))
        .unwrap();
    let budget = crate::byte_budget::Budget::new(1024 * 1024);
    let runtime = Runtime::start(
        config(origin, root),
        grant.lease(origin, 1).unwrap(),
        Publication::new("hook:test", [1, 1], None).unwrap(),
        consumer,
        budget.clone(),
    )
    .unwrap();
    (frames, budget, runtime)
}
#[test]
fn source_reconnect_keeps_consumer_monotonic_frames_new_nonce_and_requires_real_ack() {
    let (files, _, _) = pairing::Files::new();
    let (listener, client, tls, pem) = fixture_with_host("127.0.0.1");
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let (received, wait) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut value = created(&listener, &tls);
        let mut nonces = std::collections::HashSet::new();
        let mut first = socket(&listener, &tls, &mut nonces);
        frame(&mut first, 1);
        drop(first);
        let mut second = socket(&listener, &tls, &mut nonces);
        frame(&mut second, 2);
        for last in [1, 2] {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("GET ") && !head.contains("/events"));
            value["sourceConnected"] = json!(true);
            value["lastFrameId"] = json!(last);
            reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
        }
        assert_eq!(nonces.len(), 2);
        received.send(()).unwrap();
        closed(&listener, &tls, &mut value);
    });
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let (frames, budget, mut runtime) = start(&client.origin, files.root(), &mut grant);
    spin(|| runtime.recovering());
    assert!(!runtime.ready());
    wait.recv_timeout(Duration::from_secs(5)).unwrap();
    spin(|| runtime.ready());
    assert!(frames.has_consumers());
    runtime.request_stop();
    spin(|| runtime.test_finished());
    assert_eq!(
        runtime.try_join(),
        Some(crate::live_relay_source_runtime::Outcome {
            exit: Exit::Stopped,
            cleanup: Cleanup::Confirmed
        })
    );
    assert!(!frames.has_consumers());
    assert_eq!(budget.used(), 0);
    peer.join().unwrap();
}
#[test]
fn four_retries_are_not_reset_by_successful_upgrade_or_local_frame_send() {
    let (files, _, _) = pairing::Files::new();
    let (listener, client, tls, pem) = fixture_with_host("127.0.0.1");
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let peer = std::thread::spawn(move || {
        let mut value = created(&listener, &tls);
        let mut nonces = std::collections::HashSet::new();
        for id in 1..=5 {
            let mut media = socket(&listener, &tls, &mut nonces);
            frame(&mut media, id);
            if id > 1 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, _) = request(&mut stream);
                assert!(head.starts_with("GET "));
                reply(&mut stream, 503, b"{}");
            }
        }
        assert_eq!(nonces.len(), 5);
        closed(&listener, &tls, &mut value);
    });
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let (frames, budget, mut runtime) = start(&client.origin, files.root(), &mut grant);
    spin(|| runtime.test_finished());
    let outcome = runtime.try_join().unwrap();
    assert_eq!(
        outcome.exit,
        Exit::Http(crate::live_relay_http::Error::Status(503))
    );
    assert_eq!(outcome.cleanup, Cleanup::Confirmed);
    assert_eq!(budget.used(), 0);
    assert!(!frames.has_consumers());
    peer.join().unwrap();
}
#[test]
fn source_stop_or_revoke_wakes_backoff_without_reconnecting() {
    for revoke in [false, true] {
        let (files, _, _) = pairing::Files::new();
        let (listener, client, tls, pem) = fixture_with_host("127.0.0.1");
        std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
        let (done, wait) = mpsc::channel();
        let peer = std::thread::spawn(move || {
            let mut value = created(&listener, &tls);
            let mut media = socket(&listener, &tls, &mut std::collections::HashSet::new());
            frame(&mut media, 1);
            drop(media);
            if !revoke {
                closed(&listener, &tls, &mut value);
            }
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(
                matches!(listener.accept(), Err(e) if e.kind()==std::io::ErrorKind::WouldBlock)
            );
        });
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let (frames, budget, mut runtime) = start(&client.origin, files.root(), &mut grant);
        spin(|| runtime.recovering());
        let began = Instant::now();
        if revoke {
            drop(grant);
        } else {
            runtime.request_stop();
        }
        spin(|| runtime.test_finished());
        assert!(began.elapsed() < Duration::from_secs(2));
        let outcome = runtime.try_join().unwrap();
        assert_eq!(outcome.exit, Exit::Stopped);
        assert_eq!(
            outcome.cleanup,
            if revoke {
                Cleanup::AuthorizationEnded
            } else {
                Cleanup::Confirmed
            }
        );
        assert!(!frames.has_consumers());
        assert_eq!(budget.used(), 0);
        done.send(()).unwrap();
        peer.join().unwrap();
    }
}
#[test]
fn normal_close_and_invalid_messages_are_terminal_without_reconnect() {
    for malformed in [false, true] {
        let (files, _, _) = pairing::Files::new();
        let (listener, client, tls, pem) = fixture_with_host("127.0.0.1");
        std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
        let peer = std::thread::spawn(move || {
            let mut value = created(&listener, &tls);
            let mut media = socket(&listener, &tls, &mut std::collections::HashSet::new());
            frame(&mut media, 1);
            if malformed {
                media
                    .send(tungstenite::Message::Text("invalid".into()))
                    .unwrap();
            } else {
                media.close(None).unwrap();
            }
            closed(&listener, &tls, &mut value);
        });
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let (frames, budget, mut runtime) = start(&client.origin, files.root(), &mut grant);
        spin(|| runtime.test_finished());
        assert_eq!(
            runtime.try_join().unwrap().exit,
            if malformed {
                Exit::Media(crate::live_relay_media_transport::Error::Media(
                    crate::live_relay_media_socket::Error::Message,
                ))
            } else {
                Exit::RemoteClosed
            }
        );
        assert_eq!(budget.used(), 0);
        assert!(!frames.has_consumers());
        peer.join().unwrap();
    }
}
#[test]
fn local_restart_interrupts_tls_or_upgrade_without_losing_pending_rotation() {
    for upgrade in [false, true] {
        let (files, _, _) = pairing::Files::new();
        let (listener, client, tls, pem) = fixture_with_host("127.0.0.1");
        std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
        let (started, handshaking) = mpsc::channel();
        let (confirmed, wait) = mpsc::channel();
        let peer = std::thread::spawn(move || {
            let mut value = created(&listener, &tls);
            let tcp = accept(&listener);
            if upgrade {
                let mut held = StreamOwned::new(ServerConnection::new(tls.clone()).unwrap(), tcp);
                let (head, _) = request(&mut held);
                assert!(head.starts_with("GET /v1/live/media"));
                started.send(()).unwrap();
                assert!(held.read_exact(&mut [0]).is_err());
            } else {
                let mut held = tcp;
                let mut byte = [0];
                held.read_exact(&mut byte).unwrap();
                assert_eq!(byte, [22]); // TLS ClientHello record.
                started.send(()).unwrap();
                let mut rest = Vec::new();
                held.read_to_end(&mut rest).unwrap();
            }
            let mut nonces = std::collections::HashSet::new();
            let mut retired = socket(&listener, &tls, &mut nonces);
            assert!(retired.read().is_err(), "rotation must not publish a frame");
            let mut current = socket(&listener, &tls, &mut nonces);
            frame(&mut current, 1);
            let mut receipt = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut receipt);
            assert!(head.starts_with("GET /v1/live/sessions/"));
            value["sourceConnected"] = json!(true);
            value["lastFrameId"] = json!(1);
            reply(&mut receipt, 200, &serde_json::to_vec(&value).unwrap());
            assert_eq!(nonces.len(), 2);
            confirmed.send(()).unwrap();
            closed(&listener, &tls, &mut value);
        });
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let (frames, budget, mut runtime) = start(&client.origin, files.root(), &mut grant);
        handshaking.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(!runtime.ready());
        runtime.test_request_media_restart();
        wait.recv_timeout(Duration::from_secs(5)).unwrap();
        spin(|| runtime.ready());
        runtime.request_stop();
        spin(|| runtime.test_finished());
        let outcome = runtime.try_join().unwrap();
        assert_eq!(outcome.exit, Exit::Stopped);
        assert_eq!(outcome.cleanup, Cleanup::Confirmed);
        assert_eq!(budget.used(), 0);
        assert!(!frames.has_consumers());
        peer.join().unwrap();
    }
}

#[test]
fn uncancelled_tls_and_upgrade_failures_remain_terminal() {
    for upgrade in [false, true] {
        let (files, _, _) = pairing::Files::new();
        let (listener, client, tls, pem) = fixture_with_host("127.0.0.1");
        std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
        let peer = std::thread::spawn(move || {
            let mut value = created(&listener, &tls);
            let mut tcp = accept(&listener);
            if upgrade {
                let mut rejected =
                    StreamOwned::new(ServerConnection::new(tls.clone()).unwrap(), tcp);
                let (head, _) = request(&mut rejected);
                assert!(head.starts_with("GET /v1/live/media"));
                reply(&mut rejected, 403, b"{}");
            } else {
                tcp.read_exact(&mut [0]).unwrap();
                tcp.write_all(b"not tls").unwrap();
            }
            closed(&listener, &tls, &mut value);
        });
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let (frames, budget, mut runtime) = start(&client.origin, files.root(), &mut grant);
        spin(|| runtime.test_finished());
        let outcome = runtime.try_join().unwrap();
        use crate::live_relay_media_transport::Error;
        assert_eq!(
            outcome.exit,
            Exit::Media(if upgrade { Error::Upgrade } else { Error::Tls })
        );
        assert_eq!(outcome.cleanup, Cleanup::Confirmed);
        assert_eq!(budget.used(), 0);
        assert!(!frames.has_consumers());
        peer.join().unwrap();
    }
}
#[test]
fn stop_cancels_inflight_progress_but_closes_only_with_valid_original_authority() {
    for revoke in [false, true] {
        let (files, _, _) = pairing::Files::new();
        let (listener, client, tls, pem) = fixture_with_host("127.0.0.1");
        std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
        let (started, wait) = mpsc::channel();
        let (joined, done) = mpsc::channel();
        let peer = std::thread::spawn(move || {
            let mut value = created(&listener, &tls);
            let mut nonces = std::collections::HashSet::new();
            let mut first = socket(&listener, &tls, &mut nonces);
            frame(&mut first, 1);
            drop(first);
            let mut second = socket(&listener, &tls, &mut nonces);
            frame(&mut second, 2);
            let mut progress = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut progress);
            assert!(head.starts_with("GET /v1/live/sessions/") && !head.contains("/events"));
            started.send(()).unwrap();
            assert!(progress.read_exact(&mut [0]).is_err());
            if !revoke {
                closed(&listener, &tls, &mut value);
            }
            done.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(
                matches!(listener.accept(), Err(e) if e.kind()==std::io::ErrorKind::WouldBlock)
            );
        });
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let (frames, budget, mut runtime) = start(&client.origin, files.root(), &mut grant);
        wait.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(!runtime.ready());
        let began = Instant::now();
        if revoke {
            drop(grant);
        } else {
            runtime.request_stop();
        }
        spin(|| runtime.test_finished());
        assert!(began.elapsed() < Duration::from_secs(2));
        let outcome = runtime.try_join().unwrap();
        assert_eq!(outcome.exit, Exit::Stopped);
        assert_eq!(
            outcome.cleanup,
            if revoke {
                Cleanup::AuthorizationEnded
            } else {
                Cleanup::Confirmed
            }
        );
        assert_eq!(budget.used(), 0);
        assert!(!frames.has_consumers());
        joined.send(()).unwrap();
        peer.join().unwrap();
    }
}

#[path = "live_relay_source_denied_http_tests.rs"]
mod denied;
#[path = "live_relay_source_rotation_http_tests.rs"]
mod rotation;
