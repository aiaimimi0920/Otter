//! 显式接受/真实保存/删除→先标记再原认证 unlink；负回执、真实退避与撤权恢复。
use super::*;
use crate::device_authorization_config::Action;

fn no_request(fixture: &Fixture) {
    assert!(
        matches!(fixture.listener.as_ref().unwrap().accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
    );
}
fn poll(fixture: &mut Fixture) {
    fixture
        .app
        .process_authorization_with_projection(|origin| config(origin, fixture.files.root()));
}
fn until_pending(fixture: &mut Fixture) {
    let end = Instant::now() + Duration::from_secs(5);
    while !fixture.app.remote_sessions.projection_pending() {
        poll(fixture);
        assert!(Instant::now() < end, "cleanup dispatch deadline");
        std::thread::yield_now();
    }
}
fn peer(fixture: &mut Fixture, mode: u8, expected: serde_json::Value) {
    let listener = fixture.listener.take().unwrap();
    let tls = fixture.tls.clone();
    let journal = fixture.files.root().join("projection-receive.json");
    let projection = fixture.ticket().envelope().projection_id().to_owned();
    let offline = fixture.offline;
    fixture.server = Some(std::thread::spawn(move || {
        let mut previous: Option<Instant> = None;
        let steps = match mode {
            1 | 2 => 2,
            3 => 3,
            _ => 1,
        };
        for step in 0..steps {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            let renew = mode == 3 && step < 2;
            if renew {
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
                    reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({"deviceId":"device-old","token":"renewed-cleanup-token","expiresAtMs":now+60_000})).unwrap());
                }
                continue;
            }
            assert!(head.starts_with(if offline {
                "POST /v1/offline-projections/unlink "
            } else {
                "POST /v1/projections/unlink "
            }));
            let token = if mode == 3 {
                "renewed-cleanup-token"
            } else {
                "receive-owner-token"
            };
            assert!(head
                .to_ascii_lowercase()
                .contains(&format!("authorization: device {token}\r\n")));
            assert_eq!(body, serde_json::json!({"projectionId": projection}));
            let persisted: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&journal).unwrap()).unwrap();
            assert_eq!(
                persisted, expected,
                "durable original identity and cleanup:true must precede every unlink"
            );
            if (mode == 1 || mode == 2) && step == 1 {
                assert!(
                    previous.unwrap().elapsed() >= Duration::from_secs(1),
                    "original backoff must not be shortened"
                );
            }
            previous = Some(Instant::now());
            if mode == 2 && step == 0 {
                reply(&mut stream, 403, br#"{"error":"access_denied"}"#);
            } else {
                reply(
                    &mut stream,
                    200,
                    if mode == 1 && step == 0 {
                        br#"{"unlinked":false}"#
                    } else {
                        br#"{"unlinked":true}"#
                    },
                );
            }
        }
        listener
    }));
}
fn renew(fixture: &mut Fixture) {
    fixture
        .app
        .authorization_owner
        .start(Task::Configured {
            config: config(&fixture.origin, fixture.files.root()).unwrap(),
            action: Action::Renew,
        })
        .unwrap();
    let handle = fixture
        .app
        .authorization_owner
        .wait_handles()
        .next()
        .unwrap();
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
    poll(fixture);
}
unsafe fn run(mode: u8) {
    run_routed(mode, false)
}
unsafe fn run_routed(mode: u8, offline: bool) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = if offline {
        Fixture::offline(false)
    } else {
        Fixture::new()
    };
    menu_receive(&mut fixture);
    checkpoint(&mut fixture.app);
    assert!(fixture
        .app
        .projection_checkpoint_inclusion(fixture.ticket())
        .is_some());
    fixture.app.reset_projection_inbox(); // 释放原 Checkpoint Hold，不伪造 displayed。
    let mut expected: serde_json::Value = serde_json::from_slice(&fixture.journal()).unwrap();
    expected["entries"][0]["cleanup"] = true.into();
    if mode == 3 {
        fixture.app.authorization.borrow_mut().origin = Some("https://wrong.example".into());
    }
    let unit = crate::workspace_id::UnitId::parse(fixture.ticket().unit()).unwrap();
    DestroyWindow(fixture.app.source_window(&unit).unwrap()).unwrap();
    fixture.app.collect_closed();
    assert!(fixture.app.projection_cleanup.pending_local());
    if mode == 3 {
        let end = Instant::now() + Duration::from_secs(5);
        while fixture.app.projection_cleanup.pending_local() {
            fixture.app.process_authorization_with_projection(|_| {
                panic!("wrong origin must not prepare HTTP")
            });
            assert!(Instant::now() < end);
            std::thread::yield_now();
        }
        no_request(&fixture);
        fixture
            .app
            .authorization_owner
            .invalidate(&fixture.origin)
            .unwrap();
        fixture.app.authorization.borrow_mut().origin = Some(fixture.origin.clone());
        fixture.app.process_authorization_with_projection(|_| {
            panic!("revoked grant must not prepare HTTP")
        });
        no_request(&fixture);
        assert!(fixture.store.pending().unwrap()[0].cleanup_pending());
        assert!(fixture.app.projection_cleanup.deadline().is_none());
    }
    peer(&mut fixture, mode, expected);
    if mode == 3 {
        renew(&mut fixture);
    }
    until_pending(&mut fixture);
    wait_remote(&fixture.app);
    poll(&mut fixture);
    if mode == 1 || mode == 2 {
        assert!(fixture.store.pending().unwrap()[0].cleanup_pending());
        assert!(!fixture.app.remote_sessions.projection_pending());
        let due = fixture.app.projection_cleanup.deadline().unwrap();
        assert!(due > Instant::now());
        fixture
            .app
            .process_authorization_with_projection(|_| panic!("backoff must not prepare HTTP"));
        assert_eq!(fixture.app.projection_cleanup.deadline(), Some(due));
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
        until_pending(&mut fixture);
        wait_remote(&fixture.app);
        poll(&mut fixture);
    }
    assert!(fixture.store.pending().unwrap().is_empty());
    assert!(!fixture.unit_present());
    assert!(!fixture.app.projection_cleanup.pending_local());
    assert!(!fixture.app.remote_sessions.projection_pending());
    assert!(fixture.app.projection_cleanup.deadline().is_none());
    fixture.listener = Some(fixture.server.take().unwrap().join().unwrap());
    no_request(&fixture);
    fixture.app.shutdown();
    assert!(fixture.app.error.is_none());
    assert!(matches!(
        fixture.app.projection_profile,
        projection::profile::Owner::Disabled
    ));
}
#[test]
fn projection_cleanup_tls_deleted_saved_unit_marks_before_authenticated_true_unlink() {
    unsafe { run(0) }
}
#[test]
fn projection_cleanup_tls_false_receipt_keeps_marker_and_retries_after_real_backoff() {
    unsafe { run(1) }
}
#[test]
fn projection_cleanup_tls_access_denied_keeps_marker_and_retries_without_inventing_success() {
    unsafe { run(2) }
}
#[test]
fn projection_cleanup_tls_wrong_origin_and_revoked_grant_stay_local_until_actual_renewal() {
    unsafe { run(3) }
}
#[test]
fn projection_offline_cleanup_deleted_unit_uses_persisted_route_and_true_unlink() {
    unsafe { run_routed(0, true) }
}
#[test]
fn projection_offline_cleanup_false_receipt_keeps_marker_and_real_retry_backoff() {
    unsafe { run_routed(1, true) }
}
#[test]
fn projection_offline_cleanup_wrong_origin_and_revocation_wait_for_renewal() {
    unsafe { run_routed(3, true) }
}
