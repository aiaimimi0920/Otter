//! 真正 popup fixture：可见帧归代理，宿主区域全程为空；交接不改 GPU 选择或文档。
use super::*;
use std::time::Duration;
use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, GetWindowRgn, NULLREGION};

unsafe fn input(hwnd: HWND, delta: i16) {
    SendMessageW(
        hwnd,
        WM_MOUSEWHEEL,
        Some(WPARAM(
            ((delta as u16 as usize) << 16) | MK_CONTROL.0 as usize,
        )),
        Some(LPARAM(140 | (140 << 16))),
    );
}
unsafe fn empty_region(hwnd: HWND) -> bool {
    let region = CreateRectRgn(0, 0, 0, 0);
    let kind = GetWindowRgn(hwnd, region);
    let _ = DeleteObject(region.into());
    kind == NULLREGION
}
#[test]
fn zoom_preview_stays_between_its_host_and_the_pin_above_it() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let lower = Fixture::new();
        let upper = Fixture::new();
        SetWindowPos(
            upper.window.0,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
        .unwrap();
        for delta in [120, -60, 120, -120] {
            input(lower.window.0, delta);
            let proxy = preview::handle(&lower.state).unwrap();
            assert_eq!(GetWindow(proxy, GW_HWNDNEXT).unwrap(), lower.window.0);
            let mut cursor = GetWindow(upper.window.0, GW_HWNDNEXT).unwrap_or_default();
            let mut found = false;
            for _ in 0..128 {
                if cursor == proxy {
                    found = true;
                    break;
                }
                if cursor.is_invalid() {
                    break;
                }
                cursor = GetWindow(cursor, GW_HWNDNEXT).unwrap_or_default();
            }
            assert!(found, "preview must not jump over the upper pin");
        }
        animation::settle(lower.window.0, &lower.state);
    }
}
#[test]
fn deferred_final_paint_keeps_complete_preview_until_retry() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new();
        input(fixture.window.0, 120);
        let proxy = preview::handle(&fixture.state).unwrap();
        fixture.state.render_active.set(true);
        SendMessageW(fixture.window.0, WM_CANCELMODE, None, None);
        assert_eq!(preview::handle(&fixture.state), Some(proxy));
        assert!(empty_region(fixture.window.0));
        assert!(fixture.state.error.borrow().is_some());
        fixture.state.render_active.set(false);
        SendMessageW(fixture.window.0, WM_CANCELMODE, None, None);
        assert!(!IsWindow(Some(proxy)).as_bool());
        assert!(!empty_region(fixture.window.0));
        assert!(fixture.state.error.borrow().is_none());
    }
}
#[test]
fn resolved_display_zoom_preserves_document_and_display_identity() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new();
        let stamp = fixture.state.canvas.borrow().editor.document_stamp();
        let frame = crate::desktop::display_frame::Frame::capture(
            crate::workspace_id::UnitId::parse("zoom-resolved").unwrap(),
            crate::image_decode::Image {
                width: 4,
                height: 4,
                pixels: [200, 10, 80, 255].repeat(16).into(),
            },
            None,
        );
        let identity = frame.identity.clone();
        *fixture.state.resolved_display.borrow_mut() = Some(frame);
        input(fixture.window.0, 120);
        assert!(preview::handle(&fixture.state).is_some());
        animation::settle(fixture.window.0, &fixture.state);
        assert!(fixture.state.error.borrow().is_none());
        assert!(stamp.matches(&fixture.state.canvas.borrow().editor));
        assert!(std::sync::Arc::ptr_eq(
            &identity,
            &fixture
                .state
                .resolved_display
                .borrow()
                .as_ref()
                .unwrap()
                .identity
        ));
    }
}
#[test]
fn stale_source_cancels_preview_and_affinity_is_preserved() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new();
        SetWindowDisplayAffinity(fixture.window.0, WDA_EXCLUDEFROMCAPTURE).unwrap();
        input(fixture.window.0, 120);
        let proxy = preview::handle(&fixture.state).unwrap();
        let mut affinity = 0;
        GetWindowDisplayAffinity(proxy, &mut affinity).unwrap();
        assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
        fixture.state.canvas.borrow_mut().normal_opacity = 0.5;
        animation::settle(fixture.window.0, &fixture.state);
        assert!(!IsWindow(Some(proxy)).as_bool());
        assert!(!empty_region(fixture.window.0));
    }
}
#[test]
fn intermediate_frames_hide_host_pixels_without_changing_gpu_backend() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        for renderer in [
            None,
            Some(crate::desktop::render_commands::COMPATIBILITY),
            Some(crate::desktop::render_commands::SOFTWARE),
        ] {
            let fixture = Fixture::with_renderer(renderer);
            let choice = fixture.state.canvas.borrow().render_choice();
            let stamp = fixture.state.canvas.borrow().editor.document_stamp();
            let paints = fixture.state.canvas.borrow().paints;
            input(fixture.window.0, 120);
            let proxy = preview::handle(&fixture.state).unwrap();
            assert!(IsWindowVisible(proxy).as_bool());
            assert!(empty_region(fixture.window.0));
            std::thread::sleep(Duration::from_millis(25));
            let mut msg = MSG::default();
            if PeekMessageW(
                &mut msg,
                Some(fixture.window.0),
                WM_TIMER,
                WM_TIMER,
                PM_REMOVE,
            )
            .as_bool()
            {
                DispatchMessageW(&msg);
            }
            assert!(empty_region(fixture.window.0));
            assert_eq!(
                fixture.state.canvas.borrow().paints,
                paints,
                "host must not resize GPU buffers per tick"
            );
            let mut rect = RECT::default();
            GetWindowRect(proxy, &mut rect).unwrap();
            let current = fixture.rect();
            let padding =
                crate::desktop::pin_border_style::width(GetDpiForWindow(fixture.window.0)) as i32;
            assert_eq!(rect.left, current.x - padding);
            assert_eq!(rect.right, current.x + current.width as i32 + padding);
            input(proxy, 120);
            animation::settle(fixture.window.0, &fixture.state);
            assert!(!IsWindow(Some(proxy)).as_bool());
            assert!(!empty_region(fixture.window.0));
            assert_eq!(fixture.state.canvas.borrow().render_choice(), choice);
            assert!(stamp.matches(&fixture.state.canvas.borrow().editor));
            assert!(fixture.state.canvas.borrow().paints > paints);
            assert!(fixture.state.error.borrow().is_none());
        }
    }
}
#[test]
fn cancel_hide_and_destroy_release_preview_and_preserve_host_intent() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        for event in [WM_CANCELMODE, WM_CLOSE, WM_SHOWWINDOW] {
            let fixture = Fixture::new();
            input(fixture.window.0, 120);
            let proxy = preview::handle(&fixture.state).unwrap();
            if event == WM_SHOWWINDOW {
                let _ = ShowWindow(fixture.window.0, SW_HIDE);
                animation::settle(fixture.window.0, &fixture.state);
                assert!(!IsWindowVisible(fixture.window.0).as_bool());
            } else {
                SendMessageW(fixture.window.0, event, None, None);
            }
            assert!(!IsWindow(Some(proxy)).as_bool());
            assert!(!preview::active(&fixture.state));
            if event == WM_CANCELMODE {
                assert!(!empty_region(fixture.window.0));
            }
        }
    }
}
