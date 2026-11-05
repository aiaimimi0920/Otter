use super::*;
#[path = "surface_batch_decision_runtime_tests.rs"]
mod batch;
use crate::{
    device_authorization_lease::Grant,
    surface_decision_runtime::{Error as JobError, Input, Job},
};
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};
fn input(origin: &str) -> Input {
    let owner = active(origin);
    let mut event = event(&owner);
    event.begin_submission();
    let ack = ack(&event, "awaiting_confirmation");
    let decision = decision(&event, &ack, Choice::Reject, u64::MAX);
    Input {
        owner,
        event,
        ack,
        decision,
        revision: 3,
    }
}
fn wait(job: &Job) {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
}
#[test]
fn decision_worker_preserves_submission_terminal_ack_and_late_revocation() {
    let (listener, client, config) = fixture();
    let input = input(&client.origin);
    let id = input.event.id().to_owned();
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
    let mut job =
        Job::start_prepared(client, lease, input).unwrap_or_else(|e| panic!("{}", e.error));
    wait(&job);
    drop(grant);
    let outcome = job.try_join(0).unwrap();
    assert!(outcome.submission.is_ok());
    assert!(outcome.input.decision.confirmed());
    let record = crate::surface_event_record::Record::from_decision(&outcome);
    assert!(record.decision().unwrap().attempted());
    assert!(record.decision().unwrap().confirmed());
    assert!(!record.decision().unwrap().approved());
    assert!(crate::surface_event_record::encode(&[record]).is_ok());
    assert_eq!(
        outcome.input.ack.status(),
        crate::surface_action_ack::Status::Cancelled
    );
    assert!(matches!(
        outcome.convergence,
        Some(Err(JobError::Convergence(ConvergenceError::State(
            crate::surface_convergence::Error::Action
        ))))
    ));
    assert!(outcome.delivery.is_err());
    assert!(outcome.input.owner.confirmed_active());
    assert!(job.try_join(0).is_none() && job.wait_handle().is_none());
    peer.join().unwrap();
}
#[test]
fn cancelled_decision_attempt_is_retained_without_replay() {
    let (listener, client, config) = fixture();
    let input = input(&client.origin);
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, _) = request(&mut stream);
        assert!(head.starts_with("POST /v1/surfaces/confirmations/decision "));
        tx.send(()).unwrap();
        let _ = finish_rx.recv_timeout(Duration::from_secs(8));
    });
    let mut job =
        Job::start_prepared(client, lease, input).unwrap_or_else(|e| panic!("{}", e.error));
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    job.request_stop();
    wait(&job);
    let outcome = job.try_join(0).unwrap();
    let _ = finish_tx.send(());
    peer.join().unwrap();
    assert!(outcome.submission.is_err());
    assert!(outcome.convergence.is_none());
    assert!(outcome.input.decision.attempted() && !outcome.input.decision.confirmed());
    assert_eq!(
        outcome.input.ack.status(),
        crate::surface_action_ack::Status::AwaitingConfirmation
    );
    assert!(outcome.delivery.is_err());
}
#[test]
fn completed_decision_tasks_hold_shared_capacity_and_owner_transfer_is_exclusive() {
    let (_listener, client, _config) = fixture();
    let mut input = input(&client.origin);
    let mut activation = crate::surface_activation::Activation::new(input.owner);
    input.owner = activation
        .take_decision(&input.event, &input.ack, &input.decision)
        .unwrap();
    assert!(activation
        .take_decision(&input.event, &input.ack, &input.decision)
        .is_none());
    assert!(activation.take_inactive().is_none());
    assert!(activation.restore_event(&input.event, input.owner).is_ok());
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let mut leases: Vec<_> = (0..5)
        .map(|_| grant.lease(&client.origin, 0).unwrap())
        .collect();
    drop(grant);
    let mut jobs: Vec<_> = (0..4)
        .map(|_| {
            Job::start_prepared(
                client.clone(),
                leases.pop().unwrap(),
                self::input(&client.origin),
            )
            .unwrap_or_else(|e| panic!("{}", e.error))
        })
        .collect();
    for job in &jobs {
        wait(job);
    }
    let error = Job::start_prepared(
        client.clone(),
        leases.pop().unwrap(),
        self::input(&client.origin),
    )
    .err()
    .unwrap();
    assert_eq!(error.error.kind(), std::io::ErrorKind::WouldBlock);
    assert!(!error.input.decision.attempted());
    for job in &mut jobs {
        let outcome = job.try_join(0).unwrap();
        assert_eq!(outcome.submission, Err(JobError::Stopped));
        assert!(!outcome.input.decision.attempted());
    }
}
