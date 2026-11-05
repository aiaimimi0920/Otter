//! Hover 的后台/带键负向、真实隐藏和 Host 借出时的离开归还接缝。
use super::*;
#[test]
fn javascript_hover_real_hwnd_canvas_up_outside_finishes_existing_hover_after_capture_leave() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (window, mut session, a) = setup();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    let p = point(&window, &a);
    send(&window, WM_MOUSEMOVE, p);
    assert_eq!(next(&window, &mut session), ENTER);
    for (message, point) in [(WM_LBUTTONDOWN, p), (WM_LBUTTONUP, [-20, -20])] {
        send(&window, message, point);
        let prepared = window
            .prepare_javascript_edit(&fixture::binding(), None)
            .unwrap()
            .unwrap();
        assert!(matches!(
            prepared.input,
            crate::surface_javascript_wire::LocalInput::Pointer(_)
        ));
        window.set_submission_enabled(false).unwrap();
        let frame = session.local_input(prepared.input, &|| false).unwrap();
        install(&window, frame);
        window.settle_javascript_edit(&prepared.id).unwrap();
        window.set_submission_enabled(true).unwrap();
        if message == WM_LBUTTONDOWN {
            assert_eq!(unsafe { GetCapture() }, window.hwnd);
            send(&window, WM_MOUSELEAVE, p); // capture 中实际跟踪已结束，up 不能依赖 tracking bool。
        }
    }
    assert_eq!(unsafe { GetCapture() }, HWND::default());
    assert!(next(&window, &mut session).ends_with("pointerleave:w:null"));
    assert!(!window.has_pending_edit());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
}
#[test]
fn javascript_hover_real_hwnd_background_and_button_samples_are_not_queued_or_replayed() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (window, mut session, a) = setup();
    unsafe {
        SendMessageW(window.hwnd, WM_MOUSEMOVE, None, None);
    }
    assert!(!window.has_pending_edit());
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    let p = point(&window, &a);
    let packed = u32::from(p[0] as u16) | (u32::from(p[1] as u16) << 16);
    for button in [1, 2, 16, 32, 64] {
        assert_eq!(unsafe { GetForegroundWindow() }, window.hwnd);
        unsafe {
            SendMessageW(
                window.hwnd,
                WM_MOUSEMOVE,
                Some(WPARAM(button)),
                Some(LPARAM(packed as isize)),
            );
        }
        assert!(!window.has_pending_edit());
    }
    assert!(window
        .prepare_javascript_edit(&fixture::binding(), None)
        .unwrap()
        .is_none());
    send(&window, WM_MOUSEMOVE, p);
    assert_eq!(next(&window, &mut session), ENTER);
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
}
#[test]
fn javascript_hover_real_hwnd_hidden_window_delivers_leave_without_resurrecting_samples() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (window, mut session, a) = setup();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    let p = point(&window, &a);
    send(&window, WM_MOUSEMOVE, p);
    assert_eq!(next(&window, &mut session), ENTER);
    send(&window, WM_MOUSEMOVE, p);
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_HIDE);
    }
    assert!(next(&window, &mut session).ends_with("pointerleave:w:null"));
    assert!(!unsafe { IsWindowVisible(window.hwnd) }.as_bool());
    assert!(!window.has_pending_edit());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
}
#[test]
fn javascript_hover_real_hwnd_leave_while_host_borrowed_is_settled_at_original_return_seam() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (window, mut session, a) = setup();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    let p = point(&window, &a);
    send(&window, WM_MOUSEMOVE, p);
    assert_eq!(next(&window, &mut session), ENTER);
    window
        .state
        .run(|_| {
            unsafe {
                SendMessageW(window.hwnd, WM_MOUSELEAVE, None, None);
            }
            assert!(window.state.pointer_cancel_pending.get());
            Ok(())
        })
        .unwrap();
    assert!(!window.state.pointer_cancel_pending.get());
    assert!(next(&window, &mut session).ends_with("pointerleave:w:null"));
    assert!(!window.has_pending_edit());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
}
