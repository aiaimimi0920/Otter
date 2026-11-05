//! 真实固定证书 TLS/WS 回环；不读取用户发现配置，不连接运行中的 Loom。
use super::*;
use crate::{
    byte_budget::Budget,
    session_document::workflow_snapshot::{
        self,
        batch::{exchange, Batch},
    },
};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use tungstenite::Message;

#[test]
fn workflow_ack_requires_protocol_request_and_workflow_identity() {
    let cancel = AtomicBool::new(false);
    let saved = br#"{"documentRevision":1,"stickers":[],"links":[]}"#;
    let snapshot = workflow_snapshot::prepare(saved, 1, &cancel, |_, _| unreachable!()).unwrap();
    let mut batch = Batch::prepare(&snapshot, saved, &cancel).unwrap();
    let budget = Budget::new(4096);
    let request = batch.current(&budget, 4096, &cancel).unwrap().unwrap();
    let valid = json!({"protocolVersion":"loom.hook.v1","requestId":request.id(),"status":"succeeded","data":{"workflowId":"hook-live"}});
    assert_eq!(
        exchange::response(&serde_json::to_vec(&valid).unwrap(), request),
        Ok(true)
    );
    for field in ["protocolVersion", "requestId"] {
        let mut other = valid.clone();
        other[field] = json!("other");
        assert_eq!(
            exchange::response(&serde_json::to_vec(&other).unwrap(), request),
            Ok(false)
        );
    }
    let mut event = valid.clone();
    event["method"] = json!("workflow.updated");
    assert_eq!(
        exchange::response(&serde_json::to_vec(&event).unwrap(), request),
        Ok(false)
    );
    for status in ["pending", "", "running"] {
        let mut other = valid.clone();
        other["status"] = json!(status);
        assert_eq!(
            exchange::response(&serde_json::to_vec(&other).unwrap(), request),
            Err(exchange::Error::InvalidResponse)
        );
    }
    assert_eq!(exchange::response(b"not-json", request), Ok(false));
    assert_eq!(
        exchange::response(&vec![b' '; 1024 * 1024 + 1], request),
        Err(exchange::Error::InvalidResponse)
    );
}

#[test]
fn workflow_exchange_validates_remote_identity_and_does_not_advance_batch() {
    for mode in ["success", "wrong-workflow", "rejected", "cancel"] {
        let (listener, identity, config) = fixture();
        let cancel = Arc::new(AtomicBool::new(false));
        let remote_cancel = cancel.clone();
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
            let request = socket.read().unwrap();
            let request: serde_json::Value =
                serde_json::from_str(request.to_text().unwrap()).unwrap();
            assert_eq!(request["method"], "loom.hook.workflow.sync");
            assert_eq!(request["params"]["workflowId"], "hook-live");
            socket.send(Message::Ping(vec![7])).unwrap();
            assert_eq!(socket.read().unwrap(), Message::Pong(vec![7]));
            if mode == "cancel" {
                remote_cancel.store(true, Ordering::Release);
                thread::sleep(Duration::from_millis(400));
                return;
            }
            socket.send(Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":"unrelated","status":"succeeded","data":{"workflowId":"hook-live"}}).to_string())).unwrap();
            socket.send(Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":request["params"]["requestId"],"status":if mode == "rejected" {"failed"} else {"succeeded"},"data":{"workflowId":if mode == "wrong-workflow" {"other"} else {"hook-live"}}}).to_string())).unwrap();
        });
        let saved = br#"{"documentRevision":1,"stickers":[],"links":[]}"#;
        let snapshot =
            workflow_snapshot::prepare(saved, 1, &cancel, |_, _| unreachable!()).unwrap();
        let mut batch = Batch::prepare(&snapshot, saved, &cancel).unwrap();
        let budget = Budget::new(8192);
        let request = batch.current(&budget, 4096, &cancel).unwrap().unwrap();
        let id = request.id().to_owned();
        let socket = connect(&identity, Duration::from_secs(3)).unwrap();
        let result = exchange::run(socket, request, &budget, &cancel);
        assert_eq!(
            result,
            match mode {
                "success" => Ok(()),
                "wrong-workflow" => Err(exchange::Error::InvalidResponse),
                "rejected" => Err(exchange::Error::Rejected),
                _ => Err(exchange::Error::Cancelled),
            }
        );
        assert_eq!(batch.remaining(), 1);
        assert_eq!(budget.used(), 4096);
        if mode == "success" {
            assert!(batch.acknowledge(&id));
        }
        drop(batch);
        assert_eq!(budget.used(), 0);
        worker.join().unwrap();
    }
}
