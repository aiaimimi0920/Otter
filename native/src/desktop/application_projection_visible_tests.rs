//! 合成 profile/真实 TLS/显式菜单→保存→可见新帧→displayed；不触碰真实设备或激活用户窗口。
use super::*;
use crate::desktop::device_authorization_ui::projection as ui;
use crate::projection_delivery::accept::tests as png;
use std::sync::mpsc;
#[path = "application_projection_cleanup_tls_tests.rs"]
mod cleanup;
#[path = "application_projection_present_tests.rs"]
mod presentation;

unsafe fn foreground_snapshot() -> (HWND, u32, String, bool) {
    let hwnd = GetForegroundWindow();
    let mut pid = 0;
    windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid));
    let mut class = [0; 128];
    let count = windows::Win32::UI::WindowsAndMessaging::GetClassNameW(hwnd, &mut class);
    (
        hwnd,
        pid,
        String::from_utf16_lossy(&class[..count.max(0) as usize]),
        IsWindow(Some(hwnd)).as_bool(),
    )
}

unsafe fn menu_receive(fixture: &mut Fixture) {
    let entry = Arc::new(fixture.invitation());
    fixture.app.authorization.borrow_mut().projection.entries = vec![entry];
    let doc = &fixture.app.documents[0];
    ui::accept::confirm_with(doc.window.0, &doc.state, ui::ACCEPT_BASE, |_| true).unwrap();
    fixture
        .app
        .process_authorization_with_projection(|origin| config(origin, fixture.files.root()));
    assert!(fixture.app.remote_sessions.projection_pending());
    wait_remote(&fixture.app);
    fixture
        .app
        .process_authorization_with_projection(|origin| config(origin, fixture.files.root()));
    fixture.finish_accept();
    until(&mut fixture.app, |app| !app.projection_receive.preparing());
    assert!(fixture.unit_present());
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
    let journal = fixture.journal();
    assert!(fixture
        .app
        .documents
        .iter()
        .all(|doc| !IsWindowVisible(doc.window.0).as_bool()));
    let (requested, observed) = mpsc::sync_channel(1);
    let (release, released) = mpsc::sync_channel(1);
    let listener = fixture.listener.take().unwrap();
    let tls = fixture.tls.clone();
    let path = fixture.files.root().join("workspace.json");
    let unit = fixture.ticket().unit().to_owned();
    let intent = fixture.ticket().intent_id().to_owned();
    let transport_origin = fixture.origin.clone();
    fixture.server = Some(std::thread::spawn(move || {
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
            serde_json::json!({"projectionId":png::invitation("https://unused.example").envelope().projection_id(), "status":"displayed"})
        );
        // 真实磁盘已包含同一 intent；不是网络成功后再保存，也不是仅凭 Unit 存在。
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let sticker = value["stickers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == unit)
            .unwrap();
        assert_eq!(
            sticker["qrProjection"]["otterNativeReceive"]["intentId"],
            intent
        );
        if offline {
            assert_eq!(
                sticker["qrProjection"]["offlineTransport"]["origin"],
                transport_origin
            );
            assert_eq!(
                sticker["qrProjection"]["envelope"]["serverOrigin"],
                crate::projection_delivery::tests::offline::SOURCE
            );
            assert_eq!(
                sticker["qrProjection"]["otterNativeReceive"]["peerId"],
                crate::projection_delivery::tests::offline::PEER
            );
        }
        requested.send(()).unwrap();
        released.recv_timeout(Duration::from_secs(5)).unwrap();
        if mode != 2 {
            reply(
                &mut stream,
                200,
                if mode == 1 {
                    br#"{"recorded":false}"#
                } else {
                    br#"{"recorded":true}"#
                },
            );
        }
        listener
    }));
    // 尚未 checkpoint 时消费多次也不显示、不发回执。
    for _ in 0..3 {
        fixture
            .app
            .process_authorization_with_projection(|origin| config(origin, fixture.files.root()));
    }
    assert!(matches!(
        observed.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    checkpoint(&mut fixture.app);
    assert!(fixture
        .app
        .projection_checkpoint_inclusion(fixture.ticket())
        .is_some());
    let foreground = foreground_snapshot();
    fixture
        .app
        .process_authorization_with_projection(|origin| config(origin, fixture.files.root()));
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    let doc = fixture
        .app
        .documents
        .iter()
        .find(|doc| doc.state.unit_id.as_str() == fixture.ticket().unit())
        .unwrap();
    assert!(IsWindowVisible(doc.window.0).as_bool());
    assert!(doc.state.canvas.borrow().confirmed_paints > 0);
    let after = foreground_snapshot();
    assert_eq!(
        after.0,
        foreground.0,
        "receive must not activate its window; before={foreground:?}; after={after:?}; testPid={}",
        std::process::id()
    );
    assert!(fixture.app.projection_receive.wait_handle().is_some());
    assert!(fixture.app.capture_session_snapshot().is_err());
    let mut retry = fixture.ticket().retry();
    assert_eq!(
        retry.activate(),
        Err(crate::projection_delivery::journal::Error::Busy)
    );
    let slots = (0..3)
        .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
        .collect::<Vec<_>>();
    assert!(crate::live_relay_control_runtime::Slot::acquire().is_err());
    drop(slots);
    if mode == 2 {
        fixture.app.reset_projection_inbox();
        assert!(fixture.app.projection_receive.busy()); // 不 poll，仍保原 Job/Hold。
        assert!(fixture
            .app
            .projection_receive
            .deadline(crate::desktop::surface_windows::now_ms().unwrap())
            .is_none());
        assert_eq!(
            retry.activate(),
            Err(crate::projection_delivery::journal::Error::Busy)
        );
    }
    release.send(()).unwrap();
    let handle = fixture.app.projection_receive.wait_handle().unwrap();
    assert_eq!(
        WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
        WAIT_OBJECT_0
    );
    fixture
        .app
        .process_authorization_with_projection(|origin| config(origin, fixture.files.root()));
    assert!(!fixture.app.projection_receive.busy());
    assert_eq!(
        fixture.app.authorization.borrow().projection.status,
        if mode == 0 {
            "投送已呈现"
        } else {
            "接收未完成；本地内容和未结意图保留"
        }
    );
    assert!(fixture.unit_present());
    assert_eq!(fixture.journal(), journal);
    assert_eq!(fixture.store.pending().unwrap().len(), 1);
    retry.activate().unwrap();
}
#[test]
fn projection_visible_menu_accept_checkpoints_then_submits_visible_frame_before_authenticated_displayed(
) {
    unsafe { run(0) }
}
#[test]
fn projection_visible_recorded_false_is_not_success_and_keeps_image_and_journal() {
    unsafe { run(1) }
}
#[test]
fn projection_visible_cancelled_receipt_retains_original_slot_and_intent_until_ui_reaps() {
    unsafe { run(2) }
}
#[test]
fn projection_offline_visible_menu_checkpoints_real_frame_then_original_route_receipt() {
    unsafe { run_routed(0, true) }
}
#[test]
fn projection_offline_visible_false_receipt_preserves_image_and_original_journal() {
    unsafe { run_routed(1, true) }
}
#[test]
fn projection_offline_visible_cancel_keeps_original_capacity_until_receipt_is_reaped() {
    unsafe { run_routed(2, true) }
}
