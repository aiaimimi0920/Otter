//! 离线发送沿同份菜单/Job/PNG/CAS/Checkpoint/停止夹具，TLS验证只走本机Loom路由。
use super::*;
use serde_json::json;
use sha2::{Digest, Sha256};
pub(super) fn directory(
    origin: &str,
    available: bool,
) -> std::sync::Arc<crate::projection_delivery::targets::Directory> {
    let peer = format!("loom-{}", "b".repeat(64));
    let id = format!(
        "peer-target:{:x}",
        Sha256::digest(format!("{peer}\nreceiver-one"))
    );
    crate::projection_delivery::targets::Directory::parse(&serde_json::to_vec(&json!({
        "targets":[{"deviceId":id,"name":"Remote screen","policy":"confirm","route":"offline_peer",
        "peerId":peer,"peerName":"Loom B","remoteDeviceId":"receiver-one","deliveryAvailable":available,
        "transferProtocol":"loom.offline-transfer.v1","unavailableReason":"offline_peer_delivery_not_implemented"}],
        "groups":[{"groupId":"remote-group","name":"Remote","targetIds":[id],"unavailableCount":0}]
    })).unwrap(), origin).unwrap().into()
}
#[test]
fn offline_sender_actual_menu_updates_crop_noop_and_restores_canonical_identity() {
    unsafe {
        updates::run_routed(updates::Mode::Normal, true);
    }
}
#[test]
fn offline_sender_lost_update_reply_reconciles_only_original_offline_route() {
    unsafe {
        updates::run_routed(updates::Mode::Lost, true);
    }
}
#[test]
fn offline_sender_wrong_peer_response_never_changes_canonical_or_initial_intent() {
    unsafe {
        updates::run_routed(updates::Mode::WrongPeer, true);
    }
}
#[test]
fn offline_sender_peer_revocation_does_not_create_shared_invitation_or_unlink() {
    unsafe {
        updates::run_routed(updates::Mode::PeerRevoked, true);
    }
}
#[test]
fn offline_sender_workspace_replacement_rejects_late_result_without_remote_cleanup() {
    unsafe {
        updates::run_routed(updates::Mode::Workspace, true);
    }
}
#[test]
fn offline_sender_menu_stop_interrupts_update_then_commits_original_unlink_and_ack() {
    unsafe {
        updates::run_routed(updates::Mode::DurableStop, true);
    }
}
#[test]
fn offline_sender_unavailable_directory_disables_single_and_group_menu_commands() {
    let mut view = ui::View {
        origin: Some("https://loom.example".into()),
        ..Default::default()
    };
    view.sender.ready = true;
    let unit = crate::workspace_id::UnitId::parse("unit-1").unwrap();
    for available in [false, true] {
        view.targets.directory = Some(directory("https://loom.example", available));
        assert_eq!(ui::sender::enabled(&view, &unit, 0), available);
        assert_eq!(ui::sender::groups::enabled(&view, &unit, 0), available);
    }
}

