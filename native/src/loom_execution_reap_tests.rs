//! 回环 TLS 的真实远端执行：通知尾段不能阻塞 UI，待清理报告不能留下空转截止。
use super::*;
use crate::{
    desktop::{
        loom_execution_session::Failure,
        loom_execution_tasks::{
            reap_fixture::{Gate, WAIT},
            Tasks,
        },
    },
    graph_requests::Requests,
};
use std::{net::TcpListener, sync::Arc};

fn accept(
    listener: &TcpListener,
    config: Arc<rustls::ServerConfig>,
) -> tungstenite::WebSocket<StreamOwned<ServerConnection, BridgeStream>> {
    listener.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + WAIT;
    let socket = loop {
        match listener.accept() {
            Ok((socket, _)) => break socket,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "TLS fixture accept timeout");
                thread::sleep(Duration::from_millis(1));
            }
            Err(error) => panic!("TLS fixture accept: {error}"),
        }
    };
    let stream = BridgeStream::new(socket, deadline).unwrap();
    tungstenite::accept(StreamOwned::new(
        ServerConnection::new(config).unwrap(),
        stream,
    ))
    .unwrap()
}

#[test]
fn notified_remote_task_reaps_current_and_retired_without_second_notice() {
    let _serial = SERIAL.lock().unwrap();
    for retired in [false, true] {
        let (listener, identity, config) = fixture();
        let (finish, waiting) = std::sync::mpsc::channel();
        let server = thread::spawn(move || {
            let mut socket = accept(&listener, config);
            socket.read().unwrap();
            waiting.recv_timeout(WAIT).unwrap();
            socket.send(tungstenite::Message::Text(json!({
                "protocolVersion":"loom.hook.v1","requestId":"request","status":"succeeded",
                "data":{"protocolVersion":"loom.hook.v1","requestId":"request","nodeId":"unit",
                    "generation":1,"resultRevision":1,"outputs":{"output":{"kind":"value","value":42}}}
            }).to_string())).unwrap();
        });
        let gate = Gate::new(|| {});
        let mut tasks = Tasks::with_notify(gate.notify.clone());
        let mut registry = Requests::default();
        let request = registry.begin(id("unit"), id("request")).unwrap();
        tasks
            .start(
                Execution {
                    shader: None,
                    identity,
                    unit: id("unit"),
                    request,
                    message: "fixture".into(),
                    inputs: inputs(),
                    timeout: WAIT,
                },
                &registry,
            )
            .unwrap();
        assert!(tasks.test_reap_deadline().is_none());
        tasks.poll();
        assert!(tasks.take_settled(&registry).is_none());
        finish.send(()).unwrap();
        gate.noticed.recv_timeout(WAIT).unwrap();
        if retired {
            tasks.retain_units(|_| false);
        }
        assert!(tasks.test_reap_deadline().is_some());
        tasks.poll();
        assert!(tasks.take_settled(&registry).is_none());
        assert_eq!(tasks.len(), 1);
        gate.release.send(()).unwrap();
        tasks.test_wait_finished(0);
        tasks.poll();
        assert!(tasks.test_reap_deadline().is_none());
        let report = tasks.take_settled(&registry).unwrap();
        assert_eq!(report.current, !retired);
        let completion = report.completion.into_remote().unwrap().unwrap();
        assert!(completion.remote_cancel.is_none());
        if retired {
            assert!(matches!(completion.result, Err(Failure::Cancelled)));
        } else {
            assert!(completion.result.is_ok());
        }
        assert!(tasks.is_empty());
        assert!(tasks.take_completed(&registry).is_none());
        assert!(gate.noticed.try_recv().is_err());
        server.join().unwrap();
    }
}

#[test]
fn remote_cleanup_pending_keeps_slot_but_not_reap_deadline() {
    let _serial = SERIAL.lock().unwrap();
    let (listener, identity, config) = fixture();
    let (started, received) = std::sync::mpsc::channel();
    let server = thread::spawn(move || {
        let mut socket = accept(&listener, config.clone());
        socket.read().unwrap();
        started.send(()).unwrap();
        assert!(socket.read().is_err());
        for _ in 0..3 {
            let mut control = accept(&listener, config.clone());
            let wire: serde_json::Value =
                serde_json::from_str(&control.read().unwrap().into_text().unwrap()).unwrap();
            assert_eq!(wire["method"], "loom.hook.art.cancel");
            control
                .send(tungstenite::Message::Text(
                    json!({
                        "protocolVersion":"loom.hook.v1","requestId":"request","status":"failed"
                    })
                    .to_string(),
                ))
                .unwrap();
        }
    });
    let gate = Gate::new(|| {});
    let mut tasks = Tasks::with_notify(gate.notify.clone());
    let mut registry = Requests::default();
    let request = registry.begin(id("unit"), id("request")).unwrap();
    tasks
        .start(
            Execution {
                shader: None,
                identity,
                unit: id("unit"),
                request,
                message: "fixture".into(),
                inputs: inputs(),
                timeout: WAIT,
            },
            &registry,
        )
        .unwrap();
    received.recv_timeout(WAIT).unwrap();
    tasks.retain_units(|_| false);
    gate.noticed.recv_timeout(WAIT).unwrap();
    assert!(tasks.test_reap_deadline().is_some());
    tasks.poll();
    assert!(tasks.take_completed(&registry).is_none());
    gate.release.send(()).unwrap();
    tasks.test_wait_finished(0);
    tasks.poll();
    assert_eq!(tasks.len(), 1);
    assert!(tasks.take_settled(&registry).is_none());
    assert!(tasks.test_reap_deadline().is_none());
    let report = tasks.take_completed(&registry).unwrap();
    let completion = report.completion.into_remote().unwrap().unwrap();
    assert!(completion.cleanup_pending());
    assert_eq!(completion.cleanup_retries, 2);
    assert!(matches!(completion.result, Err(Failure::Cancelled)));
    assert!(tasks.is_empty());
    assert!(gate.noticed.try_recv().is_err());
    server.join().unwrap();
}
