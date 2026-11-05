//! 共享与离线收件仍只有原Job/Slot；本配对TLS、静态路径、独立nonce和partial/停止边界。
use super::*;
use crate::{
    device_authorization_lease::Grant,
    projection_delivery::tests::offline,
    projection_delivery_runtime::{Error as JobError, Job, Operation, Output},
};
use serde_json::json;
use std::{collections::BTreeSet, os::windows::io::AsRawHandle, sync::mpsc};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};
fn shared(origin: &str) -> Vec<u8> {
    let mut entry = delivery::tests::invitation(origin, false);
    entry["envelope"]["projectionId"] = format!("projection:{}", "2".repeat(32)).into();
    serde_json::to_vec(&json!({"invitations":[entry],"offlinePeers":true})).unwrap()
}
fn nonce(head: &str) -> String {
    let lower = head.to_ascii_lowercase();
    assert!(lower.contains("authorization: device fixture-secret\r\n"));
    let nonce = lower
        .lines()
        .find_map(|line| line.strip_prefix("x-loom-device-nonce: "))
        .unwrap();
    assert_eq!(nonce.len(), 32);
    nonce.into()
}
fn join(job: &mut Job) -> Result<Output, JobError> {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
    job.try_join().unwrap()
}
#[test]
fn offline_receiver_shared_then_offline_inbox_and_rejection_use_paired_tls_and_distinct_nonces() {
    let (listener, client, tls) = fixture();
    let origin = client.origin().to_owned();
    let bytes = shared(&origin);
    let peer = std::thread::spawn(move || {
        let mut nonces = BTreeSet::new();
        for (index, path) in [
            "/v1/projections/inbox",
            "/v1/offline-projections/inbox",
            "/v1/offline-projections/receipt",
        ]
        .into_iter()
        .enumerate()
        {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with(&format!("POST {path} HTTP/1.1")));
            assert!(nonces.insert(nonce(&head)));
            if index < 2 {
                assert_eq!(body, json!({"policy":"confirm"}));
            } else {
                assert_eq!(
                    body,
                    json!({"projectionId":format!("projection:{}","1".repeat(32)),"status":"rejected"})
                );
            }
            let response = match index {
                0 => bytes.clone(),
                1 => offline::body(false),
                _ => br#"{"recorded":true}"#.to_vec(),
            };
            reply(&mut stream, 200, &response);
        }
        listener
    });
    let authorized = session(&origin, u64::MAX);
    let cancel = Cancel::new();
    let control = Control::new(&client, &authorized, &cancel);
    let inbox = control.projection_inbox(InboxPolicy::Confirm).unwrap();
    assert_eq!(inbox.entries().len(), 2);
    assert!(!inbox.has_offline_peers());
    assert_eq!(inbox.entries()[1].envelope().origin(), offline::SOURCE);
    assert_eq!(inbox.entries()[1].origin(), origin);
    assert_eq!(inbox.entries()[1].peer_id(), Some(offline::PEER));
    control.reject_projection(&inbox.entries()[1]).unwrap();
    assert!(matches!(peer.join().unwrap().accept(),Err(e) if e.kind()==io::ErrorKind::WouldBlock));
}
#[test]
fn offline_receiver_partial_query_keeps_only_current_shared_entries_and_never_hides_401() {
    for mode in 0..4 {
        let (listener, client, tls) = fixture();
        let origin = client.origin().to_owned();
        let bytes = shared(&origin);
        let peer = std::thread::spawn(move || {
            for index in 0..2 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, _) = request(&mut stream);
                assert!(head.starts_with(if index == 0 {
                    "POST /v1/projections/inbox "
                } else {
                    "POST /v1/offline-projections/inbox "
                }));
                if index == 0 {
                    reply(&mut stream, 200, &bytes);
                } else {
                    let mut entry = offline::entry(false);
                    if mode == 1 {
                        entry["peerId"] = serde_json::Value::Null;
                    }
                    if mode == 2 {
                        entry["envelope"]["projectionId"] =
                            format!("projection:{}", "2".repeat(32)).into();
                    }
                    reply(
                        &mut stream,
                        match mode {
                            0 => 503,
                            3 => 401,
                            _ => 200,
                        },
                        &serde_json::to_vec(&json!({"invitations":[entry]})).unwrap(),
                    );
                }
            }
            listener
        });
        let authorized = session(&origin, u64::MAX);
        let cancel = Cancel::new();
        let result =
            Control::new(&client, &authorized, &cancel).projection_inbox(InboxPolicy::Confirm);
        if mode == 3 {
            assert_eq!(result.unwrap_err(), LiveError::AuthorizationRequired);
        } else {
            let inbox = result.unwrap();
            assert_eq!(inbox.entries().len(), 1);
            assert!(inbox.has_offline_peers());
            assert!(inbox.entries()[0].peer_id().is_none());
        }
        assert!(
            matches!(peer.join().unwrap().accept(),Err(e) if e.kind()==io::ErrorKind::WouldBlock)
        );
    }
}
#[test]
fn offline_receiver_cancel_during_second_request_cannot_publish_partial_inbox() {
    let (listener, client, tls) = fixture();
    let origin = client.origin().to_owned();
    let bytes = shared(&origin);
    let (tx, rx) = mpsc::sync_channel(1);
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(
            ServerConnection::new(tls.clone()).unwrap(),
            accept(&listener),
        );
        request(&mut stream);
        reply(&mut stream, 200, &bytes);
        drop(stream);
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        let (head, _) = request(&mut stream);
        assert!(head.starts_with("POST /v1/offline-projections/inbox "));
        tx.send(()).unwrap();
        let mut byte = [0];
        assert!(!stream.read(&mut byte).is_ok_and(|n| n != 0));
    });
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let mut job = Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        Operation::Inbox(InboxPolicy::Confirm),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    job.request_stop();
    assert!(matches!(join(&mut job), Err(JobError::Stopped)));
    peer.join().unwrap();
}
#[test]
fn offline_receiver_completed_aggregate_cannot_escape_revoked_original_grant() {
    let (listener, client, tls) = fixture();
    let origin = client.origin().to_owned();
    let bytes = shared(&origin);
    let peer = std::thread::spawn(move || {
        for index in 0..2 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            request(&mut stream);
            reply(
                &mut stream,
                200,
                if index == 0 {
                    &bytes
                } else {
                    br#"{"invitations":[]}"#
                },
            );
        }
    });
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let mut job = Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        Operation::Inbox(InboxPolicy::Confirm),
    )
    .unwrap();
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
    peer.join().unwrap();
    drop(grant);
    assert!(matches!(job.try_join(), Some(Err(JobError::Stopped))));
}
