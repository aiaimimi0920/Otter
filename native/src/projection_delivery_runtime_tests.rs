use super::*;
use crate::{
    device_authorization_lease::Grant,
    projection_delivery_runtime::{Error as JobError, Job, Operation, Output},
};
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};
#[path = "projection_receive_abandonment_http_tests.rs"]
mod abandonment;
fn grant(client: &Client) -> Grant {
    Grant::new(session(client.origin(), u64::MAX))
}
fn wait(job: &Job) {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
}
#[test]
fn background_inbox_and_rejection_keep_lease_and_return_typed_outcomes() {
    let (listener, client, config) = fixture();
    let origin = client.origin().to_owned();
    let response = delivery::tests::body(&origin);
    let peer = std::thread::spawn(move || {
        for index in 0..2 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            assert!(head.starts_with(if index == 0 {
                "POST /v1/projections/inbox "
            } else {
                "POST /v1/projections/receipt "
            }));
            reply(
                &mut stream,
                200,
                if index == 0 {
                    &response
                } else {
                    br#"{"recorded":true}"#
                },
            );
        }
    });
    let mut grant = grant(&client);
    let mut job = Job::start_prepared(
        client.clone(),
        grant.lease(&origin, 0).unwrap(),
        Operation::Inbox(InboxPolicy::Confirm),
    )
    .unwrap();
    assert_eq!(job.origin(), origin);
    wait(&job);
    let Output::Inbox(inbox) = job.try_join().unwrap().unwrap() else {
        panic!("missing inbox")
    };
    assert!(job.wait_handle().is_none() && job.try_join().is_none());
    let entry = inbox.into_entries().pop().unwrap();
    let mut reject = Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        Operation::Reject(Box::new(entry)),
    )
    .unwrap();
    wait(&reject);
    assert!(matches!(reject.try_join(), Some(Ok(Output::Rejected))));
    peer.join().unwrap();
}
#[test]
fn completed_success_cannot_be_published_after_grant_revocation() {
    let (listener, client, config) = fixture();
    let origin = client.origin().to_owned();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        reply(&mut stream, 200, br#"{"invitations":[]}"#);
    });
    let mut grant = grant(&client);
    let mut job = Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        Operation::Inbox(InboxPolicy::Confirm),
    )
    .unwrap();
    wait(&job);
    peer.join().unwrap();
    drop(grant);
    assert!(matches!(job.try_join(), Some(Err(JobError::Stopped))));
}
#[test]
fn cancellation_interrupts_pending_network_and_keeps_slot_until_join() {
    let (listener, client, config) = fixture();
    let origin = client.origin().to_owned();
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        tx.send(()).unwrap();
        let mut byte = [0];
        assert!(!stream.read(&mut byte).is_ok_and(|read| read != 0));
    });
    let mut grant = grant(&client);
    let mut job = Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        Operation::Inbox(InboxPolicy::Confirm),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    job.request_stop();
    wait(&job);
    assert!(matches!(job.try_join(), Some(Err(JobError::Stopped))));
    peer.join().unwrap();
}
#[test]
fn completed_uncollected_jobs_share_original_four_control_slots() {
    let (listener, client, config) = fixture();
    let origin = client.origin().to_owned();
    let peer = std::thread::spawn(move || {
        for _ in 0..4 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            request(&mut stream);
            reply(&mut stream, 200, br#"{"invitations":[]}"#);
        }
    });
    let mut grant = grant(&client);
    let mut jobs = Vec::new();
    for _ in 0..4 {
        let job = Job::start_prepared(
            client.clone(),
            grant.lease(&origin, 0).unwrap(),
            Operation::Inbox(InboxPolicy::Disabled),
        )
        .unwrap();
        wait(&job);
        jobs.push(job);
    }
    let fifth = Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        Operation::Inbox(InboxPolicy::Confirm),
    );
    assert_eq!(fifth.err().unwrap().kind(), io::ErrorKind::WouldBlock);
    for job in &mut jobs {
        assert!(matches!(job.try_join(), Some(Ok(Output::Inbox(_)))));
    }
    peer.join().unwrap();
}

#[test]
fn configured_job_loads_scoped_ca_and_resolves_in_background_without_repairing_identity() {
    let (files, _store, _loaded) = crate::device_session_client::tests::pairing::Files::new();
    let identity_path = files.root().join("device-identity.json");
    let before = std::fs::read(&identity_path).unwrap();
    let (listener, client, tls, pem) = crate::device_session_client::tests::fixture_with_pem();
    let origin = client.origin().to_owned();
    let ca = files.root().join("ca.pem");
    let config =
        crate::device_authorization_config::Config::from_lookup(&origin, |key| match key {
            "OTTER_NATIVE_PROFILE_DIR" => Some(files.root().as_os_str().to_owned()),
            "OTTER_LOOM_TLS_ORIGIN" => Some(origin.clone().into()),
            "OTTER_LOOM_TLS_CA_FILE" => Some(ca.as_os_str().to_owned()),
            _ => None,
        })
        .unwrap();
    assert!(!ca.exists());
    std::fs::write(&ca, pem).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/projections/inbox HTTP/1.1"));
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: device fixture-secret\r\n"));
        assert_eq!(body, serde_json::json!({"policy":"confirm"}));
        reply(&mut stream, 200, br#"{"invitations":[]}"#);
    });
    let mut grant = grant(&client);
    let mut job = Job::start(
        config,
        grant.lease(&origin, 0).unwrap(),
        Operation::Inbox(InboxPolicy::Confirm),
    )
    .unwrap();
    wait(&job);
    assert!(matches!(job.try_join(), Some(Ok(Output::Inbox(inbox))) if inbox.entries().is_empty()));
    peer.join().unwrap();
    assert_eq!(std::fs::read(identity_path).unwrap(), before);
}
