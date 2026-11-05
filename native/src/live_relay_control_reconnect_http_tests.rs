//! 实际TLS GET重试、水位/nonce、注册刷新和终态；独立wire oracle，不操作用户配置或设备。
use super::*;
use std::{
    io::Write,
    sync::mpsc,
    time::{Duration, Instant},
};
fn spin(mut condition: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(8);
    while !condition() {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn event(trigger: bool) -> Vec<u8> {
    let mut value: serde_json::Value = serde_json::from_slice(&history(false)).unwrap();
    value["next"] = 8.into();
    value["events"][0]["sequence"] = 8.into();
    if trigger {
        value["events"][0]["messageType"] = "trigger_condition".into();
        value["events"][0]["payload"] = serde_json::json!({"conditionId":"c","revision":1,"observationId":"o","operator":"equals","operand":true,"stableForMs":0,"risingEdge":false,"rearm":false,"minimumConfidence":"exact"});
    }
    serde_json::to_vec(&value).unwrap()
}
#[test]
fn read_transport_and_503_retry_same_cursor_without_rejoin_or_duplicate_consume() {
    for refresh in [false, true] {
        let (listener, client, tls) = fixture();
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let lease = grant.lease(&client.origin, 0).unwrap();
        let (fault_tx, fault_rx) = mpsc::channel();
        let (finish_tx, finish_rx) = mpsc::channel();
        let peer = std::thread::spawn(move || {
            joined(&listener, &tls);
            let mut nonces = std::collections::HashSet::new();
            for stage in 0..4 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, _) = request(&mut stream);
                assert!(head.starts_with("GET "));
                let nonce = head
                    .lines()
                    .find_map(|l| {
                        l.split_once(':')
                            .filter(|(k, _)| k.eq_ignore_ascii_case("x-loom-device-nonce"))
                            .map(|(_, v)| v.trim().to_owned())
                    })
                    .unwrap();
                assert!(nonces.insert(nonce));
                if refresh {
                    match stage {
                        0 => {
                            assert!(head.contains("after=7&"));
                            reply(&mut stream, 200, &event(true));
                        }
                        1 => {
                            assert!(!head.contains("/events?"));
                            reply(&mut stream, 503, b"{}");
                            fault_tx.send(()).unwrap();
                        }
                        2 => {
                            assert!(!head.contains("/events?"));
                            let mut value: serde_json::Value =
                                serde_json::from_slice(&snapshot()).unwrap();
                            value["requesterControl"] = serde_json::json!({"deviceId":"device-1","epoch":1,"controlSequence":1,"inputSequence":0});
                            reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
                        }
                        _ => {
                            assert!(head.contains("after=8&"));
                            finish_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                            reply(&mut stream, 401, b"{}");
                        }
                    }
                } else {
                    assert!(head.contains(if stage == 3 { "after=8&" } else { "after=7&" }));
                    match stage {
                        0 => {
                            reply(&mut stream, 503, b"{}");
                            fault_tx.send(()).unwrap();
                        }
                        1 => {
                            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\n{").unwrap();
                            stream.flush().unwrap();
                        }
                        2 => reply(&mut stream, 200, &event(false)),
                        _ => {
                            finish_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                            reply(&mut stream, 401, b"{}");
                        }
                    }
                }
            }
        });
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let recorded = seen.clone();
        let mut runtime = Runtime::start(client, lease, target(), move |events| {
            recorded
                .lock()
                .unwrap()
                .extend(events.iter().map(|e| e.sequence()));
            Ok(())
        })
        .unwrap();
        wait(runtime.ready_handle());
        let ready = runtime.try_ready().unwrap();
        fault_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        spin(|| ready.input.control_unavailable());
        assert!(ready.media.validate(&ready.permit).is_ok());
        assert!(!ready.input.request());
        spin(|| {
            ready.input.test_cursor.load(Ordering::Acquire) == 8
                && !ready.input.control_unavailable()
        });
        assert_eq!(*seen.lock().unwrap(), [7, 8]);
        assert!(ready.media.validate(&ready.permit).is_ok());
        finish_tx.send(()).unwrap();
        wait(runtime.wait_handle().unwrap());
        assert_eq!(
            runtime.try_join(),
            Some(Exit::Control(OwnerError::Http(
                LiveError::AuthorizationRequired
            )))
        );
        assert!(ready.media.validate(&ready.permit).is_err());
        peer.join().unwrap();
    }
}
#[test]
fn five_failed_reads_exhaust_four_retries_without_changing_cursor_or_rejoining() {
    let (listener, client, tls) = fixture();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let peer = std::thread::spawn(move || {
        joined(&listener, &tls);
        for _ in 0..5 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            assert!(head.contains("after=7&"));
            reply(&mut stream, 503, b"{}");
        }
    });
    let mut runtime = Runtime::start(client, lease, target(), |_| Ok(())).unwrap();
    wait(runtime.ready_handle());
    let ready = runtime.try_ready().unwrap();
    spin(|| runtime.thread_finished_for_test());
    assert_eq!(
        runtime.try_join(),
        Some(Exit::Control(OwnerError::Http(LiveError::Status(503))))
    );
    assert_eq!(ready.input.test_cursor.load(Ordering::Acquire), 7);
    assert!(ready.media.validate(&ready.permit).is_err());
    peer.join().unwrap();
}
#[test]
fn stop_or_grant_revocation_wakes_control_backoff_without_next_read() {
    for revoke in [false, true] {
        let (listener, client, tls) = fixture();
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let lease = grant.lease(&client.origin, 0).unwrap();
        let (done_tx, done_rx) = mpsc::channel();
        let peer = std::thread::spawn(move || {
            joined(&listener, &tls);
            let mut stream =
                StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
            request(&mut stream);
            reply(&mut stream, 503, b"{}");
            done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(matches!(listener.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
        });
        let mut runtime = Runtime::start(client, lease, target(), |_| Ok(())).unwrap();
        wait(runtime.ready_handle());
        let ready = runtime.try_ready().unwrap();
        spin(|| ready.input.control_unavailable());
        let start = Instant::now();
        if revoke {
            drop(grant);
        } else {
            runtime.request_stop();
        }
        wait(runtime.wait_handle().unwrap());
        assert_eq!(runtime.try_join(), Some(Exit::Stopped));
        assert!(start.elapsed() < Duration::from_secs(2));
        done_tx.send(()).unwrap();
        peer.join().unwrap();
    }
}
#[test]
fn reset_wrong_epoch_and_malformed_json_are_terminal_not_recovery() {
    for mode in 0..3 {
        let (listener, client, tls) = fixture();
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let lease = grant.lease(&client.origin, 0).unwrap();
        let peer = std::thread::spawn(move || {
            joined(&listener, &tls);
            let mut stream =
                StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
            request(&mut stream);
            let mut value: serde_json::Value = serde_json::from_slice(&event(false)).unwrap();
            if mode == 0 {
                value["reset"] = true.into();
            } else {
                value["events"][0]["epoch"] = 2.into();
            }
            reply(
                &mut stream,
                200,
                &if mode == 2 {
                    b"{".to_vec()
                } else {
                    serde_json::to_vec(&value).unwrap()
                },
            );
        });
        let mut runtime = Runtime::start(client, lease, target(), |_| Ok(())).unwrap();
        wait(runtime.ready_handle());
        let ready = runtime.try_ready().unwrap();
        wait(runtime.wait_handle().unwrap());
        assert!(matches!(
            runtime.try_join(),
            Some(Exit::Control(OwnerError::Http(LiveError::Protocol(_))))
        ));
        assert!(!ready.input.control_unavailable());
        assert!(ready.media.validate(&ready.permit).is_err());
        peer.join().unwrap();
    }
}
