//! 真实Live菜单→Application→原授权/后台TLS/WARP读回及完整回收；不接触用户桌面输入。
use super::*;
use crate::desktop::composition::live::{
    session::Session,
    window::{
        publication::{PUBLISH, STOP},
        LiveWindow,
    },
};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
#[path = "application_live_source_interaction_tests.rs"]
mod interaction;
#[path = "application_live_source_joint_tests.rs"]
mod joint;
#[path = "application_live_source_observation_tests.rs"]
mod observation;
#[path = "application_live_source_peer.rs"]
mod peer;
unsafe fn spin(
    app: &mut Application,
    root: &std::path::Path,
    ready: impl Fn(&Application) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.poll_remote_live_with(false, |origin| fixture::config(origin, root))
            .unwrap();
        if ready(app) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "source Application deadline: {}",
            app.authorization.borrow().live.status
        );
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
pub(super) fn capture(app: &mut Application) -> Arc<AtomicUsize> {
    capture_size(app, [2, 2])
}
pub(super) fn capture_size(app: &mut Application, size: [u32; 2]) -> Arc<AtomicUsize> {
    capture_kind(app, size, false, None)
}
pub(super) fn capture_stream(app: &mut Application, color: Arc<AtomicUsize>) -> Arc<AtomicUsize> {
    capture_kind(app, [2, 2], true, Some(color))
}
fn capture_kind(
    app: &mut Application,
    size: [u32; 2],
    streaming: bool,
    color: Option<Arc<AtomicUsize>>,
) -> Arc<AtomicUsize> {
    let active = Arc::new(AtomicUsize::new(0));
    let (job, window) = crate::live_job::LiveJob::spawn_bound(
        &active,
        |consumer| LiveWindow::new(consumer, size[0], size[1]).map_err(std::io::Error::other),
        move |source, _| {
            let (send, wait) = std::sync::mpsc::sync_channel(1);
            source
                .on_idle(Arc::new(move || {
                    let _ = send.try_send(());
                }))
                .unwrap();
            source
                .publish(crate::live_relay_source_runtime::test_texture::texture(
                    size,
                    [17, 83, 211, 255],
                ))
                .unwrap();
            if streaming {
                let deadline = Instant::now() + Duration::from_secs(15);
                loop {
                    match wait.recv_timeout(Duration::from_millis(100)) {
                        Ok(()) => break,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            assert!(Instant::now() < deadline);
                            // 最后Consumer可在超时返回后退出；该次publish的关闭结果是正常退休。
                            match source.publish(
                                crate::live_relay_source_runtime::test_texture::texture(
                                    size,
                                    if color
                                        .as_ref()
                                        .is_some_and(|c| c.load(Ordering::Acquire) == 1)
                                    {
                                        [193, 29, 71, 255]
                                    } else {
                                        [17, 83, 211, 255]
                                    },
                                ),
                            ) {
                                Ok(_) => {}
                                Err(
                                    crate::live_frames::Error::Closed
                                    | crate::live_frames::Error::NoConsumers,
                                ) => break,
                                Err(error) => panic!("Tile Source capture publish: {error:?}"),
                            }
                        }
                        Err(error) => panic!("Tile Source capture owner: {error}"),
                    }
                }
            } else {
                wait.recv_timeout(Duration::from_secs(10)).unwrap();
            }
            Ok(())
        },
    )
    .unwrap();
    let session = Session::new(job, window);
    app.admit_live(session, false).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.live_sessions[0].poll();
        if app.live_sessions[0].snapshot_frame().unwrap().is_some() {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    active
}
fn run(mode: u8) {
    let _com = Apartment::new().unwrap();
    let (files, store, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    let origin = format!(
        "https://127.0.0.1:{}",
        listener.local_addr().unwrap().port()
    );
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let mut peer = peer::Peer::start(listener, tls, mode == 4, mode == 3);
    unsafe {
        let (_class, mut app) = fixture::application();
        app.authorization.borrow_mut().origin = Some(origin.clone());
        app.authorization_owner
            .start(crate::device_authorization_runtime::Task::Renew { client, store })
            .unwrap();
        for handle in app.authorization_owner.wait_handles() {
            assert_eq!(
                windows::Win32::System::Threading::WaitForSingleObject(
                    HANDLE(handle.as_raw_handle()),
                    5000
                ),
                WAIT_OBJECT_0
            );
        }
        assert!(matches!(
            app.authorization_owner.poll().as_slice(),
            [crate::device_authorization_owner::Event::Authorized { .. }]
        ));
        let active = capture(&mut app);
        let source_id = app.live_sessions[0].input_identity();
        let hwnd = app.live_sessions[0].group_dialog_owner().unwrap();
        SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(PUBLISH)), None);
        spin(&mut app, files.root(), |_| {
            peer.created.load(Ordering::Acquire)
        });
        if mode == 4 {
            SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(STOP)), None);
            app.poll_live_sources_with(&mut |origin| fixture::config(origin, files.root()));
            peer.release.store(true, Ordering::Release);
        } else {
            spin(&mut app, files.root(), |app| {
                peer.received.load(Ordering::Acquire)
                    && app.remote_sessions.source_status(&source_id) == Some((true, false))
            });
            assert_eq!(app.remote_sessions.wait_handles().count(), 2);
            if mode == 6 {
                app.shutdown_live().unwrap();
                assert!(app.remote_sessions.finished());
                assert_eq!(app.remote_sessions.wait_handles().count(), 0);
                assert_eq!(active.load(Ordering::Acquire), 0);
                peer.finish();
                return;
            }
            // 重复发布不重建source或第二套捕获；旧identity不能停止当前owner。
            assert!(!app.remote_sessions.stop_source(&Arc::new(())));
            SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(PUBLISH)), None);
            match mode {
                1 => {
                    app.authorization_owner.invalidate(&origin).unwrap();
                }
                2 => {
                    app.workspace = crate::workspace::Workspace::default();
                }
                5 => {
                    app.live_sessions[0].stop();
                }
                _ => {
                    SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(STOP)), None);
                }
            }
        }
        spin(&mut app, files.root(), |app| {
            app.remote_sessions.source_status(&source_id).is_none()
        });
        assert_eq!(app.remote_sessions.wait_handles().count(), 0);
        if mode != 1 {
            assert!(peer.closed.load(Ordering::Acquire));
        }
        if mode == 4 {
            assert!(!peer.received.load(Ordering::Acquire));
        }
        if mode == 3 {
            assert!(app
                .authorization
                .borrow()
                .live
                .status
                .contains("Unconfirmed"));
        }
        if mode != 5 {
            assert!(app.live_sessions[0].is_alive());
            assert_eq!(active.load(Ordering::Acquire), 1);
        }
        peer.finish();
        app.shutdown_live().unwrap();
        assert_eq!(active.load(Ordering::Acquire), 0);
    }
}
#[test]
fn source_menu_publishes_exact_frame_and_explicit_stop_preserves_capture() {
    run(0);
}
#[test]
fn source_authorization_revocation_reclaims_media_without_new_lease() {
    run(1);
}
#[test]
fn source_workspace_change_stops_without_republishing() {
    run(2);
}
#[test]
fn source_close_failure_is_not_reported_as_confirmed() {
    run(3);
}
#[test]
fn source_stop_during_create_discards_late_ready_and_closes_original_session() {
    run(4);
}
#[test]
fn source_capture_close_reclaims_publication() {
    run(5);
}
#[test]
fn source_application_shutdown_drains_active_source_with_original_wait_budget() {
    run(6);
}
