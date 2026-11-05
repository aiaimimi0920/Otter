//! 固定授权 HTTPS 目录及原后台槽生命周期；不连接离线广告，不产生 create 副作用。
use super::*;
use crate::{
    device_authorization_lease::Grant,
    projection_delivery::targets::{tests::body, Status},
    projection_delivery_runtime::{Error as JobError, Job, Operation, Output},
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
fn projection_targets_authenticated_fixed_path_distinct_nonces_and_typed_result() {
    let (listener, client, config) = fixture();
    let origin = client.origin().to_owned();
    let peer = std::thread::spawn(move || {
        let mut nonces = std::collections::BTreeSet::new();
        for _ in 0..2 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, request_body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/projections/targets HTTP/1.1"));
            assert_eq!(request_body, serde_json::json!({}));
            let lower = head.to_ascii_lowercase();
            assert!(lower.contains("authorization: device fixture-secret\r\n"));
            let nonce = lower
                .lines()
                .find_map(|line| line.strip_prefix("x-loom-device-nonce: "))
                .unwrap();
            assert_eq!(nonce.len(), 32);
            assert!(nonces.insert(nonce.to_owned()));
            reply(&mut stream, 200, &serde_json::to_vec(&body()).unwrap());
        }
        listener
    });
    let mut grant = Grant::new(session(&origin, u64::MAX));
    for _ in 0..2 {
        let mut job = Job::start_prepared(
            client.clone(),
            grant.lease(&origin, 0).unwrap(),
            Operation::Targets,
        )
        .unwrap();
        wait(&job);
        let Output::Targets(directory) = job.try_join().unwrap().unwrap() else {
            panic!("missing targets")
        };
        assert_eq!(directory.origin(), origin);
        assert_eq!(directory.status(), Status::Partial);
        assert_eq!(directory.targets().len(), 1);
        assert_eq!(directory.groups().unwrap().len(), 1);
        assert!(job.wait_handle().is_none());
    }
    assert!(
        matches!(peer.join().unwrap().accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock)
    );
}
#[test]
fn projection_targets_denial_redirect_oversize_and_bad_groups_never_publish_or_retry() {
    for status in [302, 401, 403, 200, 201] {
        let (listener, client, config) = fixture();
        let authorized = session(client.origin(), u64::MAX);
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            if status == 201 {
                write!(
                    stream,
                    "HTTP/1.1 200 Fixture\r\nContent-Length: {}\r\n\r\n",
                    delivery::MAX_JSON + 1
                )
                .unwrap();
                stream.flush().unwrap();
            } else {
                let mut value = body();
                value["groups"][0]["targetIds"] = serde_json::json!(["missing"]);
                reply(&mut stream, status, &serde_json::to_vec(&value).unwrap());
            }
            listener
        });
        let cancel = Cancel::new();
        let error = Control::new(&client, &authorized, &cancel)
            .projection_targets()
            .unwrap_err();
        assert_eq!(
            error,
            match status {
                401 => LiveError::AuthorizationRequired,
                200 => LiveError::Projection(delivery::Error::Entry),
                201 => LiveError::Http(Error::Limit),
                _ => LiveError::Status(status),
            }
        );
        assert!(
            matches!(peer.join().unwrap().accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock)
        );
    }
}
#[test]
fn projection_targets_revoked_completed_result_and_cancelled_network_keep_original_slot_owner() {
    for cancel_pending in [false, true] {
        let (listener, client, config) = fixture();
        let origin = client.origin().to_owned();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            tx.send(()).unwrap();
            if cancel_pending {
                let mut byte = [0];
                assert!(!stream.read(&mut byte).is_ok_and(|count| count > 0));
            } else {
                reply(&mut stream, 200, br#"{"targets":[]}"#);
            }
        });
        let mut grant = Grant::new(session(&origin, u64::MAX));
        let mut job =
            Job::start_prepared(client, grant.lease(&origin, 0).unwrap(), Operation::Targets)
                .unwrap();
        rx.recv_timeout(Duration::from_secs(5)).unwrap();
        if cancel_pending {
            job.request_stop();
        }
        wait(&job);
        let slots: Vec<_> = (0..3)
            .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
            .collect();
        assert!(crate::live_relay_control_runtime::Slot::acquire().is_err());
        drop(grant);
        assert!(matches!(job.try_join(), Some(Err(JobError::Stopped))));
        let last = crate::live_relay_control_runtime::Slot::acquire().unwrap();
        drop((last, slots));
        peer.join().unwrap();
    }
}
#[test]
fn projection_targets_wrong_origin_expired_and_precancelled_credentials_never_connect() {
    let (listener, client, _) = fixture();
    for (origin, expires, stopped) in [
        ("https://other.example", u64::MAX, false),
        (client.origin(), 1, false),
        (client.origin(), u64::MAX, true),
    ] {
        let authorized = session(origin, expires);
        let cancel = Cancel::new();
        if stopped {
            cancel.cancel();
        }
        assert!(Control::new(&client, &authorized, &cancel)
            .projection_targets()
            .is_err());
    }
    assert!(matches!(listener.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
}
