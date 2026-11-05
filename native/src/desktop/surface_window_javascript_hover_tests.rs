//! 自有 HWND 无按键 hover→原 FIFO→同 child/rAF/像素；定向消息，不移动系统光标。
use super::*;
#[path = "surface_window_javascript_hover_lifetime_tests.rs"]
mod lifetime;
use crate::surface_javascript::{
    hover_tests::{ENTER, SOURCE},
    Frame,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, GetFocus, SetFocus};
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
fn setup() -> (SurfaceWindow, Session, String) {
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
    let a = frame.scene.root().children()[1].children()[0]
        .id()
        .to_owned();
    install(&window, frame);
    window.enable_javascript_keyboard();
    window.bind_edits(&fixture::binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    (window, session, a)
}
fn point(window: &SurfaceWindow, id: &str) -> [i32; 2] {
    let host = window.state.host.borrow();
    let presentation = host.as_ref().unwrap().presentation();
    let g = presentation
        .geometry()
        .nodes()
        .iter()
        .find(|g| g.id() == id)
        .unwrap();
    let scale = window.viewport().unwrap().ratio() as f32;
    g.origin().map(|v| (v * scale).round() as i32 + 4)
}
fn send(window: &SurfaceWindow, message: u32, point: [i32; 2]) {
    assert_eq!(
        unsafe { GetForegroundWindow() },
        window.hwnd,
        "hover fixture lost foreground before send"
    );
    let packed = u32::from(point[0] as u16) | (u32::from(point[1] as u16) << 16);
    unsafe {
        SendMessageW(
            window.hwnd,
            message,
            Some(WPARAM(0)),
            Some(LPARAM(packed as isize)),
        );
    }
    assert!(window.take_error().is_none());
}
impl SurfaceWindow {
    pub(crate) fn javascript_hover_message_for_test(&self, message: u32) {
        let id = self
            .state
            .host
            .borrow()
            .as_ref()
            .unwrap()
            .presentation()
            .scene()
            .root()
            .children()[1]
            .children()[0]
            .id()
            .to_owned();
        let pixels = point(self, &id);
        if message == WM_MOUSEMOVE {
            self.state
                .run(|host| {
                    assert_eq!(
                        host.hover_target(pixels)?.as_deref(),
                        Some(id.as_str()),
                        "hover fixture must hit Canvas"
                    );
                    Ok(())
                })
                .unwrap();
        }
        send(self, message, pixels);
        assert!(self.has_pending_edit(), "hover message did not enter FIFO: js={}, local={}, submitting={}, foreground={:?}, target={:?}", self.state.javascript_keyboard.get(), self.state.local_edit.get(), self.pointer_enabled(), unsafe { GetForegroundWindow() }, self.hwnd);
    }
}
fn next(window: &SurfaceWindow, session: &mut Session) -> String {
    let prepared = window
        .prepare_javascript_edit(&fixture::binding(), None)
        .unwrap()
        .expect("missing hover input");
    assert!(matches!(
        prepared.input,
        crate::surface_javascript_wire::LocalInput::Hover(_)
    ));
    window.set_submission_enabled(false).unwrap();
    session.set_viewport(window.viewport().unwrap()).unwrap();
    let frame = session.local_input(prepared.input, &|| false).unwrap();
    install(window, frame);
    window.settle_javascript_edit(&prepared.id).unwrap();
    window.set_submission_enabled(true).unwrap();
    let frame = session.animation_frame(&|| false).unwrap();
    let text = fixture::text(&frame.scene).to_owned();
    install(window, frame);
    text
}
#[test]
fn javascript_hover_real_hwnd_no_buttons_keeps_edit_focus_no_capture_backpressure_and_pixels() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (window, mut session, a) = setup();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    let edit =
        unsafe { FindWindowExW(Some(window.hwnd), None, windows::core::w!("EDIT"), None).unwrap() };
    unsafe {
        let _ = SetFocus(Some(edit));
    }
    assert_eq!(unsafe { GetFocus() }, edit);
    let p = point(&window, &a);
    send(&window, WM_MOUSEMOVE, p);
    assert!(window.has_pending_edit());
    assert_eq!(unsafe { GetFocus() }, edit);
    assert_eq!(unsafe { GetCapture() }, HWND::default());
    assert_eq!(next(&window, &mut session), ENTER);
    assert_eq!(unsafe { GetFocus() }, edit);
    assert!(!window.has_pending_edit());
    assert_eq!(
        window.javascript_canvas_pixel_for_test(p[0] as usize, p[1] as usize),
        [0, 0, 255, 255]
    );
    send(&window, WM_MOUSEMOVE, p);
    send(&window, WM_MOUSEMOVE, [p[0] + 1, p[1]]);
    assert_eq!(
        next(&window, &mut session),
        format!("{ENTER}>pointermove:a:null>pointermove:w:null")
    );
    send(&window, WM_MOUSELEAVE, p);
    let text = next(&window, &mut session);
    assert!(text
        .ends_with("pointerout:a:null>pointerout:w:null>pointerleave:a:null>pointerleave:w:null"));
    assert_eq!(
        window.javascript_canvas_pixel_for_test(p[0] as usize, p[1] as usize),
        [255, 0, 0, 255]
    );
    assert_eq!(unsafe { GetFocus() }, edit);
    assert_eq!(unsafe { GetCapture() }, HWND::default());
    assert!(!window.has_pending_edit());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
}
#[test]
fn javascript_hover_real_hwnd_hard_cancel_discards_waiting_samples_and_delivers_one_leave() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    for delivered in [false, true] {
        let (window, mut session, a) = setup();
        let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
        let p = point(&window, &a);
        send(&window, WM_MOUSEMOVE, p);
        if delivered {
            assert_eq!(next(&window, &mut session), ENTER);
        }
        send(&window, WM_MOUSEMOVE, p);
        unsafe {
            SendMessageW(window.hwnd, WM_SIZE, None, None);
        }
        if delivered {
            assert!(next(&window, &mut session).ends_with("pointerleave:w:null"));
        } else {
            assert!(window
                .prepare_javascript_edit(&fixture::binding(), None)
                .unwrap()
                .is_none());
        }
        assert!(!window.has_pending_edit());
        assert_eq!(unsafe { GetCapture() }, HWND::default());
        session.dispose(&|| false).unwrap();
        window.close().unwrap();
    }
}
