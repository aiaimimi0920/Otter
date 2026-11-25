//! Synchronous painting/reentrant destruction on owned offscreen windows; no forced system-HWND reuse or user-input injection.
use super::*;
use windows::Win32::System::Com::*;
#[path = "tile_identification_window_tests.rs"]
mod identification;
struct Apartment;
impl Apartment {
    fn new() -> Self {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        }
        Self
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
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
#[test]
fn synchronous_paint_close_stops_owner_and_drops_frame_and_renderer() {
    let _com = Apartment::new();
    let (shared, frame) = crate::tile_tests::shared_frame();
    shared.publish(1, frame);
    let mut output = crate::tile_tests::output();
    output.x = -30000;
    output.y = -30000;
    let window = Window::new(shared.clone(), &output).unwrap();
    unsafe {
        // UpdateWindow on hidden windows may not dispatch paint synchronously; show the owned offscreen window first,
        // then install destruction callbacks so this test actually covers synchronous paint reentrancy within tick.
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
        assert!(IsWindowVisible(window.hwnd).as_bool());
        let prior = SetWindowLongPtrW(
            window.hwnd,
            GWLP_WNDPROC,
            close_during_paint as *const () as isize,
        );
        assert_ne!(prior, 0);
    }
    window.tick().unwrap();
    assert!(!window.alive());
    assert!(shared.lock().stopped);
    assert!(shared.lock().frame.is_none());
    assert!(shared.lock().presented.is_none());
    assert!(window.state.frame.borrow().is_none());
    assert!(window.state.renderer.borrow().is_none());
    // Later ticks must likewise not restart or recreate closed windows.
    window.tick().unwrap();
    window.hide();
    unsafe {
        assert!(!IsWindow(Some(window.hwnd)).as_bool());
    }
}

unsafe extern "system" fn change_during_paint(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    if msg == WM_PAINT {
        procedure(hwnd, WM_DPICHANGED, WPARAM(0), LPARAM(0));
    }
    procedure(hwnd, msg, wp, lp)
}

#[test]
fn display_and_dpi_change_hide_and_invalidate_without_tick_reshowing_old_generation() {
    let _com = Apartment::new();
    unsafe {
        for message in [WM_DISPLAYCHANGE, WM_DPICHANGED] {
            let (shared, frame) = crate::tile_tests::shared_frame();
            shared.publish(1, frame);
            let mut output = crate::tile_tests::output();
            (output.x, output.y) = (-30000, -30000);
            let window = Window::new(shared.clone(), &output).unwrap();
            let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
            assert!(IsWindowVisible(window.hwnd).as_bool());
            SendMessageW(window.hwnd, message, Some(WPARAM(0)), Some(LPARAM(0)));
            assert!(window.output_changed() && window.alive());
            {
                let state = shared.lock();
                assert!(state.stopped && state.frame.is_none());
            }
            window.tick().unwrap();
            assert!(!IsWindowVisible(window.hwnd).as_bool());
        }
    }
}

#[test]
fn synchronous_paint_dpi_change_cannot_restore_renderer_or_show_old_generation() {
    let _com = Apartment::new();
    let (shared, frame) = crate::tile_tests::shared_frame();
    shared.publish(1, frame);
    let mut output = crate::tile_tests::output();
    (output.x, output.y) = (-30000, -30000);
    let window = Window::new(shared.clone(), &output).unwrap();
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
        assert_ne!(
            SetWindowLongPtrW(
                window.hwnd,
                GWLP_WNDPROC,
                change_during_paint as *const () as isize
            ),
            0
        );
    }
    window.tick().unwrap();
    assert!(window.output_changed() && shared.lock().stopped);
    assert!(window.state.renderer.borrow().is_none());
    assert!(shared.lock().presented.is_none());
    window.tick().unwrap();
    unsafe {
        assert!(!IsWindowVisible(window.hwnd).as_bool());
    }
}