#[test]
fn offline_sender_mixed_group_same_named_devices_keep_routes_through_canonical_stop() {
    use serde_json::Value;
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let remote = directory(&origin, true);
    let remote_id = remote.targets()[0].id().to_owned();
    let mut catalog = json!({
        "targets":[], "groups":[{"groupId":"mixed","name":"Mixed","targetIds":[remote_id,"receiver-one"],"unavailableCount":0}]
    });
    let peer_id = format!("loom-{}", "b".repeat(64));
    catalog["targets"] = json!([
        {"deviceId":remote_id,"name":"Remote","policy":"confirm","route":"offline_peer","peerId":peer_id,
         "peerName":"B","remoteDeviceId":"receiver-one","deliveryAvailable":true,"transferProtocol":"loom.offline-transfer.v1"},
        {"deviceId":"receiver-one","name":"Local","policy":"confirm","route":"shared_loom"}]);
    let catalog = crate::projection_delivery::targets::Directory::parse(
        &serde_json::to_vec(&catalog).unwrap(),
        &origin,
    )
    .unwrap();
    let root = files.root().to_owned();
    let server = std::thread::spawn(move || {
        let mut sent = Vec::<Value>::new();
        let mut stopped = std::collections::BTreeSet::new();
        for step in 0..8 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            if step == 0 {
                reply(
                    &mut stream,
                    200,
                    br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#,
                );
            } else if step == 1 {
                reply(&mut stream,200,&serde_json::to_vec(&json!({"deviceId":"device-old","token":"mixed-token","expiresAtMs":now().unwrap()+60_000})).unwrap());
            } else {
                assert!(head
                    .to_ascii_lowercase()
                    .contains("authorization: device mixed-token\r\n"));
                if step < 6 {
                    let offline = step < 4;
                    let base = if offline {
                        "/v1/offline-projections/"
                    } else {
                        "/v1/projections/"
                    };
                    if step % 2 == 0 {
                        assert!(head.starts_with(&format!("POST {base}read ")));
                        reply(
                            &mut stream,
                            404,
                            br#"{"error":{"code":"projection_not_found"}}"#,
                        );
                    } else {
                        assert!(head.starts_with(&format!("POST {base}create ")));
                        assert_eq!(
                            body["targetDeviceId"],
                            if offline {
                                json!(remote_id)
                            } else {
                                json!("receiver-one")
                            }
                        );
                        assert_eq!(
                            body["peerId"],
                            if offline { json!(peer_id) } else { Value::Null }
                        );
                        let png = base64::Engine::decode(
                            &base64::engine::general_purpose::STANDARD,
                            body["snapshot"]["imageBase64"].as_str().unwrap(),
                        )
                        .unwrap();
                        assert_eq!(
                            format!("{:x}", Sha256::digest(&png)),
                            body["envelope"]["content"]["digest"]
                        );
                        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
                            .read_info()
                            .unwrap();
                        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
                        let info = reader.next_frame(&mut pixels).unwrap();
                        assert_eq!((info.width, info.height), (32, 32));
                        assert!(pixels[..info.buffer_size()]
                            .chunks_exact(4)
                            .all(|p| p == [0, 0, 255, 255]));
                        let mut response = json!({"envelope":body["envelope"],"revision":1,"digest":body["envelope"]["content"]["digest"],"linked":true,
                            "receiverDeviceId":null,"receiverUnitId":null,"delivery":{"targetDeviceId":body["targetDeviceId"],"status":"awaiting_confirmation"}});
                        if offline {
                            response["route"] = "offline_peer".into();
                            response["peerId"] = peer_id.clone().into();
                        }
                        reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
                        sent.push(body);
                    }
                } else {
                    let sent = sent
                        .iter()
                        .find(|s| s["envelope"]["projectionId"] == body["projectionId"])
                        .unwrap();
                    assert!(stopped.insert(body["projectionId"].to_string()));
                    let offline = sent["targetDeviceId"] == remote_id;
                    assert!(head.starts_with(if offline {
                        "POST /v1/offline-projections/unlink "
                    } else {
                        "POST /v1/projections/unlink "
                    }));
                    let saved: Value = serde_json::from_slice(
                        &std::fs::read(root.join("workspace.json")).unwrap(),
                    )
                    .unwrap();
                    assert!(saved["stickers"][0]["projectionSenders"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|b| b["link"]["envelope"] == sent["envelope"]
                            && b["link"]["stopPending"] == true));
                    reply(&mut stream, 200, br#"{"unlinked":true}"#);
                }
            }
        }
        assert_eq!(stopped.len(), 2);
        assert!(matches!(listener.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
    });
    unsafe {
        let (_class, mut app) = application(files.root());
        app.authorization.borrow_mut().origin = Some(origin.clone());
        app.authorization.borrow_mut().consent.install_paused(true);
        app.authorization_owner
            .start(Task::Renew {
                client,
                store: identity,
            })
            .unwrap();
        let handle = app.authorization_owner.wait_handles().next().unwrap();
        assert_eq!(
            WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
            WAIT_OBJECT_0
        );
        app.process_authorization_with_projection(|o| config(o, files.root()));
        app.authorization.borrow_mut().targets.directory = Some(catalog.into());
        ui::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            ui::sender::groups::BASE,
        )
        .unwrap();
        app.process_authorization_with_projection(|o| config(o, files.root()));
        for _ in 0..2 {
            wait_remote(&app);
            app.process_authorization_with_projection(|o| config(o, files.root()));
            assert!(matches!(app.projection_sender.phase, Phase::Checkpoint(_)));
            settle(&mut app);
            app.process_authorization_with_projection(|o| config(o, files.root()));
        }
        assert!(app.projection_sender.group.is_none());
        assert!(app.authorization.borrow().sender.group_results[0].contains("2 关联已保存"));
        ui::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            ui::sender::groups::STOP_BASE,
        )
        .unwrap();
        let end = Instant::now() + Duration::from_secs(10);
        loop {
            app.process_authorization_with_projection(|o| config(o, files.root()));
            settle(&mut app);
            if app.projection_sender.stop_group.is_none()
                && app.projection_sender.profile.entries().unwrap().is_empty()
            {
                break;
            }
            assert!(Instant::now() < end, "mixed stop deadline");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(app.authorization.borrow().sender.group_results[0].contains("2 已确认"));
        app.shutdown();
    }
    server.join().unwrap();
}
