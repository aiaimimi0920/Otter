//! 隐藏 Application 的真实授权/TLS/menu/query/reject/lifecycle，复用原身份/CA/窗口夹具。
use super::*;
use crate::{
    desktop::device_authorization_ui::{self as authorization_ui, projection as ui},
    device_session_client::tests::{accept, fixture_with_pem, pairing::Files, reply, request},
};
use rustls::{ServerConnection, StreamOwned};
use std::{
    io::Read,
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};
#[path = "application_projection_inbox_guard_tests.rs"]
mod guard;
#[path = "application_projection_inbox_peer.rs"]
mod peer;
#[path = "application_projection_policy_reject_tests.rs"]
mod policy_reject;
#[path = "application_projection_receive_owner_tests.rs"]
mod receive_owner;
#[path = "application_projection_recovery_tests.rs"]
mod recovery;
#[path = "application_projection_targets_tests.rs"]
mod targets;
fn config(
    origin: &str,
    root: &std::path::Path,
) -> std::result::Result<Config, crate::device_authorization_config::Error> {
    Config::from_lookup(origin, |key| match key {
        "OTTER_NATIVE_PROFILE_DIR" => Some(root.as_os_str().to_owned()),
        "OTTER_LOOM_TLS_ORIGIN" => Some(origin.into()),
        "OTTER_LOOM_TLS_CA_FILE" => Some(root.join("ca.pem").into_os_string()),
        _ => None,
    })
}
fn wait_remote(app: &Application) {
    let handle = app.remote_sessions.wait_handles().next().unwrap();
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
}
fn run(mode: u8) {
    run_routed(mode, false)
}
fn run_routed(mode: u8, offline: bool) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let (files, store, _) = Files::new();
    let before = std::fs::read(files.root().join("device-identity.json")).unwrap();
    let (listener, client, tls, pem) = fixture_with_pem();
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let server_origin = origin.clone();
    let server = std::thread::spawn(move || {
        peer::serve_routed(listener, tls, server_origin, mode, ready_tx, offline)
    });
    unsafe {
        let (_class, mut app) = application();
        app.authorization.borrow_mut().origin = Some(origin.clone());
        app.authorization_owner
            .start(Task::Renew { client, store })
            .unwrap();
        {
            let handle = app.authorization_owner.wait_handles().next().unwrap();
            assert_eq!(
                WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
                WAIT_OBJECT_0
            );
        }
        app.process_authorization();
        assert!(!app.authorization.borrow().busy);
        let second = app.bind_window_state(State::new(None, false).unwrap());
        authorization_ui::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            ui::REFRESH,
        )
        .unwrap();
        app.process_authorization_with_projection(|o| config(o, files.root()));
        assert!(app.authorization.borrow().projection.busy);
        assert_eq!(app.remote_sessions.wait_handles().count(), 1);
        if mode == 5 {
            ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            app.shutdown();
            assert!(app.remote_sessions.finished());
            assert_eq!(app.remote_sessions.wait_handles().count(), 0);
            assert!(second.authorization.borrow().projection.entries.is_empty());
        } else {
            wait_remote(&app);
            match mode {
                2 => {
                    app.authorization.borrow_mut().request =
                        Some(Request::Select("https://changed.example".into()))
                }
                3 => {
                    let old = app.documents[0].window.0;
                    app.request_restore_input(
                        crate::desktop::session_job::Input::Bytes(
                            br#"{"documentSchemaVersion":1,"documentRevision":1,"otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1},"stickers":[],"links":[]}"#.to_vec(),
                        ),
                        None,
                        None,
                    )
                    .unwrap();
                    let end = Instant::now() + Duration::from_secs(10);
                    while app.restore_job.is_some() {
                        app.process_restore(false).unwrap();
                        assert!(Instant::now() < end);
                        std::thread::yield_now();
                    }
                    assert!(!IsWindow(Some(old)).as_bool());
                    assert_eq!(
                        app.authorization.borrow().origin.as_deref(),
                        Some(origin.as_str())
                    );
                }
                4 => {
                    app.authorization_owner.invalidate(&origin).unwrap();
                }
                _ => {}
            }
            app.process_authorization_with_projection(|_| panic!("poll must not prepare a client"));
            assert!(!app.remote_sessions.projection_pending());
            assert!(!app.authorization.borrow().projection.busy);
            if (2..=4).contains(&mode) {
                assert!(app.authorization.borrow().projection.entries.is_empty());
                if mode == 2 {
                    app.authorization.borrow_mut().request = Some(Request::Select(origin.clone()));
                    app.process_authorization();
                    assert!(app.authorization.borrow().projection.entries.is_empty());
                }
            } else {
                assert_eq!(second.authorization.borrow().projection.entries.len(), 1);
                let entry = app.authorization.borrow().projection.entries[0].clone();
                if offline {
                    assert_eq!(entry.origin(), origin);
                    assert_eq!(
                        entry.envelope().origin(),
                        crate::projection_delivery::tests::offline::SOURCE
                    );
                    assert_eq!(
                        entry.peer_id(),
                        Some(crate::projection_delivery::tests::offline::PEER)
                    );
                }
                authorization_ui::execute(
                    app.documents[0].window.0,
                    &app.documents[0].state,
                    ui::REJECT_BASE,
                )
                .unwrap();
                app.process_authorization_with_projection(|o| config(o, files.root()));
                wait_remote(&app);
                app.process_authorization_with_projection(|_| {
                    panic!("receipt poll must not prepare a client")
                });
                let view = app.authorization.borrow();
                assert_eq!(view.projection.entries.len(), usize::from(mode == 1));
                if mode == 0 {
                    assert_eq!(view.projection.status, "已拒绝投送");
                } else {
                    assert!(Arc::ptr_eq(&view.projection.entries[0], &entry));
                    assert!(view.projection.status.starts_with("投送操作失败："));
                }
            }
            app.shutdown();
        }
        assert!(app.authorization.borrow().closed);
        assert!(app.remote_sessions.finished());
    }
    let listener = server.join().unwrap();
    assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    assert_eq!(
        std::fs::read(files.root().join("device-identity.json")).unwrap(),
        before
    );
}
#[test]
fn desktop_menu_inbox_and_reject_wait_for_recorded_receipt_without_creating_identity() {
    run(0);
}
#[test]
fn failed_receipt_retains_exact_pending_candidate_for_explicit_retry() {
    run(1);
}
#[test]
fn selection_away_and_back_discards_completed_old_inbox_before_publication() {
    run(2);
}
#[test]
fn successful_same_origin_workspace_restore_discards_completed_old_inbox() {
    run(3);
}
#[test]
fn revoked_completed_inbox_never_publishes_metadata() {
    run(4);
}
#[test]
fn shutdown_cancels_pending_http_and_drains_original_wait_owner() {
    run(5);
}
#[test]
fn projection_offline_inbox_real_menu_refresh_and_reject_wait_for_original_recorded_receipt() {
    for mode in [0, 1] {
        run_routed(mode, true);
    }
}
#[test]
fn projection_offline_inbox_late_origin_workspace_revocation_and_shutdown_cannot_publish() {
    for mode in 2..6 {
        run_routed(mode, true);
    }
}
