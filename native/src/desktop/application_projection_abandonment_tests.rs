//! 原生产菜单/Sessions→取消或丢回复→原 cleanup owner；仅合成 profile/回环 TLS/隐藏 HWND。
use super::*;
use crate::desktop::device_authorization_ui::{projection as ui, Request};
use crate::projection_delivery::accept::tests as png;
use std::sync::mpsc;

fn poll(fixture: &mut Fixture) {
    fixture
        .app
        .process_authorization_with_projection(|origin| config(origin, fixture.files.root()));
}
unsafe fn start(fixture: &mut Fixture) {
    let entry = Arc::new(png::invitation(&fixture.origin));
    fixture.app.authorization.borrow_mut().projection.entries = vec![entry];
    let doc = &fixture.app.documents[0];
    ui::accept::confirm_with(doc.window.0, &doc.state, ui::ACCEPT_BASE, |_| true).unwrap();
    poll(fixture);
    assert!(fixture.app.remote_sessions.projection_pending());
}
fn marked(fixture: &mut Fixture) {
    let end = Instant::now() + Duration::from_secs(10);
    while fixture.app.projection_cleanup.pending_local()
        || fixture.app.remote_sessions.projection_pending()
    {
        poll(fixture);
        assert!(
            Instant::now() < end,
            "accept abandonment local mark deadline"
        );
        std::thread::yield_now();
    }
    assert!(fixture.store.pending().unwrap()[0].cleanup_pending());
    assert!(!fixture.unit_present());
}
pub(super) fn unlink_peer(fixture: &mut Fixture, expected: serde_json::Value, renew: bool) {
    let listener = fixture.listener.take().unwrap();
    let tls = fixture.tls.clone();
    let journal = fixture.files.root().join("projection-receive.json");
    fixture.server = Some(std::thread::spawn(move || {
        for step in 0..if renew { 3 } else { 1 } {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            if renew && step < 2 {
                if step == 0 {
                    assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                    reply(
                        &mut stream,
                        200,
                        br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#,
                    );
                } else {
                    assert!(head.starts_with("POST /v1/device-sessions "));
                    let now = crate::desktop::surface_windows::now_ms().unwrap();
                    reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({"deviceId":"device-old","token":"abandonment-renewed","expiresAtMs":now+60_000})).unwrap());
                }
                continue;
            }
            assert!(head.starts_with("POST /v1/projections/unlink "));
            assert_eq!(
                body,
                serde_json::json!({"projectionId":expected["entries"][0]["envelope"]["projectionId"]})
            );
            let token = if renew {
                "abandonment-renewed"
            } else {
                "receive-owner-token"
            };
            assert!(head
                .to_ascii_lowercase()
                .contains(&format!("authorization: device {token}\r\n")));
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&std::fs::read(&journal).unwrap())
                    .unwrap(),
                expected
            );
            reply(&mut stream, 200, br#"{"unlinked":true}"#);
        }
        listener
    }));
}
unsafe fn run(case: u8) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let (sent, received) = mpsc::sync_channel(1);
    let (release, released) = mpsc::sync_channel(1);
    let mut fixture = Fixture::with_accept(move |_, _| {
        sent.send(()).unwrap();
        released.recv_timeout(Duration::from_secs(10)).unwrap(); // 接受副作用可能已发生，故意丢全部回复。
    });
    start(&mut fixture);
    received.recv_timeout(Duration::from_secs(5)).unwrap();
    let mut expected: serde_json::Value = serde_json::from_slice(&fixture.journal()).unwrap();
    expected["entries"][0]["cleanup"] = true.into();
    if case == 1 || case == 2 {
        fixture.app.authorization.borrow_mut().request = Some(if case == 1 {
            Request::Cancel
        } else {
            Request::Select("https://other.example".into())
        });
        poll(&mut fixture);
        // 原授权取消后既不能呈现旧像素，也不能偷偷为清理恢复授权。
        assert!(!fixture.app.projection_receive.busy());
    }
    release.send(()).unwrap();
    if case == 3 {
        fixture.app.shutdown(); // 原 drain 丢弃 UI Event，仍须先领取精确票据再 flush。
        fixture.finish_accept();
        assert!(fixture.store.pending().unwrap()[0].cleanup_pending());
        assert!(!fixture.unit_present());
        assert!(fixture.app.error.is_none());
        return;
    }
    if case == 4 {
        fixture
            .app
            .authorization_owner
            .invalidate(&fixture.origin)
            .unwrap();
        DestroyWindow(fixture.app.documents[0].window.0).unwrap();
        assert!(
            fixture.app.projection_profile.entries().unwrap().is_empty(),
            "no ordinary poll may refresh the profile before direct terminal reap"
        );
        let mut message = MSG::default();
        // WM_QUIT 是低优先级生成消息；沿原 smoke 的有界窗口泵，不调用会刷新 profile 的 Application poll。
        let mut quit = false;
        for _ in 0..1024 {
            if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                break;
            }
            if message.message == WM_QUIT {
                assert_eq!(message.wParam.0, 0);
                quit = true;
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        assert!(
            quit,
            "last Unit must yield its actual WM_QUIT within the existing smoke bound"
        );
        assert!(fixture.app.projection_profile.entries().unwrap().is_empty());
        fixture.app.finish_deleted_workspace().unwrap();
        fixture.finish_accept();
        assert!(fixture.store.pending().unwrap()[0].cleanup_pending());
        assert!(crate::session_document::decode(
            &std::fs::read(fixture.files.root().join("workspace.json")).unwrap()
        )
        .unwrap()
        .workspace
        .units()
        .is_empty());
        return;
    }
    if fixture.app.remote_sessions.projection_pending() {
        wait_remote(&fixture.app);
    }
    fixture.finish_accept();
    if case == 1 || case == 2 {
        marked(&mut fixture);
        assert!(
            matches!(fixture.listener.as_ref().unwrap().accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
        );
    }
    unlink_peer(&mut fixture, expected, case != 0);
    if case != 0 {
        fixture.app.authorization.borrow_mut().origin = Some(fixture.origin.clone());
        fixture
            .app
            .authorization_owner
            .start(Task::Configured {
                config: config(&fixture.origin, fixture.files.root()).unwrap(),
                action: crate::device_authorization_config::Action::Renew,
            })
            .unwrap();
    }
    let end = Instant::now() + Duration::from_secs(10);
    while !fixture.store.pending().unwrap().is_empty()
        || fixture.app.remote_sessions.projection_pending()
    {
        poll(&mut fixture);
        assert!(
            Instant::now() < end,
            "accept abandonment authenticated cleanup deadline"
        );
        std::thread::yield_now();
    }
    fixture.listener = Some(fixture.server.take().unwrap().join().unwrap());
    assert!(!fixture.unit_present());
    assert!(!fixture.app.projection_receive.busy());
    assert!(!fixture.app.projection_cleanup.pending_local());
}
#[test]
fn projection_accept_abandonment_lost_reply_marks_exact_intent_before_original_authenticated_unlink(
) {
    unsafe { run(0) }
}
#[test]
fn projection_accept_abandonment_cancel_keeps_cleanup_local_until_actual_renewal() {
    unsafe { run(1) }
}
#[test]
fn projection_accept_abandonment_origin_switch_keeps_original_target_until_actual_renewal() {
    unsafe { run(2) }
}
#[test]
fn projection_accept_abandonment_shutdown_drain_retains_and_flushes_discarded_ticket() {
    unsafe { run(3) }
}
#[test]
fn projection_accept_abandonment_last_unit_marks_before_empty_checkpoint() {
    unsafe { run(4) }
}

#[test]
fn projection_accept_abandonment_unclaimed_single_handoff_blocks_new_projection_job() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let (sent, received) = mpsc::sync_channel(1);
        let (release, released) = mpsc::sync_channel(1);
        let mut fixture = Fixture::with_accept(move |_, _| {
            sent.send(()).unwrap();
            released.recv_timeout(Duration::from_secs(5)).unwrap();
        });
        start(&mut fixture);
        received.recv_timeout(Duration::from_secs(5)).unwrap();
        release.send(()).unwrap();
        wait_remote(&fixture.app);
        assert!(fixture
            .app
            .remote_sessions
            .poll_projection()
            .unwrap()
            .result
            .is_err());
        fixture.finish_accept();
        let lease = fixture
            .app
            .authorization_owner
            .lease(
                &fixture.origin,
                crate::desktop::surface_windows::now_ms().unwrap(),
            )
            .unwrap();
        let result = fixture.app.remote_sessions.query_projection_inbox(
            config(&fixture.origin, fixture.files.root()).unwrap(),
            lease,
        );
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
        assert!(!fixture.app.projection_cleanup.pending_local());
        fixture.app.shutdown();
        assert!(fixture.store.pending().unwrap()[0].cleanup_pending());
        assert!(fixture.app.error.is_none());
    }
}
