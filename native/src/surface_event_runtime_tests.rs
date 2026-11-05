//! 真实 TLS 后台任务的取消、迟到撤权与共享容量；不操作真实服务。
use super::*;
use crate::{
    device_authorization_lease::Grant,
    surface_event_runtime::{Error as JobError, Job},
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
fn late_revocation_retains_ack_and_event_but_denies_delivery() {
    let (listener, client, config) = fixture();
    let owner = active(&client.origin);
    let event = event(&owner);
    let id = event.id().to_owned();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let peer =
        std::thread::spawn(move || {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (_, body) = request(&mut stream);
            reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({
            "protocolVersion":"loom.surface.v1","instanceId":body["instanceId"],
            "eventId":body["eventId"],"requestId":"request:1","accepted":true,"status":"succeeded"
        })).unwrap());
            drop(stream);
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("GET /v1/surfaces/stream?after=0&timeoutMs=0 "));
            reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({
            "protocolVersion":"loom.surface-stream.v1","next":1,"reset":true,
            "messages":[{"method":"loom.surface.snapshot","params":{"generation":2,"snapshot":{
                "protocolVersion":"loom.surface.v1","instanceId":"surface:test/part",
                "attachmentId":"attachment:1","revision":4,
                "scene":{"id":"root","type":"text","props":{"text":"fresh"}}
            }}}]
        })).unwrap());
        });
    let mut job = Job::start_prepared(client, lease, owner, 3, event)
        .unwrap_or_else(|e| panic!("{}", e.error));
    wait(&job);
    drop(grant);
    let outcome = job.try_join(0).unwrap();
    assert!(outcome.delivery.is_err());
    assert!(outcome.result.unwrap().succeeded());
    assert_eq!(outcome.convergence.unwrap().unwrap().revision, 4);
    assert_eq!(outcome.event.id(), id);
    assert!(outcome.event.attempted());
    assert!(outcome.owner.confirmed_active());
    assert!(job.try_join(0).is_none() && job.wait_handle().is_none());
    peer.join().unwrap();
}
#[test]
fn cancellation_after_post_returns_attempted_event_without_replay() {
    let (listener, client, config) = fixture();
    let owner = active(&client.origin);
    let event = event(&owner);
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let (sent_tx, sent_rx) = std::sync::mpsc::channel();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        sent_tx.send(()).unwrap();
        let _ = finish_rx.recv_timeout(std::time::Duration::from_secs(8));
    });
    let mut job = Job::start_prepared(client, lease, owner, 3, event)
        .unwrap_or_else(|e| panic!("{}", e.error));
    sent_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    job.request_stop();
    wait(&job);
    let outcome = job.try_join(0).unwrap();
    let _ = finish_tx.send(());
    peer.join().unwrap();
    assert!(outcome.delivery.is_err());
    assert!(outcome.result.is_err());
    assert!(outcome.convergence.is_none());
    assert!(outcome.event.attempted());
    assert!(outcome.owner.confirmed_active());
}
#[test]
fn completed_tasks_hold_capacity_and_failed_start_returns_owner_and_event() {
    let (_listener, client, _config) = fixture();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let mut leases: Vec<_> = (0..5)
        .map(|_| grant.lease(&client.origin, 0).unwrap())
        .collect();
    drop(grant);
    let mut jobs: Vec<_> = (0..4)
        .map(|_| {
            let owner = active(&client.origin);
            let event = event(&owner);
            Job::start_prepared(client.clone(), leases.pop().unwrap(), owner, 3, event)
                .unwrap_or_else(|e| panic!("{}", e.error))
        })
        .collect();
    for job in &jobs {
        wait(job);
    }
    let owner = active(&client.origin);
    let pending = event(&owner);
    let id = pending.id().to_owned();
    let Err(error) = Job::start_prepared(client, leases.pop().unwrap(), owner, 3, pending) else {
        panic!("capacity released before join")
    };
    assert_eq!(error.error.kind(), std::io::ErrorKind::WouldBlock);
    assert!(error.owner.confirmed_active());
    assert_eq!(error.event.id(), id);
    assert!(!error.event.attempted());
    for job in &mut jobs {
        let outcome = job.try_join(0).unwrap();
        assert!(matches!(outcome.result, Err(JobError::Stopped)));
        assert!(!outcome.event.attempted());
        assert!(outcome.delivery.is_err());
    }
}

