//! displayed 恢复测试复用原合成身份/TLS/窗口，只驱动真实 checkpoint/menu/read/receipt。
use super::*;
use crate::desktop::device_authorization_ui::projection as ui;
use crate::projection_delivery::accept::tests as png;

pub(super) unsafe fn saved(fixture: &mut Fixture) {
    fixture.accepted(true);
    until(&mut fixture.app, |app| !app.projection_receive.preparing());
    fixture
        .app
        .start_checkpoint_with(&mut |_, _, message, warning| assert!(!warning, "{message}"));
    checkpoint(&mut fixture.app);
    fixture.app.reset_projection_inbox();
    fixture.app.process_authorization();
    assert!(!fixture.app.projection_receive.busy());
    assert!(fixture
        .app
        .projection_checkpoint_inclusion(fixture.ticket())
        .is_some());
}
pub(super) unsafe fn choose(fixture: &mut Fixture) {
    let doc = &fixture.app.documents[0];
    crate::desktop::device_authorization_ui::execute(
        doc.window.0,
        &doc.state,
        ui::recovery::DISPLAY_BASE,
    )
    .unwrap();
    poll(fixture);
}
pub(super) fn poll(fixture: &mut Fixture) {
    fixture
        .app
        .process_authorization_with_projection(|o| config(o, fixture.files.root()));
}
pub(super) fn settle(fixture: &mut Fixture) {
    let end = Instant::now() + Duration::from_secs(10);
    while fixture.app.projection_receive.busy() {
        poll(fixture);
        assert!(Instant::now() < end, "display recovery completion deadline");
        std::thread::yield_now();
    }
}
pub(super) fn response(origin: &str) -> serde_json::Value {
    let mut response = png::response(origin);
    response["receiverDeviceId"] = "device-old".into();
    response
}
pub(super) fn peer(fixture: &mut Fixture, mode: u8, renew: bool) {
    let listener = fixture.listener.take().unwrap();
    let tls = fixture.tls.clone();
    let origin = fixture.origin.clone();
    let offline = fixture.offline;
    fixture.server = Some(std::thread::spawn(move || {
        if renew {
            for index in 0..2 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
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
                    let now = crate::desktop::surface_windows::now_ms().unwrap();
                    reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({
                        "deviceId":"device-old", "token":"receive-owner-token", "expiresAtMs":now+60_000
                    })).unwrap());
                }
            }
        }
        let mut stream = StreamOwned::new(
            ServerConnection::new(tls.clone()).unwrap(),
            accept(&listener),
        );
        let (head, body) = request(&mut stream);
        assert!(head.starts_with(if offline {
            "POST /v1/offline-projections/read "
        } else {
            "POST /v1/projections/read "
        }));
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: device receive-owner-token\r\n"));
        assert_eq!(
            body,
            serde_json::json!({"projectionId":png::invitation(&origin).envelope().projection_id(),"knownRevision":0})
        );
        let mut value = if offline {
            crate::projection_delivery::tests::offline::response("device-old")
        } else {
            response(&origin)
        };
        match mode {
            1 => {
                reply(&mut stream, 403, b"denied");
                return listener;
            }
            2 => value["revision"] = 6.into(),
            3 => value["digest"] = "0".repeat(64).into(),
            4 => {} // 当前保存图像被替换；远端仍只返回原图。
            7 => value["receiverDeviceId"] = "different-device".into(),
            _ => {}
        }
        reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
        drop(stream);
        if matches!(mode, 1..=4 | 7) {
            return listener;
        }
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with(if offline {
            "POST /v1/offline-projections/receipt "
        } else {
            "POST /v1/projections/receipt "
        }));
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: device receive-owner-token\r\n"));
        assert_eq!(
            body,
            serde_json::json!({"projectionId":png::invitation(&origin).envelope().projection_id(),"status":"displayed"})
        );
        if mode != 6 {
            reply(
                &mut stream,
                200,
                if mode == 5 {
                    br#"{"recorded":false}"#
                } else {
                    br#"{"recorded":true}"#
                },
            );
        }
        listener
    }));
}
pub(super) fn joined(fixture: &mut Fixture) {
    fixture.listener = Some(fixture.server.take().unwrap().join().unwrap());
}
pub(super) unsafe fn reopen(fixture: &mut Fixture) {
    reopen_workspace(fixture);
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
        WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
        WAIT_OBJECT_0
    );
    poll(fixture);
}
pub(super) unsafe fn reopen_workspace(fixture: &mut Fixture) {
    use crate::desktop::{document_window::DocumentWindow, workspace_profile::Profile};
    use crate::projection_delivery::journal::Store;
    fixture.app.shutdown();
    let instance = GetModuleHandleW(None).unwrap().into();
    let document =
        DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
    let old = std::mem::replace(&mut fixture.app, Application::new(instance, document));
    drop(old);
    fixture.ticket = None;
    // 释放所有旧 Store/活动许可，确实重新打开原路径，不用相同字节的另一份 profile 冒充。
    let (placeholder, _, _) = Files::new();
    drop(std::mem::replace(
        &mut fixture.store,
        Store::open(placeholder.root()).unwrap(),
    ));
    fixture
        .app
        .install_workspace_checkpoint(Some(Profile::open(fixture.files.root()).unwrap()), true)
        .unwrap();
    fixture.app.projection_profile.start(
        fixture.files.root().to_owned(),
        fixture.app.execution_wake.callback(),
    );
    until(&mut fixture.app, |app| {
        !matches!(
            app.projection_profile,
            projection::profile::Owner::Opening(_)
        )
    });
    fixture.store = match &fixture.app.projection_profile {
        projection::profile::Owner::Ready { store, .. } => store.clone(),
        _ => panic!("reopened profile unavailable"),
    };
    fixture.ticket = Some(fixture.store.pending().unwrap().pop().unwrap());
    fixture.app.authorization.borrow_mut().origin = Some(fixture.origin.clone());
    fixture.app.defer_projection_poll_for_test(&fixture.origin);
}
