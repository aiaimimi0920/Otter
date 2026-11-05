//! 真实菜单→原授权/Job→TLS read/Create→后台PNG/绑定→canonical；覆盖原意图重试和晚取消。
use super::*;
use base64::Engine;
use serde_json::{json, Value};
use sha2::Digest;
use std::io::Write as _;
fn assert_snapshot(body: &Value, envelope: &Value, expected: &Image, tolerance: u8) {
    let _com = Apartment::new().unwrap();
    let png = base64::engine::general_purpose::STANDARD
        .decode(body["snapshot"]["imageBase64"].as_str().unwrap())
        .unwrap();
    assert_eq!(body["snapshot"]["width"], expected.width);
    assert_eq!(body["snapshot"]["height"], expected.height);
    assert_eq!(
        format!("{:x}", sha2::Sha256::digest(&png)),
        envelope["content"]["digest"]
    );
    // 仅隔离测试显式开启证据输出；不记录token、真实profile或整份网络envelope。
    let evidence = std::env::var_os("OTTER_PNG_EVIDENCE_DIR").map(|root| {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        let test = std::thread::current()
            .name()
            .unwrap_or("unnamed")
            .to_owned();
        let name = format!("{:x}", sha2::Sha256::digest(test.as_bytes()));
        let file = root.join(&name[..16]);
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(file.with_extension("png"))
            .unwrap()
            .write_all(&png)
            .unwrap();
        (file, test)
    });
    let decoded = crate::desktop::export::decode_png(png);
    assert_eq!(
        (decoded.width, decoded.height),
        (expected.width, expected.height)
    );
    // PBGRA→PNG BGRA→解码有整数舍入。独立 raw WIC oracle 不执行选图、裁剪或合成；
    // 全图逐通道比较；仅独立高斯/wash用例有显式2级舍入容差，其余保持精确。
    let expected_png = crate::desktop::export::png_raw(expected, &AtomicBool::new(false)).unwrap();
    let expected = crate::desktop::export::decode_png(expected_png.to_vec());
    let difference = decoded
        .pixels
        .iter()
        .zip(expected.pixels.iter())
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    assert!(
        difference <= tolerance,
        "PNG pixel mismatch: {:?}",
        decoded
            .pixels
            .iter()
            .zip(expected.pixels.iter())
            .enumerate()
            .find(|(_, (a, b))| a.abs_diff(**b) > tolerance)
    );
    if let Some((file, test)) = evidence {
        let metadata = serde_json::to_vec_pretty(&json!({
            "test":test,"digest":envelope["content"]["digest"],
            "metadataWidth":body["snapshot"]["width"],"metadataHeight":body["snapshot"]["height"],
            "pngWidth":decoded.width,"pngHeight":decoded.height,"independentPixelsMatch":true,
            "pixelTolerance":tolerance,"maxPixelDifference":difference
        }))
        .unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(file.with_extension("json"))
            .unwrap()
            .write_all(&metadata)
            .unwrap();
    }
}
fn run(mode: u8) {
    run_source(mode, source::Case::Current);
}
pub(super) fn run_source(mode: u8, case: source::Case) {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let identity_before = std::fs::read(files.root().join("device-identity.json")).unwrap();
    let (listener, client, tls, pem) = fixture_with_pem();
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let outbox = files.root().join("projection-sender.json");
    let peer_outbox = outbox.clone();
    let tolerance = case.pixel_tolerance();
    let (expected_tx, expected_rx) = std::sync::mpsc::sync_channel(1);
    // 子线程沿用外层测试名，避免所有接收端都叫unnamed而覆盖其他case的PNG证据。
    let test_name = std::thread::current()
        .name()
        .unwrap_or("unnamed")
        .to_owned();
    let server = std::thread::Builder::new().name(test_name).spawn(move || {
        let expected = expected_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let mut original: Option<Value> = None;
        let count = if mode == 3 || mode >= 6 { 5 } else { 4 };
        for step in 0..count {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            if step == 0 {
                assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                reply(
                    &mut stream,
                    200,
                    br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#,
                );
            } else if step == 1 {
                assert!(head.starts_with("POST /v1/device-sessions "));
                let now = now().unwrap();
                reply(&mut stream, 200, &serde_json::to_vec(&json!({"deviceId":"device-old","token":"sender-token","expiresAtMs":now+60_000})).unwrap());
            } else {
                assert!(head
                    .to_ascii_lowercase()
                    .contains("authorization: device sender-token\r\n"));
                let journal: Value =
                    serde_json::from_slice(&std::fs::read(&peer_outbox).unwrap()).unwrap();
                let envelope = journal["entries"][0]["envelope"].clone();
                if let Some(original) = &original {
                    assert_eq!(&envelope, original);
                } else {
                    original = Some(envelope.clone());
                }
                if step == 2 {
                    assert!(head.starts_with("POST /v1/projections/read "));
                    assert_eq!(body["projectionId"], envelope["projectionId"]);
                    if mode == 2 {
                        reply(&mut stream, 503, b"temporary");
                    } else {
                        reply(
                            &mut stream,
                            404,
                            br#"{"error":{"code":"projection_not_found"}}"#,
                        );
                    }
                } else if step == 4 {
                    assert!(head.starts_with("POST /v1/projections/unlink "));
                    assert_eq!(body["projectionId"], envelope["projectionId"]);
                    if mode >= 6 {
                        assert_eq!(journal["entries"][0]["stop"], "requested");
                        let saved: Value = serde_json::from_slice(
                            &std::fs::read(peer_outbox.parent().unwrap().join("workspace.json"))
                                .unwrap(),
                        )
                        .unwrap();
                        assert_eq!(
                            saved["stickers"][0]["projectionSenders"][0]["link"]["stopPending"],
                            true,
                            "network unlink must follow canonical stopPending commit"
                        );
                    }
                    reply(&mut stream, 200, br#"{"unlinked":true}"#);
                } else {
                    if mode == 2 {
                        assert!(head.starts_with("POST /v1/projections/read "));
                    } else {
                        assert!(head.starts_with("POST /v1/projections/create "));
                        assert_eq!(body["envelope"], envelope);
                        assert_eq!(body["targetDeviceId"], "receiver-one");
                        assert_snapshot(&body, &envelope, &expected, tolerance);
                    }
                    reply(&mut stream, 200, &serde_json::to_vec(&json!({"envelope":envelope,"revision":1,"digest":envelope["content"]["digest"],"linked":true,"receiverDeviceId":null,"receiverUnitId":null,"snapshot":null,"delivery":{"targetDeviceId":"receiver-one","status":"awaiting_confirmation"}})).unwrap());
                }
            }
        }
        listener
    }).unwrap();
    unsafe {
        let (_class, mut app) = application(files.root());
        expected_tx.send(case.install(&mut app)).unwrap();
        let unit = app.documents[0].state.unit_id.clone();
        let original = app.restored_session.as_ref().unwrap().original.clone();
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
        app.authorization.borrow_mut().targets.directory = Some(
            crate::projection_delivery::targets::Directory::parse(
                &serde_json::to_vec(&crate::projection_delivery::targets::tests::body()).unwrap(),
                &origin,
            )
            .unwrap()
            .into(),
        );
        ui::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            ui::sender::BASE,
        )
        .unwrap();
        app.process_authorization_with_projection(|o| config(o, files.root()));
        assert!(matches!(app.projection_sender.phase, Phase::Waiting(_)));
        wait_remote(&app);
        if mode == 1 {
            app.authorization.borrow_mut().request = Some(ui::Request::Cancel);
        }
        app.process_authorization_with_projection(|o| config(o, files.root()));
        if mode == 2 {
            assert!(matches!(app.projection_sender.phase, Phase::Retry(_)));
            let before = std::fs::read(&outbox).unwrap();
            assert!(
                !serde_json::from_slice::<Value>(&before).unwrap()["entries"][0]["cancelled"]
                    .as_bool()
                    .unwrap()
            );
            ui::execute(
                app.documents[0].window.0,
                &app.documents[0].state,
                ui::sender::RETRY,
            )
            .unwrap();
            app.process_authorization_with_projection(|o| config(o, files.root()));
            wait_remote(&app);
            app.process_authorization_with_projection(|o| config(o, files.root()));
            assert_eq!(
                std::fs::read(&outbox).unwrap(),
                before,
                "retry must retain exact signed bytes"
            );
        }
        if mode == 4 {
            assert!(matches!(app.projection_sender.phase, Phase::Checkpoint(_)));
            let before = std::fs::read(&outbox).unwrap();
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut app,
                Some(files.root().join("manual.json")),
            )
            .unwrap();
            app.process_authorization_with_projection(|o| config(o, files.root()));
            assert!(
                matches!(app.projection_sender.phase, Phase::Checkpoint(_)),
                "manual Save As is not canonical success"
            );
            assert_ne!(
                app.authorization.borrow().sender.status,
                "邀请已发送，等待接收回执"
            );
            app.stop_projection_sender();
            assert_eq!(
                std::fs::read(&outbox).unwrap(),
                before,
                "saved manual copy must not be implicitly unlinked"
            );
            assert!(!app.projection_sender.pending_local());
            app.shutdown();
        } else if mode == 0 || mode == 2 || mode >= 6 {
            assert!(matches!(app.projection_sender.phase, Phase::Checkpoint(_)));
            let expected = match &app.projection_sender.phase {
                Phase::Checkpoint(saved) => saved.expected.clone(),
                _ => unreachable!(),
            };
            assert!(!app.projection_sender_inclusion(&expected));
            settle(&mut app);
            app.process_authorization_with_projection(|o| config(o, files.root()));
            assert!(matches!(app.projection_sender.phase, Phase::Idle));
            assert!(app.projection_sender_inclusion(&expected));
            assert_eq!(
                app.authorization.borrow().sender.status,
                "邀请已发送，等待接收回执"
            );
            let before = std::fs::read(&outbox).unwrap();
            if mode >= 6 {
                stop::run(&mut app, files.root(), &expected, mode);
            } else {
                app.shutdown();
                drop(app);
                drain_messages();
                let instance = GetModuleHandleW(None).unwrap().into();
                let doc = DocumentWindow::create(instance, State::new(None, false).unwrap(), false)
                    .unwrap();
                let mut reopened = Application::new(instance, doc);
                reopened
                    .install_workspace_checkpoint(Some(Profile::open(files.root()).unwrap()), true)
                    .unwrap();
                assert!(reopened.projection_sender_inclusion(&expected));
                assert_eq!(
                    std::fs::read(&outbox).unwrap(),
                    before,
                    "stable shutdown/reopen must not unlink"
                );
                reopened.shutdown();
            }
        } else {
            if mode == 3 || mode == 5 {
                assert!(matches!(app.projection_sender.phase, Phase::Checkpoint(_)));
                app.stop_projection_sender();
            }
            let end = Instant::now() + Duration::from_secs(5);
            while app.projection_sender.pending_local()
                || (mode == 3 && !app.remote_sessions.projection_pending())
            {
                app.process_authorization_with_projection(|o| {
                    if mode == 5 {
                        Err(crate::device_authorization_config::Error::Trust)
                    } else {
                        config(o, files.root())
                    }
                });
                assert!(Instant::now() < end, "sender cleanup deadline");
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(app
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .same_storage(&original));
            if mode == 5 {
                assert!(!app.remote_sessions.projection_pending());
                assert!(app.projection_sender.deadline(now().unwrap()).unwrap() > Instant::now());
                app.process_authorization_with_projection(|_| {
                    panic!("backoff must not prepare another config before its deadline")
                });
            }
            if mode == 3 {
                wait_remote(&app);
                app.authorization.borrow_mut().request = Some(ui::Request::Cancel);
                app.process_authorization_with_projection(|o| config(o, files.root()));
                assert!(app.projection_sender.cleanup.network.is_none());
                assert_eq!(app.projection_sender.profile.entries().unwrap().len(), 0);
                assert_eq!(
                    serde_json::from_slice::<Value>(&std::fs::read(&outbox).unwrap()).unwrap()
                        ["entries"],
                    json!([])
                );
            } else {
                assert!(
                    serde_json::from_slice::<Value>(&std::fs::read(&outbox).unwrap()).unwrap()
                        ["entries"][0]["cancelled"]
                        .as_bool()
                        .unwrap()
                );
            }
            assert!(app.workspace.contains(&unit));
            app.shutdown();
        }
    }
    assert!(
        matches!(server.join().unwrap().accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
    assert_eq!(
        std::fs::read(files.root().join("device-identity.json")).unwrap(),
        identity_before
    );
}
#[test]
fn native_sender_menu_create_checkpoint_and_restart_keep_stable_binding() {
    run(0);
}
#[test]
fn native_sender_completed_send_cancel_before_consume_keeps_exact_cancel_intent() {
    run(1);
}
#[test]
fn native_sender_explicit_retry_reuses_original_without_resign_or_second_create() {
    run(2);
}
#[test]
fn native_sender_successful_unlink_then_discard_reconciles_real_retirement() {
    run(3);
}
#[test]
fn native_sender_manual_save_retains_binding_without_claiming_canonical_success() {
    run(4);
}
#[test]
fn native_sender_cleanup_config_failure_keeps_retry_wake_without_immediate_spin() {
    run(5);
}
