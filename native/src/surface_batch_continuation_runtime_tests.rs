//! 真实 TLS 接续只发送后缀；失败和撤权仍返还原批准与前缀 ACK。
use super::*;
use crate::{
    desktop::surface_authorized_window::BatchContinuationInput,
    surface_event_batch_runtime::continuation::Job as ContinuationJob,
};
fn continuation_input(origin: &str) -> BatchContinuationInput {
    let mut input = batch_input(origin);
    let receipt = &mut input.prepared.receipt;
    let submitted = receipt.submission.as_mut().unwrap();
    let mut approved = decision(
        &receipt.events[0],
        &submitted.acknowledgements[0],
        Choice::Approve,
        u64::MAX,
    );
    approved.begin_submission();
    approved.acknowledge();
    submitted.acknowledgements[0] = ack(&receipt.events[0], "succeeded");
    let item = &mut receipt
        .observation
        .as_mut()
        .unwrap()
        .as_mut()
        .unwrap()
        .items[0];
    item.result = Ok(3);
    item.decision = Some(crate::surface_decision_runtime::Evidence {
        decision: approved,
        revision: 3,
        submission: Ok(()),
    });
    BatchContinuationInput {
        owner: input.owner,
        revision: 3,
        receipt: input.prepared.receipt,
        next: 1,
    }
}
fn wait_continuation(job: &ContinuationJob) {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
}
#[test]
fn continuation_posts_only_tail_and_keeps_prefix_after_http_failure() {
    let (listener, client, config) = fixture();
    let input = continuation_input(&client.origin);
    let tail_id = input.receipt.events[1].id().to_owned();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/surfaces/instances/"));
        assert_eq!(body["eventId"], tail_id);
        reply(&mut stream, 500, b"{}");
    });
    let mut job = ContinuationJob::start_prepared(client, lease, input)
        .unwrap_or_else(|e| panic!("{}", e.error));
    wait_continuation(&job);
    drop(grant);
    let (outcome, result) = job.try_join(0).unwrap().into_batch();
    assert!(result.is_ok());
    assert!(outcome.delivery.is_err());
    assert!(outcome.input.events.iter().all(|event| event.attempted()));
    let submitted = outcome.submission.as_ref().unwrap();
    assert_eq!(submitted.acknowledgements.len(), 1);
    assert!(matches!(submitted.stop, BatchStop::Submit(_)));
    let records = crate::surface_event_record::Record::from_batch_outcome(&outcome).unwrap();
    assert!(records[0].decision().unwrap().confirmed());
    assert!(crate::surface_event_record::encode(&records).is_ok());
    assert!(job.wait_handle().is_none());
    peer.join().unwrap();
}
#[test]
fn revoked_continuation_returns_unattempted_tail_and_confirmed_prefix() {
    let (_listener, client, _config) = fixture();
    let input = continuation_input(&client.origin);
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    drop(grant);
    let mut job = ContinuationJob::start_prepared(client, lease, input)
        .unwrap_or_else(|e| panic!("{}", e.error));
    wait_continuation(&job);
    let (outcome, result) = job.try_join(0).unwrap().into_batch();
    assert_eq!(
        result,
        Err(crate::surface_event_batch_runtime::Error::Stopped)
    );
    assert!(!outcome.input.events[1].attempted());
    assert!(outcome.submission.unwrap().acknowledgements[0].succeeded());
    assert!(outcome.observation.unwrap().unwrap().items[0]
        .decision
        .as_ref()
        .unwrap()
        .decision
        .confirmed());
}
