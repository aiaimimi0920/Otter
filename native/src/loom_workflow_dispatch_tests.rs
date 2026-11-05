//! 顺序批次的真实 TLS/WS 测试，验证部分成功、逐条门禁和失败保留。
use super::*;
use crate::{
    byte_budget::Budget,
    session_document::workflow_snapshot::{
        self,
        batch::{
            dispatch::{self, Failure},
            exchange, Batch,
        },
    },
};
use serde_json::json;
use std::sync::atomic::AtomicBool;
use tungstenite::Message;

#[test]
fn cancelled_or_unencodable_batch_never_connects_or_requests_admission() {
    for cancelled in [false, true] {
        let (listener, identity, _) = fixture();
        listener.set_nonblocking(true).unwrap();
        let saved = br#"{"documentRevision":1,"stickers":[],"links":[]}"#;
        let snapshot =
            workflow_snapshot::prepare(saved, 1, &AtomicBool::new(false), |_, _| unreachable!())
                .unwrap();
        let mut batch = Batch::prepare(&snapshot, saved, &AtomicBool::new(false)).unwrap();
        let budget = Budget::new(4096);
        let report = dispatch::run(
            &mut batch,
            &identity,
            &budget,
            1,
            &AtomicBool::new(cancelled),
            |_| panic!("No request may be admitted"),
        );
        assert_eq!(report.acknowledged, 0);
        assert!(!report.may_have_reached_remote);
        assert_eq!(
            report.failure,
            Some(if cancelled {
                Failure::Cancelled
            } else {
                Failure::Prepare(workflow_snapshot::Error::Budget)
            })
        );
        assert_eq!(batch.remaining(), 1);
        assert_eq!(budget.used(), 0);
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[test]
fn batch_dispatch_checks_each_request_and_preserves_partial_progress() {
    for mode in ["success", "deny-second", "reject-second"] {
        let (listener, identity, config) = fixture();
        let worker = thread::spawn(move || {
            let mut ids = Vec::new();
            for index in 0..2 {
                let stream = BridgeStream::new(
                    listener.accept().unwrap().0,
                    Instant::now() + Duration::from_secs(3),
                )
                .unwrap();
                let mut socket = tungstenite::accept(StreamOwned::new(
                    ServerConnection::new(config.clone()).unwrap(),
                    stream,
                ))
                .unwrap();
                if mode == "deny-second" && index == 1 {
                    assert!(socket.read().is_err());
                    break;
                }
                let text = socket.read().unwrap();
                let value: serde_json::Value =
                    serde_json::from_str(text.to_text().unwrap()).unwrap();
                assert_eq!(
                    value["params"]["workflowId"],
                    if index == 0 { "hook-live" } else { "origin" }
                );
                let id = value["params"]["requestId"].as_str().unwrap().to_owned();
                assert!(!ids.contains(&id));
                ids.push(id.clone());
                socket.send(Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":id,"status":if index == 1 && mode == "reject-second" {"failed"} else {"succeeded"},"data":{"workflowId":value["params"]["workflowId"]}}).to_string())).unwrap();
            }
        });
        let saved = serde_json::to_vec(&json!({"documentRevision":1,"stickers":[{"id":"local","x":0,"y":0,"w":10,"h":10,"originWorkflowId":"origin"}],"links":[]})).unwrap();
        let cancel = AtomicBool::new(false);
        let snapshot = workflow_snapshot::prepare(&saved, 1, &cancel, |_, _| {
            Ok(workflow_snapshot::Content {
                parameters: Default::default(),
                image: Default::default(),
            })
        })
        .unwrap();
        let mut batch = Batch::prepare(&snapshot, &saved, &cancel).unwrap();
        let budget = Budget::new(8192);
        let mut admissions = Vec::new();
        let report = dispatch::run(&mut batch, &identity, &budget, 4096, &cancel, |request| {
            admissions.push(request.id().to_owned());
            !(mode == "deny-second" && admissions.len() == 2)
        });
        assert_eq!(admissions.len(), 2);
        assert_eq!(report.acknowledged, if mode == "success" { 2 } else { 1 });
        assert_eq!(
            report.failure,
            match mode {
                "success" => None,
                "deny-second" => Some(Failure::NotAdmitted),
                _ => Some(Failure::Exchange(exchange::Error::Rejected)),
            }
        );
        assert_eq!(report.may_have_reached_remote, mode == "reject-second");
        assert_eq!(batch.remaining(), usize::from(mode != "success"));
        if mode != "success" {
            assert_eq!(
                batch.current(&budget, 4096, &cancel).unwrap().unwrap().id(),
                admissions[1]
            );
            assert_eq!(budget.used(), 4096);
        }
        drop(batch);
        assert_eq!(budget.used(), 0);
        worker.join().unwrap();
    }
}
