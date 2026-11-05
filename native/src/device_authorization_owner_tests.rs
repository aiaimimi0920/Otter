//! 授权 owner 的真实文件/HTTPS 回归；验证旧结果抑制及按 origin 撤销。
use super::*;
use crate::device_authorization_owner::{Error as OwnerError, Event, Owner};
use crate::device_authorization_runtime::Task;
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};

fn wait(owner: &Owner) {
    for handle in owner.wait_handles() {
        assert_eq!(
            unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000) },
            WAIT_OBJECT_0
        );
    }
}
fn session_reply(listener: &TcpListener, config: &Arc<ServerConfig>, token: &str) {
    for index in 0..2 {
        let mut stream = StreamOwned::new(
            ServerConnection::new(config.clone()).unwrap(),
            accept(listener),
        );
        let (head, _) = request(&mut stream);
        if index == 0 {
            assert!(head.starts_with("POST /v1/device-sessions/challenges "));
            reply(
                &mut stream,
                200,
                br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#,
            );
        } else {
            assert!(head.starts_with("POST /v1/device-sessions "));
            reply(
                &mut stream,
                200,
                &serde_json::to_vec(&serde_json::json!({
                "deviceId":"device-old", "token":token, "expiresAtMs":u64::MAX}))
                .unwrap(),
            );
        }
    }
}

#[test]
fn invalidation_discards_completed_old_result_but_allows_new_task_same_origin() {
    let (files, store, _) = super::pairing::Files::new();
    let (listener, client, config) = fixture();
    let peer = std::thread::spawn(move || {
        session_reply(&listener, &config, "old-token");
        session_reply(&listener, &config, "new-token");
    });
    let mut owner = Owner::new();
    owner
        .start(Task::Renew {
            client: client.clone(),
            store,
        })
        .unwrap();
    assert_eq!(
        owner.start(Task::Renew {
            client: client.clone(),
            store: files.store()
        }),
        Err(OwnerError::Pending)
    );
    wait(&owner);
    owner.invalidate(&client.origin).unwrap();
    owner
        .start(Task::Renew {
            client: client.clone(),
            store: files.store(),
        })
        .unwrap();
    wait(&owner);
    let events = owner.poll();
    assert_eq!(events.len(), 1);
    assert!(
        matches!(&events[0], Event::Authorized { origin, device } if origin == &client.origin && device == "device-old")
    );
    assert_eq!(owner.token(&client.origin, 0), Some("new-token"));
    let lease = owner.lease(&client.origin, 0).unwrap();
    assert!(owner.finished());
    owner.close();
    assert!(lease.is_revoked());
    assert_eq!(owner.token(&client.origin, 0), None);
    assert_eq!(
        owner.start(Task::Renew {
            client,
            store: files.store()
        }),
        Err(OwnerError::Closed)
    );
    peer.join().unwrap();
}

#[test]
fn failed_pairing_keeps_old_credential_success_even_same_id_invalidates_it() {
    let (files, store, loaded) = super::pairing::Files::new();
    let public = loaded.identity().public_key().unwrap();
    let (listener, client, config) = fixture();
    let peer = std::thread::spawn(move || {
        session_reply(&listener, &config, "cached-token");
        for status in [403, 200] {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("POST /v1/devices/requests "));
            let body = serde_json::to_vec(
                &serde_json::json!({"pending":[{"id":"device-old", "publicKey":public}]}),
            )
            .unwrap();
            reply(&mut stream, status, &body);
        }
    });
    let mut owner = Owner::new();
    owner
        .start(Task::Renew {
            client: client.clone(),
            store,
        })
        .unwrap();
    wait(&owner);
    owner.poll();
    assert_eq!(owner.token(&client.origin, 0), Some("cached-token"));
    let lease = owner.lease(&client.origin, 0).unwrap();
    for success in [false, true] {
        owner
            .start(Task::RequestPairing {
                client: client.clone(),
                store: files.store(),
                name: "PC".into(),
            })
            .unwrap();
        wait(&owner);
        let events = owner.poll();
        assert_eq!(events.len(), 1);
        if success {
            assert!(lease.is_revoked());
            assert!(matches!(&events[0], Event::Pairing { changed: false, .. }));
            assert_eq!(owner.token(&client.origin, 0), None);
        } else {
            assert!(!lease.is_revoked());
            assert!(matches!(&events[0], Event::Failed { .. }));
            assert_eq!(owner.token(&client.origin, 0), Some("cached-token"));
        }
    }
    peer.join().unwrap();
}

#[test]
fn invalidate_one_origin_preserves_other_origin_and_expiry_is_checked() {
    let (files, _, _) = super::pairing::Files::new();
    let mut owner = Owner::new();
    let mut origins = Vec::new();
    for token in ["one", "two"] {
        let (listener, client, config) = fixture();
        origins.push(client.origin.clone());
        let peer = std::thread::spawn(move || session_reply(&listener, &config, token));
        owner
            .start(Task::Renew {
                client,
                store: files.store(),
            })
            .unwrap();
        wait(&owner);
        owner.poll();
        peer.join().unwrap();
    }
    let first = owner.lease(&origins[0], 0).unwrap();
    let second = owner.lease(&origins[1], 0).unwrap();
    owner.invalidate(&origins[0]).unwrap();
    assert!(first.is_revoked());
    assert!(!second.is_revoked());
    assert_eq!(owner.token(&origins[0], 0), None);
    assert_eq!(owner.token(&origins[1], 0), Some("two"));
    assert_eq!(owner.token(&origins[1], u64::MAX), None);
    assert_eq!(
        owner.invalidate("http://localhost"),
        Err(OwnerError::Origin)
    );
}
