//! 自有 HWND 的同步 capture-change 重入；只观察原 FIFO，不注入全局输入。
use super::*;
use crate::surface_javascript_wire::PointerPhase;
use std::cell::Cell;
use windows::Win32::UI::{Input::KeyboardAndMouse::SetCapture, Shell::*};
struct Probe {
    target: HWND,
    fired: Cell<bool>,
}
unsafe extern "system" fn hide_on_capture_change(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let probe = &*(data as *const Probe);
    if message == WM_CAPTURECHANGED && !probe.fired.replace(true) {
        // 先标记再重入，不跨 ShowWindow 保存 mutable borrow。
        let _ = ShowWindow(probe.target, SW_HIDE);
    }
    DefSubclassProc(hwnd, message, wp, lp)
}
struct Observer {
    hwnd: HWND,
    probe: Box<Probe>,
}
impl Observer {
    unsafe fn new(hwnd: HWND, target: HWND) -> Self {
        let probe = Box::new(Probe {
            target,
            fired: Cell::new(false),
        });
        assert!(SetWindowSubclass(
            hwnd,
            Some(hide_on_capture_change),
            1,
            (&*probe as *const Probe) as usize
        )
        .as_bool());
        Self { hwnd, probe }
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        unsafe {
            let _ = RemoveWindowSubclass(self.hwnd, Some(hide_on_capture_change), 1);
        }
    }
}
#[test]
fn javascript_pointer_real_hwnd_capture_acquisition_reentry_cannot_queue_hidden_target() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let previous = SurfaceWindow::new(
        Scene::parse(br#"{"id":"old","type":"view"}"#).unwrap(),
        80,
        80,
    )
    .unwrap();
    let (window, session) = setup();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    let observer = unsafe {
        let _ = SetCapture(previous.hwnd);
        assert_eq!(GetCapture(), previous.hwnd);
        Observer::new(previous.hwnd, window.hwnd)
    };
    window.javascript_pointer_message_for_test(WM_LBUTTONDOWN, 0);
    assert!(observer.probe.fired.get());
    assert!(!unsafe { IsWindowVisible(window.hwnd) }.as_bool());
    assert_ne!(unsafe { GetCapture() }, window.hwnd);
    assert!(!window.has_pending_edit());
    drop(observer);
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    previous.close().unwrap();
}
#[test]
fn javascript_pointer_real_hwnd_release_reentry_hard_cancels_before_prepare_fallback() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (window, mut session) = setup();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    window.javascript_pointer_message_for_test(WM_LBUTTONDOWN, 0);
    assert_eq!(next(&window, &mut session), "pointerdown");
    let observer = unsafe { Observer::new(window.hwnd, window.hwnd) };
    window.javascript_pointer_message_for_test(WM_LBUTTONUP, 0);
    assert!(observer.probe.fired.get());
    assert!(!unsafe { IsWindowVisible(window.hwnd) }.as_bool());
    assert_ne!(unsafe { GetCapture() }, window.hwnd);
    // 领取入口也能兜底取消；在调用它前观察 FIFO，避免错误实现被兜底掩盖。
    assert_eq!(
        window
            .state
            .host
            .borrow()
            .as_ref()
            .unwrap()
            .javascript_pointer_front_for_test(),
        Some(PointerPhase::Pointercancel)
    );
    assert_eq!(next(&window, &mut session), "pointerdown>pointercancel");
    assert!(!window.has_pending_edit());
    drop(observer);
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
}
