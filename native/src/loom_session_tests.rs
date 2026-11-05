//! 真实认证连接、响应解析、多输出解码组合；无 UI 发布和真实服务访问。
use super::*;
use crate::{
    desktop::loom_execution_session::{Failure, Session, Update},
    graph_requests::Request,
    workspace_id::UnitId,
};
use serde_json::json;
use std::sync::atomic::AtomicBool;
#[test]
fn session_delivers_progress_latest_preview_and_final_values_over_tls() {
    let (listener, identity, config) = fixture();
    let worker = thread::spawn(move || {
        let stream = BridgeStream::new(
            listener.accept().unwrap().0,
            Instant::now() + Duration::from_secs(3),
        )
        .unwrap();
        let mut socket = tungstenite::accept(StreamOwned::new(
            ServerConnection::new(config).unwrap(),
            stream,
        ))
        .unwrap();
        socket.read().unwrap();
        let message = |method: &str, params: serde_json::Value| {
            tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":"request","method":method,"params":params}).to_string())
        };
        socket
            .send(message("loom.hook.art.progress", json!({"value":0.5})))
            .unwrap();
        for revision in [2, 1, 2, 3] {
            socket.send(message("loom.hook.art.preview",json!({"protocolVersion":"loom.hook.v1","requestId":"request","nodeId":"unit","generation":1,"previewRevision":revision,"value":{"kind":"value","value":revision}}))).unwrap();
        }
        socket.send(message("loom.hook.art.result",json!({"protocolVersion":"loom.hook.v1","requestId":"request","nodeId":"unit","generation":1,"resultRevision":1,"outputs":{"output":{"kind":"value","value":"final"}},"candidates":{"items":[]}}))).unwrap();
    });
    let unit = UnitId::parse("unit").unwrap();
    let request = Request {
        id: UnitId::parse("request").unwrap(),
        generation: 1,
    };
    let cancel = AtomicBool::new(false);
    let session = Session {
        identity: &identity,
        unit: &unit,
        request: &request,
        cancel: &cancel,
        timeout: Duration::from_secs(3),
    };
    let mut progress = Vec::new();
    let mut previews = Vec::new();
    let mut interrupted = None;
    let result = session
        .run(
            "fixture".into(),
            |handle| interrupted = Some(handle),
            |update| {
                match update {
                    Update::Progress(value) => progress.push(value),
                    Update::Preview { revision, delivery } => {
                        assert!(delivery.result().is_ok());
                        assert_eq!(delivery.pending_release_count(), 0);
                        previews.push(revision);
                    }
                }
                true
            },
        )
        .unwrap_or_else(|_| panic!("session failed"));
    assert_eq!(progress, [0.5]);
    assert_eq!(previews, [2, 3]);
    assert_eq!(result.revision, 1);
    assert_eq!(result.candidates, Some(json!({"items":[]})));
    assert!(result.delivery.result().is_ok());
    assert_eq!(result.delivery.request(), &request);
    assert!(interrupted.is_some());
    worker.join().unwrap();
}
#[test]
fn cancelled_session_never_connects_or_invokes_callbacks() {
    let (_, identity, _) = fixture();
    let unit = UnitId::parse("unit").unwrap();
    let request = Request {
        id: UnitId::parse("request").unwrap(),
        generation: 1,
    };
    let cancel = AtomicBool::new(true);
    let session = Session {
        identity: &identity,
        unit: &unit,
        request: &request,
        cancel: &cancel,
        timeout: Duration::from_secs(1),
    };
    assert!(matches!(
        session.run(
            "fixture".into(),
            |_| panic!("connect"),
            |_| panic!("update")
        ),
        Err(Failure::Cancelled)
    ));
}

#[test]
fn cancellation_flag_alone_ends_idle_read_without_socket_interrupt() {
    let (listener, identity, config) = fixture();
    let (send, receive) = std::sync::mpsc::channel();
    let server = thread::spawn(move || {
        let stream = BridgeStream::new(
            listener.accept().unwrap().0,
            Instant::now() + Duration::from_secs(3),
        )
        .unwrap();
        let mut socket = tungstenite::accept(StreamOwned::new(
            ServerConnection::new(config).unwrap(),
            stream,
        ))
        .unwrap();
        socket.read().unwrap();
        send.send(()).unwrap();
        assert!(socket.read().is_err());
    });
    let cancel = Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    let canceller = thread::spawn(move || {
        receive.recv_timeout(Duration::from_secs(3)).unwrap();
        signal.store(true, std::sync::atomic::Ordering::Release);
    });
    let unit = UnitId::parse("unit").unwrap();
    let request = Request {
        id: UnitId::parse("request").unwrap(),
        generation: 1,
    };
    let session = Session {
        identity: &identity,
        unit: &unit,
        request: &request,
        cancel: &cancel,
        timeout: Duration::from_secs(30),
    };
    let started = Instant::now();
    assert!(matches!(
        session.run("fixture".into(), |_| {}, |_| true),
        Err(Failure::Cancelled)
    ));
    assert!(started.elapsed() < Duration::from_secs(3));
    canceller.join().unwrap();
    server.join().unwrap();
}

#[test]
fn failed_preview_release_returns_pending_owner_without_publishing_update() {
    let (listener, identity, config) = fixture();
    let worker = thread::spawn(move || {
        let stream = BridgeStream::new(
            listener.accept().unwrap().0,
            Instant::now() + Duration::from_secs(5),
        )
        .unwrap();
        let mut execution = tungstenite::accept(StreamOwned::new(
            ServerConnection::new(config.clone()).unwrap(),
            stream,
        ))
        .unwrap();
        execution.read().unwrap();
        execution.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":"request","method":"loom.hook.art.preview","params":{
            "protocolVersion":"loom.hook.v1","requestId":"request","nodeId":"unit","generation":1,"previewRevision":1,
            "value":{"kind":"shared_memory","handle":"Loom_Buffer_missing_session_fixture","format":"rgba8","width":1,"height":1,"size":4}
        }}).to_string())).unwrap();
        let stream = BridgeStream::new(
            listener.accept().unwrap().0,
            Instant::now() + Duration::from_secs(3),
        )
        .unwrap();
        let mut control = tungstenite::accept(StreamOwned::new(
            ServerConnection::new(config).unwrap(),
            stream,
        ))
        .unwrap();
        let message = control.read().unwrap();
        let value: serde_json::Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
        assert_eq!(value["method"], "loom.hook.art.resources.release");
        control.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":value["params"]["requestId"],"status":"failed"}).to_string())).unwrap();
    });
    let unit = UnitId::parse("unit").unwrap();
    let request = Request {
        id: UnitId::parse("request").unwrap(),
        generation: 1,
    };
    let cancel = AtomicBool::new(false);
    let session = Session {
        identity: &identity,
        unit: &unit,
        request: &request,
        cancel: &cancel,
        timeout: Duration::from_secs(5),
    };
    let error = session.run(
        "fixture".into(),
        |_| {},
        |_| panic!("failed release must not publish"),
    );
    let Err(Failure::PendingRelease(delivery)) = error else {
        panic!("expected pending owner");
    };
    assert_eq!(delivery.pending_release_count(), 1);
    assert!(delivery.result().is_err());
    assert_eq!(delivery.request(), &request);
    worker.join().unwrap();
}
