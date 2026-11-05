//! 真实TLS控制GET矩阵；只消费测试Bridge，不触碰用户窗口。真实媒体屏障另由LAN联合验证。
use super::*;
use crate::device_authorization_lease::Grant;
use crate::device_session_client::tests::{accept, fixture_with_host, reply, request, session};
use rustls::{ServerConnection, StreamOwned};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
fn snapshot() -> Snapshot {
    Snapshot::parse(br#"{"session":{"protocolVersion":"loom.live.v1","sessionId":"live:test","sourceDeviceId":"device-1","sourceHookId":"hook:test","revision":1,"viewerDevices":[]},"epoch":1,"closed":false}"#, "live:test").unwrap()
}
fn signal(interaction: Arc<Interaction>) -> Arc<Signal> {
    let event = unsafe { CreateEventW(None, true, false, None) }.unwrap();
    Arc::new(Signal {
        stop: AtomicBool::new(false),
        socket: Mutex::new(None),
        worker: Mutex::new(None),
        ready: AtomicBool::new(true),
        recovering: AtomicBool::new(false),
        control_recovering: AtomicBool::new(false),
        observation_recovering: AtomicBool::new(false),
        restart_media: AtomicBool::new(false),
        generation: AtomicU64::new(0),
        event: unsafe { OwnedHandle::from_raw_handle(event.0) },
        interaction: Some(interaction),
        observation: None,
    })
}
fn history(first: u64, rows: Vec<(&str, Value)>) -> Vec<u8> {
    serde_json::to_vec(&json!({"protocolVersion":"loom.live.v1","next":first+rows.len() as u64-1,"reset":false,
        "events":rows.into_iter().enumerate().map(|(i,(kind,payload))|json!({"protocolVersion":"loom.live.v1","sessionId":"live:test","epoch":1,"sequence":first+i as u64,"messageType":kind,"payload":payload})).collect::<Vec<_>>()})).unwrap()
}
fn state(controller: Value, closed: bool) -> Value {
    json!({"revision":2,"visibility":if closed {"closed"} else {"visible"},"viewers":["a","b"],"controllerDeviceId":controller})
}
fn key(actor: &str, seq: u64, code: u8) -> Value {
    json!({"inputSequence":seq,"issuedAtMs":1,"sourceDeviceId":actor,"kind":{"kind":"key","data":{"code":format!("vk:{code}"),"state":"pressed","modifiers":0}}})
}
fn pump(
    thread: &JoinHandle<Result<(), Error>>,
    bridge: &input::Bridge,
    mut tick: impl FnMut(),
    limit: u64,
) -> Vec<input::Action> {
    let deadline = Instant::now() + Duration::from_secs(limit);
    let mut actions = Vec::new();
    while !thread.is_finished() {
        if let Some(request) = bridge.take() {
            actions.push(request.action);
            request.finish(Ok(()));
        }
        tick();
        assert!(Instant::now() < deadline, "control fixture deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
    actions
}
#[test]
fn source_control_transport_loss_keeps_cursor_consumes_old_input_and_requires_none_then_new_grant()
{
    let (listener, client, tls, _) = fixture_with_host("127.0.0.1");
    let mut grant = Grant::new(session(client.origin(), u64::MAX));
    let bridge = input::Bridge::new();
    let interaction = Interaction::new(grant.lease(client.origin(), 1).unwrap(), bridge.clone());
    let signal = signal(interaction.clone());
    let (ready, wait) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut nonces = std::collections::HashSet::new();
        for (stage, after) in [0, 2, 2, 4, 6].into_iter().enumerate() {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("GET /v1/live/sessions/"));
            assert!(head.contains(&format!("/events?after={after}&timeoutMs=750 ")));
            let nonce = head
                .lines()
                .find(|s| s.to_ascii_lowercase().starts_with("x-loom-device-nonce:"))
                .unwrap()
                .to_owned();
            assert!(nonces.insert(nonce));
            if stage == 1 {
                reply(&mut stream, 503, b"{}");
                continue;
            }
            if stage == 2 {
                wait.recv_timeout(Duration::from_secs(2)).unwrap();
            }
            let rows = match stage {
                0 => vec![
                    ("session_state", state(json!("a"), false)),
                    ("input_event", key("a", 1, 65)),
                ],
                2 => vec![
                    ("input_event", key("a", 2, 66)),
                    ("session_state", state(Value::Null, false)),
                ],
                3 => vec![
                    ("session_state", state(json!("b"), false)),
                    ("input_event", key("b", 1, 67)),
                ],
                _ => vec![("session_state", state(Value::Null, true))],
            };
            reply(&mut stream, 200, &history(after + 1, rows));
        }
    });
    let worker = start(client, snapshot(), interaction, signal.clone()).unwrap();
    let mut sent = false;
    let actions = pump(
        &worker,
        &bridge,
        || {
            if !sent && signal.restart_media.swap(false, Ordering::AcqRel) {
                assert!(signal.control_recovering.load(Ordering::Acquire));
                assert!(signal.recovering.load(Ordering::Acquire));
                // 单元门仅证明状态隔离；真实WSS退休/服务None由独立联合门证明。
                signal.recovery(false);
                ready.send(()).unwrap();
                sent = true;
            }
        },
        5,
    );
    assert_eq!(worker.join().unwrap(), Ok(()));
    let inputs: Vec<_> = actions
        .iter()
        .filter_map(|a| {
            if let input::Action::Input(e) = a {
                Some(*e)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        inputs,
        vec![
            crate::live_input::Event::Key {
                code: 65,
                down: true
            },
            crate::live_input::Event::Key {
                code: 67,
                down: true
            }
        ]
    );
    assert!(
        actions
            .iter()
            .filter(|a| **a == input::Action::Release)
            .count()
            >= 2
    );
    assert!(!signal.control_recovering.load(Ordering::Acquire));
    peer.join().unwrap();
}
#[test]
fn source_control_exhausts_four_retries_and_does_not_retry_authority_protocol_or_epoch() {
    for case in 0..4 {
        let (listener, client, tls, _) = fixture_with_host("127.0.0.1");
        let mut grant = Grant::new(session(client.origin(), u64::MAX));
        let bridge = input::Bridge::new();
        let interaction =
            Interaction::new(grant.lease(client.origin(), 1).unwrap(), bridge.clone());
        let signal = signal(interaction.clone());
        let peer = std::thread::spawn(move || {
            for _ in 0..if case == 0 { 5 } else { 1 } {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, _) = request(&mut stream);
                assert!(head.contains("after=0&"));
                if case < 2 {
                    reply(&mut stream, if case == 0 { 503 } else { 401 }, b"{}");
                } else {
                    let mut value: Value = serde_json::from_slice(&history(
                        1,
                        vec![("session_state", state(Value::Null, false))],
                    ))
                    .unwrap();
                    if case == 2 {
                        value["reset"] = json!(true);
                    } else {
                        value["events"][0]["epoch"] = json!(2);
                    }
                    reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
                }
            }
        });
        let worker = start(client, snapshot(), interaction, signal.clone()).unwrap();
        let _ = pump(&worker, &bridge, || {}, 8);
        let error = worker.join().unwrap().unwrap_err();
        assert!(match case {
            0 => error == Error::Http(live_relay_http::Error::Status(503)),
            1 => error == Error::Http(live_relay_http::Error::AuthorizationRequired),
            _ => matches!(error, Error::Http(live_relay_http::Error::Protocol(_))),
        });
        assert_eq!(signal.restart_media.load(Ordering::Acquire), case == 0);
        peer.join().unwrap();
    }
}
#[test]
fn source_control_stop_or_revoke_wakes_backoff_without_new_request() {
    for revoke in [false, true] {
        let (listener, client, tls, _) = fixture_with_host("127.0.0.1");
        let mut grant = Grant::new(session(client.origin(), u64::MAX));
        let bridge = input::Bridge::new();
        let interaction =
            Interaction::new(grant.lease(client.origin(), 1).unwrap(), bridge.clone());
        let signal = signal(interaction.clone());
        let (done, wait) = std::sync::mpsc::channel();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
            request(&mut stream);
            reply(&mut stream, 503, b"{}");
            wait.recv_timeout(Duration::from_secs(2)).unwrap();
            assert!(matches!(listener.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
        });
        let worker = start(client, snapshot(), interaction.clone(), signal.clone()).unwrap();
        let mut cancelled = false;
        let began = Instant::now();
        pump(
            &worker,
            &bridge,
            || {
                if !cancelled && signal.control_recovering.load(Ordering::Acquire) {
                    if revoke {
                        interaction.lease.revoke();
                    } else {
                        signal.stop();
                    }
                    cancelled = true;
                }
            },
            2,
        );
        assert_eq!(worker.join().unwrap(), Ok(()));
        assert!(began.elapsed() < Duration::from_secs(2));
        done.send(()).unwrap();
        peer.join().unwrap();
    }
}
