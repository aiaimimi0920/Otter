use super::*;
use crate::{
    device_authorization_lease::Grant,
    surface_lifecycle_owner::Owner,
    surface_lifecycle_runtime::{Action, Error as JobError, Job},
    surface_live_binding::Lifecycle,
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
fn cancellation_returns_uncertain_owner_without_delivering_late_success() {
    let (listener, client, config) = fixture();
    let owner = Owner::new(binding(&client.origin));
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
    let mut job = Job::start_prepared(client, lease, owner, Action::Transition(Lifecycle::Active))
        .unwrap_or_else(|e| panic!("{}", e.error));
    sent_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let start = std::time::Instant::now();
    job.request_stop();
    wait(&job);
    let outcome = job.try_join().unwrap();
    let _ = finish_tx.send(());
    peer.join().unwrap();
    assert!(outcome.owner.uncertain());
    assert!(matches!(outcome.result, Err(JobError::Stopped)));
    assert!(start.elapsed() < std::time::Duration::from_secs(2));
    assert!(job.wait_handle().is_none() && job.try_join().is_none());
}
#[test]
fn completed_tasks_hold_shared_capacity_and_failed_start_returns_owner() {
    let (_listener, client, _config) = fixture();
    let original = binding(&client.origin);
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
                Owner::new(original.clone()),
                Action::Recover,
            )
            .unwrap_or_else(|e| panic!("{}", e.error))
        })
        .collect();
    for job in &jobs {
        wait(job);
    }
    let Err(error) = Job::start_prepared(
        client,
        leases.pop().unwrap(),
        Owner::new(original.clone()),
        Action::Recover,
    ) else {
        panic!("capacity was released before join")
    };
    assert_eq!(error.error.kind(), std::io::ErrorKind::WouldBlock);
    assert_eq!(error.owner.binding(), &original);
    assert!(!error.owner.uncertain());
    for job in &mut jobs {
        let outcome = job.try_join().unwrap();
        assert!(matches!(outcome.result, Err(JobError::Stopped)));
        assert!(!outcome.owner.uncertain());
    }
}

#[test]
fn revoked_late_success_keeps_confirmed_revision_but_is_not_delivered_as_success() {
    let (listener, client, config) = fixture();
    let owner = Owner::new(binding(&client.origin));
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        let mut ack = mounted()["instance"]["attachments"]["attachment:1"].clone();
        ack["lifecycle"] = "active".into();
        ack["lifecycleRevision"] = 8.into();
        reply(&mut stream, 200, &serde_json::to_vec(&ack).unwrap());
    });
    let mut job = Job::start_prepared(client, lease, owner, Action::Transition(Lifecycle::Active))
        .unwrap_or_else(|e| panic!("{}", e.error));
    wait(&job);
    drop(grant);
    let outcome = job.try_join().unwrap();
    assert!(matches!(outcome.result, Err(JobError::Stopped)));
    assert!(!outcome.owner.uncertain());
    assert_eq!(outcome.owner.binding().lifecycle_revision(), 8);
    peer.join().unwrap();
}
