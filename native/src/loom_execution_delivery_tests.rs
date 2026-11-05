//! 应用领取更新/完成的身份检查，使用自建 TLS 服务，不访问真实 Loom。
use super::*;
use crate::{desktop::loom_execution_tasks::Tasks, graph_requests::Requests};

#[test]
fn updates_are_single_use_and_claimed_result_rechecks_request_and_document() {
    let _serial = SERIAL.lock().unwrap();
    for mode in ["commit", "replace", "reject"] {
        let (listener, identity, config) = fixture();
        let (send, receive) = std::sync::mpsc::channel();
        let server = thread::spawn(move || {
            let stream = BridgeStream::new(
                listener.accept().unwrap().0,
                Instant::now() + Duration::from_secs(5),
            )
            .unwrap();
            let mut socket = tungstenite::accept(StreamOwned::new(
                ServerConnection::new(config).unwrap(),
                stream,
            ))
            .unwrap();
            socket.read().unwrap();
            socket
                .send(tungstenite::Message::Text(
                    json!({"protocolVersion":"loom.hook.v1",
            "requestId":"request","method":"loom.hook.art.progress","params":{"value":37}})
                    .to_string(),
                ))
                .unwrap();
            socket
                .send(tungstenite::Message::Text(
                    json!({"protocolVersion":"loom.hook.v1",
            "requestId":"request","method":"loom.hook.art.preview","params":{
            "protocolVersion":"loom.hook.v1","requestId":"request","nodeId":"unit",
            "generation":1,"previewRevision":1,"value":{"kind":"value","value":"preview"}}})
                    .to_string(),
                ))
                .unwrap();
            receive.recv_timeout(Duration::from_secs(5)).unwrap();
            socket
                .send(tungstenite::Message::Text(
                    json!({"protocolVersion":"loom.hook.v1",
            "requestId":"request","status":"succeeded","data":{"protocolVersion":"loom.hook.v1",
            "requestId":"request","nodeId":"unit","generation":1,"resultRevision":1,
                "outputs":{"output":{"kind":"value","value":42}},"candidates":{"items":[]}}})
                    .to_string(),
                ))
                .unwrap();
        });
        let editor = Editor::default();
        let mut registry = Requests::default();
        let request = registry.begin(id("unit"), id("request")).unwrap();
        let mut tasks = Tasks::default();
        tasks
            .start(
                Execution {
                    shader: None,
                    identity,
                    unit: id("unit"),
                    request: request.clone(),
                    message: "fixture".into(),
                    inputs: loom_input_job::PreparedBatch::empty_for_test(id("unit"), &editor),
                    timeout: Duration::from_secs(5),
                },
                &registry,
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut progress = None;
        let mut preview = None;
        while progress.is_none() || preview.is_none() {
            if let Some(update) = tasks.take_updates(&id("unit"), &registry, |_| Some(&editor)) {
                assert_eq!(update.request, request);
                if update.progress.is_some() {
                    progress = update.progress;
                }
                if update.preview.is_some() {
                    preview = update.preview;
                }
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(progress, Some(37.0));
        let crate::desktop::loom_execution_session::Update::Preview { revision, delivery } =
            preview.unwrap()
        else {
            panic!("preview expected");
        };
        assert_eq!(revision, 1);
        assert_eq!(delivery.pending_release_count(), 0);
        assert!(delivery.result().is_ok());
        assert!(tasks
            .take_updates(&id("unit"), &registry, |_| Some(&editor))
            .is_none());
        send.send(()).unwrap();
        let mut report = loop {
            tasks.poll();
            if let Some(report) = tasks.take_settled(&registry) {
                break report;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(1));
        };
        assert!(report.is_current(&registry, |_| Some(&editor)));
        assert!(!report.is_current(&registry, |_| None));
        let replacement = Editor::default();
        assert!(!report.is_current(&registry, |_| Some(&replacement)));
        let mut state = crate::desktop::loom_result_state::Results::new(id("unit"));
        state.begin(request.clone());
        let mut wrong = crate::desktop::loom_result_state::Results::new(id("wrong"));
        wrong.begin(request.clone());
        assert_eq!(
            report.commit(&mut registry, &mut wrong, |_| Some(&editor)),
            Err(crate::desktop::loom_execution_tasks::CommitError::Stale)
        );
        assert_eq!(
            report.commit(&mut registry, &mut state, |_| Some(&replacement)),
            Err(crate::desktop::loom_execution_tasks::CommitError::Stale)
        );
        assert!(state.ports().is_empty());
        if mode == "replace" {
            registry.begin(id("unit"), id("replacement")).unwrap();
            assert_eq!(
                report.commit(&mut registry, &mut state, |_| Some(&editor)),
                Err(crate::desktop::loom_execution_tasks::CommitError::Stale)
            );
            assert_eq!(registry.active(&id("unit")).unwrap().id, id("replacement"));
            assert!(!report.reject(&mut registry, &mut state, "old failure"));
            assert_eq!(registry.active(&id("unit")).unwrap().id, id("replacement"));
        } else if mode == "reject" {
            assert!(!report.reject(&mut registry, &mut wrong, "wrong node"));
            assert!(report.reject(&mut registry, &mut state, "document changed"));
            assert_eq!(
                state.status(),
                crate::desktop::loom_result_state::Status::Failed
            );
            assert_eq!(state.error(), Some("document changed"));
            assert!(state.active().is_none());
            assert!(registry.active(&id("unit")).is_none());
            assert!(!report.reject(&mut registry, &mut state, "duplicate"));
            let crate::desktop::loom_execution_tasks::Completion::Remote(completion) =
                &report.completion
            else {
                panic!("remote completion required")
            };
            assert!(completion
                .as_ref()
                .unwrap()
                .result
                .as_ref()
                .unwrap_or_else(|_| panic!("successful execution result retained"))
                .delivery
                .result()
                .is_ok());
        } else {
            report
                .commit(&mut registry, &mut state, |_| Some(&editor))
                .unwrap();
            assert!(registry.active(&id("unit")).is_none());
            assert!(state.active().is_none());
            assert_eq!(state.ports().len(), 1);
            assert_eq!(state.candidates(), Some(&json!({"items":[]})));
            let crate::desktop::loom_output_batch::Output::Value(value) = &state.ports()[0].value
            else {
                panic!("value");
            };
            assert_eq!(*value, json!(42));
            assert_eq!(
                report.commit(&mut registry, &mut state, |_| Some(&editor)),
                Err(crate::desktop::loom_execution_tasks::CommitError::Stale)
            );
        }
        assert!(!report.is_current(&registry, |_| panic!(
            "stale request must fail before document lookup"
        )));
        assert!(report
            .completion
            .into_remote()
            .unwrap()
            .unwrap()
            .result
            .is_ok());
        assert!(tasks.is_empty());
        server.join().unwrap();
    }
}
