//! 重启 journal→真实确认菜单→授权→HTTPS unlink→持久退休；失败/撤权/切换不伪报成功。
use super::*;
use crate::{
    desktop::{
        application::projection::profile::Owner as ProfileOwner, workspace_profile::Profile,
    },
    projection_delivery::{accept::tests::invitation, journal::Store},
};
fn seed(root: &std::path::Path, origin: &str, device: &str) -> Vec<u8> {
    let store = Store::open(root).unwrap();
    drop(store.begin(&invitation(origin), device).unwrap());
    std::fs::read(root.join("projection-receive.json")).unwrap()
}
fn settle_profile(app: &mut Application) {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        app.process_authorization();
        if !matches!(app.projection_profile, ProfileOwner::Opening(_)) {
            break;
        }
        assert!(Instant::now() < end, "recovery profile deadline");
        std::thread::yield_now();
    }
}
unsafe fn choose(app: &Application) {
    ui::recovery::confirm_with(
        app.documents[0].window.0,
        &app.documents[0].state,
        ui::recovery::BASE,
        |_| true,
    )
    .unwrap();
}
fn run_recovery(mode: u8) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let identity_before = std::fs::read(files.root().join("device-identity.json")).unwrap();
    let (listener, client, tls, pem) = fixture_with_pem();
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let bytes = seed(
        files.root(),
        &origin,
        if mode == 5 {
            "different-device"
        } else {
            "device-old"
        },
    );
    let (tx, rx) = mpsc::sync_channel(1);
    let server = std::thread::spawn(move || {
        for index in 0..if mode >= 5 {
            2
        } else if mode == 1 {
            4
        } else {
            3
        } {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
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
                    reply(&mut stream, 200, br#"{"deviceId":"device-old","token":"recovery-token","expiresAtMs":18446744073709551615}"#);
                }
                _ => {
                    assert!(head.starts_with("POST /v1/projections/unlink "));
                    assert!(head
                        .to_ascii_lowercase()
                        .contains("authorization: device recovery-token\r\n"));
                    assert_eq!(
                        body,
                        serde_json::json!({"projectionId":format!("projection:{}", "1".repeat(32))})
                    );
                    if mode == 4 {
                        tx.send(()).unwrap();
                        let mut byte = [0];
                        assert!(!stream.read(&mut byte).is_ok_and(|n| n != 0));
                    } else {
                        reply(
                            &mut stream,
                            200,
                            if mode == 1 && index == 2 {
                                br#"{"unlinked":false}"#
                            } else {
                                br#"{"unlinked":true}"#
                            },
                        );
                    }
                }
            }
        }
        listener
    });
    unsafe {
        let (_class, mut app) = application();
        app.add_state(
            State::new(
                Some(
                    crate::desktop::image::Image::from_encoded(
                        crate::desktop::application::smoke::image_fixture::PNG,
                    )
                    .unwrap(),
                ),
                false,
            )
            .unwrap(),
            false,
        )
        .unwrap();
        app.install_workspace_checkpoint(Some(Profile::open(files.root()).unwrap()), false)
            .unwrap();
        settle_profile(&mut app);
        assert!(app.authorization.borrow().projection.recovery.entries.len() == 1);
        let entry = app.authorization.borrow().projection.recovery.entries[0].clone();
        app.authorization.borrow_mut().origin = Some(origin.clone());
        app.defer_projection_poll_for_test(&origin);
        app.authorization_owner
            .start(Task::Renew {
                client,
                store: identity,
            })
            .unwrap();
        {
            let handle = app.authorization_owner.wait_handles().next().unwrap();
            assert_eq!(
                WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
                WAIT_OBJECT_0
            );
        }
        app.process_authorization();
        assert_eq!(
            std::fs::read(files.root().join("projection-receive.json")).unwrap(),
            bytes
        );
        assert!(!app.remote_sessions.projection_pending()); // 启动/授权均不自动联网解绑。
        let pixels = app.documents[1]
            .state
            .canvas
            .borrow()
            .image
            .as_ref()
            .unwrap()
            .pixels
            .clone();
        let local_unit = app.documents[1].state.unit_id.clone();
        if mode == 6 {
            let mut changed = bytes.clone();
            changed.push(b' ');
            std::fs::write(files.root().join("projection-receive.json"), changed).unwrap();
        }
        choose(&app);
        app.process_authorization_with_projection(|o| config(o, files.root()));
        assert!(app.remote_sessions.projection_pending());
        assert_eq!(app.remote_sessions.wait_handles().count(), 1);
        if mode == 4 {
            rx.recv_timeout(Duration::from_secs(5)).unwrap();
            app.shutdown();
            assert_eq!(
                std::fs::read(files.root().join("projection-receive.json")).unwrap(),
                bytes
            );
        } else {
            wait_remote(&app);
            match mode {
                2 => app.authorization.borrow_mut().request = Some(Request::Cancel),
                3 => {
                    app.authorization.borrow_mut().request =
                        Some(Request::Select("https://changed.example".into()))
                }
                _ => {}
            }
            app.process_authorization_with_projection(|_| {
                panic!("completion read client configuration")
            });
            assert!(!app.remote_sessions.projection_pending());
            let retained = mode == 1 || mode >= 5;
            assert_eq!(
                app.authorization.borrow().projection.recovery.entries.len(),
                usize::from(retained)
            );
            if retained {
                assert!(Arc::ptr_eq(
                    &entry,
                    &app.authorization.borrow().projection.recovery.entries[0]
                ));
                assert_ne!(
                    app.authorization.borrow().projection.status,
                    "投送解绑已确认"
                );
            } else if mode <= 1 {
                assert_eq!(
                    app.authorization.borrow().projection.status,
                    "投送解绑已确认"
                );
            }
            if mode == 1 {
                assert_eq!(
                    std::fs::read(files.root().join("projection-receive.json")).unwrap(),
                    bytes
                );
                choose(&app);
                app.process_authorization_with_projection(|o| config(o, files.root()));
                wait_remote(&app);
                app.process_authorization_with_projection(|_| {
                    panic!("retry completion read configuration")
                });
                assert!(app
                    .authorization
                    .borrow()
                    .projection
                    .recovery
                    .entries
                    .is_empty());
                assert_eq!(
                    app.authorization.borrow().projection.status,
                    "投送解绑已确认"
                );
            }
            assert_eq!(app.documents.len(), 2);
            assert_eq!(app.documents[1].state.unit_id, local_unit);
            assert!(app.documents[1]
                .state
                .canvas
                .borrow()
                .image
                .as_ref()
                .unwrap()
                .pixels
                .same_storage(&pixels));
            assert!(!IsWindowVisible(app.documents[0].window.0).as_bool());
            app.shutdown();
        }
        assert!(app.remote_sessions.finished());
        assert!(app.authorization.borrow().closed);
        drop(entry);
        let pending = Store::open(files.root()).unwrap().pending().unwrap();
        assert_eq!(pending.len(), usize::from(mode >= 4));
    }
    let listener = server.join().unwrap();
    assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    assert_eq!(
        std::fs::read(files.root().join("device-identity.json")).unwrap(),
        identity_before
    );
}
#[test]
fn restarted_intent_is_unlinked_only_after_explicit_confirmed_menu_and_authenticated_true_receipt()
{
    run_recovery(0);
}
#[test]
fn negative_receipt_keeps_exact_candidate_and_only_a_second_explicit_request_retries() {
    run_recovery(1);
}
#[test]
fn revoked_completed_compensation_is_reconciled_from_store_without_resurrecting_a_retired_candidate(
) {
    run_recovery(2);
}
#[test]
fn service_switch_discards_success_ui_but_reconciles_already_committed_retirement() {
    run_recovery(3);
}
#[test]
fn shutdown_drains_inflight_compensation_and_keeps_unconfirmed_durable_intent() {
    run_recovery(4);
}
#[test]
fn wrong_device_sends_no_unlink_and_preserves_the_recovered_intent() {
    run_recovery(5);
}
#[test]
fn externally_changed_journal_blocks_http_and_is_never_overwritten() {
    run_recovery(6);
}
#[test]
fn recovery_request_cannot_substitute_a_same_record_arc_not_owned_by_the_profile() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let directory = crate::projection_delivery::journal::tests::Directory::new();
    seed(&directory.0, "https://loom.example", "device-old");
    unsafe {
        let (_class, mut app) = application();
        app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), false)
            .unwrap();
        settle_profile(&mut app);
        app.authorization.borrow_mut().origin = Some("https://loom.example".into());
        let original = app.authorization.borrow().projection.recovery.entries[0].clone();
        let replacement = Arc::new(original.retry());
        app.authorization.borrow_mut().projection.recovery.entries = vec![replacement.clone()];
        app.authorization.borrow_mut().request =
            Some(Request::Projection(ui::Request::Compensate(replacement)));
        app.process_authorization_with_projection(|_| panic!("unowned record read configuration"));
        assert_eq!(
            app.authorization.borrow().projection.status,
            "未结投送选择已失效"
        );
        assert!(!app.remote_sessions.projection_pending());
        app.shutdown();
    }
}
