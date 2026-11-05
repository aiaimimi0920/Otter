//! Author :hover 的同 child 经真实 HWND/FIFO/Presentation/D2D；仅定向自有消息。
use super::*;
use crate::surface_javascript::{css_hover_tests::SOURCE, Frame};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, GetFocus};
fn install(window: &SurfaceWindow, frame: Frame) {
    assert!(frame.images.is_empty());
    window
        .replace_javascript_frame(frame.scene, Default::default())
        .unwrap();
}
fn next(window: &SurfaceWindow, session: &mut Session) {
    let prepared = window
        .prepare_javascript_edit(&fixture::binding(), None)
        .unwrap()
        .unwrap();
    assert!(matches!(
        prepared.input,
        crate::surface_javascript_wire::LocalInput::Hover(_)
    ));
    window.set_submission_enabled(false).unwrap();
    session.set_viewport(window.viewport().unwrap()).unwrap();
    install(
        window,
        session.local_input(prepared.input, &|| false).unwrap(),
    );
    window.settle_javascript_edit(&prepared.id).unwrap();
    window.set_submission_enabled(true).unwrap();
    assert!(!window.has_pending_edit());
}
fn send(window: &SurfaceWindow, message: u32) {
    assert_eq!(unsafe { GetForegroundWindow() }, window.hwnd);
    unsafe {
        SendMessageW(
            window.hwnd,
            message,
            Some(WPARAM(0)),
            Some(LPARAM(8 | (8 << 16))),
        );
    }
    assert!(window.take_error().is_none());
}
#[test]
fn javascript_css_hover_real_hwnd_styles_pixels_geometry_leave_hide_and_same_child() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let window = SurfaceWindow::new(
        Scene::parse(br#"{"id":"root","type":"column"}"#).unwrap(),
        200,
        80,
    )
    .unwrap();
    let (mut session, frame) = Session::mount_in(
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
    let left = frame.scene.root().children()[1].id().to_owned();
    install(&window, frame);
    window.enable_javascript_keyboard();
    window.bind_edits(&fixture::binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    assert_eq!(
        css_tests::geometry(&window, &a),
        Some(([0., 0.], [20., 20.]))
    );
    assert_eq!(
        css_tests::pixel(&css_tests::pixels(&window), 8, 8),
        [255, 0, 0, 255]
    );
    for hidden in [false, true] {
        send(&window, WM_MOUSEMOVE);
        next(&window, &mut session);
        assert_eq!(
            css_tests::geometry(&window, &a),
            Some(([0., 0.], [30., 24.]))
        );
        let pixels = css_tests::pixels(&window);
        assert_eq!(css_tests::pixel(&pixels, 8, 8), [0, 0, 255, 255]);
        assert_eq!(css_tests::pixel(&pixels, 65, 8), [0, 255, 0, 255]);
        assert_eq!(
            window
                .state
                .host
                .borrow()
                .as_ref()
                .unwrap()
                .presentation()
                .hit_test([28., 8.]),
            Some(a.as_str())
        );
        assert_eq!(unsafe { GetFocus() }, window.hwnd);
        assert_eq!(unsafe { GetCapture() }, HWND::default());
        if hidden {
            unsafe {
                let _ = ShowWindow(window.hwnd, SW_HIDE);
            }
        } else {
            send(&window, WM_MOUSELEAVE);
        }
        next(&window, &mut session);
        assert_eq!(
            css_tests::geometry(&window, &a),
            Some(([0., 0.], [20., 20.]))
        );
        assert_eq!(
            css_tests::pixel(&css_tests::pixels(&window), 8, 8),
            [255, 0, 0, 255]
        );
        assert_eq!(
            window
                .state
                .host
                .borrow()
                .as_ref()
                .unwrap()
                .presentation()
                .hit_test([65., 8.]),
            Some(left.as_str())
        );
    }
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
    println!("native-css-hover-hwnd: fifo=true pixels=true geometry=true hits=true leave=true hidden=true same-child=true closed=true");
}
