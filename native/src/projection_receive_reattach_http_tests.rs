//! accepted 只读重建的真实 TLS/Grant/journal 链；任何结果均不能拥有自动 unlink 资格。
use super::*;
use crate::{
    device_authorization_lease::Grant,
    projection_delivery::{
        accept::tests as png,
        journal::{tests::Directory, Store},
    },
    projection_delivery_runtime::{Job, Operation, Output},
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
fn projection_reattach_read_validates_binding_pixels_and_original_grant_without_accept_or_unlink() {
    reattach_routed(false)
}
fn reattach_routed(offline: bool) {
    for mode in 0..7 {
        let directory = Directory::new();
        let store = Store::open(&directory.0).unwrap();
        let (listener, client, tls) = fixture();
        let origin = client.origin().to_owned();
        let server_origin = origin.clone();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
            let (head, body) = request(&mut stream);
            assert!(head.starts_with(if offline {
                "POST /v1/offline-projections/read "
            } else {
                "POST /v1/projections/read "
            }));
            assert!(head
                .to_ascii_lowercase()
                .contains("authorization: device fixture-secret\r\n"));
            assert_eq!(
                body,
                serde_json::json!({"projectionId":png::invitation(&server_origin).envelope().projection_id(),"knownRevision":0})
            );
            let mut value = if offline {
                delivery::tests::offline::response("device-1")
            } else {
                png::response(&server_origin)
            };
            match mode {
                1 => value["linked"] = false.into(),
                2 => value["receiverDeviceId"] = "different-device".into(),
                3 => value["receiverUnitId"] = "different-unit".into(),
                4 => value["revision"] = 3.into(),
                5 => value["editing"] = serde_json::json!({"document":{}}),
                6 => {
                    reply(&mut stream, 403, b"denied");
                    return listener;
                }
                _ => {}
            }
            reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
            listener
        });
        let mut grant = Grant::new(session(&origin, u64::MAX));
        let destination = grant.lease(&origin, 0).unwrap();
        let mut job = Job::start_prepared(
            client,
            grant.lease(&origin, 0).unwrap(),
            Operation::Reattach {
                invitation: Box::new(if offline {
                    delivery::tests::offline::invitation(&origin, true)
                } else {
                    png::accepted(&origin)
                }),
                journal: store.clone(),
            },
        )
        .unwrap();
        wait(&job);
        let result = job.try_join().unwrap();
        let listener = peer.join().unwrap();
        assert!(job.take_abandoned().is_none());
        if mode == 0 {
            let Output::Received(ready) = result.unwrap() else {
                panic!("missing read result")
            };
            assert!(ready.cleanup_ticket().is_none());
            let mut other = Grant::new(session(&origin, u64::MAX));
            assert!(ready
                .validate(&other.lease(&origin, 0).unwrap(), 0)
                .is_err());
            let (value, held) = ready.transfer(&destination, 0).unwrap();
            assert_eq!((value.image().width, value.image().height), (2, 1));
            assert!(held.cleanup_ticket().is_none());
            drop(grant);
            assert!(held.validate(&destination, 0).is_err());
            drop(held);
        } else {
            assert!(result.is_err(), "mode {mode}");
        }
        assert_eq!(store.pending().unwrap().len(), 1);
        assert!(!store.pending().unwrap()[0].cleanup_pending());
        assert!(matches!(listener.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
    }
}
#[test]
fn offline_receiver_reattach_uses_original_read_route_and_checks_identity_without_accept_or_unlink()
{
    reattach_routed(true)
}

#[test]
fn projection_reattach_new_invitation_and_changed_journal_cannot_start_http() {
    for new in [true, false] {
        let directory = Directory::new();
        let store = Store::open(&directory.0).unwrap();
        let (listener, client, _tls) = fixture();
        let origin = client.origin().to_owned();
        if !new {
            store.begin(&png::accepted(&origin), "device-1").unwrap();
            std::fs::write(
                directory.0.join("projection-receive.json"),
                b"competing bytes",
            )
            .unwrap();
        }
        let mut grant = Grant::new(session(&origin, u64::MAX));
        let mut job = Job::start_prepared(
            client,
            grant.lease(&origin, 0).unwrap(),
            Operation::Reattach {
                invitation: Box::new(if new {
                    png::invitation(&origin)
                } else {
                    png::accepted(&origin)
                }),
                journal: store.clone(),
            },
        )
        .unwrap();
        wait(&job);
        assert!(job.try_join().unwrap().is_err());
        assert!(job.take_abandoned().is_none());
        assert!(matches!(listener.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
        if new {
            assert!(store.pending().unwrap().is_empty());
        } else {
            assert_eq!(
                std::fs::read(directory.0.join("projection-receive.json")).unwrap(),
                b"competing bytes"
            );
        }
    }
}
