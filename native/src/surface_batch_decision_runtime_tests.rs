//! 批次决定只操作选中项；其他事件与未提交后缀在取消/撤权后仍完整保留。
use super::*;
#[path = "surface_batch_continuation_runtime_tests.rs"]
mod continuation;
use crate::{
    desktop::surface_authorized_window::{BatchReceipt, PreparedBatchDecision},
    live_relay_http::{BatchStop, BatchSubmission},
    surface_batch_decision_runtime::{Input as BatchInput, Job as BatchJob},
};
fn batch_input(origin: &str) -> BatchInput {
    let input = input(origin);
    let tail = event(&input.owner);
    BatchInput {
        owner: input.owner,
        revision: input.revision,
        prepared: PreparedBatchDecision {
            index: 0,
            decision: input.decision,
            receipt: BatchReceipt {
                events: vec![input.event, tail],
                revision: input.revision,
                submission: Ok(BatchSubmission {
                    acknowledgements: vec![input.ack],
                    stop: BatchStop::Confirmation,
                }),
                observation: Some(Ok(crate::surface_event_batch_runtime::Observation {
                    snapshot: None,
                    items: vec![crate::surface_event_batch_runtime::ObservedItem {
                        result: Err(
                            crate::surface_event_batch_runtime::ObservationError::Convergence(
                                crate::live_relay_http::ConvergenceError::ConfirmationRequired,
                            ),
                        ),
                        commit: None,
                        confirmation: None,
                        decision: None,
                    }],
                })),
                delivery: Ok(()),
                presentation: Ok(None),
            },
        },
    }
}
fn wait_batch(job: &BatchJob) {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
}
#[test]
fn batch_decision_preserves_tail_and_terminal_ack_after_late_revocation() {
    let (listener, client, config) = fixture();
    let input = batch_input(&client.origin);
    let id = input.prepared.receipt.events[0].id().to_owned();
    let tail = input.prepared.receipt.events[1].id().to_owned();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let peer = std::thread::spawn(move || {
        for index in 0..2 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            if index == 0 {
                assert!(head.starts_with("POST /v1/surfaces/confirmations/decision "));
                assert_eq!(body["approved"], false);
                reply(&mut stream, 204, b"");
            } else {
                assert!(head.starts_with("GET "));
                reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({"protocolVersion":"loom.surface-stream.v1",
                    "next":1,"reset":true,"messages":[{"method":"loom.surface.action.ack","params":{
                    "protocolVersion":"loom.surface.v1","instanceId":"surface:test/part","eventId":id,
                    "requestId":"request:1","accepted":true,"status":"cancelled"}}]})).unwrap());
            }
        }
    });
    let mut job = BatchJob::start_prepared(client, lease, input)
        .unwrap_or_else(|error| panic!("{}", error.error));
    wait_batch(&job);
    drop(grant);
    let outcome = job.try_join(0).unwrap();
    assert!(outcome.submission.is_ok());
    assert!(outcome.input.prepared.decision.confirmed());
    assert!(outcome.convergence.as_ref().unwrap().is_err());
    assert!(outcome.delivery.is_err());
    let merged = outcome.into_batch().unwrap_or_else(|_| panic!("merge"));
    let records = crate::surface_event_record::Record::from_batch_outcome(&merged).unwrap();
    assert!(records[0].decision().unwrap().confirmed());
    assert!(!records[0].decision().unwrap().approved());
    assert!(crate::surface_event_record::encode(&records).is_ok());
    assert_eq!(merged.input.events[1].id(), tail);
    assert!(!merged.input.events[1].attempted());
    assert_eq!(
        merged.submission.unwrap().acknowledgements[0].status(),
        crate::surface_action_ack::Status::Cancelled
    );
    assert!(job.wait_handle().is_none() && job.try_join(0).is_none());
    peer.join().unwrap();
}
#[test]
fn invalid_batch_decision_merge_returns_all_input_evidence() {
    let mut input = batch_input("https://localhost:1234");
    input.prepared.index = 2;
    let outcome = crate::surface_batch_decision_runtime::Outcome {
        input,
        submission: Err(JobError::Stopped),
        convergence: None,
        delivery: Ok(()),
    };
    let returned = outcome.into_batch().err().unwrap();
    assert_eq!(returned.input.prepared.receipt.events.len(), 2);
    assert!(!returned.input.prepared.decision.attempted());
    assert!(returned.input.owner.confirmed_active());
}
#[test]
fn stopped_batch_decision_returns_unattempted_choice_and_all_events() {
    let (_listener, client, _config) = fixture();
    let input = batch_input(&client.origin);
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    drop(grant);
    let mut job = BatchJob::start_prepared(client, lease, input)
        .unwrap_or_else(|error| panic!("{}", error.error));
    wait_batch(&job);
    let outcome = job.try_join(0).unwrap();
    assert_eq!(outcome.submission, Err(JobError::Stopped));
    assert!(outcome.convergence.is_none());
    assert_eq!(outcome.input.prepared.receipt.events.len(), 2);
    assert!(!outcome.input.prepared.decision.attempted());
}
