//! 自有 HWND 定向鼠标消息→原 FIFO→真实 child→Canvas 像素/捕获清理；不注入全局输入。
use super::*;
#[path = "surface_window_javascript_pointer_reentry_tests.rs"]
mod reentry;
#[test]
fn javascript_pointer_stale_prepare_consumes_cancel_in_same_call_without_external_wake() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    for delivered in [false, true] {
        let (window, mut session) = setup();
        let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
        window.javascript_pointer_message_for_test(WM_LBUTTONDOWN, 0);
        if delivered {
            assert_eq!(next(&window, &mut session), "pointerdown");
        }
        window.javascript_pointer_message_for_test(WM_LBUTTONUP, 0);
        // 精确注入领取边界的 stale epoch；不依赖 WM_SIZE 自身的提前取消来通过此回归。
        window
            .state
            .viewport_epoch
            .set(window.state.viewport_epoch.get() + 1);
        if delivered {
            assert_eq!(next(&window, &mut session), "pointerdown>pointercancel");
        } else {
            assert!(window
                .prepare_javascript_edit(&fixture::binding(), None)
                .unwrap()
                .is_none());
        }
        assert!(!window.has_pending_edit());
        session.dispose(&|| false).unwrap();
        window.close().unwrap();
    }
}
#[test]
fn javascript_pointer_real_hwnd_two_queued_gestures_keep_all_edges_and_clicks() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (window, mut session) = setup();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    for _ in 0..2 {
        window.javascript_pointer_message_for_test(WM_LBUTTONDOWN, 0);
        window.javascript_pointer_message_for_test(WM_LBUTTONUP, 0);
    }
    for expected in [
        "pointerdown",
        "pointerdown>pointerup>click",
        "pointerdown>pointerup>click>pointerdown",
        "pointerdown>pointerup>click>pointerdown>pointerup>click",
    ] {
        assert_eq!(next(&window, &mut session), expected);
    }
    assert!(!window.has_pending_edit());
    assert_ne!(unsafe { GetCapture() }, window.hwnd);
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
}
#[test]
fn javascript_pointer_real_hwnd_queued_up_is_discarded_on_deactivate_resize_and_hide() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    for message in [WM_ACTIVATEAPP, WM_KILLFOCUS, WM_SIZE, WM_SHOWWINDOW] {
        let (window, session) = setup();
        let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
        window.javascript_pointer_message_for_test(WM_LBUTTONDOWN, 0);
        window.javascript_pointer_message_for_test(WM_LBUTTONUP, 0);
        assert!(window.has_pending_edit());
        unsafe {
            SendMessageW(window.hwnd, message, Some(WPARAM(0)), None);
        }
        assert!(!window.has_pending_edit());
        assert_ne!(unsafe { GetCapture() }, window.hwnd);
        assert!(window
            .prepare_javascript_edit(&fixture::binding(), None)
            .unwrap()
            .is_none());
        assert_eq!(window.javascript_text_for_test(), "");
        session.dispose(&|| false).unwrap();
        window.close().unwrap();
    }
}
#[test]
fn javascript_pointer_real_hwnd_active_hide_delivers_only_cancel_and_releases_capture() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (window, mut session) = setup();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    window.javascript_pointer_message_for_test(WM_LBUTTONDOWN, 0);
    next(&window, &mut session);
    window.javascript_pointer_message_for_test(WM_MOUSEMOVE, 12);
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_HIDE);
    }
    assert_ne!(unsafe { GetCapture() }, window.hwnd);
    assert_eq!(next(&window, &mut session), "pointerdown>pointercancel");
    assert!(!window.has_pending_edit());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
}
use crate::surface_javascript::{pointer_tests::SOURCE, Frame};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, GetFocus, SetFocus};
impl SurfaceWindow {
    pub(crate) fn javascript_pointer_message_for_test(
        &self,
        message: u32,
        offset: i32,
    ) -> [i32; 2] {
        unsafe {
            assert_eq!(
                GetForegroundWindow(),
                self.hwnd,
                "pointer fixture lost foreground before send"
            );
            assert_eq!(GetFocus(), self.hwnd);
        }
        let point = {
            let host = self.state.host.borrow();
            let presentation = host.as_ref().unwrap().presentation();
            let canvas = presentation
                .scene()
                .root()
                .children()
                .iter()
                .find(|n| n.canvas_size().is_some())
                .unwrap();
            let geometry = presentation
                .geometry()
                .nodes()
                .iter()
                .find(|n| n.id() == canvas.id())
                .unwrap();
            let scale = self.viewport().unwrap().ratio() as f32;
            [
                (geometry.origin()[0] * scale).round() as i32 + 4 + offset,
                (geometry.origin()[1] * scale).round() as i32 + 4,
            ]
        };
        let packed = u32::from(point[0] as u16) | (u32::from(point[1] as u16) << 16);
        unsafe {
            SendMessageW(
                self.hwnd,
                message,
                Some(WPARAM(usize::from(message != WM_LBUTTONUP))),
                Some(LPARAM(packed as isize)),
            );
        }
        assert!(self.take_error().is_none());
        point
    }
}
fn install(window: &SurfaceWindow, frame: Frame) {
    let images = frame
        .images
        .into_iter()
        .map(|(id, image)| (id, crate::surface_image_paint::Bitmap::new(image).unwrap()))
        .collect();
    window
        .replace_javascript_frame(frame.scene, images)
        .unwrap();
}
fn setup() -> (SurfaceWindow, Session) {
    let window = SurfaceWindow::new(
        Scene::parse(br#"{"id":"root","type":"column"}"#).unwrap(),
        420,
        260,
    )
    .unwrap();
    let (session, frame) = Session::mount_in(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(window.viewport().unwrap()),
        &|| false,
    )
    .unwrap();
    install(&window, frame);
    window.enable_javascript_keyboard();
    window.bind_edits(&fixture::binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    (window, session)
}
fn next(window: &SurfaceWindow, session: &mut Session) -> String {
    let prepared = window
        .prepare_javascript_edit(&fixture::binding(), None)
        .unwrap()
        .expect("missing pointer input");
    window.set_submission_enabled(false).unwrap();
    session.set_viewport(window.viewport().unwrap()).unwrap();
    let frame = session.local_input(prepared.input, &|| false).unwrap();
    let text = fixture::text(&frame.scene).to_owned();
    install(window, frame);
    window.settle_javascript_edit(&prepared.id).unwrap();
    window.set_submission_enabled(true).unwrap();
    text
}
#[test]
fn javascript_pointer_real_hwnd_drag_survives_backpressure_and_same_canvas_frames_updates_pixels() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (window, mut session) = setup();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    let point = window.javascript_pointer_message_for_test(WM_LBUTTONDOWN, 0);
    assert_eq!(unsafe { GetCapture() }, window.hwnd);
    assert_eq!(next(&window, &mut session), "pointerdown");
    assert_eq!(unsafe { GetCapture() }, window.hwnd);
    assert_eq!(
        window.javascript_canvas_pixel_for_test(point[0] as usize, point[1] as usize),
        [0, 0, 255, 255]
    );
    window.javascript_pointer_message_for_test(WM_MOUSEMOVE, 12);
    window.javascript_pointer_message_for_test(WM_MOUSEMOVE, 20);
    assert_eq!(next(&window, &mut session), "pointerdown>pointermove");
    assert_eq!(unsafe { GetCapture() }, window.hwnd);
    window.javascript_pointer_message_for_test(WM_LBUTTONUP, 20);
    assert_ne!(unsafe { GetCapture() }, window.hwnd);
    assert_eq!(
        next(&window, &mut session),
        "pointerdown>pointermove>pointerup"
    );
    assert!(!window.has_pending_edit() && !window.has_pending_click());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
}
#[test]
fn javascript_pointer_real_hwnd_up_during_inflight_down_preserves_fifo_and_click() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (window, mut session) = setup();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    window.javascript_pointer_message_for_test(WM_LBUTTONDOWN, 0);
    let prepared = window
        .prepare_javascript_edit(&fixture::binding(), None)
        .unwrap()
        .unwrap();
    window.set_submission_enabled(false).unwrap();
    window.javascript_pointer_message_for_test(WM_LBUTTONUP, 0);
    let frame = session.local_input(prepared.input, &|| false).unwrap();
    install(&window, frame);
    window.settle_javascript_edit(&prepared.id).unwrap();
    window.set_submission_enabled(true).unwrap();
    assert_eq!(next(&window, &mut session), "pointerdown>pointerup>click");
    assert!(!window.has_pending_edit());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
}
#[test]
fn javascript_pointer_real_hwnd_capture_loss_resize_hide_and_revoke_do_not_replay() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    for message in [WM_CANCELMODE, WM_SIZE] {
        let (window, mut session) = setup();
        let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
        window.javascript_pointer_message_for_test(WM_LBUTTONDOWN, 0);
        assert_eq!(next(&window, &mut session), "pointerdown");
        window.javascript_pointer_message_for_test(WM_MOUSEMOVE, 12);
        unsafe {
            SendMessageW(window.hwnd, message, None, None);
        }
        assert_ne!(unsafe { GetCapture() }, window.hwnd);
        assert_eq!(next(&window, &mut session), "pointerdown>pointercancel");
        window.javascript_pointer_message_for_test(WM_LBUTTONDOWN, 0);
        window.set_input_policy(false, false).unwrap();
        window.set_input_policy(true, true).unwrap();
        assert!(!window.has_pending_edit());
        assert_ne!(unsafe { GetCapture() }, window.hwnd);
        unsafe {
            let _ = ShowWindow(window.hwnd, SW_HIDE);
            let _ = SetFocus(None);
        }
        assert!(!window.has_pending_edit());
        session.dispose(&|| false).unwrap();
        window.close().unwrap();
    }
}
