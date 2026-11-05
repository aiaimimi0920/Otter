//! 捕获/计时器在prepare、减权、窗口同步重入和替代输入下的实际所有权。
use super::*;
#[test]
fn preparation_gap_keeps_capture_but_never_samples_then_resumes_same_geometry() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup();
    send(&w, WM_LBUTTONDOWN, [60, 5]);
    send(&w, WM_MOUSEMOVE, [60, 15]);
    let before = field(&w).bar.thumb.top;
    f.shared
        .lock()
        .unavailable(f.job.generation, &f.frame.key, "prepare");
    w.tick().unwrap();
    assert_eq!(unsafe { GetCapture() }, w.hwnd);
    send(&w, WM_MOUSEMOVE, [60, 51]);
    f.shared.lock().frame = Some(f.frame.clone());
    w.tick().unwrap();
    assert!((field(&w).bar.thumb.top - before).abs() < 0.001);
    send(&w, WM_MOUSEMOVE, [60, 51]);
    thumb(&w, 956.);
    send(&w, WM_LBUTTONUP, [60, 51]);
    assert!(events(&f).is_empty());
}
#[test]
fn release_during_preparation_does_not_apply_late_pointer_or_leave_capture() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup();
    send(&w, WM_LBUTTONDOWN, [60, 5]);
    send(&w, WM_MOUSEMOVE, [60, 15]);
    let before = field(&w).bar.thumb.top;
    f.shared
        .lock()
        .unavailable(f.job.generation, &f.frame.key, "prepare");
    w.tick().unwrap();
    send(&w, WM_LBUTTONUP, [60, 51]);
    assert_ne!(unsafe { GetCapture() }, w.hwnd);
    f.shared.lock().frame = Some(f.frame.clone());
    w.tick().unwrap();
    assert!((field(&w).bar.thumb.top - before).abs() < 0.001);
    assert!(w.state.scrollbar.borrow().gesture_is_idle());
}
#[test]
fn freeze_black_expire_lease_hide_cancel_and_capture_loss_retire_both_gestures() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for page in [false, true] {
        for case in 0..8 {
            let (f, w) = setup();
            send(&w, WM_LBUTTONDOWN, if page { [60, 50] } else { [60, 5] });
            let id = if page { Some(timer(&w)) } else { None };
            match case {
                0 => f.shared.lock().frozen = true,
                1 => f.shared.lock().black = true,
                2 => {
                    let until = f.shared.lock().until.unwrap();
                    f.shared.lock().expire(until);
                }
                3 => f.shared.lock().desired.as_mut().unwrap().lease = "other".into(),
                4 => unsafe {
                    let _ = ShowWindow(w.hwnd, SW_HIDE);
                },
                5 => unsafe {
                    SendMessageW(w.hwnd, WM_CANCELMODE, None, None);
                },
                6 => unsafe {
                    let _ = ReleaseCapture();
                },
                _ => unsafe {
                    SendMessageW(w.hwnd, WM_DPICHANGED, None, None);
                },
            }
            tick(w.hwnd, &w.state);
            assert!(
                w.state.scrollbar.borrow().gesture_is_idle(),
                "{page}/{case}"
            );
            assert_ne!(unsafe { GetCapture() }, w.hwnd);
            if let Some(id) = id {
                assert!(!drive(
                    w.hwnd,
                    &w.state,
                    WM_TIMER,
                    [0, 0],
                    id,
                    Instant::now() + Duration::from_secs(10),
                    false
                )
                .unwrap());
            }
            assert!(events(&f).is_empty());
        }
    }
}
#[test]
fn wheel_tab_and_close_release_scrollbar_before_the_next_input_owner() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for case in 0..3 {
        let (_f, w) = setup();
        send(&w, WM_LBUTTONDOWN, [60, 50]);
        match case {
            0 => {
                assert!(scroll::wheel(w.hwnd, &w.state, [8, 8], 1, 10., false).unwrap());
            }
            1 => keyboard::navigate(w.hwnd, &w.state, false),
            _ => unsafe {
                SendMessageW(w.hwnd, WM_CLOSE, None, None);
            },
        }
        assert!(w.state.scrollbar.borrow().gesture_is_idle());
        assert_ne!(unsafe { GetCapture() }, w.hwnd);
    }
}

unsafe extern "system" fn successor_during_paint(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    if msg == WM_PAINT {
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
        let result = procedure(hwnd, msg, wp, lp);
        procedure(hwnd, WM_CANCELMODE, WPARAM(0), LPARAM(0));
        procedure(hwnd, WM_LBUTTONDOWN, WPARAM(0), LPARAM(60 | (50 << 16)));
        result
    } else {
        procedure(hwnd, msg, wp, lp)
    }
}
#[test]
fn synchronous_paint_cancel_and_successor_cannot_restore_old_thumb_or_clear_new_page_owner() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (_f, w) = setup();
    send(&w, WM_LBUTTONDOWN, [60, 5]);
    let old = w.state.scrollbar.borrow().token.clone();
    unsafe {
        SetWindowLongPtrW(
            w.hwnd,
            GWLP_WNDPROC,
            successor_during_paint as *const () as isize,
        );
    }
    send(&w, WM_MOUSEMOVE, [60, 15]);
    let id = timer(&w);
    assert!(!Rc::ptr_eq(&old, &w.state.scrollbar.borrow().token));
    assert_eq!(unsafe { GetCapture() }, w.hwnd);
    send(&w, WM_MOUSEMOVE, [60, 63]);
    assert_eq!(timer(&w), id);
    send(&w, WM_LBUTTONUP, [60, 63]);
    assert!(w.state.scrollbar.borrow().gesture_is_idle());
}
#[test]
fn synchronous_paint_destroy_retires_stack_owned_thumb_and_does_not_restore_resources() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup();
    send(&w, WM_LBUTTONDOWN, [60, 5]);
    unsafe {
        SetWindowLongPtrW(
            w.hwnd,
            GWLP_WNDPROC,
            close_during_paint as *const () as isize,
        );
    }
    send(&w, WM_MOUSEMOVE, [60, 15]);
    assert!(!w.alive());
    assert!(w.state.scrollbar.borrow().gesture_is_idle());
    assert!(w.state.renderer.borrow().is_none());
    assert_ne!(unsafe { GetCapture() }, w.hwnd);
    assert!(events(&f).is_empty());
    w.tick().unwrap();
}
unsafe extern "system" fn close_during_paint(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    if msg == WM_PAINT {
        let _ = DestroyWindow(hwnd);
        LRESULT(0)
    } else {
        procedure(hwnd, msg, wp, lp)
    }
}
