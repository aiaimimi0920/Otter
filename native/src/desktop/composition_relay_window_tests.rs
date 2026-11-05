//! 隐藏窗口生命周期与消息合同，不显示 UI，不执行网络。
use super::*;
#[path = "composition_relay_session_end_tests.rs"]
mod session_end;
use crate::{
    desktop::Apartment,
    live_relay_frame::{Codec, ColorSpace, Metadata},
    live_relay_pixels::PixelFrame,
    live_relay_viewer_identity::Snapshot,
    pixels::Pixels,
};
#[path = "composition_relay_window_observation_tests.rs"]
mod observation_tests;
#[path = "composition_relay_window_trigger_tests.rs"]
mod trigger_tests;

fn media() -> (Arc<ViewerSession>, Permit) {
    let snapshot = Snapshot::parse(br#"{"session":{"protocolVersion":"loom.live.v1","sessionId":"live:test","sourceDeviceId":"source:test","sourceHookId":"hook:test","revision":1,"viewerDevices":["viewer:test"]},"epoch":1,"closed":false}"#, "live:test").unwrap();
    let media =
        Arc::new(ViewerSession::new(snapshot.identity().clone(), "viewer:test".into()).unwrap());
    media.activate(&snapshot).unwrap();
    let permit = media.admit(1).unwrap();
    (media, permit)
}
fn frame(pixels: Pixels) -> PixelFrame {
    PixelFrame::new(
        Metadata {
            epoch: 1,
            frame_id: 1,
            capture_timestamp_ms: 0,
            encode_timestamp_ms: 0,
            width: 1,
            height: 1,
            dropped_frames: 0,
            keyframe: true,
            color_space: ColorSpace::Srgb,
            codec: Codec::RawBgra,
        },
        pixels,
    )
    .unwrap()
}
#[test]
fn hidden_window_excludes_capture_consumes_notice_and_closes_on_revocation() {
    let _apartment = Apartment::new().unwrap();
    let (media, permit) = media();
    let window = RelayWindow::new(media.clone(), permit.clone(), 128, 96).unwrap();
    assert!(window.is_alive());
    assert!(!unsafe { IsWindowVisible(window.hwnd) }.as_bool());
    let mut affinity = 0;
    unsafe { GetWindowDisplayAffinity(window.hwnd, &mut affinity) }.unwrap();
    assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
    let pixels = Pixels::from(vec![255; 4]);
    let weak = pixels.downgrade();
    media.commit(&permit, frame(pixels), false).unwrap();
    window.refresh().unwrap();
    assert!(weak.upgrade().is_some());
    unsafe {
        SendMessageW(window.hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0)));
    }
    assert!(window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .surface_for_test()
        .is_none());
    media.close();
    window.refresh().unwrap();
    assert!(!window.is_alive());
    assert!(window.change_handle().is_none());
    assert!(weak.upgrade().is_none());
    assert!(window.show().is_err());
    assert!(window.take_error().is_none());
}
#[test]
fn close_message_destroys_display_without_joining_or_revoking_background_owner() {
    let _apartment = Apartment::new().unwrap();
    let (media, permit) = media();
    let window = RelayWindow::new(media.clone(), permit.clone(), 128, 96).unwrap();
    unsafe {
        SendMessageW(window.hwnd, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
    }
    assert!(!window.is_alive());
    // 后台资源必须由上层 Session 在收到窗口终态后停止/回收，回调不 join。
    assert!(media.validate(&permit).is_ok());
    window.close().unwrap();
    window.refresh().unwrap();
}
#[test]
fn reentrant_destroy_does_not_restore_borrowed_host() {
    let _apartment = Apartment::new().unwrap();
    let (media, permit) = media();
    let window = RelayWindow::new(media, permit, 128, 96).unwrap();
    window
        .state
        .with_host(|_| {
            unsafe {
                DestroyWindow(window.hwnd)?;
            }
            Ok(())
        })
        .unwrap();
    assert!(!window.is_alive());
    assert!(window.state.host.borrow().is_none());
}
