//! 真实后台线程、认证连接和单槽交付，只有自建回环服务。
use super::*;
use crate::{
    desktop::{
        loom_execution_job::{Execution, Job},
        loom_input_job,
    },
    editor::Editor,
    graph_requests::Request,
    workspace_id::UnitId,
};
use serde_json::json;
pub(crate) static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
#[path = "loom_execution_delivery_tests.rs"]
mod delivery;
#[path = "loom_shader_fetch_tests.rs"]
pub(crate) mod shader_fetch;
fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
fn inputs() -> loom_input_job::PreparedBatch {
    loom_input_job::PreparedBatch::empty_for_test(id("unit"), &Editor::default())
}
#[test]
fn worker_coalesces_updates_and_hands_off_matching_final_result() {
    let _serial = SERIAL.lock().unwrap();
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
        for index in 0..20 {
            socket.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":"request","method":"loom.hook.art.progress","params":{"value":index}}).to_string())).unwrap();
        }
        receive.recv_timeout(Duration::from_secs(3)).unwrap();
        socket.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":"request","method":"loom.hook.art.progress","params":{"value":99}}).to_string())).unwrap();
        socket.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":"request","method":"loom.hook.art.preview","params":{"protocolVersion":"loom.hook.v1","requestId":"request","nodeId":"unit","generation":1,"previewRevision":1,"value":{"kind":"value","value":"last preview"}}}).to_string())).unwrap();
        socket.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":"request","status":"succeeded","data":{"protocolVersion":"loom.hook.v1","requestId":"request","nodeId":"unit","generation":1,"resultRevision":1,"outputs":{"output":{"kind":"value","value":42}}}}).to_string())).unwrap();
    });
    let mut job = Job::default();
    job.start(Execution {
        shader: None,
        identity,
        unit: id("unit"),
        request: Request {
            id: id("request"),
            generation: 1,
        },
        message: "fixture".into(),
        inputs: inputs(),
        timeout: Duration::from_secs(3),
    })
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let (progress, preview) = job.take_updates();
        assert!(preview.is_none());
        if progress == Some(19.0) {
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    assert!(job.take_updates().0.is_none());
    send.send(()).unwrap();
    let completion = loop {
        if let Some(result) = job.poll() {
            break result.unwrap();
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(completion.unit, id("unit"));
    assert_eq!(completion.request.id, id("request"));
    assert!(completion.result.is_ok());
    assert!(completion.remote_cancel.is_none());
    assert_eq!(completion.cleanup_retries, 0);
    assert!(!completion.cleanup_pending());
    assert!(!job.is_busy());
    let (progress, preview) = job.take_updates();
    assert_eq!(progress, Some(99.0));
    assert!(matches!(
        preview,
        Some(crate::desktop::loom_execution_session::Update::Preview { revision: 1, .. })
    ));
    assert!(job.take_updates().0.is_none());
    assert!(job.take_updates().1.is_none());
    assert!(job.poll().is_none());
    server.join().unwrap();
}

#[test]
fn worker_shutdown_interrupts_read_and_reaps_thread() {
    let _serial = SERIAL.lock().unwrap();
    for status in ["succeeded", "failed", "retry-success"] {
        let (listener, identity, config) = fixture();
        let (send, receive) = std::sync::mpsc::channel();
        let server = thread::spawn(move || {
            let stream = BridgeStream::new(
                listener.accept().unwrap().0,
                Instant::now() + Duration::from_secs(5),
            )
            .unwrap();
            let mut socket = tungstenite::accept(StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                stream,
            ))
            .unwrap();
            socket.read().unwrap();
            send.send(()).unwrap();
            assert!(socket.read().is_err());
            let attempts = match status {
                "succeeded" => 1,
                "failed" => 3,
                _ => 2,
            };
            for attempt in 0..attempts {
                let stream = BridgeStream::new(
                    listener.accept().unwrap().0,
                    Instant::now() + Duration::from_secs(3),
                )
                .unwrap();
                let mut control = tungstenite::accept(StreamOwned::new(
                    ServerConnection::new(config.clone()).unwrap(),
                    stream,
                ))
                .unwrap();
                let message = control.read().unwrap();
                let value: serde_json::Value =
                    serde_json::from_str(message.to_text().unwrap()).unwrap();
                assert_eq!(value["method"], "loom.hook.art.cancel");
                assert_eq!(value["params"]["nodeId"], "unit");
                assert_eq!(value["params"]["requestId"], "request");
                assert_eq!(value["params"]["generation"], 1);
                let receipt = if status == "retry-success" {
                    if attempt == 0 {
                        "failed"
                    } else {
                        "succeeded"
                    }
                } else {
                    status
                };
                control
                .send(tungstenite::Message::Text(
                    json!({"protocolVersion":"loom.hook.v1","requestId":"request","status":receipt})
                        .to_string(),
                ))
                .unwrap();
            }
        });
        let mut job = Job::default();
        job.start(Execution {
            shader: None,
            identity,
            unit: id("unit"),
            request: Request {
                id: id("request"),
                generation: 1,
            },
            message: "fixture".into(),
            inputs: inputs(),
            timeout: Duration::from_secs(30),
        })
        .unwrap();
        receive.recv_timeout(Duration::from_secs(3)).unwrap();
        let started = Instant::now();
        let completion = job.shutdown().unwrap().unwrap();
        assert!(matches!(
            completion.result,
            Err(crate::desktop::loom_execution_session::Failure::Cancelled)
        ));
        assert!(started.elapsed() < Duration::from_secs(3));
        assert_eq!(
            completion.remote_cancel,
            Some(Ok(if status != "failed" {
                crate::loom_art_control::Outcome::Succeeded
            } else {
                crate::loom_art_control::Outcome::Failed
            }))
        );
        assert_eq!(
            completion.cleanup_retries,
            match status {
                "succeeded" => 0,
                "failed" => 2,
                _ => 1,
            }
        );
        assert_eq!(completion.cleanup_pending(), status == "failed");
        assert!(!job.is_busy());
        assert!(job.shutdown().is_none());
        server.join().unwrap();
    }
}

#[test]
fn failed_release_is_retried_without_reclassifying_failed_execution_as_success() {
    let _serial = SERIAL.lock().unwrap();
    let (listener, identity, config) = fixture();
    let server = thread::spawn(move || {
        let stream = BridgeStream::new(
            listener.accept().unwrap().0,
            Instant::now() + Duration::from_secs(5),
        )
        .unwrap();
        let mut socket = tungstenite::accept(StreamOwned::new(
            ServerConnection::new(config.clone()).unwrap(),
            stream,
        ))
        .unwrap();
        socket.read().unwrap();
        socket.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":"request","method":"loom.hook.art.preview","params":{
            "protocolVersion":"loom.hook.v1","requestId":"request","nodeId":"unit","generation":1,"previewRevision":1,
            "value":{"kind":"shared_memory","handle":"Loom_Buffer_missing_cleanup_fixture","format":"rgba8","width":1,"height":1,"size":4}
        }}).to_string())).unwrap();
        for (method, status) in [
            ("loom.hook.art.resources.release", "failed"),
            ("loom.hook.art.cancel", "succeeded"),
            ("loom.hook.art.resources.release", "succeeded"),
        ] {
            let stream = BridgeStream::new(
                listener.accept().unwrap().0,
                Instant::now() + Duration::from_secs(3),
            )
            .unwrap();
            let mut control = tungstenite::accept(StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                stream,
            ))
            .unwrap();
            let message = control.read().unwrap();
            let value: serde_json::Value =
                serde_json::from_str(message.to_text().unwrap()).unwrap();
            assert_eq!(value["method"], method);
            control.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":value["params"]["requestId"],"status":status}).to_string())).unwrap();
        }
    });
    let mut job = Job::default();
    job.start(Execution {
        shader: None,
        identity,
        unit: id("unit"),
        request: Request {
            id: id("request"),
            generation: 1,
        },
        message: "fixture".into(),
        inputs: inputs(),
        timeout: Duration::from_secs(5),
    })
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    let completion = loop {
        if let Some(result) = job.poll() {
            break result.unwrap();
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(completion.cleanup_retries, 1);
    assert!(!completion.cleanup_pending());
    let Err(crate::desktop::loom_execution_session::Failure::PendingRelease(delivery)) =
        completion.result
    else {
        panic!("preserve original failure");
    };
    assert_eq!(delivery.pending_release_count(), 0);
    assert!(delivery.result().is_err());
    server.join().unwrap();
}

#[test]
fn task_collection_retires_closed_or_replaced_requests_without_blocking_ui_owner() {
    let _serial = SERIAL.lock().unwrap();
    for reason in ["closed", "replaced", "edited"] {
        let (listener, identity, config) = fixture();
        let (send, receive) = std::sync::mpsc::channel();
        let server = thread::spawn(move || {
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
            send.send(()).unwrap();
            assert!(execution.read().is_err());
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
            control.read().unwrap();
            control.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":"request","status":"succeeded"}).to_string())).unwrap();
        });
        let mut registry = crate::graph_requests::Requests::default();
        let request = registry.begin(id("unit"), id("request")).unwrap();
        let mut tasks = crate::desktop::loom_execution_tasks::Tasks::default();
        tasks
            .start(
                Execution {
                    shader: None,
                    identity,
                    unit: id("unit"),
                    request: request.clone(),
                    message: "fixture".into(),
                    inputs: inputs(),
                    timeout: Duration::from_secs(30),
                },
                &registry,
            )
            .unwrap();
        receive.recv_timeout(Duration::from_secs(3)).unwrap();
        if reason == "closed" {
            tasks.retain_units(|_| false);
        } else if reason == "replaced" {
            registry.begin(id("unit"), id("replacement")).unwrap();
            tasks.retire_stale(&registry);
        } else {
            tasks.retire_changed(|_, _| false);
        }
        // 取消后槽仍归旧 owner，直到后台完成被显式领取。
        assert_eq!(tasks.len(), 1);
        let deadline = Instant::now() + Duration::from_secs(5);
        let report = loop {
            tasks.poll();
            if let Some(report) = tasks.take_completed(&registry) {
                break report;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(1));
        };
        assert!(!report.current);
        assert!(!report.is_current(&registry, |_| panic!(
            "retired report must not inspect documents"
        )));
        assert_eq!(report.request, request);
        let completion = report.completion.into_remote().unwrap().unwrap();
        assert!(!completion.cleanup_pending());
        assert!(tasks.is_empty());
        if reason == "replaced" {
            assert_eq!(registry.active(&id("unit")).unwrap().id, id("replacement"));
        } else if reason == "edited" {
            assert_eq!(registry.active(&id("unit")), Some(&request));
        }
        server.join().unwrap();
    }
}
