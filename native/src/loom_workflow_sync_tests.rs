//! TLS/WS end-to-end checks using real prepared results, synchronous workers, and per-item UI replies; no user services.
use super::*;
use crate::desktop::session_candidate::{WorkflowInput, WorkflowJob, WorkflowSyncJob};
use crate::session_document::workflow_snapshot::batch::dispatch::Failure;
use std::sync::mpsc;
use tungstenite::Message;

#[test]
fn sync_worker_waits_for_admission_and_returns_owned_preparation_and_report() {
    for mode in 0..3 {
        let allow = mode == 0;
        let (listener, identity, config) = fixture();
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
            if !allow {
                assert!(socket.read().is_err());
                return;
            }
            let value: serde_json::Value =
                serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
            socket.send(Message::Text(serde_json::json!({"protocolVersion":"loom.hook.v1","requestId":value["params"]["requestId"],"status":"succeeded","data":{"workflowId":"hook-live"}}).to_string())).unwrap();
        });
        let (tx, rx) = mpsc::channel();
        let notify: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let _ = tx.send(());
        });
        let saved = br#"{"documentRevision":1,"stickers":[],"links":[]}"#.to_vec();
        let mut prepare = WorkflowJob::start(
            WorkflowInput {
                saved: saved.clone(),
                revision: 1,
                root: None,
                catalog: Vec::new(),
            },
            notify.clone(),
        )
        .unwrap();
        rx.recv_timeout(Duration::from_secs(10)).unwrap();
        prepare.test_wait_finished();
        let ready = prepare.poll().unwrap().unwrap();
        let budget = crate::byte_budget::Budget::new(8192);
        let mut sync =
            WorkflowSyncJob::start(ready, identity, budget.clone(), 4096, notify).unwrap();
        rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(sync.poll().is_none());
        let admission = sync.take_admission().unwrap();
        assert_eq!(admission.workflow_id(), "hook-live");
        assert!(admission.request_id().starts_with("workflow-sync:"));
        assert!(sync.take_admission().is_none());
        if mode == 2 {
            // UI has taken the request without replying; task-owner destruction must still cancel/reap threads/connections.
            drop(sync);
            admission.resolve(true);
            assert_eq!(budget.used(), 0);
            server.join().unwrap();
            continue;
        }
        admission.resolve(allow);
        rx.recv_timeout(Duration::from_secs(10)).unwrap();
        sync.test_wait_finished();
        let delivery = sync.poll().unwrap().unwrap();
        assert!(sync.poll().is_none());
        assert!(delivery.ready.matches_source(&saved));
        assert_eq!(delivery.report.acknowledged, usize::from(allow));
        assert_eq!(
            delivery.report.failure,
            if allow {
                None
            } else {
                Some(Failure::NotAdmitted)
            }
        );
        assert_eq!(budget.used(), 0);
        server.join().unwrap();
    }
}