#[test]
fn stream_failure_keeps_ack_and_confirmation_is_read_once_without_approval() {
    for (status, invalid_prompt) in [
        ("succeeded", false),
        ("awaiting_confirmation", false),
        ("awaiting_confirmation", true),
    ] {
        let (listener, client, config) = fixture();
        let owner = active(&client.origin);
        let event = event(&owner);
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
                "protocolVersion":"loom.surface.v1","instanceId":body["instanceId"],
                "eventId":body["eventId"],"requestId":"request:1","accepted":true,"status":status
            })).unwrap());
            drop(stream);
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("GET "));
            if status == "succeeded" || invalid_prompt {
                reply(&mut stream, 200, b"{}");
            } else {
                reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({
                    "protocolVersion":"loom.surface-stream.v1","next":1,"reset":true,"messages":[{
                        "method":"loom.surface.confirmation.request","params":{
                            "protocolVersion":"loom.surface.v1","confirmationId":"confirmation:test",
                            "instanceId":binding.instance(),"attachmentId":binding.attachment(),"deviceId":binding.device(),
                            "hookNodeId":binding.node(),"eventId":body["eventId"],"requestId":"request:1",
                            "actionId":body["action"],"risk":"high","expiresAtMs":u64::MAX
                        }
                    }]
                })).unwrap());
            }
        });
        let mut job = Job::start_prepared(client, lease, owner, 3, event)
            .unwrap_or_else(|e| panic!("{}", e.error));
        wait(&job);
        let outcome = job.try_join(0).unwrap();
        assert!(outcome.delivery.is_ok());
        assert!(outcome.event.attempted());
        let ack = outcome.result.unwrap();
        if status == "succeeded" {
            assert!(outcome.confirmation.is_none());
            assert!(ack.succeeded());
            assert!(matches!(
                outcome.convergence,
                Some(Err(JobError::Convergence(
                    crate::live_relay_http::ConvergenceError::Stream(_)
                )))
            ));
        } else {
            if invalid_prompt {
                assert!(matches!(
                    outcome.confirmation,
                    Some(Err(JobError::Confirmation(_)))
                ));
            } else {
                assert_eq!(
                    outcome.confirmation.unwrap().unwrap().id(),
                    "confirmation:test"
                );
            }
            assert_eq!(
                ack.status(),
                crate::surface_action_ack::Status::AwaitingConfirmation
            );
            assert!(matches!(
                outcome.convergence,
                Some(Err(JobError::Convergence(
                    crate::live_relay_http::ConvergenceError::ConfirmationRequired
                )))
            ));
        }
        peer.join().unwrap();
    }
}

#[test]
fn cancellation_during_stream_preserves_received_ack() {
    for status in ["running", "awaiting_confirmation"] {
        cancelled_stream(status);
    }
}
fn cancelled_stream(status: &'static str) {
    let (listener, client, config) = fixture();
    let owner = active(&client.origin);
    let event = event(&owner);
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let peer =
        std::thread::spawn(move || {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (_, body) = request(&mut stream);
            reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({
            "protocolVersion":"loom.surface.v1","instanceId":body["instanceId"],
            "eventId":body["eventId"],"requestId":"request:1","accepted":true,"status":status
        })).unwrap());
            drop(stream);
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("GET "));
            tx.send(()).unwrap();
            let _ = finish_rx.recv_timeout(std::time::Duration::from_secs(8));
        });
    let mut job = Job::start_prepared(client, lease, owner, 3, event)
        .unwrap_or_else(|e| panic!("{}", e.error));
    rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
    job.request_stop();
    wait(&job);
    let outcome = job.try_join(0).unwrap();
    let _ = finish_tx.send(());
    peer.join().unwrap();
    assert!(outcome.delivery.is_err());
    assert!(outcome.event.attempted());
    if status == "awaiting_confirmation" {
        assert_eq!(
            outcome.result.unwrap().status(),
            crate::surface_action_ack::Status::AwaitingConfirmation
        );
        assert!(matches!(
            outcome.confirmation,
            Some(Err(JobError::Confirmation(
                crate::live_relay_http::ConfirmationError::Stopped
            )))
        ));
    } else {
        assert_eq!(
            outcome.result.unwrap().status(),
            crate::surface_action_ack::Status::Running
        );
        assert!(matches!(
            outcome.convergence,
            Some(Err(JobError::Convergence(
                crate::live_relay_http::ConvergenceError::Stopped
            )))
        ));
    }
}
