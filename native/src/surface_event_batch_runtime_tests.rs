//! 批次线程在取消/迟到撤权/共享容量失败时仍返还全部事件和唯一 owner。
use super::*;
use crate::{
    device_authorization_lease::Grant,
    surface_event_batch_runtime::{Error as JobError, Input, Job},
};
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};
fn wait(job: &Job) {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
}
#[test]
fn confirmation_is_read_once_and_kept_separate_from_batch_convergence() {
    let (listener, client, config) = fixture();
    let owner = active(&client.origin);
    let events = vec![event(&owner), event(&owner)];
    let binding = owner.binding().clone();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(
            ServerConnection::new(config.clone()).unwrap(),
            accept(&listener),
        );
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST "));
        reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({
            "protocolVersion":"loom.surface.v1","instanceId":body["instanceId"],"eventId":body["eventId"],
            "requestId":"request:confirm","accepted":true,"status":"awaiting_confirmation"
        })).unwrap());
        drop(stream);
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, _) = request(&mut stream);
        assert!(head.starts_with("GET /v1/surfaces/stream?after=0&timeoutMs=0 "));
        reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({
            "protocolVersion":"loom.surface-stream.v1","next":1,"reset":true,
            "messages":[{"method":"loom.surface.confirmation.request","params":{
                "protocolVersion":"loom.surface.v1","confirmationId":"confirmation:batch",
                "instanceId":body["instanceId"],"attachmentId":binding.attachment(),
                "deviceId":binding.device(),"hookNodeId":binding.node(),"eventId":body["eventId"],
                "requestId":"request:confirm","actionId":body["action"],"risk":"high","expiresAtMs":u64::MAX
            }}]
        })).unwrap());
    });
    let mut job = Job::start_prepared(
        client,
        lease,
        Input {
            owner,
            revision: 3,
            events,
        },
    )
    .unwrap_or_else(|error| panic!("{}", error.error));
    wait(&job);
    let outcome = job.try_join(0).unwrap();
    let submission = outcome.submission.unwrap();
    assert_eq!(submission.stop, BatchStop::Confirmation);
    assert_eq!(submission.acknowledgements.len(), 1);
    assert!(!outcome.input.events[1].attempted());
    let observation = outcome.observation.unwrap().unwrap();
    assert!(observation.snapshot.is_none());
    assert_eq!(observation.items.len(), 1);
    let item = observation.items.into_iter().next().unwrap();
    assert_eq!(
        item.result,
        Err(
            crate::surface_event_batch_runtime::ObservationError::Convergence(
                crate::live_relay_http::ConvergenceError::ConfirmationRequired
            )
        )
    );
    assert_eq!(
        item.confirmation.unwrap().unwrap().id(),
        "confirmation:batch"
    );
    peer.join().unwrap();
}
#[test]
fn cancelled_batch_and_full_slot_return_every_unattempted_event() {
    let (_listener, client, _config) = fixture();
    let owner = active(&client.origin);
    let events = vec![event(&owner), event(&owner)];
    let ids: Vec<_> = events.iter().map(|event| event.id().to_owned()).collect();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    drop(grant);
    let slots: Vec<_> = (0..4)
        .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
        .collect();
    let error = Job::start_prepared(
        client,
        lease,
        Input {
            owner,
            revision: 3,
            events,
        },
    )
    .err()
    .unwrap();
    assert_eq!(error.error.kind(), std::io::ErrorKind::WouldBlock);
    assert_eq!(
        error
            .input
            .events
            .iter()
            .map(|event| event.id().to_owned())
            .collect::<Vec<_>>(),
        ids
    );
    drop(slots);
    let (_listener, client, _config) = fixture();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    drop(grant);
    let mut job = Job::start_prepared(client, lease, *error.input)
        .unwrap_or_else(|error| panic!("{}", error.error));
    wait(&job);
    let outcome = job.try_join(0).unwrap();
    assert!(matches!(outcome.submission, Err(JobError::Stopped)));
    assert!(outcome.delivery.is_err());
    assert!(outcome.input.events.iter().all(|event| !event.attempted()));
    assert!(outcome.input.owner.confirmed_active());
    assert!(job.try_join(0).is_none() && job.wait_handle().is_none());
}
#[test]
fn completed_batch_preserves_ack_when_authority_is_revoked_before_join() {
    observe_revisions([4, 5]);
}
#[test]
fn later_batch_observation_cannot_replace_a_newer_snapshot_with_an_older_one() {
    observe_revisions([5, 4]);
}
fn observe_revisions(revisions: [u64; 2]) {
    let (listener, client, config) = fixture();
    let owner = active(&client.origin);
    let events = vec![event(&owner), event(&owner)];
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let peer = std::thread::spawn(move || {
        for index in 0..2 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST "));
            reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({
            "protocolVersion":"loom.surface.v1","instanceId":body["instanceId"],"eventId":body["eventId"],
            "requestId":format!("request:{index}"),"accepted":true,"status":"succeeded"
        })).unwrap());
        }
        for revision in revisions {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("GET /v1/surfaces/stream?after=0&timeoutMs=0 "));
            reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({
                "protocolVersion":"loom.surface-stream.v1","next":1,"reset":true,
                "messages":[{"method":"loom.surface.snapshot","params":{"generation":2,"snapshot":{
                    "protocolVersion":"loom.surface.v1","instanceId":"surface:test/part",
                    "attachmentId":"attachment:1","revision":revision,"scene":{"id":"root","type":"text"}
                }}}]
            })).unwrap());
        }
    });
    let mut job = Job::start_prepared(
        client,
        lease,
        Input {
            owner,
            revision: 3,
            events,
        },
    )
    .unwrap_or_else(|error| panic!("{}", error.error));
    wait(&job);
    drop(grant);
    let outcome = job.try_join(0).unwrap();
    assert!(outcome.delivery.is_err());
    let submitted = outcome.submission.unwrap();
    assert_eq!(submitted.stop, BatchStop::Complete);
    assert_eq!(submitted.acknowledgements.len(), 2);
    assert!(submitted.acknowledgements[0].accepted());
    assert!(outcome.input.events[0].attempted());
    let observation = outcome.observation.unwrap().unwrap();
    assert_eq!(observation.items.len(), 2);
    assert_eq!(observation.items[0].result, Ok(revisions[0]));
    assert_eq!(observation.items[1].result, Ok(revisions[1]));
    assert_eq!(observation.snapshot.unwrap().revision, 5);
    peer.join().unwrap();
}
