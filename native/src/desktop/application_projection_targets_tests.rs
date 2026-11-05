//! 原生菜单→共享授权 owner→后台固定 TLS→typed 目录；不需要可保存的工作区。
use super::*;
use crate::desktop::application::projection;
use crate::desktop::device_authorization_ui::targets as ui_targets;
use crate::projection_delivery::targets::{tests::body, Directory};
fn run(mode: u8) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let before = std::fs::read(files.root().join("device-identity.json")).unwrap();
    let (listener, client, tls, pem) = fixture_with_pem();
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let server =
        std::thread::spawn(move || {
            for index in 0..3 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, request_body) = request(&mut stream);
                match index {
                    0 => {
                        assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                        reply(
                            &mut stream,
                            200,
                            br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#,
                        );
                    }
                    1 => {
                        assert!(head.starts_with("POST /v1/device-sessions "));
                        let now = crate::desktop::surface_windows::now_ms().unwrap();
                        reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({
                        "deviceId":"device-old","token":"targets-token","expiresAtMs":now+60_000
                    })).unwrap());
                    }
                    _ => {
                        assert!(head.starts_with("POST /v1/projections/targets HTTP/1.1"));
                        assert!(head
                            .to_ascii_lowercase()
                            .contains("authorization: device targets-token\r\n"));
                        assert_eq!(request_body, serde_json::json!({}));
                        ready_tx.send(()).unwrap();
                        if mode == 4 {
                            let mut byte = [0];
                            assert!(!stream.read(&mut byte).is_ok_and(|count| count > 0));
                        } else {
                            let mut value = body();
                            if mode == 5 {
                                value["groups"][0]["targetIds"] = serde_json::json!(["missing"]);
                            }
                            reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
                        }
                    }
                }
            }
            listener
        });
    unsafe {
        let (_class, mut app) = application();
        // Application::new 会合成空白会话；本用例显式移除，验证目录不依赖保存目标。
        app.restored_session = None;
        assert!(app.restored_session.is_none());
        assert!(projection::receive::Scope::capture(&mut app).is_err());
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
        app.process_authorization();
        let second = app.bind_window_state(State::new(None, false).unwrap());
        app.authorization.borrow_mut().targets.directory = Some(
            Directory::parse(&serde_json::to_vec(&body()).unwrap(), &origin)
                .unwrap()
                .into(),
        );
        authorization_ui::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            ui_targets::REFRESH,
        )
        .unwrap();
        app.process_authorization_with_projection(|o| config(o, files.root()));
        assert!(second.authorization.borrow().targets.busy);
        assert!(second.authorization.borrow().targets.directory.is_none());
        assert!(!app.projection_receive.busy());
        assert!(!app.authorization.borrow().projection.busy);
        if mode == 4 {
            ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            app.shutdown();
        } else {
            wait_remote(&app);
            match mode {
                1 => {
                    app.authorization.borrow_mut().request =
                        Some(Request::Select("https://changed.example".into()))
                }
                2 => {
                    app.authorization_owner.invalidate(&origin).unwrap();
                }
                3 => app.authorization.borrow_mut().request = Some(Request::Cancel),
                _ => {}
            }
            app.process_authorization_with_projection(|_| {
                panic!("poll must not prepare a new client")
            });
            assert!(!app.remote_sessions.projection_pending());
            assert!(!app.authorization.borrow().targets.busy);
            if mode == 0 {
                let view = second.authorization.borrow();
                assert_eq!(
                    view.targets.directory.as_ref().unwrap().targets()[0].id(),
                    "receiver-one"
                );
                assert_eq!(view.targets.status, "投送目标：1 项 · 离线目录不完整");
                assert!(view.consent.paused());
                assert!(view.projection.entries.is_empty());
            } else {
                assert!(second.authorization.borrow().targets.directory.is_none());
                if mode == 1 {
                    app.authorization.borrow_mut().request = Some(Request::Select(origin.clone()));
                    app.process_authorization();
                    assert!(second.authorization.borrow().targets.directory.is_none());
                }
                if mode == 5 {
                    assert!(second
                        .authorization
                        .borrow()
                        .targets
                        .status
                        .starts_with("目标目录查询失败："));
                }
            }
            app.shutdown();
        }
        assert!(app.remote_sessions.finished());
        assert_eq!(app.remote_sessions.wait_handles().count(), 0);
        assert!(second.authorization.borrow().targets.directory.is_none());
        assert!(!second.authorization.borrow().targets.busy);
        assert!(app
            .documents
            .iter()
            .all(|doc| !IsWindowVisible(doc.window.0).as_bool()));
    }
    assert!(
        matches!(server.join().unwrap().accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
    );
    assert_eq!(
        std::fs::read(files.root().join("device-identity.json")).unwrap(),
        before
    );
    for name in [
        "workspace.json",
        "projection-receive.json",
        "projection-receive.lock",
        "projection-receive-policy.json",
    ] {
        assert!(!files.root().join(name).exists(), "{name}");
    }
}
#[test]
fn projection_targets_native_menu_works_paused_without_a_savable_workspace() {
    run(0);
}
#[test]
fn projection_targets_native_away_and_back_discards_completed_old_directory() {
    run(1);
}
#[test]
fn projection_targets_native_revocation_discards_completed_directory() {
    run(2);
}
#[test]
fn projection_targets_native_cancel_discards_completed_directory() {
    run(3);
}
#[test]
fn projection_targets_native_shutdown_drains_pending_network() {
    run(4);
}
#[test]
fn projection_targets_native_malformed_groups_do_not_restore_stale_directory() {
    run(5);
}
#[test]
fn projection_targets_native_missing_grant_and_busy_owner_do_not_prepare_network() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let (_class, mut app) = application();
        app.authorization.borrow_mut().origin = Some("https://loom.example".into());
        app.authorization.borrow_mut().request = Some(Request::ProjectionTargets);
        app.process_authorization_with_projection(|_| {
            panic!("missing grant must not prepare network")
        });
        assert!(!app.remote_sessions.projection_pending());
        assert!(app
            .authorization
            .borrow()
            .targets
            .status
            .starts_with("请先获取有效授权："));
        app.authorization.borrow_mut().targets.busy = true;
        projection::targets::start(
            &mut app.authorization.borrow_mut(),
            &mut app.authorization_owner,
            &mut app.remote_sessions,
            &mut |_| panic!("busy must not prepare network"),
        );
        assert!(app.authorization.borrow().targets.busy);
        app.shutdown();
    }
}
