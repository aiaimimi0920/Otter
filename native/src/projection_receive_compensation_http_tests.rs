//! 写前落盘→丢失 accept 回复→重启→认证解绑；仅回环 TLS/linshi，不伪造 displayed。
use super::*;
use crate::{
    device_authorization_lease::Grant,
    projection_delivery::accept::tests as png,
    projection_delivery::journal::{tests::Directory, Store},
    projection_delivery_runtime::{Error as JobError, Job, Operation, Output},
};
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};
fn join(job: &mut Job) -> Result<Output, JobError> {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
    job.try_join().unwrap()
}
fn operation(origin: &str, store: &Store) -> Operation {
    Operation::Accept {
        invitation: Box::new(png::invitation(origin)),
        journal: store.clone(),
    }
}
fn no_request(listener: &std::net::TcpListener) {
    assert!(matches!(listener.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
}
#[test]
fn lost_accept_reply_keeps_durable_intent_then_authenticated_unlink_retires_after_restart() {
    lost_reply_routed(false)
}
fn lost_reply_routed(offline: bool) {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let path = directory.path();
    let (listener, client, tls) = fixture();
    let origin = client.origin().to_owned();
    let entry = if offline {
        delivery::tests::offline::invitation(&origin, false)
    } else {
        png::invitation(&origin)
    };
    let expected = entry.accept_body(&origin).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(
            ServerConnection::new(tls.clone()).unwrap(),
            accept(&listener),
        );
        let (head, body) = request(&mut stream);
        assert!(head.starts_with(if offline {
            "POST /v1/offline-projections/accept HTTP/1.1"
        } else {
            "POST /v1/projections/accept HTTP/1.1"
        }));
        assert_eq!(
            body,
            serde_json::from_slice::<serde_json::Value>(&expected).unwrap()
        );
        let persisted: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(persisted["entries"].as_array().unwrap().len(), 1);
        assert_eq!(persisted["entries"][0]["unit"], body["receiverUnitId"]);
        if offline {
            assert_eq!(
                persisted["entries"][0]["offline"]["peerId"],
                delivery::tests::offline::PEER
            );
            assert_eq!(
                persisted["entries"][0]["envelope"]["serverOrigin"],
                delivery::tests::offline::SOURCE
            );
        }
        assert_eq!(
            persisted["entries"][0]["expectedRevision"],
            body["expectedRevision"]
        );
        // 服务端已可能绑定，但主动丢失所有 HTTP 回复；不能当作 accept 从未发生。
        drop(stream);
        (listener, tls)
    });
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let mut job = Job::start_prepared(
        client.clone(),
        grant.lease(&origin, 0).unwrap(),
        Operation::Accept {
            invitation: Box::new(entry),
            journal: store.clone(),
        },
    )
    .unwrap();
    assert!(join(&mut job).is_err());
    drop((job, store));
    let (listener, tls) = peer.join().unwrap();
    let store = Store::open(&directory.0).unwrap();
    let ticket = store.pending().unwrap().pop().unwrap();
    let stale = store.pending().unwrap().pop().unwrap();
    let projection = ticket.envelope().projection_id().to_owned();
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with(if offline {
            "POST /v1/offline-projections/unlink HTTP/1.1"
        } else {
            "POST /v1/projections/unlink HTTP/1.1"
        }));
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: device fixture-secret\r\n"));
        assert_eq!(body, serde_json::json!({"projectionId":projection}));
        reply(&mut stream, 200, br#"{"unlinked":true}"#);
        drop(stream);
        listener
    });
    let mut job = Job::start_prepared(
        client.clone(),
        grant.lease(&origin, 0).unwrap(),
        Operation::Compensate(Box::new(ticket)),
    )
    .unwrap();
    assert!(matches!(join(&mut job), Ok(Output::Compensated)));
    let listener = peer.join().unwrap();
    let mut old = Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        Operation::Compensate(Box::new(stale)),
    )
    .unwrap();
    assert!(matches!(
        join(&mut old),
        Err(JobError::Journal(delivery::journal::Error::Changed))
    ));
    no_request(&listener);
    drop(old);
    drop((job, store));
    assert!(Store::open(&directory.0)
        .unwrap()
        .pending()
        .unwrap()
        .is_empty());
    no_request(&listener);
}
#[test]
fn offline_receiver_lost_accept_reply_reopens_original_route_before_authenticated_compensation() {
    lost_reply_routed(true)
}
#[test]
fn shared_store_allows_distinct_ready_owners_but_same_key_waits_until_owner_drop() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let (listener, client, tls) = fixture();
    let origin = client.origin().to_owned();
    let response = png::response(&origin);
    let peer = std::thread::spawn(move || {
        for _ in 0..3 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/projections/accept "));
            let mut response = response.clone();
            response["envelope"] = body["envelope"].clone();
            response["receiverUnitId"] = body["receiverUnitId"].clone();
            reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
        }
    });
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let mut results = Vec::new();
    for index in 1..=2 {
        let mut wire = delivery::tests::invitation(&origin, false);
        wire["envelope"]["projectionId"] = format!("projection:{index:032x}").into();
        // 第一项沿共用夹具的 111... ID，第二项是另一独立 projection key。
        if index == 1 {
            wire["envelope"] = serde_json::to_value(png::invitation(&origin).envelope()).unwrap();
        }
        let invitation = Inbox::parse(
            &serde_json::to_vec(&serde_json::json!({"invitations":[wire]})).unwrap(),
            &origin,
        )
        .unwrap()
        .into_entries()
        .pop()
        .unwrap();
        let mut job = Job::start_prepared(
            client.clone(),
            grant.lease(&origin, 0).unwrap(),
            Operation::Accept {
                invitation: Box::new(invitation),
                journal: store.clone(),
            },
        )
        .unwrap();
        let Output::Received(ready) = join(&mut job).unwrap() else {
            panic!("missing ready")
        };
        results.push(ready);
    }
    assert_eq!(store.pending().unwrap().len(), 2);
    assert_eq!(results.len(), 2);
    let mut busy = Job::start_prepared(
        client.clone(),
        grant.lease(&origin, 0).unwrap(),
        operation(&origin, &store),
    )
    .unwrap();
    assert!(matches!(
        join(&mut busy),
        Err(JobError::Journal(delivery::journal::Error::Busy))
    ));
    drop((busy, results));
    let mut retry = Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        operation(&origin, &store),
    )
    .unwrap();
    assert!(matches!(join(&mut retry), Ok(Output::Received(_))));
    assert_eq!(store.pending().unwrap().len(), 2);
    peer.join().unwrap();
}
#[test]
fn denial_false_missing_duplicate_or_malformed_unlink_receipts_never_retire() {
    for (status, body) in [
        (403, br#"{"unlinked":true}"#.as_slice()),
        (500, br#"{"unlinked":true}"#),
        (200, br#"{"unlinked":false}"#),
        (200, br#"{}"#),
        (200, br#"{"unlinked":true,"unlinked":true}"#),
        (200, b"broken"),
        (200, br#"{"unlinked":"true"}"#),
    ] {
        let directory = Directory::new();
        let store = Store::open(&directory.0).unwrap();
        let (listener, client, tls) = fixture();
        let origin = client.origin().to_owned();
        let ticket = store.begin(&png::invitation(&origin), "device-1").unwrap();
        let original = std::fs::read(directory.path()).unwrap();
        let body = body.to_vec();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("POST /v1/projections/unlink "));
            reply(&mut stream, status, &body);
            drop(stream);
            listener
        });
        let mut grant = Grant::new(session(&origin, u64::MAX));
        let mut job = Job::start_prepared(
            client,
            grant.lease(&origin, 0).unwrap(),
            Operation::Compensate(Box::new(ticket)),
        )
        .unwrap();
        assert!(join(&mut job).is_err());
        assert_eq!(std::fs::read(directory.path()).unwrap(), original);
        assert_eq!(store.pending().unwrap().len(), 1);
        no_request(&peer.join().unwrap());
    }
}
#[test]
fn wrong_device_cross_origin_revoked_or_expired_authorization_sends_nothing_and_keeps_intent() {
    for case in 0..4 {
        let directory = Directory::new();
        let store = Store::open(&directory.0).unwrap();
        let (listener, client, _) = fixture();
        let origin = client.origin().to_owned();
        let entry_origin = if case == 1 {
            "https://other.example"
        } else {
            &origin
        };
        let ticket = store
            .begin(
                &png::invitation(entry_origin),
                if case == 0 {
                    "other-device"
                } else {
                    "device-1"
                },
            )
            .unwrap();
        let original = std::fs::read(directory.path()).unwrap();
        let mut grant = Grant::new(session(&origin, if case == 3 { 1 } else { u64::MAX }));
        let lease = grant.lease(&origin, 0).unwrap();
        if case == 2 {
            lease.revoke();
        }
        let mut job =
            Job::start_prepared(client, lease, Operation::Compensate(Box::new(ticket))).unwrap();
        assert!(join(&mut job).is_err());
        no_request(&listener);
        assert_eq!(std::fs::read(directory.path()).unwrap(), original);
    }
}
#[test]
fn preflight_origin_or_lease_failure_does_not_even_write_an_accept_intent() {
    for case in 0..4 {
        let directory = Directory::new();
        let store = Store::open(&directory.0).unwrap();
        let (listener, client, _) = fixture();
        let origin = client.origin().to_owned();
        let credential_origin = if case == 2 {
            "https://other.example"
        } else {
            &origin
        };
        let mut grant = Grant::new(session(
            credential_origin,
            if case == 3 { 1 } else { u64::MAX },
        ));
        let lease = grant.lease(credential_origin, 0).unwrap();
        if case == 1 {
            lease.revoke();
        }
        let invitation_origin = if case == 0 {
            "https://other.example"
        } else {
            &origin
        };
        let mut job =
            Job::start_prepared(client, lease, operation(invitation_origin, &store)).unwrap();
        assert!(join(&mut job).is_err());
        assert!(store.pending().unwrap().is_empty());
        assert!(!directory.path().exists());
        no_request(&listener);
    }
}
#[test]
fn changed_or_unwritable_journal_blocks_accept_without_network_or_overwrite() {
    for case in 0..3 {
        let directory = Directory::new();
        let store = Store::open(&directory.0).unwrap();
        let (listener, client, _) = fixture();
        let origin = client.origin().to_owned();
        if case == 0 {
            std::fs::write(directory.path(), b"external writer").unwrap();
        } else {
            store.begin(&png::invitation(&origin), "device-1").unwrap();
            if case == 1 {
                std::fs::write(directory.path(), b"external writer").unwrap();
            } else {
                std::fs::remove_file(directory.path()).unwrap();
                std::fs::create_dir(directory.path()).unwrap();
            }
        }
        let mut grant = Grant::new(session(&origin, u64::MAX));
        let mut job = Job::start_prepared(
            client,
            grant.lease(&origin, 0).unwrap(),
            operation(&origin, &store),
        )
        .unwrap();
        assert!(matches!(join(&mut job), Err(JobError::Journal(_))));
        no_request(&listener);
        if case < 2 {
            assert_eq!(std::fs::read(directory.path()).unwrap(), b"external writer");
        } else {
            assert!(directory.path().is_dir());
        }
    }
}
#[test]
fn cancel_after_accept_or_unlink_request_keeps_the_exact_record_for_explicit_retry() {
    for accepting in [false, true] {
        let directory = Directory::new();
        let store = Store::open(&directory.0).unwrap();
        let (listener, client, tls) = fixture();
        let origin = client.origin().to_owned();
        let ticket = store.begin(&png::invitation(&origin), "device-1").unwrap();
        let original = std::fs::read(directory.path()).unwrap();
        let (sent, received) = std::sync::mpsc::sync_channel(1);
        let (release, released) = std::sync::mpsc::sync_channel(1);
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
            let (head, _) = request(&mut stream);
            let route = if accepting {
                "POST /v1/projections/accept "
            } else {
                "POST /v1/projections/unlink "
            };
            assert!(head.starts_with(route));
            sent.send(()).unwrap();
            released.recv_timeout(Duration::from_secs(5)).unwrap();
            drop(stream);
        });
        let mut grant = Grant::new(session(&origin, u64::MAX));
        let mut job = Job::start_prepared(
            client,
            grant.lease(&origin, 0).unwrap(),
            if accepting {
                operation(&origin, &store)
            } else {
                Operation::Compensate(Box::new(ticket))
            },
        )
        .unwrap();
        received.recv_timeout(Duration::from_secs(5)).unwrap();
        job.request_stop();
        assert!(join(&mut job).is_err());
        release.send(()).unwrap();
        peer.join().unwrap();
        assert_eq!(std::fs::read(directory.path()).unwrap(), original);
        assert_eq!(store.pending().unwrap().len(), 1);
    }
}
#[test]
fn full_journal_blocks_accept_and_invalid_document_cannot_supply_a_store() {
    let directory = Directory::new();
    let (listener, client, _) = fixture();
    let origin = client.origin().to_owned();
    std::fs::write(directory.path(), b"broken").unwrap();
    assert!(Store::open(&directory.0).is_err());
    no_request(&listener);
    // 构造合法满队列，不进行 128 次磁盘替换，也不放松 Record 的严格解码。
    let seed_directory = Directory::new();
    let seed = Store::open(&seed_directory.0).unwrap();
    seed.begin(&png::invitation(&origin), "device-1").unwrap();
    let doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(seed_directory.path()).unwrap()).unwrap();
    let entries: Vec<_> = (2..2 + delivery::journal::MAX_ENTRIES)
        .map(|index| {
            let mut entry = doc["entries"][0].clone();
            entry["intentId"] = format!("00000000-0000-0000-0000-{index:012x}").into();
            entry["envelope"]["projectionId"] = format!("projection:{index:032x}").into();
            entry["unit"] = format!("delivery-{index:032x}").into();
            entry
        })
        .collect();
    let bytes = serde_json::to_vec(&serde_json::json!({"version":1,"entries":entries})).unwrap();
    std::fs::write(directory.path(), &bytes).unwrap();
    let store = Store::open(&directory.0).unwrap();
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let mut job = Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        operation(&origin, &store),
    )
    .unwrap();
    assert!(matches!(
        join(&mut job),
        Err(JobError::Journal(delivery::journal::Error::Limit))
    ));
    no_request(&listener);
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
}
#[test]
fn confirmed_unlink_cannot_overwrite_an_external_journal_change() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let (listener, client, tls) = fixture();
    let origin = client.origin().to_owned();
    let ticket = store.begin(&png::invitation(&origin), "device-1").unwrap();
    let path = directory.path();
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        request(&mut stream);
        std::fs::write(path, b"external writer").unwrap();
        reply(&mut stream, 200, br#"{"unlinked":true}"#);
    });
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let mut job = Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        Operation::Compensate(Box::new(ticket)),
    )
    .unwrap();
    assert!(matches!(
        join(&mut job),
        Err(JobError::Journal(delivery::journal::Error::Changed))
    ));
    peer.join().unwrap();
    assert_eq!(std::fs::read(directory.path()).unwrap(), b"external writer");
}
