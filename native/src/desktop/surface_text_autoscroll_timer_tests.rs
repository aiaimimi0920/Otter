//! Actual Win32 timer-queue evidence: no due changes, synthesized WM_TIMER, or desktop-input injection.
use super::*;
fn wait_timer(window: &SurfaceWindow, id: usize, remove: PEEK_MESSAGE_REMOVE_TYPE) -> MSG {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let mut message = MSG::default();
        if unsafe { PeekMessageW(&mut message, Some(window.hwnd), WM_TIMER, WM_TIMER, remove) }
            .as_bool()
        {
            assert_eq!(message.hwnd, window.hwnd);
            assert_eq!(message.wParam.0, id);
            return message;
        }
        assert!(
            Instant::now() < deadline,
            "owned system timer was not delivered"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}
fn assert_stopped(window: &SurfaceWindow, id: usize) {
    assert!(window.state.pointer.borrow().selection_scroll.is_none());
    assert!(window.state.pointer.borrow().selection_owner.is_none());
    let before = y(window);
    let text = selected(window);
    // KillTimer does not guarantee removal of generated messages; dispatch this HWND's residue before verifying no new messages appear.
    let mut message = MSG::default();
    for _ in 0..16 {
        if !unsafe {
            PeekMessageW(
                &mut message,
                Some(window.hwnd),
                WM_TIMER,
                WM_TIMER,
                PM_REMOVE,
            )
        }
        .as_bool()
        {
            break;
        }
        assert_eq!(message.wParam.0, id);
        unsafe {
            DispatchMessageW(&message);
        }
    }
    let deadline = Instant::now() + Duration::from_millis(200);
    while Instant::now() < deadline {
        assert!(
            !unsafe {
                PeekMessageW(
                    &mut message,
                    Some(window.hwnd),
                    WM_TIMER,
                    WM_TIMER,
                    PM_REMOVE,
                )
            }
            .as_bool(),
            "stopped drag generated another timer"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(y(window), before);
    assert_eq!(selected(window), text);
    assert!(!window.has_pending_click());
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn text_autoscroll_system_timer_drives_stationary_selection_and_stops_after_release() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    let id = start(&window);
    let before = selected(&window);
    let measured = crate::surface_text::measure_calls();
    for _ in 0..16 {
        let message = wait_timer(&window, id, PM_REMOVE);
        unsafe {
            DispatchMessageW(&message);
        }
        if y(&window) <= -48. {
            break;
        }
    }
    assert!(y(&window) <= -48.);
    assert!(selected(&window).len() > before.len());
    assert_eq!(crate::surface_text::measure_calls(), measured);
    unsafe {
        send(&window, WM_LBUTTONUP, 5, 58);
    }
    assert_ne!(unsafe { GetCapture() }, window.hwnd);
    assert_stopped(&window, id);
}
#[test]
fn text_autoscroll_system_timer_queued_before_cancel_cannot_mutate_after_cancel() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for revoke in [false, true] {
        let window = window();
        let id = start(&window);
        // PM_NOREMOVE prompts actual system-message generation and deliberately leaves it queued across cancellation.
        wait_timer(&window, id, PM_NOREMOVE);
        assert_eq!(y(&window), 0.);
        if revoke {
            window.set_input_policy(false, false).unwrap();
        } else {
            unsafe {
                send(&window, WM_CANCELMODE, 0, 0);
            }
        }
        let message = wait_timer(&window, id, PM_REMOVE);
        unsafe {
            DispatchMessageW(&message);
        }
        assert_eq!(y(&window), 0.);
        assert!(selected(&window).is_empty());
        assert_stopped(&window, id);
    }
}
