//! 实际认证目录与选中身份绑定；首次选择不要求既有成员，来源替换禁止 POST 加入。
use super::*;
use crate::live_relay_http::Join;
fn list(mut value: serde_json::Value) -> Vec<u8> {
    value["session"]["viewerDevices"] = serde_json::json!([]);
    serde_json::to_vec(&serde_json::json!({"protocolVersion":"loom.live.v1","sessions":[value]}))
        .unwrap()
}
#[test]
fn catalog_selection_allows_explicit_first_join_without_weakening_identity() {
    let (listener, client, config) = fixture();
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        for index in 0..4 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            match index {
                0 | 1 => {
                    assert!(head.starts_with("GET /v1/live/sessions HTTP/1.1"));
                    reply(
                        &mut stream,
                        200,
                        &list(serde_json::from_slice(&snapshot()).unwrap()),
                    );
                }
                2 => {
                    assert!(head.starts_with("POST /v1/live/sessions/live:test%2Fpart/viewers "));
                    assert_eq!(body["envelope"]["sequence"], 1);
                    assert!(body.get("requireExistingMembership").is_none());
                    reply(&mut stream, 200, &snapshot());
                }
                _ => {
                    assert!(head.contains("after=0&timeoutMs=0"));
                    reply(&mut stream, 200, &history(true));
                }
            }
        }
    });
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    let entries = control.catalog().unwrap();
    assert_eq!(entries.len(), 1);
    assert!(!entries[0].member);
    let selected = &entries[0].identity;
    let prepared = control
        .prepare_selected_viewer(
            Join {
                session: selected.session(),
                surface: "surface-1",
                attachment: "attachment-1",
                expected: None,
            },
            selected,
        )
        .unwrap();
    assert_eq!(prepared.snapshot.identity(), selected);
    assert_eq!(prepared.cursor.after(), 6);
    peer.join().unwrap();
}
#[test]
fn selected_source_or_epoch_replacement_fails_before_remote_join() {
    for changed in ["sourceDeviceId", "sourceHookId", "epoch"] {
        let (listener, client, config) = fixture();
        let session = session(&client.origin, u64::MAX);
        let selected =
            crate::live_relay_viewer_identity::Snapshot::parse(&snapshot(), "live:test/part")
                .unwrap();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("GET /v1/live/sessions HTTP/1.1"));
            let mut value: serde_json::Value = serde_json::from_slice(&snapshot()).unwrap();
            if changed == "epoch" {
                value["epoch"] = serde_json::json!(2);
            } else {
                value["session"][changed] = serde_json::json!("other-source");
            }
            reply(&mut stream, 200, &list(value));
            listener
        });
        let cancel = Cancel::new();
        assert!(Control::new(&client, &session, &cancel)
            .prepare_selected_viewer(
                Join {
                    session: "live:test/part",
                    surface: "surface-1",
                    attachment: "attachment-1",
                    expected: None
                },
                selected.identity()
            )
            .is_err());
        let listener = peer.join().unwrap();
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
    }
}

#[test]
fn older_attach_revision_cannot_replace_the_fresh_discovery_state() {
    let (listener, client, config) = fixture();
    let session = session(&client.origin, u64::MAX);
    let selected =
        crate::live_relay_viewer_identity::Snapshot::parse(&snapshot(), "live:test/part").unwrap();
    let peer = std::thread::spawn(move || {
        for index in 0..2 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            if index == 0 {
                assert!(head.starts_with("GET /v1/live/sessions HTTP/1.1"));
                let mut value: serde_json::Value = serde_json::from_slice(&snapshot()).unwrap();
                value["session"]["revision"] = serde_json::json!(5);
                reply(&mut stream, 200, &list(value));
            } else {
                assert!(head.starts_with("POST "));
                reply(&mut stream, 200, &snapshot());
            }
        }
        listener
    });
    let cancel = Cancel::new();
    let result = Control::new(&client, &session, &cancel).prepare_selected_viewer(
        Join {
            session: "live:test/part",
            surface: "surface-1",
            attachment: "attachment-1",
            expected: None,
        },
        selected.identity(),
    );
    assert!(matches!(
        result,
        Err(LiveError::Protocol(
            crate::live_relay_viewer_identity::Error::Revision
        ))
    ));
    let listener = peer.join().unwrap();
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
}
